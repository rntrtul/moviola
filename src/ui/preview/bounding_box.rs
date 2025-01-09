use crate::geometry::{bounding_point_on_edges, Corner, CornerType, EdgeType, Rectangle};
use crate::ui::preview::input::DragType;
use crate::ui::preview::preview::Preview;
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::graphene::{Point, Rect};
use gtk4::prelude::{SnapshotExt, SnapshotExtManual, WidgetExt};
use gtk4::{gdk, gsk};
use gtk4::{graphene, Snapshot};
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
}

impl Preview {
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

    pub fn maintain_aspect_ratio(&self) {
        if self.crop_mode.get() == CropMode::Free {
            return;
        }

        let target_aspect_ratio = if self.crop_mode.get() == CropMode::Original {
            self.original_aspect_ratio.get()
        } else {
            self.crop_mode.get().value()
        };

        let crop_rect = if self.active_drag_type.get().is_active() {
            self.bounding_box_rect()
        } else {
            self.preview_rect()
        };

        let right_x = crop_rect.x() + crop_rect.width();
        let bottom_y = crop_rect.y() + crop_rect.height();

        let is_width_constrained = crop_rect.width() < (crop_rect.height() * target_aspect_ratio);

        let (new_width, new_height) = if is_width_constrained {
            let new_height = crop_rect.width() / target_aspect_ratio;
            (crop_rect.width(), new_height)
        } else {
            let new_width = crop_rect.height() * target_aspect_ratio;
            (new_width, crop_rect.height())
        };

        let preview = self.preview_rect();

        // todo: combine this and get_cordinate_percent_from_drag logic into point_in_percent_preview_relative
        let adjusted_left_x =
            (right_x - new_width - preview.x()).clamp(0., preview.width()) / preview.width();
        let adjusted_right_x =
            (crop_rect.x() + new_width - preview.x()).clamp(0., preview.width()) / preview.width();
        let adjusted_top_y =
            (bottom_y - new_height - preview.y()).clamp(0., preview.height()) / preview.height();
        let adjusted_bottom_y = (crop_rect.y() + new_height - preview.y())
            .clamp(0., preview.height())
            / preview.height();

        match self.active_handle.get() {
            HandleType::TopLeft => {
                self.left_x.set(adjusted_left_x);
                self.top_y.set(adjusted_top_y);
            }
            HandleType::TopRight => {
                self.right_x.set(adjusted_right_x);
                self.top_y.set(adjusted_top_y);
            }
            HandleType::BottomLeft => {
                self.left_x.set(adjusted_left_x);
                self.bottom_y.set(adjusted_bottom_y);
            }
            HandleType::BottomRight => {
                self.right_x.set(adjusted_right_x);
                self.bottom_y.set(adjusted_bottom_y);
            }
            HandleType::None => {
                self.right_x.set(adjusted_right_x);
                self.bottom_y.set(adjusted_bottom_y);
            }
        }
    }

    // todo: make a corner enum. Since handle type will get edges added evnetually
    fn nearest_point_on_rect_form_point(
        rect: &Rectangle,
        point: Point,
        handle_type: HandleType,
    ) -> Point {
        match handle_type {
            HandleType::TopLeft => {
                bounding_point_on_edges(rect.top_left, rect.top_right, rect.bottom_left, point)
            }
            HandleType::TopRight => {
                bounding_point_on_edges(rect.top_right, rect.top_left, rect.bottom_right, point)
            }
            HandleType::BottomLeft => {
                bounding_point_on_edges(rect.bottom_left, rect.top_left, rect.bottom_right, point)
            }
            HandleType::BottomRight => {
                bounding_point_on_edges(rect.bottom_right, rect.top_right, rect.bottom_left, point)
            }
            // should not be called
            HandleType::None => Point::zero(),
        }
    }

    pub fn nearest_point_rect(&self) -> Rectangle {
        let visible = self.visible_preview_rect();
        let rect = self.bounding_box_rect();

        let top_left =
            Self::nearest_point_on_rect_form_point(&visible, rect.top_left(), HandleType::TopLeft);
        let bottom_left = Self::nearest_point_on_rect_form_point(
            &visible,
            rect.bottom_left(),
            HandleType::BottomLeft,
        );
        let top_right = Self::nearest_point_on_rect_form_point(
            &visible,
            rect.top_right(),
            HandleType::TopRight,
        );
        let bottom_right = Self::nearest_point_on_rect_form_point(
            &visible,
            rect.bottom_right(),
            HandleType::BottomRight,
        );

        Rectangle {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    pub(crate) fn update_to_fit_in_visible_frame(&self) {
        let rect = self.bounding_box_rect();
        let visible = self.visible_preview_rect();

        let mut left = f32::NEG_INFINITY;
        let mut top = f32::NEG_INFINITY;
        let mut right = f32::INFINITY;
        let mut bottom = f32::INFINITY;

        let nearest_points = self.nearest_point_rect();

        if !visible.contains(rect.top_left()) {
            let (left_x, top_y) = self.point_as_percent(nearest_points.top_left);
            top = top_y;
            left = left_x;
        }

        if !visible.contains(rect.top_right()) {
            let (x, y) = self.point_as_percent(nearest_points.top_right);
            top = top.max(y);
            right = x;
        }

        if !visible.contains(rect.bottom_left()) {
            let (x, y) = self.point_as_percent(nearest_points.bottom_left);
            left = left.max(x);
            bottom = y;
        }

        if !visible.contains(rect.bottom_right()) {
            let (x, y) = self.point_as_percent(nearest_points.bottom_right);
            right = right.min(x);
            bottom = bottom.min(y);
        }

        if left != f32::NEG_INFINITY {
            self.left_x.set(left);
        }
        if top != f32::NEG_INFINITY {
            self.top_y.set(top);
        }
        if right != f32::INFINITY {
            self.right_x.set(right);
        }
        if bottom != f32::INFINITY {
            self.bottom_y.set(bottom);
        }
    }

    pub fn aspect_ratio_respecting_offset(&self, offset: Point) -> (f32, f32) {
        if self.crop_mode.get() == CropMode::Free {
            return (offset.x(), offset.y());
        }

        let aspect_ratio = self.crop_aspect_ratio();

        let x_sign = if offset.x() >= 0.0 { 1.0 } else { -1.0 };
        let y_sign = if offset.y() >= 0.0 { 1.0 } else { -1.0 };

        if aspect_ratio > 1.0 {
            let corrected_y = (offset.x().abs() / aspect_ratio) * y_sign;
            (offset.x(), corrected_y)
        } else {
            let corrected_x = (offset.y().abs() * aspect_ratio) * x_sign;
            (corrected_x, offset.y())
        }
    }

    fn corner_constraining_edge(&self, corner: Corner) -> EdgeType {
        if self.straighten_angle.get() <= 0.0 {
            match corner.corner_type {
                CornerType::TopLeft => EdgeType::Top,
                CornerType::TopRight => EdgeType::Right,
                CornerType::BottomLeft => EdgeType::Left,
                CornerType::BottomRight => EdgeType::Bottom,
            }
        } else {
            match corner.corner_type {
                CornerType::TopLeft => EdgeType::Left,
                CornerType::TopRight => EdgeType::Top,
                CornerType::BottomLeft => EdgeType::Bottom,
                CornerType::BottomRight => EdgeType::Right,
            }
        }
    }

    fn corner_constraints(&self, rect: &Rectangle, corner: Corner) -> (f32, f32) {
        let edge = self.corner_constraining_edge(corner);
        rect.distance_to_edge(corner.point, edge)
    }

    fn corner_drag_edge_intersection(
        &self,
        rect: &Rectangle,
        corner: Corner,
        target: Point,
    ) -> Option<Point> {
        let edge = self.corner_constraining_edge(corner);
        rect.line_intersection_to_edge(corner.point, target, edge)
    }

    fn corner_x_y_allowance(&self, rect: &Rectangle, corner: Corner) -> (f32, f32) {
        let (vertical_edge, horizontal_edge) = match corner.corner_type {
            CornerType::TopLeft => (EdgeType::Left, EdgeType::Top),
            CornerType::TopRight => (EdgeType::Right, EdgeType::Top),
            CornerType::BottomLeft => (EdgeType::Left, EdgeType::Bottom),
            CornerType::BottomRight => (EdgeType::Right, EdgeType::Bottom),
        };

        let (x_1, y_1) = rect.distance_to_edge(corner.point, vertical_edge);
        let (x_2, y_2) = rect.distance_to_edge(corner.point, horizontal_edge);

        let binding_x = if x_1.abs() > x_2.abs() { x_2 } else { x_1 };
        let binding_y = if y_1.abs() > y_2.abs() { y_2 } else { y_1 };

        (binding_x, binding_y)
    }

    fn shrink_direction_from_corner(corner: Corner) -> (f32, f32) {
        match corner.corner_type {
            CornerType::TopLeft => (1.0, 1.0),
            CornerType::TopRight => (-1.0, 1.0),
            CornerType::BottomLeft => (1.0, -1.0),
            CornerType::BottomRight => (-1.0, -1.0),
        }
    }

    fn is_same_sign(a: f32, b: f32) -> bool {
        (a == 0.0) || b == 0.0 || ((a >= 0.0) == (b >= 0.0))
    }

    fn contain_offset_to_visible(&self, offset: Point) -> (f32, f32) {
        let visible = self.visible_preview_rect();
        let bound = self.bounding_box_rect();

        let top_left = Corner::new(bound.top_left(), CornerType::TopLeft);
        let top_right = Corner::new(bound.top_right(), CornerType::TopRight);
        let bottom_left = Corner::new(bound.bottom_left(), CornerType::BottomLeft);
        let bottom_right = Corner::new(bound.bottom_right(), CornerType::BottomRight);

        let active_corner = match self.active_handle.get() {
            HandleType::TopLeft => top_left,
            HandleType::BottomLeft => bottom_left,
            HandleType::TopRight => top_right,
            HandleType::BottomRight => bottom_right,
            HandleType::None => {
                panic!("Cant make Corner");
            }
        };
        let (shrink_direction_x, shrink_direction_y) =
            Self::shrink_direction_from_corner(active_corner);

        if Self::is_same_sign(shrink_direction_x, offset.x())
            && Self::is_same_sign(shrink_direction_y, offset.y())
        {
            return (offset.x(), offset.y()); // Box is being shrunk, so can't hit borders
        }

        let target = Point::new(
            active_corner.point.x() + offset.x(),
            active_corner.point.y() + offset.y(),
        );

        if visible.is_point_on_boundary(active_corner.point) && !visible.contains(target) {
            return (0.0, 0.0); // trying to move out of rect, but at border for active corner
        }

        let vertically_adjacent_corner = match active_corner.corner_type {
            CornerType::TopLeft => bottom_left,
            CornerType::TopRight => bottom_right,
            CornerType::BottomLeft => top_left,
            CornerType::BottomRight => top_right,
        };

        let horizontally_adjacent_corner = match active_corner.corner_type {
            CornerType::TopLeft => top_right,
            CornerType::TopRight => top_left,
            CornerType::BottomLeft => bottom_right,
            CornerType::BottomRight => bottom_left,
        };

        let x_constraint = self
            .corner_constraints(&visible, vertically_adjacent_corner)
            .0;
        let y_constraint = self
            .corner_constraints(&visible, horizontally_adjacent_corner)
            .1;
        let active_constraints = self.corner_x_y_allowance(&visible, active_corner);

        let x_bounds = [x_constraint, active_constraints.0]
            .into_iter()
            .filter(|x| x.is_finite() && Self::is_same_sign(*x, offset.x()))
            .min_by(|a, b| a.abs().total_cmp(&b.abs()));

        let y_bounds = [y_constraint, active_constraints.1]
            .into_iter()
            .filter(|y| y.is_finite() && Self::is_same_sign(*y, offset.y()))
            .min_by(|a, b| a.abs().total_cmp(&b.abs()));

        let x_sign = if offset.x() >= 0.0 { 1.0 } else { -1.0 };
        let y_sign = if offset.y() >= 0.0 { 1.0 } else { -1.0 };

        let aspect_ratio = if self.crop_mode.get() == CropMode::Free {
            offset.x() / offset.y()
        } else {
            self.crop_aspect_ratio()
        };

        let mut constraints: Vec<(f32, f32)> = Vec::with_capacity(4);

        // only worry about bounding if moving in that direction. Also avoids cases in free crop mode
        // where the aspect ratio could be 0.0 or inf and mess up the bounds
        if x_bounds.is_some() && offset.x() != 0.0 {
            let x = x_bounds.unwrap();
            let y = (x.abs() / aspect_ratio) * y_sign;
            constraints.push((x, y));
        }

        if y_bounds.is_some() && offset.y() != 0.0 {
            let y = y_bounds.unwrap();
            let x = (y.abs() * aspect_ratio) * x_sign;
            constraints.push((x, y));
        }

        let active_boundary_constraint =
            self.corner_drag_edge_intersection(&visible, active_corner, target);

        if let Some(boundary_constraint) = active_boundary_constraint {
            if visible.is_point_on_boundary(boundary_constraint) {
                let x = boundary_constraint.x() - active_corner.point.x();
                let y = boundary_constraint.y() - active_corner.point.y();

                constraints.push((x, y));
            }
        }

        constraints.push((offset.x(), offset.y()));

        let (x, y) = *constraints
            .iter()
            .min_by(|a, b| (a.0.powi(2) + a.1.powi(2)).total_cmp(&(b.0.powi(2) + b.1.powi(2))))
            .unwrap();

        (x, y)
    }

    pub(crate) fn update_handle_pos(&self, offset_coords: Point) {
        let (x_offset, y_offset) = self.aspect_ratio_respecting_offset(offset_coords);
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

    pub(crate) fn translate_box(&self, offset_x: f32, offset_y: f32) {
        if offset_x == 0.0 && offset_y == 0.0 {
            return;
        }

        // fixme: stop using 0,1 and use distance to visible rect on x and y from point
        //  function similair to corner_x_yallowance. except for edges. Will test each edge with
        //  2 corners and choose min. (eg. Top will be abs min of topleft and topright distance to edge)

        // The edges are clamped from [0,1]. When the clamp activates it means the clamped edge will
        // move a different amount compared to the trailing edge.
        // To ensure they move same amount we check if the current offset will cause clamping and if
        // it will, we take the max value that will not cause clamping.
        let step_x = if offset_x < 0.0 && -offset_x > self.left_x.get() {
            -self.left_x.get()
        } else if offset_x > 0.0 && (1.0 - self.right_x.get()) < offset_x {
            1.0 - self.right_x.get()
        } else {
            offset_x
        };

        let step_y = if offset_y < 0.0 && -offset_y > self.top_y.get() {
            -self.top_y.get()
        } else if offset_y > 0.0 && (1.0 - self.bottom_y.get()) < offset_y {
            1.0 - self.bottom_y.get()
        } else {
            offset_y
        };

        // only translate if the leading edge can move
        if (self.left_x.get() > 0.0 && step_x < 0.0) || (self.right_x.get() < 1.0 && step_x > 0.0) {
            self.left_x
                .set((self.left_x.get() + step_x).clamp(0.0, self.right_x.get()));
            self.right_x
                .set((self.right_x.get() + step_x).clamp(self.left_x.get(), 1.0));
        }

        if (self.bottom_y.get() < 1.0 && step_y > 0.0) || (self.top_y.get() > 0.0 && step_y < 0.0) {
            self.bottom_y
                .set((self.bottom_y.get() + step_y).clamp(self.top_y.get(), 1.0));
            self.top_y
                .set((self.top_y.get() + step_y).clamp(0.0, self.bottom_y.get()))
        }
    }
}
