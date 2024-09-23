use crate::ui::preview::preview::Preview;
use ges::glib;
use ges::glib::clone;
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::prelude::{GestureDragExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;

impl Preview {
    pub(crate) fn pan_connect_gestures(&self) {
        let obj = self.obj();
        let drag_gesture = gtk4::GestureDrag::new();

        drag_gesture.connect_drag_update(glib::clone!(
            #[weak]
            obj,
            move |drag, x_offset, y_offset| {
                if obj.imp().handle_drag_active.get() || obj.imp().zoom.get() == 1f64 {
                    return;
                }

                let (start_x, start_y) = drag.start_point().unwrap();

                let x = (start_x + x_offset) as f32;
                let y = (start_y + y_offset) as f32;

                if obj.imp().prev_drag_x.get() == 0. && obj.imp().prev_drag_y.get() == 0. {
                    obj.imp().prev_drag_x.set(x);
                    obj.imp().prev_drag_y.set(y);
                }

                let offset_x = x - obj.imp().prev_drag_x.get();
                let offset_y = y - obj.imp().prev_drag_y.get();

                let preview = obj.imp().preview_rect();
                // todo: maybe update preview rect on resize and store. stop recomputes on all drag events
                let min_translate_x =
                    -(preview.width() - (preview.width() / obj.imp().zoom.get() as f32));
                let min_translate_y =
                    -(preview.height() - (preview.height() / obj.imp().zoom.get() as f32));

                let translate_x =
                    (obj.imp().translate_x.get() + offset_x).clamp(min_translate_x, 0f32);
                let translate_y =
                    (obj.imp().translate_y.get() + offset_y).clamp(min_translate_y, 0f32);

                obj.imp().translate_x.set(translate_x);
                obj.imp().translate_y.set(translate_y);

                obj.imp().prev_drag_x.set(x);
                obj.imp().prev_drag_y.set(y);

                obj.queue_draw();
            }
        ));

        drag_gesture.connect_drag_end(clone!(
            #[weak]
            obj,
            move |_, _, _| {
                // todo: should this only be in one drag handle?
                obj.imp().prev_drag_x.set(0f32);
                obj.imp().prev_drag_y.set(0f32);
            }
        ));

        obj.add_controller(drag_gesture);
    }
}

impl Preview {
    fn get_cordinate_percent_from_drag_zoom(&self, x: f64, y: f64) -> (f64, f64) {
        let preview = self.preview_rect();

        let x_adj = (x - preview.x() as f64).clamp(0., preview.width() as f64);
        let y_adj = (y - preview.y() as f64).clamp(0., preview.height() as f64);

        (
            x_adj / preview.width() as f64,
            y_adj / preview.height() as f64,
        )
    }
}
