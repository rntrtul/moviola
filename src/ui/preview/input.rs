use crate::ui::preview::preview::Preview;
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::prelude::{GestureDragExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{glib, graphene};

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
                let (start_x, start_y) = drag.start_point().unwrap();

                let target_x = (start_x + x_offset) as f32; // graphene uses f32, so not using f64
                let target_y = (start_y + y_offset) as f32;
                let (clamped_x, clamped_y) = obj.imp().clamp_coords_to_preview(target_x, target_y);

                let (target_x_percent, target_y_percent) =
                    obj.imp().coords_as_percent(target_x, target_y);

                let mut prev_drag = obj.imp().prev_drag.get();

                if prev_drag.x() == 0. && prev_drag.y() == 0. {
                    prev_drag = graphene::Point::new(clamped_x, clamped_y);
                    obj.imp().prev_drag.set(prev_drag);
                }

                let offset_x = target_x - prev_drag.x();
                let offset_y = target_y - prev_drag.y();

                if obj.imp().show_crop_box.get() {
                    if obj.imp().handle_drag_active.get() {
                        obj.imp()
                            .update_handle_pos(target_x_percent, target_y_percent);
                    } else {
                        obj.imp().translate_box(target_x_percent, target_y_percent);
                    }
                    obj.queue_draw();
                } else if obj.imp().zoom.get() != 1f64 {
                    obj.imp().pan_preview(offset_x, offset_y);
                    obj.queue_draw();
                }

                obj.imp()
                    .prev_drag
                    .set(graphene::Point::new(clamped_x, clamped_y));
            }
        ));

        drag_gesture.connect_drag_end(glib::clone!(
            #[weak]
            obj,
            move |_, _, _| {
                if obj.imp().handle_drag_active.get() {
                    obj.imp().is_cropped.set(
                        obj.imp().right_x.get() != 1.0
                            || obj.imp().left_x.get() != 0.0
                            || obj.imp().bottom_y.get() != 1.0
                            || obj.imp().top_y.get() != 0.0,
                    );

                    obj.queue_draw();
                }

                obj.imp().handle_drag_active.set(false);
                obj.imp().prev_drag.set(graphene::Point::zero());
            }
        ));

        obj.add_controller(drag_gesture);
    }
}
