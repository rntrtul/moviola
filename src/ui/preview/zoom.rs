use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

impl crate::ui::preview::Preview {
    pub fn set_zoom(&self, zoom: f64) {
        self.imp().zoom.set(zoom);
        self.queue_draw();
    }
}
