use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

impl crate::ui::preview::Preview {
    pub fn zoom(&self) -> f64 {
        self.imp().zoom.get()
    }

    pub fn set_zoom(&self, zoom: f64) {
        self.imp().zoom.set(zoom);

        // todo: clamp translate on zoom changes
        if zoom == 1f64 {
            self.imp().translate_x.set(0f32);
            self.imp().translate_y.set(0f32);
        }

        self.queue_draw();
    }

    pub fn hide_zoom(&self) {
        self.imp().show_zoom.set(false);
    }

    pub fn show_zoom(&self) {
        self.imp().show_zoom.set(true);
    }
}
