use gst::glib;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::gdk::Paintable;
use gtk4::prelude::{PaintableExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, Orientation};
use std::cell::RefCell;

static DEFAULT_WIDTH: f64 = 640f64;
static DEFAULT_HEIGHT: f64 = 360f64;

pub struct Preview {
    paintable: RefCell<Paintable>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            paintable: RefCell::new(Paintable::new_empty(0, 0)),
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
    fn measure(&self, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        if orientation == Orientation::Horizontal {
            let concrete_size = self.paintable.borrow().compute_concrete_size(
                0.,
                0f64.max(for_size as f64),
                DEFAULT_WIDTH,
                DEFAULT_HEIGHT,
            );

            (0, concrete_size.0 as i32, 0, 0)
        } else {
            let concrete_size = self.paintable.borrow().compute_concrete_size(
                0f64.max(for_size as f64),
                0.,
                DEFAULT_WIDTH,
                DEFAULT_HEIGHT,
            );

            (0, concrete_size.1 as i32, 0, 0)
        }
    }

    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        let width = self.obj().width();
        let height = self.obj().height();
        //  rotate will rotate
        //  zoom in and out with scale
        //  to crop just zoom in on cropped area and don't show other area add mask or set overflow to none?
        snapshot.save();
        // snapshot.rotate(5f32);
        // snapshot.scale(2f32, 2f32);
        // snapshot.translate(&graphene::Point::new(100f32, 100f32));
        gdk::Paintable::snapshot(
            &*self.paintable.borrow(),
            snapshot,
            width as f64,
            height as f64,
        );
        snapshot.restore();
    }
}

impl Preview {
    pub(super) fn set_paintable(&self, paintable: Paintable) {
        self.paintable.replace(paintable);
    }
}
