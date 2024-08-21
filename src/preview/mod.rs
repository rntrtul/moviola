mod imp;

use gtk4::{glib};
use gtk4::gdk::Paintable;
use gtk4::glib::{clone};
use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct Preview(ObjectSubclass<imp::Preview>)
        @extends gtk4::Widget;
}

impl Preview {
    pub fn test(&self) {
        println!("TEST");
    }
    pub fn set_paintable(&self, paintable: Paintable) {
        println!("{}x{}", paintable.intrinsic_height(), paintable.intrinsic_width());


        paintable.connect_invalidate_contents(clone!(
            @strong self as preview =>
            move |_x| {
                preview.queue_draw();
            }
        ));

        self.imp().set_paintable(paintable);
    }

    pub(crate) fn new() -> Self {
        glib::Object::builder()
            .build()

    }
}