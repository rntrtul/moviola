use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExt, SnapshotExtManual, WidgetExt};
use gtk4::subclass::prelude::{
    DerivedObjectProperties, ObjectImpl, ObjectSubclass, ObjectSubclassExt,
};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, glib, graphene, gsk, Snapshot};
use relm4::gtk;

pub static MARGIN: f32 = 4.;

enum CircleType {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy)]
pub enum CropType {
    CropFree = 0,
    CropOriginal,
    CropSquare,
    Crop5To4,
    Crop4To3,
    Crop3To2,
    Crop16To9,
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
    drag_circle: Cell<i32>,
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

        // removing margin from dimensions to get actual video dimensions
        let right_x = Self::get_box_width(widget.width() as f32, widget.target_width());
        let bottom_y = Self::get_box_height(widget.height() as f32, widget.target_height());

        let left_x = (widget.width() as f32 * widget.x()) + MARGIN;
        let top_y = (widget.height() as f32 * widget.y()) + MARGIN;

        let border_rect = graphene::Rect::new(left_x, top_y, right_x, bottom_y);

        let border = gsk::RoundedRect::from_rect(border_rect, 0.);
        let border_widths = [1.; 4];
        let border_colours = [gdk::RGBA::GREEN; 4];

        let thirds_box_stroke = gsk::Stroke::builder(1.).build();

        let horizontal_step = (right_x - left_x) / 3.;
        for step in 1..3 {
            let x_step = horizontal_step * step as f32;
            let x = left_x + x_step;

            let path_builder = gsk::PathBuilder::new();
            path_builder.move_to(x, top_y);
            path_builder.line_to(x, bottom_y);

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &thirds_box_stroke, &gdk::RGBA::GREEN);
        }

        let vertical_step = (bottom_y - top_y) / 3.;
        for step in 1..3 {
            let y_step = vertical_step * step as f32;
            let y = y_step + top_y;

            let path_builder = gsk::PathBuilder::new();
            path_builder.move_to(left_x, y);
            path_builder.line_to(right_x, y);

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &thirds_box_stroke, &gdk::RGBA::GREEN);
        }

        let circle_points = CropBoxWidget::get_circle_points(
            widget.width() as f32,
            widget.height() as f32,
            widget.target_width(),
            widget.target_height(),
            widget.x(),
            widget.y(),
        );

        for point in circle_points {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&point, MARGIN);
            let circle = path_builder.to_path();
            snapshot.append_fill(&circle, gsk::FillRule::Winding, &gdk::RGBA::GREEN);
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
            .build()
    }
}

// fixme: figure out scope of functions to allow &self and be called from widgetImpl and ui:CropBoxWidget
impl CropBoxWidget {
    fn get_box_width(widget_width: f32, target_width_precent: f32) -> f32 {
        (widget_width - (MARGIN * 2.)) * target_width_precent
    }

    fn get_box_height(widget_height: f32, target_height_precent: f32) -> f32 {
        (widget_height - (MARGIN * 2.)) * target_height_precent
    }

    fn get_circle_points(
        widget_width: f32,
        widget_height: f32,
        target_width: f32,
        target_height: f32,
        x: f32,
        y: f32,
    ) -> [graphene::Point; 4] {
        let right_x = Self::get_box_width(widget_width, target_width) + MARGIN;
        let bottom_y = Self::get_box_height(widget_height, target_height) + MARGIN;

        let left_x = (widget_width * x) + MARGIN;
        let top_y = (widget_height * y) + MARGIN;

        println!("for with {left_x}x{top_y} until {right_x}x{bottom_y}");

        [
            graphene::Point::new(left_x, top_y),
            graphene::Point::new(left_x, bottom_y),
            graphene::Point::new(right_x, top_y),
            graphene::Point::new(right_x, bottom_y),
        ]
    }
}

impl crate::ui::CropBoxWidget {
    pub fn is_point_in_handle(&self, x: f64, y: f64) {
        // removing margin from dimensions to get actual video dimensions
        // todo: remove duplicate code

        let target_point = graphene::Point::new(x as f32, y as f32);

        let circle_points = CropBoxWidget::get_circle_points(
            self.width() as f32,
            self.height() as f32,
            self.target_width(),
            self.target_height(),
            self.x(),
            self.y(),
        );

        for (idx, point) in circle_points.iter().enumerate() {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&point, MARGIN);
            let circle = path_builder.to_path();

            if circle.in_fill(&target_point, gsk::FillRule::Winding) {
                println!("Circle num {idx} was clicked");
                self.set_drag_circle(idx as i32);
                break;
            }
        }
    }

    pub fn update_drag_pos(&self, x: f32, y: f32) {
        // circle 0: update x, y
        //        1: update x, target_height
        //        2: update target_width, y
        //        3: update target_width ,target_height
        // todo: use enum for circl position
        // todo: clamp x and y values
        // println!("drag update {x} {y}");
        match self.drag_circle() {
            0 => {
                self.set_x(x);
                self.set_y(y);
            }
            1 => {
                // println!("x{x} y{y}");
                self.set_x(x);
                self.set_target_height(y);
            }
            2 => {
                self.set_target_width(x);
                self.set_y(y);
            }
            3 => {
                self.set_target_width(x);
                self.set_target_height(y);
            }
            _ => {}
        }
    }
}
