use crate::ui::preview::input::DragType;
use crate::ui::preview::preview::Preview;
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::graphene::{Point, Rect};
use gtk4::prelude::{GestureExt, SnapshotExt, SnapshotExtManual, WidgetExt};
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
    fn value(&self) -> f32 {
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

    // fixme: move all point geometry/2d math into own file
    pub(crate) fn rotate_point(point: graphene::Point, degrees: f32) -> graphene::Point {
        let angle = degrees.to_radians();

        let rotated_x = (point.x() * angle.cos()) - (point.y() * angle.sin());
        let rotated_y = (point.y() * angle.cos()) + (point.x() * angle.sin());

        graphene::Point::new(rotated_x, rotated_y)
    }

    pub(crate) fn rotate_point_around(
        point: graphene::Point,
        origin: Point,
        degrees: f32,
    ) -> graphene::Point {
        let rotated = Self::rotate_point(
            Point::new(point.x() - origin.x(), point.y() - origin.y()),
            degrees,
        );

        Point::new(rotated.x() + origin.x(), rotated.y() + origin.y())
    }

    fn is_left_of_line(line_a: Point, line_b: Point, point: Point) -> f32 {
        (line_b.x() - line_a.x()) * (point.y() - line_a.y())
            - (point.x() - line_a.x()) * (line_b.y() - line_a.y())
    }

    pub(crate) fn point_distance(a: Point, b: Point) -> f32 {
        ((b.x() - a.x()).powi(2) + (b.y() - a.y()).powi(2)).sqrt()
    }

    pub(crate) fn closest_point_on_edge_from_point(
        line_a: Point,
        line_b: Point,
        point: Point,
    ) -> Point {
        let a = line_a.y() - line_b.y();
        let b = line_b.x() - line_a.x();
        let c = (line_a.x() * line_b.y()) - (line_b.x() * line_a.y());

        let denom = a.powi(2) + b.powi(2);

        let x = ((b * ((b * point.x()) - (a * point.y()))) - (a * c)) / denom;
        let y = ((a * ((-b * point.x()) + (a * point.y()))) - (b * c)) / denom;

        // Need to ensure the point is within line bounds
        let min_x = line_a.x().min(line_b.x());
        let max_x = line_a.x().max(line_b.x());

        let min_y = line_a.y().min(line_b.y());
        let max_y = line_a.y().max(line_b.y());

        Point::new(x.clamp(min_x, max_x), y.clamp(min_y, max_y))
    }

    pub(crate) fn is_point_in_rotated_frame(&self, point: graphene::Point) -> bool {
        let visible = self.visible_preview_rect();

        Self::is_left_of_line(visible.top_left, visible.top_right, point) >= 0.0
            && Self::is_left_of_line(visible.top_right, visible.bottom_right, point) >= 0.0
            && Self::is_left_of_line(visible.bottom_right, visible.bottom_left, point) >= 0.0
            && Self::is_left_of_line(visible.bottom_left, visible.top_left, point) >= 0.0
    }

    pub(crate) fn constrain_in_visible_frame(&self, x: f32, y: f32) -> bool {
        self.is_point_in_rotated_frame(Point::new(x, y))
    }

    pub(crate) fn bounding_point_on_edges(a: Point, b: Point, c: Point, point: Point) -> Point {
        let ab_closest = Self::closest_point_on_edge_from_point(a, b, point);
        let ac_closest = Self::closest_point_on_edge_from_point(a, c, point);

        if Self::point_distance(ab_closest, point) > Self::point_distance(ac_closest, point) {
            ac_closest
        } else {
            ab_closest
        }
    }

    pub(crate) fn update_to_fit_in_visible_frame(&self) {
        let visible = self.visible_preview_rect();
        let rect = self.bounding_box_rect();

        let mut left = -f32::INFINITY;
        let mut top = -f32::INFINITY;
        let mut right = f32::INFINITY;
        let mut bottom = f32::INFINITY;

        if !self.is_point_in_rotated_frame(rect.top_left()) {
            let bounder = Self::bounding_point_on_edges(
                visible.top_left,
                visible.top_right,
                visible.bottom_left,
                rect.top_left(),
            );

            let (left_x, top_y) = self.point_as_percent(bounder);
            left = left_x;
            top = top_y;
        }

        if !self.is_point_in_rotated_frame(rect.top_right()) {
            let bounder = Self::bounding_point_on_edges(
                visible.top_right,
                visible.top_left,
                visible.bottom_right,
                rect.top_right(),
            );

            let (x, y) = self.point_as_percent(bounder);
            right = x;
            top = top.max(y);
        }

        if !self.is_point_in_rotated_frame(rect.bottom_left()) {
            let bounder = Self::bounding_point_on_edges(
                visible.bottom_left,
                visible.top_left,
                visible.bottom_right,
                rect.bottom_left(),
            );

            let (x, y) = self.point_as_percent(bounder);
            left = left.max(x);
            bottom = y;
        }

        if !self.is_point_in_rotated_frame(rect.bottom_right()) {
            let bounder = Self::bounding_point_on_edges(
                visible.bottom_right,
                visible.top_right,
                visible.bottom_left,
                rect.bottom_right(),
            );

            let (x, y) = self.point_as_percent(bounder);
            right = right.min(x);
            bottom = bottom.min(y);
        }

        if left != -f32::INFINITY {
            self.left_x.set(left);
        }
        if top != -f32::INFINITY {
            self.top_y.set(top);
        }
        if right != f32::INFINITY {
            self.right_x.set(right);
        }
        if bottom != f32::INFINITY {
            self.bottom_y.set(bottom);
        }
    }

    pub(crate) fn update_handle_pos(&self, x_offset: f32, y_offset: f32) {
        // fixme: cleanup handling of bounds
        let left_x = (self.left_x.get() + x_offset).min(self.right_x.get());
        let top_y = (self.top_y.get() + y_offset).min(self.bottom_y.get());
        let right_x = (self.right_x.get() + x_offset).max(self.left_x.get());
        let bottom_y = (self.bottom_y.get() + y_offset).max(self.top_y.get());

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

        self.maintain_aspect_ratio();
        self.update_to_fit_in_visible_frame();
        self.obj().queue_draw();
    }

    pub(crate) fn translate_box(&self, offset_x: f32, offset_y: f32) {
        if offset_x == 0.0 && offset_y == 0.0 {
            return;
        }

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
