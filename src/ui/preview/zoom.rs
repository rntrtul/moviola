use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

impl crate::ui::preview::Preview {
    pub fn set_zoom(&self, zoom: f64) {
        self.imp().zoom.set(zoom);

        if zoom == 1f64 {
            self.imp().translate_x.set(0f32);
            self.imp().translate_y.set(0f32);
        }

        self.queue_draw();
    }
}
