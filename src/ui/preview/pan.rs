use crate::ui::preview::preview::Preview;
use ges::glib;
use ges::glib::clone;
use ges::subclass::prelude::ObjectSubclassExt;
use gtk4::graphene;
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

                let preview = obj.imp();
                let mut prev_drag = preview.prev_drag.get();

                let (start_x, start_y) = drag.start_point().unwrap();

                let x = (start_x + x_offset) as f32;
                let y = (start_y + y_offset) as f32;

                if prev_drag.x() == 0. && prev_drag.y() == 0. {
                    prev_drag = graphene::Point::new(x, y);
                }

                let offset_x = x - prev_drag.x();
                let offset_y = y - prev_drag.y();

                let preview_rect = obj.imp().preview_rect();
                // todo: maybe update preview rect on resize and store. stop recomputes on all drag events
                let min_translate_x =
                    -(preview_rect.width() - (preview_rect.width() / preview.zoom.get() as f32));
                let min_translate_y =
                    -(preview_rect.height() - (preview_rect.height() / preview.zoom.get() as f32));

                let translate_x =
                    (preview.translate.get().x() + offset_x).clamp(min_translate_x, 0f32);
                let translate_y =
                    (preview.translate.get().y() + offset_y).clamp(min_translate_y, 0f32);

                preview
                    .translate
                    .set(graphene::Point::new(translate_x, translate_y));
                preview.prev_drag.set(graphene::Point::new(x, y));

                obj.queue_draw();
            }
        ));

        drag_gesture.connect_drag_end(clone!(
            #[weak]
            obj,
            move |_, _, _| {
                // todo: should this only be in one drag handle?
                obj.imp().prev_drag.set(graphene::Point::zero());
            }
        ));

        obj.add_controller(drag_gesture);
    }
}
