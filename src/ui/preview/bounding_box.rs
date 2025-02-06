use crate::geometry;
use crate::geometry::{point_distance, Corner, CornerType, Rectangle};
use crate::ui::preview::input::DragType;
use crate::ui::preview::preview::Preview;
use gst::subclass::prelude::ObjectSubclassExt;
use relm4::gtk::graphene::{Point, Rect};
use relm4::gtk::prelude::{SnapshotExt, SnapshotExtManual, WidgetExt};
use relm4::gtk::{gdk, gsk};
use relm4::gtk::{graphene, Snapshot};
use std::cmp::PartialEq;

pub(crate) static BOX_HANDLE_WIDTH: f32 = 3f32;
static BOX_HANDLE_HEIGHT: f32 = 30f32;
static BOX_COLOUR: gdk::RGBA = gdk::RGBA::WHITE;
static HANDLE_FILL_RULE: gsk::FillRule = gsk::FillRule::Winding;
static DIRECTIONS: [(f32, f32); 4] = [(1f32, 1f32), (1f32, -1f32), (-1f32, 1f32), (-1f32, -1f32)];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CropMode {
    Free,
    Original,
    Square,
    _16To9,
    _4To5,
    _5To7,
    _4To3,
    _3To5,
    _3To2,
}

impl CropMode {
    pub(crate) fn value(&self) -> f32 {
        match *self {
            CropMode::Free => 0.,
            CropMode::Original => 0.,
            CropMode::Square => 1.,
            CropMode::_16To9 => 16. / 9.,
            CropMode::_4To3 => 4. / 3.,
            CropMode::_3To2 => 2. / 3.,
            CropMode::_4To5 => 4. / 5.,
            CropMode::_5To7 => 5. / 7.,
            CropMode::_3To5 => 3. / 5.,
        }
    }
}

pub struct BoundingBoxDimensions {
    pub(crate) left_x: f32,
    pub(crate) top_y: f32,
    pub(crate) right_x: f32,
    pub(crate) bottom_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum HandleType {
    None,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

struct RectangleConstraints {
    top: Option<f32>,
    left: Option<f32>,
    bottom: Option<f32>,
    right: Option<f32>,
}

impl Preview {
    pub(crate) fn draw_bounding_box(&self, snapshot: &Snapshot) {
        let rect = self.bounding_box_rect();

        let border = gsk::RoundedRect::from_rect(rect, 0.);
        let border_widths = [1.; 4];
        let border_colours = [BOX_COLOUR; 4];

        snapshot.append_border(&border, &border_widths, &border_colours);

        if self.active_drag_type.get().is_active() {
            let grid_size = match self.active_drag_type.get() {
                DragType::Handle => 3,
                DragType::Straighten => 10,
                _ => 0,
            };
            self.draw_box_grid(snapshot, &rect, grid_size, grid_size);
        }
        self.draw_box_handles(snapshot, &rect);
    }

    pub(crate) fn box_handle_drag_begin(&self, x: f32, y: f32) {
        let target_point = graphene::Point::new(x, y);
        let box_rect = self.bounding_box_rect();

        let handle_paths = self.box_handle_paths(&box_rect);

        self.active_drag_type.set(DragType::None);
        self.active_handle.set(HandleType::None);

        for (idx, handle_path) in handle_paths.iter().enumerate() {
            if handle_path.in_fill(&target_point, HANDLE_FILL_RULE) {
                let handle = match idx {
                    0 => HandleType::TopLeft,
                    1 => HandleType::BottomLeft,
                    2 => HandleType::TopRight,
                    3 => HandleType::BottomRight,
                    _ => panic!("too many handle indicies"),
                };
                self.active_handle.set(handle);
                self.active_drag_type.set(DragType::Handle);
                break;
            }
        }

        if self.active_drag_type.get().is_none() && box_rect.contains_point(&target_point) {
            self.active_drag_type.set(DragType::BoxTranslate);
        }
    }

    pub(crate) fn bounding_box_rect(&self) -> Rect {
        let preview = self.preview_rect();
        let width = preview.width() * (self.right_x.get() - self.left_x.get());
        let height = preview.height() * (self.bottom_y.get() - self.top_y.get());
        let (x, y) = self.centered_start(width, height);

        Rect::new(x, y, width, height)
    }

    fn draw_box_grid(&self, snapshot: &Snapshot, rect: &Rect, rows: u32, columns: u32) {
        let stroke = gsk::Stroke::builder(1.).build();

        let horizontal_step_size = rect.width() / (columns as f32);
        let vertical_step_size = rect.height() / (rows as f32);

        let end_x = rect.x() + rect.width();
        for step in 1..rows {
            let y = rect.y() + (step as f32 * vertical_step_size);

            let path_builder = gsk::PathBuilder::new();
            path_builder.move_to(rect.x(), y);
            path_builder.line_to(end_x, y);

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &stroke, &BOX_COLOUR);
        }

        let end_y = rect.y() + rect.height();
        for step in 1..columns {
            let x = rect.x() + (step as f32 * horizontal_step_size);

            let path_builder = gsk::PathBuilder::new();
            path_builder.move_to(x, rect.y());
            path_builder.line_to(x, end_y);

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &stroke, &BOX_COLOUR);
        }
    }

    fn draw_box_handles(&self, snapshot: &Snapshot, rect: &Rect) {
        let paths = self.box_handle_paths(rect);
        let stroke = gsk::Stroke::builder(BOX_HANDLE_WIDTH).build();

        paths.into_iter().for_each(|handle_path| {
            snapshot.append_stroke(&handle_path, &stroke, &BOX_COLOUR);
        });
    }

    fn box_handle_paths(&self, rect: &Rect) -> Vec<gsk::Path> {
        let handle_center = self.handle_centers(rect);

        let mut paths = Vec::with_capacity(4);

        for (center, direction) in handle_center.iter().zip(DIRECTIONS.iter()) {
            let path_builder = gsk::PathBuilder::new();
            let x = center.x() - (BOX_HANDLE_WIDTH * direction.0);
            let y = center.y() - (BOX_HANDLE_WIDTH * direction.1);

            path_builder.add_rect(&Rect::new(
                x,
                y,
                BOX_HANDLE_WIDTH * direction.0,
                BOX_HANDLE_HEIGHT * direction.1,
            ));
            path_builder.add_rect(&Rect::new(
                x,
                y,
                BOX_HANDLE_HEIGHT * direction.0,
                BOX_HANDLE_WIDTH * direction.1,
            ));

            let handle = path_builder.to_path();
            paths.push(handle);
        }

        paths
    }

    fn handle_centers(&self, rect: &Rect) -> [graphene::Point; 4] {
        [
            graphene::Point::new(rect.x(), rect.y()),
            graphene::Point::new(rect.x(), rect.y() + rect.height()),
            graphene::Point::new(rect.x() + rect.width(), rect.y()),
            graphene::Point::new(rect.x() + rect.width(), rect.y() + rect.height()),
        ]
    }

    pub fn shrink_box_to_new_aspect_ratio(&self) {
        if self.crop_mode.get() == CropMode::Free {
            return;
        }

        let target_aspect_ratio = self.crop_aspect_ratio();
        let rect = self.bounding_box_rect();

        let is_width_constrained = rect.width() < (rect.height() * target_aspect_ratio);
        let (new_width, new_height) = if is_width_constrained {
            let new_height = rect.width() / target_aspect_ratio;
            (rect.width(), new_height)
        } else {
            let new_width = rect.height() * target_aspect_ratio;
            (new_width, rect.height())
        };

        let (new_left, new_top) = self.centered_start(new_width, new_height);

        let (width, height) = self.size_as_percent(new_width, new_height);
        let (left, top) = self.size_as_percent(new_left, new_top);

        if is_width_constrained {
            self.top_y.set(top);
            self.bottom_y.set(top + height);
        } else {
            self.left_x.set(left);
            self.right_x.set(left + width);
        }
    }

    pub(crate) fn constrain_crop_box_to_visible_preview(&self) {
        let crop_box = self.bounding_box_rect();
        let preview = self.visible_preview_rect();

        let mut left = None;
        let mut top = None;
        let mut right = None;
        let mut bottom = None;

        let nearest_points = nearest_point_rect(&preview, &crop_box);

        if !preview.contains(crop_box.top_left()) {
            let (x, y) = (nearest_points.top_left.x(), nearest_points.top_left.y());
            top = Some(y);
            left = Some(x);
        }

        if !preview.contains(crop_box.top_right()) {
            let (x, y) = (nearest_points.top_right.x(), nearest_points.top_right.y());
            top = Some(top.map_or(y, |t| t.max(y)));
            right = Some(x);
        }

        if !preview.contains(crop_box.bottom_left()) {
            let (x, y) = (
                nearest_points.bottom_left.x(),
                nearest_points.bottom_left.y(),
            );
            left = Some(left.map_or(x, |l| l.max(x)));
            bottom = Some(y);
        }

        if !preview.contains(crop_box.bottom_right()) {
            let (x, y) = (
                nearest_points.bottom_right.x(),
                nearest_points.bottom_right.y(),
            );
            right = Some(right.map_or(x, |r| r.min(x)));
            bottom = Some(bottom.map_or(y, |b| b.min(y)));
        }

        if left.is_none() && top.is_none() && right.is_none() && bottom.is_none() {
            return;
        }

        let mut left_offset = left.map_or(0.0, |l| l - crop_box.x());
        let mut right_offset = right.map_or(0.0, |r| r - (crop_box.x() + crop_box.width()));
        let mut top_offset = top.map_or(0.0, |t| t - crop_box.y());
        let mut bottom_offset = bottom.map_or(0.0, |b| b - (crop_box.y() + crop_box.height()));

        let total_x_offset = left_offset - right_offset;
        let total_y_offset = top_offset - bottom_offset;
        let aspect_ratio = (total_x_offset / total_y_offset).abs();

        let target_aspect_ratio = self.crop_aspect_ratio();

        if self.crop_mode.get() != CropMode::Free && target_aspect_ratio != aspect_ratio {
            if aspect_ratio > target_aspect_ratio {
                let target_y_offset = total_y_offset / target_aspect_ratio;
                if target_y_offset.is_sign_negative() {
                    bottom_offset = -(target_y_offset.abs() - top_offset);
                } else {
                    top_offset = target_y_offset.abs() - bottom_offset.abs();
                }
            } else {
                let target_x_offset = total_y_offset * target_aspect_ratio;
                if target_x_offset.is_sign_negative() {
                    right_offset = -(target_x_offset.abs() - left_offset);
                } else {
                    left_offset = target_x_offset.abs() - right_offset.abs();
                }
            }
        }

        self.left_x
            .set(self.left_x.get() + self.x_as_percent(left_offset));
        self.right_x
            .set(self.right_x.get() + self.x_as_percent(right_offset));
        self.top_y
            .set(self.top_y.get() + self.y_as_percent(top_offset));
        self.bottom_y
            .set(self.bottom_y.get() + self.y_as_percent(bottom_offset));
    }

    pub fn aspect_ratio_respecting_offset(&self, offset: Point) -> (f32, f32) {
        if self.crop_mode.get() == CropMode::Free {
            return (offset.x(), offset.y());
        }

        let aspect_ratio = self.crop_aspect_ratio();
        adjust_offset_to_aspect_ratio(aspect_ratio, offset)
    }

    fn contain_offset_to_visible(&self, offset: Point) -> (f32, f32) {
        let visible = self.visible_preview_rect();
        let bound = self.bounding_box_rect();

        let (top_left, top_right, bottom_left, bottom_right) = rect_as_corners(&bound);

        let active_corner = match self.active_handle.get() {
            HandleType::TopLeft => top_left,
            HandleType::BottomLeft => bottom_left,
            HandleType::TopRight => top_right,
            HandleType::BottomRight => bottom_right,
            HandleType::None => {
                panic!("Cant make Corner");
            }
        };
        let (shrink_direction_x, shrink_direction_y) = shrink_direction_from_corner(active_corner);

        if is_same_sign(shrink_direction_x, offset.x())
            && is_same_sign(shrink_direction_y, offset.y())
        {
            return (offset.x(), offset.y()); // Box is being shrunk, so can't hit borders
        }

        let target = geometry::point_add(&active_corner.point, &offset);

        if visible.is_point_on_boundary(active_corner.point) && !visible.contains(target) {
            return (0.0, 0.0); // trying to move out of rect, but at border for active corner
        }

        let vertically_adjacent_corner =
            corner_of_rect(&bound, active_corner.corner_type.vertically_adjacent());

        let horizontally_adjacent_corner =
            corner_of_rect(&bound, active_corner.corner_type.horizontally_adjacent());

        let aspect_ratio = if self.crop_mode.get() == CropMode::Free {
            offset.x() / offset.y()
        } else {
            self.crop_aspect_ratio()
        };

        let (x, y) = corner_x_y_bounds(
            &visible,
            active_corner,
            vertically_adjacent_corner,
            horizontally_adjacent_corner,
            offset,
            aspect_ratio,
        );

        (x, y)
    }

    pub(crate) fn update_handle_pos(&self, offset: Point) {
        let (x_offset, y_offset) = self.aspect_ratio_respecting_offset(offset);
        let (x_offset, y_offset) = self.contain_offset_to_visible(Point::new(x_offset, y_offset));

        let x_offset_percent = self.x_as_percent(x_offset);
        let y_offset_percent = self.y_as_percent(y_offset);

        let left_x = (self.left_x.get() + x_offset_percent).min(self.right_x.get());
        let top_y = (self.top_y.get() + y_offset_percent).min(self.bottom_y.get());
        let right_x = (self.right_x.get() + x_offset_percent).max(self.left_x.get());
        let bottom_y = (self.bottom_y.get() + y_offset_percent).max(self.top_y.get());

        match self.active_handle.get() {
            HandleType::TopLeft => {
                self.left_x.set(left_x);
                self.top_y.set(top_y);
            }
            HandleType::BottomLeft => {
                self.left_x.set(left_x);
                self.bottom_y.set(bottom_y);
            }
            HandleType::TopRight => {
                self.right_x.set(right_x);
                self.top_y.set(top_y);
            }
            HandleType::BottomRight => {
                self.right_x.set(right_x);
                self.bottom_y.set(bottom_y);
            }
            HandleType::None => {
                panic!("should not be trying to update handle position when no handle selected");
            }
        }

        self.obj().queue_draw();
    }

    pub(crate) fn translate_box(&self, offset: Point) {
        if offset.x() == 0.0 && offset.y() == 0.0 {
            return;
        }

        let visible = self.visible_preview_rect();
        let bound = self.bounding_box_rect();
        let constraints = constrained_rect_bounds(&visible, &bound);

        // To ensure they move same amount we check if the current offset will cause clamping and if
        // it will, we take the max value that will not cause clamping.
        let step_x = if offset.x() < 0.0 {
            one_sided_clamp(constraints.left, offset.x())
        } else if offset.x() > 0.0 {
            one_sided_clamp(constraints.right, offset.x())
        } else {
            None
        };

        let step_y = if offset.y() < 0.0 {
            one_sided_clamp(constraints.top, offset.y())
        } else if offset.y() > 0.0 {
            one_sided_clamp(constraints.bottom, offset.y())
        } else {
            None
        };

        if let Some(step_x) = step_x {
            let x_percent = self.x_as_percent(step_x);
            self.left_x
                .set((self.left_x.get() + x_percent).min(self.right_x.get()));
            self.right_x
                .set((self.right_x.get() + x_percent).max(self.left_x.get()));
        }

        if let Some(step_y) = step_y {
            let y_percent = self.y_as_percent(step_y);

            self.top_y
                .set((self.top_y.get() + y_percent).min(self.bottom_y.get()));
            self.bottom_y
                .set((self.bottom_y.get() + y_percent).max(self.top_y.get()));
        }
    }
}

fn rect_as_corners(rect: &graphene::Rect) -> (Corner, Corner, Corner, Corner) {
    let top_left = Corner::new(rect.top_left(), CornerType::TopLeft);
    let top_right = Corner::new(rect.top_right(), CornerType::TopRight);
    let bottom_left = Corner::new(rect.bottom_left(), CornerType::BottomLeft);
    let bottom_right = Corner::new(rect.bottom_right(), CornerType::BottomRight);

    (top_left, top_right, bottom_left, bottom_right)
}

fn corner_of_rect(rect: &graphene::Rect, corner_type: CornerType) -> Corner {
    let point = match corner_type {
        CornerType::TopLeft => rect.top_left(),
        CornerType::TopRight => rect.top_right(),
        CornerType::BottomLeft => rect.bottom_left(),
        CornerType::BottomRight => rect.bottom_right(),
    };

    Corner::new(point, corner_type)
}

fn min_by_abs(a: f32, b: f32) -> f32 {
    if a.abs() > b.abs() {
        b
    } else {
        a
    }
}

fn is_same_sign(a: f32, b: f32) -> bool {
    (a == 0.0) || b == 0.0 || ((a >= 0.0) == (b >= 0.0))
}

fn finite_abs_min(arr: &[f32]) -> Option<f32> {
    finite_abs_min_same_direction(arr, 0.0)
}

fn finite_abs_min_same_direction(arr: &[f32], direction: f32) -> Option<f32> {
    let min = arr
        .into_iter()
        .filter(|num| num.is_finite() && is_same_sign(**num, direction))
        .min_by(|a, b| a.abs().total_cmp(&b.abs()));

    if min.is_some() {
        Some(*min.unwrap())
    } else {
        None
    }
}

fn one_sided_clamp(bound: Option<f32>, val: f32) -> Option<f32> {
    match bound {
        Some(bound) => {
            let (min, max) = if bound > 0.0 {
                (0.0, bound)
            } else {
                (bound, 0.0)
            };
            let clamped = val.clamp(min, max);

            if clamped == 0.0 && val != 0.0 {
                None // clamped into bounds from wrong side
            } else {
                Some(clamped)
            }
        }
        None => None,
    }
}

fn shrink_direction_from_corner(corner: Corner) -> (f32, f32) {
    match corner.corner_type {
        CornerType::TopLeft => (1.0, 1.0),
        CornerType::TopRight => (-1.0, 1.0),
        CornerType::BottomLeft => (1.0, -1.0),
        CornerType::BottomRight => (-1.0, -1.0),
    }
}

fn corner_x_y_allowance(rect: &Rectangle, corner: Corner) -> (f32, f32) {
    let (vertical_edge, horizontal_edge) = corner.corner_type.edges();

    let (x_1, y_1) = rect.distance_to_edge(corner.point, vertical_edge);
    let (x_2, y_2) = rect.distance_to_edge(corner.point, horizontal_edge);

    let binding_x = min_by_abs(x_1, x_2);
    let binding_y = min_by_abs(y_1, y_2);

    (binding_x, binding_y)
}

fn adjust_offset_to_aspect_ratio(aspect_ratio: f32, offset: Point) -> (f32, f32) {
    if aspect_ratio > 1.0 {
        if offset.x().abs() == 0.0 {
            (offset.y() * aspect_ratio, offset.y())
        } else {
            (offset.x(), offset.x() / aspect_ratio)
        }
    } else {
        if offset.y() == 0.0 {
            (offset.x(), offset.x() / aspect_ratio)
        } else {
            (offset.y() * aspect_ratio, offset.y())
        }
    }
}

fn corner_constraints(rect: &Rectangle, corner: Corner) -> (f32, f32) {
    let edge = rect.corner_constraining_edge(corner.corner_type);
    rect.distance_to_edge(corner.point, edge)
}

fn corner_drag_edge_intersection(rect: &Rectangle, corner: Corner, target: Point) -> Option<Point> {
    let edge = rect.corner_constraining_edge(corner.corner_type);
    rect.line_intersection_to_edge(corner.point, target, edge)
}

fn min_by_distance(arr: &[(f32, f32)]) -> Option<(f32, f32)> {
    let min = arr.iter().min_by(|a, b| {
        let a_dist = a.0.powi(2) + a.1.powi(2);
        let b_dist = b.0.powi(2) + b.1.powi(2);
        a_dist.total_cmp(&b_dist)
    });

    if min.is_some() {
        Some(*min.unwrap())
    } else {
        None
    }
}

fn corner_x_y_bounds(
    bounding_rect: &Rectangle,
    active: Corner,
    vertically_adjacent: Corner,
    horizontally_adjacent: Corner,
    offset: Point,
    aspect_ratio: f32,
) -> (f32, f32) {
    let x_constraint = corner_constraints(&bounding_rect, vertically_adjacent).0;
    let y_constraint = corner_constraints(&bounding_rect, horizontally_adjacent).1;
    let active_constraints = corner_x_y_allowance(&bounding_rect, active);

    let x_bounds = finite_abs_min_same_direction(&[x_constraint, active_constraints.0], offset.x());
    let y_bounds = finite_abs_min_same_direction(&[y_constraint, active_constraints.1], offset.y());

    let mut constraints: Vec<(f32, f32)> = Vec::with_capacity(4);

    // only worry about bounding if moving in that direction. Also avoids cases in free crop mode
    // where the aspect ratio could be 0.0 or inf and mess up the bounds
    if x_bounds.is_some() && offset.x() != 0.0 {
        let x = x_bounds.unwrap();
        let y = (x.abs() / aspect_ratio).copysign(offset.y());
        constraints.push((x, y));
    }

    if y_bounds.is_some() && offset.y() != 0.0 {
        let y = y_bounds.unwrap();
        let x = (y.abs() * aspect_ratio).copysign(offset.x());
        constraints.push((x, y));
    }

    let target = geometry::point_add(&active.point, &offset);
    let active_boundary_constraint = corner_drag_edge_intersection(&bounding_rect, active, target);

    if let Some(boundary_constraint) = active_boundary_constraint {
        if bounding_rect.is_point_on_boundary(boundary_constraint) {
            let x = boundary_constraint.x() - active.point.x();
            let y = boundary_constraint.y() - active.point.y();

            constraints.push((x, y));
        }
    }

    constraints.push((offset.x(), offset.y()));

    min_by_distance(&constraints).unwrap()
}

fn constrained_rect_bounds(
    outer_rect: &Rectangle,
    inner_rect: &graphene::Rect,
) -> RectangleConstraints {
    let (top_left, top_right, bottom_left, bottom_right) = rect_as_corners(&inner_rect);

    let (left_1, top_1) = corner_constraints(&outer_rect, top_left);
    let (right_1, top_2) = corner_constraints(&outer_rect, top_right);
    let (left_2, bottom_1) = corner_constraints(&outer_rect, bottom_left);
    let (right_2, bottom_2) = corner_constraints(&outer_rect, bottom_right);

    RectangleConstraints {
        top: finite_abs_min(&[top_1, top_2]),
        bottom: finite_abs_min(&[bottom_1, bottom_2]),
        left: finite_abs_min(&[left_1, left_2]),
        right: finite_abs_min(&[right_1, right_2]),
    }
}

fn nearest_point_on_rect_form_point(
    rect: &Rectangle,
    point: Point,
    corner_type: CornerType,
) -> Point {
    let (vertical_edge, horizontal_edge) = corner_type.edges();
    let vertical_point = rect.closest_point_on_edge(vertical_edge, point);
    let horizontal_point = rect.closest_point_on_edge(horizontal_edge, point);

    if point_distance(vertical_point, point) > point_distance(horizontal_point, point) {
        horizontal_point
    } else {
        vertical_point
    }
}

fn nearest_point_rect(visible: &Rectangle, rect: &graphene::Rect) -> Rectangle {
    let top_left = nearest_point_on_rect_form_point(&visible, rect.top_left(), CornerType::TopLeft);
    let bottom_left =
        nearest_point_on_rect_form_point(&visible, rect.bottom_left(), CornerType::BottomLeft);
    let top_right =
        nearest_point_on_rect_form_point(&visible, rect.top_right(), CornerType::TopRight);
    let bottom_right =
        nearest_point_on_rect_form_point(&visible, rect.bottom_right(), CornerType::BottomRight);

    Rectangle {
        top_left,
        top_right,
        bottom_left,
        bottom_right,
        angle: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{assert_relative_eq, relative_eq};

    #[test]
    fn offset_aspect_ratio_does_not_change_zero() {
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::zero()),
            (0.0, 0.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::zero()),
            (0.0, 0.0)
        );
    }

    #[test]
    fn offset_aspect_ratio_adjust_correct_axis() {
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::new(1.0, 1.0)),
            (1.0, 0.5)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::new(1.0, 1.0)),
            (0.5, 1.0)
        );
    }

    #[test]
    fn offset_aspect_ratio_maintain_direction() {
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::new(-10.0, 1.0)),
            (-10.0, -5.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::new(8.0, -2.0)),
            (8.0, 4.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::new(10.0, -16.0)),
            (-8.0, -16.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::new(-100.0, 4.0)),
            (2.0, 4.0)
        );
    }

    #[test]
    fn offset_aspect_ratio_1_axis_zero() {
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::new(1.0, 0.0)),
            (1.0, 0.5)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::new(0.0, 1.0)),
            (2.0, 1.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::new(1.0, 0.0)),
            (1.0, 2.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::new(-0.0, 1.0)),
            (0.5, 1.0)
        );
    }

    #[test]
    fn offset_aspect_ratio_1_axis_zero_override_direction() {
        assert_eq!(
            adjust_offset_to_aspect_ratio(2.0, Point::new(0.0, -1.0)),
            (-2.0, -1.0)
        );
        assert_eq!(
            adjust_offset_to_aspect_ratio(0.5, Point::new(1.0, -0.0)),
            (1.0, 2.0)
        );
    }

    #[test]
    fn test_min_by_distance() {
        assert_eq!(min_by_distance(&[]), None);
        assert_eq!(
            min_by_distance(&[(0.0, 0.0), (0.0, 0.0)]).unwrap(),
            (0.0, 0.0)
        );

        assert_eq!(
            min_by_distance(&[(0.1, 0.1), (0.0, 0.0)]).unwrap(),
            (0.0, 0.0)
        );
        assert_eq!(
            min_by_distance(&[(0.1, 0.2), (-0.1, -0.1)]).unwrap(),
            (-0.1, -0.1)
        );

        assert_eq!(
            min_by_distance(&[(0.0, 1.0), (0.0, 0.5), (0.0, 4.0)]).unwrap(),
            (0.0, 0.5)
        );
        assert_eq!(
            min_by_distance(&[(0.0, -1.0), (0.0, -0.5), (0.0, -4.0)]).unwrap(),
            (0.0, -0.5)
        );
    }

    // todo: put in some common folder for line and here
    fn tuple_relatively_same(a: (f32, f32), b: (f32, f32), threshold: f32) -> bool {
        relative_eq!(a.0, b.0, epsilon = threshold) && relative_eq!(a.1, b.1, epsilon = threshold)
    }

    #[test]
    fn corner_allowances() {
        // only testing with top left since using other corners only tests the corner enums edges match
        // function and the rectangles distance to edge. corner_x_y_allowance is choosing between 4
        // possible values.
        // at 45 deg rotation edge slopes are 1, so x,y distances are the same
        let rect = Rectangle::new(Rect::new(0.0, 0.0, 60.0, 100.0), 45.0);

        // 4 bounds present. x is equidistance between left edge and top edge. so second value chosen
        assert!(tuple_relatively_same(
            corner_x_y_allowance(
                &rect,
                Corner::new(Point::new(44.14241, 0.0), CornerType::TopLeft)
            ),
            (6.56854, -6.56854),
            0.01
        ));

        // missing y allowance on top edge
        assert!(tuple_relatively_same(
            corner_x_y_allowance(
                &rect,
                Corner::new(Point::new(42.0, 0.0), CornerType::TopLeft)
            ),
            (-4.42641, -4.42641),
            0.01
        ));

        // missing x on top edge
        assert!(tuple_relatively_same(
            corner_x_y_allowance(
                &rect,
                Corner::new(Point::new(44.14241, 36.0), CornerType::TopLeft)
            ),
            (-42.56882, -42.56854),
            0.01
        ));

        // missing x on both and y on top edge
        assert!(tuple_relatively_same(
            corner_x_y_allowance(
                &rect,
                Corner::new(Point::new(0.0, 70.0), CornerType::TopLeft)
            ),
            (f32::NEG_INFINITY, -32.42641),
            0.01
        ));
    }

    #[test]
    fn test_finite_abs_min() {
        assert!(finite_abs_min_same_direction(&[-10.0, -0.5, -14.0], 1.0).is_none());
        assert!(finite_abs_min_same_direction(&[15.2, 1.5, 4.0], -1.0).is_none(),);
        assert!(finite_abs_min_same_direction(&[f32::NEG_INFINITY, f32::INFINITY], -1.0).is_none(),);

        assert_eq!(
            finite_abs_min_same_direction(&[0.0, 1.0, -0.5], 0.0).unwrap(),
            0.0
        );
        assert_eq!(
            finite_abs_min_same_direction(&[0.6, 1.0, -0.5], 0.0).unwrap(),
            -0.5
        );
        assert_eq!(
            finite_abs_min_same_direction(&[f32::NEG_INFINITY, 4.5, -6.0], -1.0).unwrap(),
            -6.0
        );
    }

    #[test]
    fn test_one_sided_clamp() {
        assert!(one_sided_clamp(None, -15.0).is_none());
        assert!(one_sided_clamp(None, 1.0).is_none());

        assert_eq!(one_sided_clamp(Some(-10.0), -15.0).unwrap(), -10.0);
        assert_eq!(one_sided_clamp(Some(-10.0), -10.0).unwrap(), -10.0);
        assert_eq!(one_sided_clamp(Some(-10.0), -6.0).unwrap(), -6.0);
        assert_eq!(one_sided_clamp(Some(-10.0), 0.0).unwrap(), 0.0);
        assert!(one_sided_clamp(Some(-10.0), 1.0).is_none());
        assert_eq!(one_sided_clamp(Some(0.0), 0.0).unwrap(), 0.0);
        assert!(one_sided_clamp(Some(0.0), -1.0).is_none());

        assert_eq!(one_sided_clamp(Some(20.0), 215.0).unwrap(), 20.0);
        assert_eq!(one_sided_clamp(Some(20.0), 20.0).unwrap(), 20.0);
        assert_eq!(one_sided_clamp(Some(20.0), 13.0).unwrap(), 13.0);
        assert_eq!(one_sided_clamp(Some(20.0), 0.0).unwrap(), 0.0);
        assert!(one_sided_clamp(Some(20.0), -0.1).is_none());
        assert_eq!(one_sided_clamp(Some(0.0), 0.0).unwrap(), 0.0);
        assert!(one_sided_clamp(Some(0.0), 1.0).is_none());
    }

    #[test]
    fn test_bound_rect_constraints() {
        let outer = Rectangle::new(Rect::new(4.0, 4.0, 36.0, 87.0), 38.0);
        let inner_rect = Rect::new(35.0, 20.0, 15.0, 20.0);
        let bounds = constrained_rect_bounds(&outer, &inner_rect);

        assert_relative_eq!(bounds.top.unwrap(), -5.82629, epsilon = 0.01);
        assert_relative_eq!(bounds.bottom.unwrap(), 0.89848, epsilon = 0.01);
        assert_relative_eq!(bounds.right.unwrap(), 0.70197, epsilon = 0.01);
        assert_relative_eq!(bounds.left.unwrap(), -14.35697, epsilon = 0.01);
    }
}
