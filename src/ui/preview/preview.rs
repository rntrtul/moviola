use crate::ui::preview::bounding_box::HandleType;
use crate::ui::preview::CropMode;
use gst::glib;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::gdk::Paintable;
use gtk4::prelude::{PaintableExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{graphene, Orientation};
use std::cell::{Cell, RefCell};

static DEFAULT_WIDTH: f64 = 640f64;
static DEFAULT_HEIGHT: f64 = 360f64;

pub struct Preview {
    pub(crate) paintable: RefCell<Paintable>,
    pub(crate) left_x: Cell<f32>,
    pub(crate) top_y: Cell<f32>,
    pub(crate) right_x: Cell<f32>,
    pub(crate) bottom_y: Cell<f32>,
    pub(crate) prev_drag_x: Cell<f32>,
    pub(crate) prev_drag_y: Cell<f32>,
    pub(crate) active_handle: Cell<HandleType>,
    pub(crate) handle_drag_active: Cell<bool>,
    pub(crate) zoom: Cell<f64>,
    pub(crate) crop_mode: Cell<CropMode>,
    pub(crate) show_crop_box: Cell<bool>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            paintable: RefCell::new(Paintable::new_empty(0, 0)),
            left_x: Cell::new(0f32),
            top_y: Cell::new(0f32),
            right_x: Cell::new(1f32),
            bottom_y: Cell::new(1f32),
            prev_drag_x: Cell::new(0f32),
            prev_drag_y: Cell::new(0f32),
            active_handle: Cell::new(HandleType::None),
            handle_drag_active: Cell::new(false),
            zoom: Cell::new(1f64),
            crop_mode: Cell::new(CropMode::Free),
            show_crop_box: Cell::new(false),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Preview {
    const NAME: &'static str = "Preview";
    type Type = super::Preview;
    type ParentType = gtk4::Widget;
}

impl ObjectImpl for Preview {
    fn constructed(&self) {
        self.box_connect_gestures();
    }
}

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

        // fixme: account for margin?
        let widget_width = self.obj().width() as f64;
        let widget_height = self.obj().height() as f64;

        let preview = self.preview_rect();
        // todo: need to make glow smaller around video and remove black blending
        //          call centerd start with a rect slightly larger than preview

        //  rotate will rotate
        //  zoom in and out with scale
        //  flip with scale (set to -1 for flip direction)
        //  to crop just zoom in on cropped area and don't show other area add mask or set overflow to none?
        snapshot.save();

        snapshot.push_opacity(0.4);
        snapshot.push_blur(100.);
        paintable.snapshot(snapshot, widget_width, widget_height);
        snapshot.pop();
        snapshot.pop();

        snapshot.scale(self.zoom.get() as f32, self.zoom.get() as f32);
        snapshot.translate(&graphene::Point::new(preview.x(), preview.y()));
        paintable.snapshot(snapshot, preview.width() as f64, preview.height() as f64);

        snapshot.restore();

        if self.show_crop_box.get() {
            self.draw_bounding_box(snapshot);
        }
    }
}

impl Preview {
    // returns (width, height)
    fn preview_size(&self) -> (f32, f32) {
        let widget_width = self.obj().width() as f32;
        let widget_height = self.obj().height() as f32;
        let widget_aspect_ratio = widget_width / widget_height;

        let video_aspect_ratio = self.paintable.borrow().intrinsic_aspect_ratio() as f32;

        if widget_aspect_ratio > video_aspect_ratio {
            // more width available then height, so change width to fit aspect ratio
            (widget_height * video_aspect_ratio, widget_height)
        } else {
            (widget_width, widget_width / video_aspect_ratio)
        }
    }

    fn centered_start(&self, width: f32, height: f32) -> (f32, f32) {
        let widget_width = self.obj().width() as f32;
        let widget_height = self.obj().height() as f32;

        let x_instep = (widget_width - width) / 2.;
        let y_instep = (widget_height - height).floor() / 2.;

        (x_instep, y_instep)
    }

    pub(crate) fn preview_rect(&self) -> graphene::Rect {
        let (preview_width, preview_height) = self.preview_size();
        let (x, y) = self.centered_start(preview_width, preview_height);

        graphene::Rect::new(x, y, preview_width, preview_height)
    }

    pub(super) fn set_paintable(&self, paintable: Paintable) {
        self.paintable.replace(paintable);
    }
}
