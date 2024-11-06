use crate::ui::preview::bounding_box::{HandleType, BOX_HANDLE_WIDTH};
use crate::ui::preview::effects_pipeline::renderer::Renderer;
use crate::ui::preview::effects_pipeline::FRAME_TIME_IDX;
use crate::ui::preview::{BoundingBoxDimensions, CropMode, EffectParameters};
use crate::ui::sidebar::CropExportSettings;
use ges::subclass::prelude::ObjectSubclassIsExt;
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
    pub(crate) texture: RefCell<Option<gdk::Texture>>,
    pub(crate) crop_scale: Cell<f32>,
    pub(crate) is_cropped: Cell<bool>,
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
            crop_scale: Cell::new(1.0),
            is_cropped: Cell::new(false),
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

        let (translate_x, translate_y) = if !self.show_crop_box.get() && self.is_cropped.get() {
            let cropped_area = self.cropped_region_clip();

            let left = preview.width() * self.left_x.get();
            let top = preview.height() * self.top_y.get();

            let x = cropped_area.x() - left;
            let y = cropped_area.y() - top;
            snapshot.push_clip(&cropped_area);

            (x, y)
        } else {
            (preview.x(), preview.y())
        };

        snapshot.translate(&Point::new(translate_x, translate_y));

        if self.show_zoom.get() {
            snapshot.scale(self.zoom.get() as f32, self.zoom.get() as f32);
            let translate = self.translate.get();
            snapshot.translate(&translate);
        }

        if let Some(ref texture) = *self.texture.borrow() {
            texture.snapshot(snapshot, preview.width() as f64, preview.height() as f64);
        }

        if !self.show_crop_box.get() && self.is_cropped.get() {
            snapshot.pop();
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
    fn preview_size(&self, video_aspect_ratio: f32) -> (f32, f32) {
        let widget_width = self.widget_width();
        let widget_height = self.widget_height();

        let widget_aspect_ratio = widget_width / widget_height;

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
        let (preview_width, preview_height) = self.preview_size(self.current_aspect_ratio());
        let (x, y) = self.centered_start(preview_width, preview_height);

        graphene::Rect::new(x, y, preview_width, preview_height)
    }

    fn cropped_region_clip(&self) -> graphene::Rect {
        let preview = self.preview_rect();
        let width = preview.width() * (self.right_x.get() - self.left_x.get());
        let height = preview.height() * (self.bottom_y.get() - self.top_y.get());
        let (x, y) = self.centered_start(width, height);

        graphene::Rect::new(x, y, width, height)
    }

    // todo: determine if taking sample and if memory not copied
    pub(super) fn upload_new_sample(&self, sample: Sample) {
        let mut renderer = self.renderer.borrow_mut();
        renderer.timer.start_time(FRAME_TIME_IDX);

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

        renderer.sample_to_texture(sample);
    }

    pub(super) fn update_effect_parameters(&self, parameters: EffectParameters) {
        let mut renderer = self.renderer.borrow_mut();

        pollster::block_on(renderer.update_effects(parameters));
    }

    // todo: try to make this async
    pub(super) fn render_frame(&self) {
        let mut renderer = self.renderer.borrow_mut();
        renderer.timer.start_time(FRAME_TIME_IDX);

        let command_buffer = renderer.prepare_video_frame_render_pass();
        let texture =
            pollster::block_on(renderer.render(command_buffer)).expect("Could not render");
        self.texture.borrow_mut().replace(texture);

        renderer.timer.stop_time(FRAME_TIME_IDX);
        // renderer.timer.print_results();
    }
}

impl crate::ui::preview::Preview {
    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_crop_mode(&self, crop_modes: CropMode) {
        self.imp().crop_mode.set(crop_modes);
        self.imp().maintain_aspect_ratio();
        self.queue_draw();
    }

    pub fn show_crop_box(&self) {
        self.imp().show_crop_box.set(true);
        self.queue_draw();
    }

    pub fn hide_crop_box(&self) {
        self.imp().show_crop_box.set(false);
        self.queue_draw();
    }

    pub fn upload_new_sample(&self, sample: Sample) {
        self.imp().upload_new_sample(sample);
    }

    pub fn update_effect_parameters(&self, parameters: EffectParameters) {
        self.imp().update_effect_parameters(parameters);
    }

    pub fn render_frame(&self) {
        self.imp().render_frame();
        self.queue_draw();
    }

    pub fn export_settings(&self) -> CropExportSettings {
        CropExportSettings {
            bounding_box: BoundingBoxDimensions {
                left_x: self.imp().left_x.get(),
                top_y: self.imp().top_y.get(),
                right_x: self.imp().right_x.get(),
                bottom_y: self.imp().bottom_y.get(),
            },
            orientation: self.imp().orientation.get(),
        }
    }

    pub fn reset_preview(&self) {
        self.imp().left_x.set(0.0);
        self.imp().top_y.set(0.0);
        self.imp().right_x.set(1.0);
        self.imp().bottom_y.set(1.0);

        self.imp().zoom.set(1.0);
        self.imp().orientation.set(crate::ui::preview::Orientation {
            angle: 0.0,
            mirrored: false,
        });
    }
}
