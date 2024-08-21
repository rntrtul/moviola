use std::cell::{Cell, RefCell};
use gst::glib;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::{gdk, graphene, gsk};
use gtk4::gdk::{Paintable, Snapshot};
use gtk4::prelude::{PaintableExt, SnapshotExt};
use gtk4::subclass::widget::WidgetImpl;
use relm4::adw::subclass::prelude::PaintableImpl;


pub struct Preview {
    paintable: RefCell<Paintable>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            paintable: RefCell::new(Paintable::new_empty(0,0)),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Preview {
    const NAME: &'static str = "Preview";
    type Type = super::Preview;
    type ParentType = gtk4::Widget;
}

impl ObjectImpl for Preview {}

impl WidgetImpl for Preview {
    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        //  rotate will rotate
        //  zoom in and out with scale
        //  to crop just zoom in on cropped area and don't show other area
        //          add mask or set overflow to none?
        snapshot.save();
        // snapshot.rotate(5f32);
        // snapshot.scale(2f32, 2f32);
        snapshot.translate(&graphene::Point::new(100f32, 100f32));

        gdk::Paintable::snapshot(&*self.paintable.borrow(), snapshot, 640f64, 360f64);
        snapshot.restore();
    }
}

impl Preview {
    pub(super) fn set_paintable(&self, paintable: Paintable) {
        self.paintable.replace(paintable);
    }
}

