use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExt, SnapshotExtManual, WidgetExt};
use gtk4::subclass::prelude::{
    DerivedObjectProperties, ObjectImpl, ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt,
};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, glib, graphene, gsk, Snapshot};
use relm4::gtk;

pub static MARGIN: f32 = 5.;
static HANDLE_FILL_RULE: gsk::FillRule = gsk::FillRule::Winding;
static BOX_COLOUR: gdk::RGBA = gdk::RGBA::WHITE;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "HandleType")]
pub enum HandleType {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "CropMode")]
pub enum CropMode {
    #[default]
    Free,
    Original,
    Square,
    _5To4,
    _4To3,
    _3To2,
    _16To9,
}

impl CropMode {
    fn value(&self) -> f32 {
        match *self {
            CropMode::Free => 0.,
            CropMode::Original => 0.,
            CropMode::Square => 1.,
            CropMode::_5To4 => 1.25,
            CropMode::_4To3 => 1.33333,
            CropMode::_3To2 => 1.5,
            CropMode::_16To9 => 1.77777,
        }
    }
}

// properties are represented in percentages since preview is not 1:1 to output
#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::CropBoxWidget)]
pub struct CropBoxWidget {
    #[property(get, set)]
    pub left_x: Cell<f32>,
    #[property(get, set)]
    pub top_y: Cell<f32>,
    #[property(get, set)]
    pub right_x: Cell<f32>,
    #[property(get, set)]
    pub bottom_y: Cell<f32>,
    #[property(get, set)]
    pub drag_active: Cell<bool>,
    #[property(get, set = Self::set_aspect_ratio)]
    pub asepct_ratio: Cell<f64>,
    #[property(get, set, builder(HandleType::TopLeft))]
    active_handle: Cell<HandleType>,
    #[property(get, set = Self::set_crop_mode, builder(CropMode::Free))]
    crop_mode: Cell<CropMode>,
}

#[glib::object_subclass]
impl ObjectSubclass for CropBoxWidget {
    const NAME: &'static str = "CropBoxWidget";
    type Type = super::CropBoxWidget;
    type ParentType = gtk::Widget;
}

#[glib::derived_properties]
impl ObjectImpl for CropBoxWidget {}

impl WidgetImpl for CropBoxWidget {
    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();

        let (left_x, top_y, right_x, bottom_y) =
            self.get_box_bounds(widget.width() as f32, widget.height() as f32);

        let border_rect = graphene::Rect::new(left_x, top_y, right_x - left_x, bottom_y - top_y);

        let border = gsk::RoundedRect::from_rect(border_rect, 0.);
        let border_widths = [1.; 4];
        let border_colours = [BOX_COLOUR; 4];

        if self.drag_active.get() {
            let horizontal_step = (right_x - left_x) / 3.;
            Self::draw_evenly_spaced_line(snapshot, true, horizontal_step, left_x, top_y, bottom_y);

            let vertical_step = (bottom_y - top_y) / 3.;
            Self::draw_evenly_spaced_line(snapshot, false, vertical_step, top_y, left_x, right_x);
        }

        let handle_center = self.get_handle_centers(widget.width() as f32, widget.height() as f32);

        for center in handle_center {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&center, MARGIN);
            let handle = path_builder.to_path();
            snapshot.append_fill(&handle, HANDLE_FILL_RULE, &BOX_COLOUR);
        }

        snapshot.append_border(&border, &border_widths, &border_colours);
    }
}

impl Default for crate::ui::CropBoxWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("left_x", 0f32)
            .property("top_y", 0f32)
            .property("right_x", 1f32)
            .property("bottom_y", 1f32)
            .property("drag_active", false)
            .build()
    }
}

impl CropBoxWidget {
    fn draw_evenly_spaced_line(
        snapshot: &Snapshot,
        is_horizontal: bool,
        step_size: f32,
        step_start: f32,
        start: f32,
        end: f32,
    ) {
        let thirds_box_stroke = gsk::Stroke::builder(1.).build();

        for step in 1..3 {
            let pos = step_start + (step_size * step as f32);
            let path_builder = gsk::PathBuilder::new();

            if is_horizontal {
                path_builder.move_to(pos, start);
                path_builder.line_to(pos, end);
            } else {
                path_builder.move_to(start, pos);
                path_builder.line_to(end, pos);
            }

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &thirds_box_stroke, &BOX_COLOUR);
        }
    }
    // returns (x, y, width, height)
    fn get_preview_rect(&self, widget_width: f32, widget_height: f32) -> (f32, f32, f32, f32) {
        let marginless_width = widget_width - (MARGIN * 2f32);
        let marginless_height = widget_height - (MARGIN * 2f32);

        let height_constrained_width = (marginless_height as f64 * self.asepct_ratio.get()) as f32;
        let width_constrained_height = (marginless_width as f64 / self.asepct_ratio.get()) as f32;

        let preview_width = marginless_width.min(height_constrained_width);
        let preview_height = marginless_height.min(width_constrained_height);

        let x = (widget_width - preview_width) / 2f32;
        // picture does not center vertically so do not need to have y_instep, besides marin
        // let y = (widget_height - preview_height) / 2f32;

        (x, MARGIN, preview_width, preview_height)
    }

    fn get_box_bounds(&self, widget_width: f32, widget_height: f32) -> (f32, f32, f32, f32) {
        let (x_instep, y_instep, preview_width, preview_height) =
            self.get_preview_rect(widget_width, widget_height);

        let left_x = (preview_width * self.left_x.get()) + x_instep;
        let top_y = (preview_height * self.top_y.get()) + y_instep;

        let right_x = ((preview_width) * self.right_x.get()) + x_instep;
        let bottom_y = ((preview_height) * self.bottom_y.get()) + y_instep;

        (left_x, top_y, right_x, bottom_y)
    }

    fn get_handle_centers(&self, widget_width: f32, widget_height: f32) -> [graphene::Point; 4] {
        let (left_x, top_y, right_x, bottom_y) = self.get_box_bounds(widget_width, widget_height);

        [
            graphene::Point::new(left_x, top_y),
            graphene::Point::new(left_x, bottom_y),
            graphene::Point::new(right_x, top_y),
            graphene::Point::new(right_x, bottom_y),
        ]
    }

    pub fn set_aspect_ratio(&self, aspect_ratio: f64) {
        self.asepct_ratio.set(aspect_ratio);
    }

    pub fn set_crop_mode(&self, mode: CropMode) {
        self.crop_mode.set(mode);
        // todo: deal with landscape vs portrait
        // fixme: when dealing with non 16:9, since ges pipeline puts it in 16:9 container,
        //          the percents will be with respect to the container and not video. But conversion
        //          is based on the videos aspect ratio.

        let height_relative_to_width =
            (self.bottom_y.get() - self.top_y.get()) / self.asepct_ratio.get() as f32;
        let new_width = match self.crop_mode.get() {
            CropMode::Free => self.right_x.get() - self.left_x.get(),
            CropMode::Original => height_relative_to_width * self.asepct_ratio.get() as f32,
            _ => height_relative_to_width * mode.value(),
        };

        // todo: deal with new width being too big
        let new_targ_width = new_width + self.left_x.get();
        self.right_x.set(new_targ_width);
    }
}

impl crate::ui::CropBoxWidget {
    fn box_respects_aspect_ratio(&self) -> bool {
        let w = self.right_x() - self.left_x();
        let h = self.bottom_y() - self.top_y();
        let ar = (w * self.asepct_ratio() as f32) / h;

        match self.crop_mode() {
            CropMode::Free => true,
            CropMode::Original => w - h < f32::EPSILON,
            _ => self.crop_mode().value() - ar < f32::EPSILON,
        }
    }
    fn update_box(&self, x: f32, y: f32, changing_left_x: bool, changing_top_y: bool) {
        // todo: respect aspect ratio now
        if changing_left_x && x < self.right_x() {
            self.set_left_x(x);
        } else if !changing_left_x && x > self.left_x() {
            self.set_right_x(x);
        }

        if changing_top_y && y < self.bottom_y() {
            self.set_top_y(y);
        } else if !changing_top_y && y > self.top_y() {
            self.set_bottom_y(y);
        }
    }

    pub fn get_cordinate_percent_from_drag(&self, x: f64, y: f64) -> (f64, f64) {
        let (left_x, top_y, preview_width, preview_height) = self
            .imp()
            .get_preview_rect(self.width() as f32, self.height() as f32);

        let x_adj = (x - left_x as f64).clamp(0., preview_width as f64);
        let y_adj = (y - top_y as f64).clamp(0., preview_height as f64);

        (x_adj / preview_width as f64, y_adj / preview_height as f64)
    }

    pub fn is_point_in_handle(&self, x: f32, y: f32) {
        let target_point = graphene::Point::new(x, y);

        let handle_centers = self
            .imp()
            .get_handle_centers(self.width() as f32, self.height() as f32);

        let mut point_in_circle = false;

        for (idx, point) in handle_centers.iter().enumerate() {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&point, MARGIN);
            let circle = path_builder.to_path();

            if circle.in_fill(&target_point, HANDLE_FILL_RULE) {
                let handle = match idx {
                    0 => HandleType::TopLeft,
                    1 => HandleType::BottomLeft,
                    2 => HandleType::TopRight,
                    3 => HandleType::BottomRight,
                    _ => panic!("too many handle indicies"),
                };
                self.set_active_handle(handle);
                point_in_circle = true;
                break;
            }
        }

        self.set_drag_active(point_in_circle);
    }

    pub fn update_drag_pos(&self, x: f64, y: f64) {
        if !self.drag_active() {
            return;
        }

        let (x_percent, y_percent) = self.get_cordinate_percent_from_drag(x, y);
        let x = x_percent as f32;
        let y = y_percent as f32;

        match self.active_handle() {
            HandleType::TopLeft => {
                self.update_box(x, y, true, true);
            }
            HandleType::BottomLeft => {
                self.update_box(x, y, true, false);
            }
            HandleType::TopRight => {
                self.update_box(x, y, false, true);
            }
            HandleType::BottomRight => {
                self.update_box(x, y, false, false);
            }
        }
    }

    pub fn reset_box(&self) {
        self.set_top_y(0f32);
        self.set_left_x(0f32);
        self.set_bottom_y(1f32);
        self.set_right_x(1f32);

        self.set_asepct_ratio(0f64);
    }
}
