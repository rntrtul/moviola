use crate::ui::preview::preview::Preview;
use crate::ui::preview::{CropMode, MARGIN};
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::glib;
use gtk4::glib::clone;
use gtk4::graphene::Rect;
use gtk4::prelude::{GestureDragExt, PaintableExt, SnapshotExt, SnapshotExtManual, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{gdk, gsk};
use gtk4::{graphene, Snapshot};

static BOX_COLOUR: gdk::RGBA = gdk::RGBA::WHITE;
static HANDLE_FILL_RULE: gsk::FillRule = gsk::FillRule::Winding;
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

        if self.handle_drag_active.get() {
            self.draw_box_grid(snapshot, &rect, 3, 3);
        }
        self.draw_box_handles(snapshot, &rect);
    }

    pub(crate) fn box_connect_gestures(&self) {
        let obj = self.obj();
        let drag_gesture = gtk4::GestureDrag::new();

        drag_gesture.connect_drag_begin(clone!(
            #[weak]
            obj,
            move |drag, x, y| {
                let target_point = graphene::Point::new(x as f32, y as f32);
                let box_rect = obj.imp().bounding_box_rect();

                let handle_centers = obj.imp().handle_centers(&box_rect);

                let mut point_on_handle = false;

                for (idx, point) in handle_centers.iter().enumerate() {
                    let path_builder = gsk::PathBuilder::new();
                    path_builder.add_circle(&point, MARGIN);
                    let circle = path_builder.to_path();

                    if circle.in_fill(&target_point, HANDLE_FILL_RULE) {
                        let handle = match idx {
                            0 => HandleType::TopLeft,
                            1 => HandleType::BottomLeft,
                            2 => HandleType::TopRight,
                            3 => HandleType::BottomRight,
                            _ => panic!("too many handle indicies"),
                        };
                        obj.imp().active_handle.set(handle);
                        point_on_handle = true;
                        break;
                    }
                }

                obj.imp().handle_drag_active.set(point_on_handle);
                if !obj.imp().handle_drag_active.get() {
                    obj.imp().active_handle.set(HandleType::None);
                }
            }
        ));

        drag_gesture.connect_drag_update(clone!(
            #[weak]
            obj,
            move |drag, x_offset, y_offset| {
                let (start_x, start_y) = drag.start_point().unwrap();

                let x = start_x + x_offset;
                let y = start_y + y_offset;
                obj.imp().update_drag_pos((x, y));
            }
        ));

        drag_gesture.connect_drag_end(clone!(
            #[weak]
            obj,
            move |_, _, _| {
                obj.imp().handle_drag_active.set(false);
                obj.queue_draw();
            }
        ));

        obj.add_controller(drag_gesture);
    }
}

impl Preview {
    fn bounding_box_rect(&self) -> Rect {
        let preview = self.preview_rect();

        let left_x = (preview.width() * self.left_x.get()) + preview.x();
        let top_y = (preview.height() * self.top_y.get()) + preview.y();

        let right_x = ((preview.width()) * self.right_x.get()) + preview.x();
        let bottom_y = ((preview.height()) * self.bottom_y.get()) + preview.y();

        Rect::new(left_x, top_y, right_x - left_x, bottom_y - top_y)
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
        let handle_center = self.handle_centers(rect);

        for center in handle_center {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&center, MARGIN);
            let handle = path_builder.to_path();
            snapshot.append_fill(&handle, HANDLE_FILL_RULE, &BOX_COLOUR);
        }
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
            self.paintable.borrow().intrinsic_aspect_ratio() as f32
        } else {
            self.crop_mode.get().value()
        };

        let crop_rect = if self.handle_drag_active.get() {
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

    fn get_cordinate_percent_from_drag(&self, x: f64, y: f64) -> (f64, f64) {
        // todo: be aware of scale factor and convert screen pixel to video pixel.
        //      probably handled in zoom with func like screen_coords_to_video
        let preview = self.preview_rect();

        let x_adj = (x - preview.x() as f64).clamp(0., preview.width() as f64);
        let y_adj = (y - preview.y() as f64).clamp(0., preview.height() as f64);

        (
            x_adj / preview.width() as f64,
            y_adj / preview.height() as f64,
        )
    }

    fn update_drag_pos(&self, target: (f64, f64)) {
        let (x_percent, y_percent) = self.get_cordinate_percent_from_drag(target.0, target.1);
        let x = x_percent as f32;
        let y = y_percent as f32;

        match self.active_handle.get() {
            HandleType::TopLeft => {
                self.left_x.set(x);
                self.top_y.set(y);
                self.maintain_aspect_ratio();
                self.obj().queue_draw();
            }
            HandleType::BottomLeft => {
                self.left_x.set(x);
                self.bottom_y.set(y);
                self.maintain_aspect_ratio();
                self.obj().queue_draw();
            }
            HandleType::TopRight => {
                self.right_x.set(x);
                self.top_y.set(y);
                self.maintain_aspect_ratio();
                self.obj().queue_draw();
            }
            HandleType::BottomRight => {
                self.right_x.set(x);
                self.bottom_y.set(y);
                self.maintain_aspect_ratio();
                self.obj().queue_draw();
            }
            HandleType::None => {}
        }
    }
}
