mod preview;

use gtk4::gdk::Paintable;
use gtk4::glib;
use gtk4::glib::clone;
use gtk4::prelude::{PaintableExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct Preview(ObjectSubclass<preview::Preview>)
        @extends gtk4::Widget;
}

impl Preview {
    pub fn set_paintable(&self, paintable: Paintable) {
        paintable.connect_invalidate_contents(clone!(
            #[strong(rename_to=preview)]
            self,
            move |_x| {
                preview.queue_draw();
            }
        ));

        self.imp().set_paintable(paintable);
    }

    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }
}
