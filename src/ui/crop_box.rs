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

// properties are represented in percentages since preview is not 1:1 to output
#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::CropBoxWidget)]
pub struct CropBoxWidget {
    #[property(get, set)]
    pub x: Cell<f32>,
    #[property(get, set)]
    pub y: Cell<f32>,
    #[property(get, set)]
    pub target_width: Cell<f32>,
    #[property(get, set)]
    pub target_height: Cell<f32>,
    #[property(get, set)]
    pub drag_active: Cell<bool>,
    #[property(get, set = Self::set_aspect_ratio)]
    pub asepct_ratio: Cell<f64>,
    #[property(get, set, builder(HandleType::TopLeft))]
    active_handle: Cell<HandleType>,
    #[property(get, set, builder(CropMode::Free))]
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
        let border_colours = [gdk::RGBA::GREEN; 4];

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
            snapshot.append_fill(&handle, HANDLE_FILL_RULE, &gdk::RGBA::GREEN);
        }

        snapshot.append_border(&border, &border_widths, &border_colours);
    }
}

impl Default for crate::ui::CropBoxWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0f32)
            .property("y", 0f32)
            .property("target_width", 1f32)
            .property("target_height", 1f32)
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
            snapshot.append_stroke(&line, &thirds_box_stroke, &gdk::RGBA::GREEN);
        }
    }

    fn get_box_bounds(&self, widget_width: f32, widget_height: f32) -> (f32, f32, f32, f32) {
        let left_x = (widget_width * self.x.get()) + MARGIN;
        let top_y = (widget_height * self.y.get()) + MARGIN;

        // subtract margin to convert percent to actual frame cords
        let right_x = (widget_width - (MARGIN * 2.)) * self.target_width.get() + MARGIN;
        let bottom_y = (widget_height - (MARGIN * 2.)) * self.target_height.get() + MARGIN;

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

    pub fn get_cordinate_percent_from_drag(width: i32, height: i32, x: f64, y: f64) -> (f32, f32) {
        let frame_width = width as f32 - (MARGIN * 2.);
        let frame_height = height as f32 - (MARGIN * 2.);

        let x_adj = (x as f32 - MARGIN).clamp(0., frame_width);
        let y_adj = (y as f32 - MARGIN).clamp(0., frame_height);

        (x_adj / frame_width, y_adj / frame_height)
    }

    pub fn set_aspect_ratio(&self, aspect_ratio: f64) {
        // todo: update target x,y
        self.asepct_ratio.set(aspect_ratio);
    }
}

impl crate::ui::CropBoxWidget {
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

    pub fn update_drag_pos(&self, x: f32, y: f32) {
        if !self.drag_active() {
            return;
        }

        match self.active_handle() {
            HandleType::TopLeft => {
                if x < self.target_width() {
                    self.set_x(x);
                }

                if y < self.target_height() {
                    self.set_y(y);
                }
            }
            HandleType::BottomLeft => {
                if x < self.target_width() {
                    self.set_x(x);
                }

                if y > self.y() {
                    self.set_target_height(y);
                }
            }
            HandleType::TopRight => {
                if x > self.x() {
                    self.set_target_width(x);
                }

                if y < self.target_height() {
                    self.set_y(y);
                }
            }
            HandleType::BottomRight => {
                if x > self.x() {
                    self.set_target_width(x);
                }

                if y > self.y() {
                    self.set_target_height(y);
                }
            }
        }
    }
}
