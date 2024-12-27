use crate::ui::preview::bounding_box::{HandleType, BOX_HANDLE_WIDTH};
use crate::ui::preview::input::DragType;
use crate::ui::preview::{BoundingBoxDimensions, CropMode};
use crate::ui::sidebar::CropExportSettings;
use ges::subclass::prelude::ObjectSubclassIsExt;
use gst::glib;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::graphene::Point;
use gtk4::prelude::{GestureExt, PaintableExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, graphene, Orientation};
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
    pub(crate) orientation: Cell<crate::ui::preview::Orientation>,
    pub(crate) straighten_angle: Cell<f64>,
    pub(crate) texture: RefCell<Option<gdk::Texture>>,
    pub(crate) _crop_scale: Cell<f32>,
    pub(crate) is_cropped: Cell<bool>,
    pub(crate) is_new_drag: Cell<bool>,
    pub(crate) original_aspect_ratio: Cell<f32>,
    //todo: only using native frame to calc aspect ratio
    pub(crate) native_frame_width: Cell<u32>,
    pub(crate) native_frame_height: Cell<u32>,
}

// todo: move somewhere else
pub struct Rectangle {
    pub(crate) top_left: Point,
    pub(crate) top_right: Point,
    pub(crate) bottom_left: Point,
    pub(crate) bottom_right: Point,
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
            orientation: Cell::new(crate::ui::preview::Orientation::default()),
            straighten_angle: Cell::new(0f64),
            texture: RefCell::new(None),
            _crop_scale: Cell::new(1.0),
            is_cropped: Cell::new(false),
            is_new_drag: Cell::new(true),
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

    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
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
                // todo: use crop box instead of preview for determinig translate and scaling
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

    fn current_aspect_ratio(&self) -> f32 {
        if self.orientation.get().is_width_flipped() {
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

    pub(crate) fn scale_for_straightening(&self, preview: &graphene::Rect) -> f32 {
        let angle = (self.straighten_angle.get() as f32).abs().to_radians();

        let theta = (preview.height() / preview.width()).atan();
        let phi = (preview.width() / preview.height()).atan();

        let beta = phi - angle;
        let gamma = theta - angle;

        let diagonal = (preview.width().powi(2) + preview.height().powi(2)).sqrt();

        diagonal * (beta.cos().abs() / preview.height()).max(gamma.cos().abs() / preview.width())
    }

    pub(crate) fn translate_rotated_rect_to_center(&self, preview: &graphene::Rect) -> (f32, f32) {
        let angle = (self.straighten_angle.get() as f32).abs().to_radians();

        let half_width = preview.width() / 2.0;
        let half_height = preview.height() / 2.0;

        let cx = -half_width
            + ((preview.width() * angle.cos()) + (preview.height() * angle.sin())) / 2.0;
        let cy = -half_height
            + ((preview.height() * angle.cos()) + (preview.width() * angle.sin())) / 2.0;

        (-cx, -cy)
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

    fn visible_preview_center(&self) -> Point {
        let display = self.display_preview_rect();
        let angle = self.straighten_angle.get() as f32;

        let scale = self.scale_for_straightening(&display);
        let scaled_width = display.width() * scale;
        let scaled_height = display.height() * scale;

        let (sin, cos) = angle.to_radians().sin_cos();

        let horizontal_run = scaled_width * cos;
        let horizontal_rise = scaled_width * sin;
        let vertical_run = scaled_height * sin;
        let vertical_rise = scaled_height * cos;

        let top_left = Self::rotate_point_around(display.top_left(), display.center(), angle);
        let center_x = top_left.x() + ((horizontal_run - vertical_run) / 2.0);
        let center_y = top_left.y() + ((horizontal_rise + vertical_rise) / 2.0);

        Point::new(center_x, center_y)
    }

    fn translate_point(point: &Point, translate: &Point) -> Point {
        Point::new(point.x() + translate.x(), point.y() + translate.y())
    }

    fn subtract_points(a: &Point, b: &Point) -> Point {
        Point::new(a.x() - b.x(), a.y() - b.y())
    }

    pub(crate) fn visible_preview_rect(&self) -> Rectangle {
        let display = self.display_preview_rect();
        let angle = self.straighten_angle.get() as f32;

        let (sin, cos) = angle.to_radians().sin_cos();

        let horizontal_run = display.width() * cos;
        let horizontal_rise = display.width() * sin;
        let vertical_run = display.height() * sin;
        let vertical_rise = display.height() * cos;

        let top_left = Self::rotate_point_around(display.top_left(), display.center(), angle);

        // These corners are built relative to top_left. Do not need to translate for centering since
        // top_left already adjusted.
        let top_right = Point::new(
            top_left.x() + horizontal_run,
            top_left.y() + horizontal_rise,
        );

        let bottom_left = Point::new(top_left.x() - vertical_run, top_left.y() + vertical_rise);

        let bottom_right = Point::new(
            bottom_left.x() + horizontal_run,
            bottom_left.y() + horizontal_rise,
        );

        Rectangle {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    fn is_straightened(&self) -> bool {
        self.straighten_angle.get().round() != 0f64
    }

    fn cropped_region_box(&self) -> graphene::Rect {
        let preview = self.preview_rect();
        let width = preview.width() * (self.right_x.get() - self.left_x.get());
        let height = preview.height() * (self.bottom_y.get() - self.top_y.get());
        let (x, y) = self.centered_start(width, height);

        graphene::Rect::new(x, y, width, height)
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

    pub fn preview_frame_size(&self) -> (i32, i32) {
        let (width, height) = self
            .imp()
            .preview_size(self.imp().original_aspect_ratio.get());

        (width as i32, height as i32)
    }

    pub fn update_texture(&self, texture: gdk::Texture) {
        self.imp().update_texture(texture);
        self.queue_draw();
    }

    pub fn update_native_resolution(&self, width: u32, height: u32) {
        self.imp().native_frame_width.set(width);
        self.imp().native_frame_height.set(height);
        self.imp()
            .original_aspect_ratio
            .set(width as f32 / height as f32)
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
        self.imp()
            .orientation
            .set(crate::ui::preview::Orientation::default());
    }
}
