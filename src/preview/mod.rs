mod imp;

use gtk4::{gdk, glib};
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

        // let closure = clone!(
        //     @weak self as preview =>
        //     move |x| {
        //            preview.queue_draw();
        //     println!("contents: {}x{}", x.intrinsic_height(), x.intrinsic_width());
        //     }
        // );

        paintable.connect_invalidate_contents(clone!(
            @strong self as preview =>
            move |x| {
                // preview.test();
                preview.queue_draw();
                println!("contents: {}x{}", x.intrinsic_height(), x.intrinsic_width());
            }
        ));

        // paintable.connect_invalidate_contents(move |x| {
        //     println!("contents2: {}x{}", x.intrinsic_height(), x.intrinsic_width());
        // });

        // paintable.connect_invalidate_size(move |x| {
        //     self.queue_draw();
        //     println!("size: {}x{}", x.intrinsic_height(), x.intrinsic_width());
        // });

        self.imp().set_paintable(paintable);
    }

    pub(crate) fn new() -> Self {
        glib::Object::builder()
            .build()

    }
}