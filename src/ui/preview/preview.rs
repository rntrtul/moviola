use crate::geometry::Rectangle;
use crate::ui::preview::bounding_box::{HandleType, BOX_HANDLE_WIDTH};
use crate::ui::preview::input::DragType;
use crate::ui::preview::{BoundingBoxDimensions, CropMode};
use crate::ui::sidebar::CropExportSettings;
use gst::glib;
use gst::subclass::prelude::ObjectSubclassIsExt;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use relm4::gtk::graphene::Point;
use relm4::gtk::prelude::TextureExt;
use relm4::gtk::prelude::{PaintableExt, SnapshotExt, WidgetExt};
use relm4::gtk::subclass::prelude::ObjectSubclassExt;
use relm4::gtk::subclass::widget::WidgetImpl;
use relm4::gtk::{gdk, graphene, Orientation};
use std::cell::{Cell, RefCell};

static DEFAULT_WIDTH: f64 = 640f64;
static DEFAULT_HEIGHT: f64 = 360f64;

pub struct Preview {
    pub(crate) left_x: Cell<f32>,
    pub(crate) top_y: Cell<f32>,
    pub(crate) right_x: Cell<f32>,
    pub(crate) bottom_y: Cell<f32>,
    pub(crate) prev_drag: Cell<Point>,
    pub(crate) translate: Cell<Point>,
    pub(crate) active_handle: Cell<HandleType>,
    pub(crate) active_drag_type: Cell<DragType>,
    pub(crate) zoom: Cell<f64>,
    pub(crate) crop_mode: Cell<CropMode>,
    pub(crate) show_crop_box: Cell<bool>,
    pub(crate) show_zoom: Cell<bool>,
    pub(crate) straighten_angle: Cell<f64>,
    pub(crate) texture: RefCell<Option<gdk::Texture>>,
    pub(crate) is_cropped: Cell<bool>,
    pub(crate) is_new_drag: Cell<bool>,
    pub(crate) original_aspect_ratio: Cell<f32>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            translate: Cell::new(Point::zero()),
            prev_drag: Cell::new(Point::zero()),
            left_x: Cell::new(0f32),
            top_y: Cell::new(0f32),
            right_x: Cell::new(1f32),
            bottom_y: Cell::new(1f32),
            active_handle: Cell::new(HandleType::None),
            active_drag_type: Cell::new(DragType::None),
            zoom: Cell::new(1f64),
            crop_mode: Cell::new(CropMode::Free),
            show_crop_box: Cell::new(false),
            show_zoom: Cell::new(true),
            straighten_angle: Cell::new(0f64),
            texture: RefCell::new(None),
            is_cropped: Cell::new(false),
            is_new_drag: Cell::new(true),
            original_aspect_ratio: Cell::new(1.77f32),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Preview {
    const NAME: &'static str = "Preview";
    type Type = super::Preview;
    type ParentType = relm4::gtk::Widget;
}

impl ObjectImpl for Preview {
    fn constructed(&self) {
        self.connect_gestures();
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

    fn snapshot(&self, snapshot: &relm4::gtk::Snapshot) {
        let preview = self.display_preview_rect();
        snapshot.save();

        if !self.show_crop_box.get() && self.is_cropped.get() {
            let cropped_area = self.bounding_box_rect();
            snapshot.push_clip(&cropped_area);
        }
        snapshot.translate(&Point::new(preview.x(), preview.y()));

        if self.show_zoom.get() {
            snapshot.scale(self.zoom.get() as f32, self.zoom.get() as f32);
            let translate = self.translate.get();
            snapshot.translate(&translate);
        }

        if let Some(ref texture) = *self.texture.borrow() {
            if self.is_straightened() {
                // todo: try and get higher res frame when straightend.
                // todo: grey out outside region
                snapshot.save();
                snapshot.translate(&Point::new(preview.width() / 2.0, preview.height() / 2.0));
                snapshot.rotate(self.straighten_angle.get() as f32);
                snapshot.translate(&Point::new(-preview.width() / 2.0, -preview.height() / 2.0));
            }

            texture.snapshot(snapshot, preview.width() as f64, preview.height() as f64);

            if self.is_straightened() {
                snapshot.restore();
            }
        }

        if !self.show_crop_box.get() && self.is_cropped.get() {
            snapshot.pop(); // popping crop region clip
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

    // todo: remove function
    fn current_aspect_ratio(&self) -> f32 {
        self.original_aspect_ratio.get()
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

    pub(crate) fn centered_start(&self, width: f32, height: f32) -> (f32, f32) {
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

    pub(crate) fn display_preview_rect(&self) -> graphene::Rect {
        let mut preview = self.preview_rect();

        if self.show_crop_box.get() || self.is_cropped.get() {
            let bounding_rect = self.bounding_box_rect();

            // these are the coordinates of the box if it wasn't centered.
            let box_left_x = (preview.width() * self.left_x.get()) + preview.x();
            let box_top_y = (preview.height() * self.top_y.get()) + preview.y();

            let x_dist = bounding_rect.x() - box_left_x;
            let y_dist = bounding_rect.y() - box_top_y;
            preview.offset(x_dist, y_dist);
        }

        preview
    }

    pub(crate) fn visible_preview_rect(&self) -> Rectangle {
        let display = self.display_preview_rect();
        let angle = self.straighten_angle.get() as f32;
        Rectangle::new(display, angle)
    }

    fn is_straightened(&self) -> bool {
        self.straighten_angle.get().round() != 0f64
    }

    pub(crate) fn crop_aspect_ratio(&self) -> f32 {
        if self.crop_mode.get() == CropMode::Original {
            self.original_aspect_ratio.get()
        } else {
            self.crop_mode.get().value()
        }
    }

    pub(super) fn update_texture(&self, texture: gdk::Texture) {
        self.texture.borrow_mut().replace(texture);
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

    pub fn straigtening_begun(&self) {
        self.imp().active_drag_type.set(DragType::Straighten)
    }

    pub fn set_straigten_angle(&self, angle: f64) {
        self.imp().straighten_angle.set(angle);
        self.imp().update_to_fit_in_visible_frame();
        self.queue_draw();
    }

    pub fn straigtening_finished(&self) {
        self.imp().active_drag_type.set(DragType::None);
        self.queue_draw();
    }

    pub fn update_texture(&self, texture: gdk::Texture) {
        self.imp()
            .original_aspect_ratio
            .set(texture.width() as f32 / texture.height() as f32);
        self.imp().update_texture(texture);
        self.queue_draw();
    }

    pub fn crop_settings(&self) -> CropExportSettings {
        CropExportSettings {
            bounding_box: BoundingBoxDimensions {
                left_x: self.imp().left_x.get(),
                top_y: self.imp().top_y.get(),
                right_x: self.imp().right_x.get(),
                bottom_y: self.imp().bottom_y.get(),
            },
        }
    }

    pub fn reset_preview(&self) {
        self.imp().left_x.set(0.0);
        self.imp().top_y.set(0.0);
        self.imp().right_x.set(1.0);
        self.imp().bottom_y.set(1.0);

        self.imp().zoom.set(1.0);
    }
}
