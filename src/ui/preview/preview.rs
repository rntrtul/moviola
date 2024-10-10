use crate::ui::preview::bounding_box::{HandleType, BOX_HANDLE_WIDTH};
use crate::ui::preview::effects_pipeline::renderer::Renderer;
use crate::ui::preview::CropMode;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gst::{glib, Sample};
use gtk4::graphene::Point;
use gtk4::prelude::{PaintableExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, graphene, Orientation};
use std::cell::{Cell, RefCell};

static DEFAULT_WIDTH: f64 = 640f64;
static DEFAULT_HEIGHT: f64 = 360f64;

pub struct Preview {
    pub(crate) renderer: RefCell<Renderer>,
    pub(crate) left_x: Cell<f32>,
    pub(crate) top_y: Cell<f32>,
    pub(crate) right_x: Cell<f32>,
    pub(crate) bottom_y: Cell<f32>,
    pub(crate) prev_drag: Cell<Point>,
    pub(crate) translate: Cell<Point>,
    pub(crate) active_handle: Cell<HandleType>,
    pub(crate) handle_drag_active: Cell<bool>,
    pub(crate) zoom: Cell<f64>,
    pub(crate) crop_mode: Cell<CropMode>,
    pub(crate) show_crop_box: Cell<bool>,
    pub(crate) show_zoom: Cell<bool>,
    pub(crate) orientation: Cell<crate::ui::preview::Orientation>,
    // todo: store sample as well. for paused video and effects changing.
    pub(crate) texture: RefCell<Option<gdk::Texture>>,
    //todo: accept orignal dimensions as struct?
    pub(crate) original_aspect_ratio: Cell<f32>,
    pub(crate) native_frame_width: Cell<u32>,
    pub(crate) native_frame_height: Cell<u32>,
}

impl Default for Preview {
    fn default() -> Self {
        let renderer = pollster::block_on(Renderer::new());
        Self {
            renderer: RefCell::new(renderer),
            translate: Cell::new(Point::zero()),
            prev_drag: Cell::new(Point::zero()),
            left_x: Cell::new(0f32),
            top_y: Cell::new(0f32),
            right_x: Cell::new(1f32),
            bottom_y: Cell::new(1f32),
            active_handle: Cell::new(HandleType::None),
            handle_drag_active: Cell::new(false),
            zoom: Cell::new(1f64),
            crop_mode: Cell::new(CropMode::Free),
            show_crop_box: Cell::new(false),
            show_zoom: Cell::new(true),
            orientation: Cell::new(crate::ui::preview::Orientation {
                angle: 0f32,
                mirrored: false,
            }),
            texture: RefCell::new(None),
            original_aspect_ratio: Cell::new(1.77f32),
            native_frame_width: Cell::new(0),
            native_frame_height: Cell::new(0),
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
        self.pan_connect_gestures();
    }
}

impl WidgetImpl for Preview {
    fn measure(&self, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        if orientation == Orientation::Horizontal {
            let width = if for_size <= 0 {
                DEFAULT_WIDTH as i32
            } else {
                (for_size as f32 * self.current_aspect_ratio()) as i32
            };

            (0, width, 0, 0)
        } else {
            let height = if for_size <= 0 {
                DEFAULT_HEIGHT as i32
            } else {
                (for_size as f32 / self.current_aspect_ratio()) as i32
            };

            (0, height, 0, 0)
        }
    }

    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        let preview = self.preview_rect();
        snapshot.save();

        self.orient_snapshot(&snapshot);
        if self.orientation.get().is_vertical() {
            snapshot.translate(&Point::new(preview.y(), preview.x()));
        } else {
            snapshot.translate(&Point::new(preview.x(), preview.y()));
        };

        if self.show_zoom.get() {
            snapshot.scale(self.zoom.get() as f32, self.zoom.get() as f32);
            snapshot.translate(&self.translate.get());
        }

        if let Some(ref texture) = *self.texture.borrow() {
            let (width, height) = if self.orientation.get().is_vertical() {
                (preview.height() as f64, preview.width() as f64)
            } else {
                (preview.width() as f64, preview.height() as f64)
            };
            texture.snapshot(snapshot, width, height);
        }

        snapshot.restore();

        if self.show_crop_box.get() {
            self.draw_bounding_box(snapshot);
        }
    }
}

impl Preview {
    // widths + height accounting for space needed for bounding box handles
    pub(crate) fn widget_width(&self) -> f32 {
        self.obj().width() as f32 - (BOX_HANDLE_WIDTH * 2f32)
    }

    fn widget_height(&self) -> f32 {
        self.obj().height() as f32 - (BOX_HANDLE_WIDTH * 2f32)
    }

    fn current_aspect_ratio(&self) -> f32 {
        if self.orientation.get().is_vertical() {
            self.native_frame_height.get() as f32 / self.native_frame_width.get() as f32
        } else {
            self.original_aspect_ratio.get()
        }
    }

    // returns (width, height)
    fn preview_size(&self) -> (f32, f32) {
        let widget_width = self.widget_width();
        let widget_height = self.widget_height();

        let widget_aspect_ratio = widget_width / widget_height;
        let video_aspect_ratio = self.current_aspect_ratio();

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

    // todo: determine if taking sample and if memory not copied
    pub(super) fn render_sample(&self, sample: Sample) {
        let mut renderer = self.renderer.borrow_mut();

        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        if info.width() != self.native_frame_width.get()
            && info.height() != self.native_frame_height.get()
        {
            self.native_frame_width.replace(info.width());
            self.native_frame_height.replace(info.height());
            self.original_aspect_ratio
                .set(info.width() as f32 / info.height() as f32);

            // todo: add blur on edge of target, so make size slightly larger
            renderer.update_input_texture_output_texture_size(
                info.width(),
                info.height(),
                info.width(),
                info.height(),
            );
        }

        let cb = renderer.prepare_video_frame_render_pass(sample);
        let texture = pollster::block_on(renderer.render(cb)).expect("Could not render");
        self.texture.borrow_mut().replace(texture);
    }
}
