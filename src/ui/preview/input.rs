use crate::ui::preview::bounding_box::HandleType;
use crate::ui::preview::preview::Preview;
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::prelude::{GestureDragExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{glib, graphene};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DragType {
    Handle,
    BoxTranslate,
    Straighten,
    None,
}

impl DragType {
    pub fn is_handle(&self) -> bool {
        matches!(self, DragType::Handle)
    }

    pub fn is_box_translate(&self) -> bool {
        matches!(self, DragType::BoxTranslate)
    }

    pub fn is_straighten(&self) -> bool {
        matches!(self, DragType::Straighten)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, DragType::None)
    }

    /// All types are active except for None.
    pub fn is_active(&self) -> bool {
        !self.is_none()
    }
}

impl Preview {
    pub(crate) fn connect_gestures(&self) {
        let obj = self.obj();
        let drag_gesture = gtk4::GestureDrag::new();

        drag_gesture.connect_drag_begin(glib::clone!(
            #[weak]
            obj,
            move |_, x, y| {
                obj.imp().box_handle_drag_begin(x as f32, y as f32);
            }
        ));

        drag_gesture.connect_drag_update(glib::clone!(
            #[weak]
            obj,
            move |drag, x_offset, y_offset| {
                let preview = obj.imp();
                let (start_x, start_y) = drag.start_point().unwrap();

                let target_x = (start_x + x_offset) as f32; // graphene uses f32, so not using f64
                let target_y = (start_y + y_offset) as f32;
                let (clamped_x, clamped_y) = preview.clamp_coords_to_preview(target_x, target_y);

                let (target_x_percent, target_y_percent) =
                    preview.coords_as_percent(target_x, target_y);

                let mut prev_drag = preview.prev_drag.get();

                if prev_drag.x() == 0. && prev_drag.y() == 0. {
                    prev_drag = graphene::Point::new(clamped_x, clamped_y);
                    preview.prev_drag.set(prev_drag);
                }

                let offset_x = target_x - prev_drag.x();
                let offset_y = target_y - prev_drag.y();

                if preview.show_crop_box.get() {
                    match preview.active_drag_type.get() {
                        DragType::Handle => {
                            preview.update_handle_pos(target_x_percent, target_y_percent)
                        }
                        DragType::BoxTranslate => {
                            preview.translate_box(target_x_percent, target_y_percent)
                        }
                        _ => {}
                    }

                    obj.queue_draw();
                } else if preview.zoom.get() != 1f64 {
                    preview.pan_preview(offset_x, offset_y);
                    obj.queue_draw();
                }

                preview
                    .prev_drag
                    .set(graphene::Point::new(clamped_x, clamped_y));
            }
        ));

        drag_gesture.connect_drag_end(glib::clone!(
            #[weak]
            obj,
            move |_, _, _| {
                let preview = obj.imp();

                if preview.active_drag_type.get().is_handle() {
                    preview.is_cropped.set(
                        preview.right_x.get() != 1.0
                            || preview.left_x.get() != 0.0
                            || preview.bottom_y.get() != 1.0
                            || preview.top_y.get() != 0.0,
                    );
                    preview.active_handle.set(HandleType::None);

                    obj.queue_draw();
                }

                preview.active_drag_type.set(DragType::None);
                preview.prev_drag.set(graphene::Point::zero());
            }
        ));

        obj.add_controller(drag_gesture);
    }
}
