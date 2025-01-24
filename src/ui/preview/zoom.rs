use relm4::gtk::graphene;
use relm4::gtk::prelude::WidgetExt;
use relm4::gtk::subclass::prelude::ObjectSubclassIsExt;

impl crate::ui::preview::Preview {
    pub fn zoom(&self) -> f64 {
        self.imp().zoom.get()
    }

    pub fn set_zoom(&self, zoom: f64) {
        self.imp().zoom.set(zoom);

        // todo: clamp translate on zoom changes
        if zoom == 1f64 {
            self.imp().translate.set(graphene::Point::zero());
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
