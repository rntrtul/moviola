use gst::glib;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::gdk::Paintable;
use gtk4::prelude::{PaintableExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{graphene, Orientation};
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
        let paintable = self.paintable.borrow();

        let widget_width = self.obj().width() as f64;
        let widget_height = self.obj().height() as f64;
        let widget_aspect_ratio = widget_width / widget_height;

        let video_aspect_ratio = paintable.intrinsic_aspect_ratio();

        let (preview_width, preview_height) = if widget_aspect_ratio > video_aspect_ratio {
            // more width available then height, so change width to fit aspect ratio
            (widget_height * video_aspect_ratio, widget_height)
        } else {
            (widget_width, widget_width / video_aspect_ratio)
        };

        let x_instep = (widget_width - preview_width) / 2.;
        let y_instep = (widget_height - preview_height).floor() / 2.;

        //  rotate will rotate
        //  zoom in and out with scale
        //  to crop just zoom in on cropped area and don't show other area add mask or set overflow to none?
        snapshot.save();

        // todo: need to make it smaller around video and remove black blending
        snapshot.push_opacity(0.4);
        snapshot.push_blur(100.);
        paintable.snapshot(snapshot, widget_width, widget_height);
        snapshot.pop();
        snapshot.pop();

        snapshot.translate(&graphene::Point::new(x_instep as f32, y_instep as f32));
        paintable.snapshot(snapshot, preview_width, preview_height);
        snapshot.restore();
    }
}

impl Preview {
    pub(super) fn set_paintable(&self, paintable: Paintable) {
        self.paintable.replace(paintable);
    }
}
