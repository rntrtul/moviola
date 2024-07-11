use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExt, SnapshotExtManual, WidgetExt};
use gtk4::subclass::prelude::{
    DerivedObjectProperties, ObjectImpl, ObjectSubclass, ObjectSubclassExt,
};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, glib, graphene, gsk, Snapshot};
use relm4::gtk;

// todo: better name. RADIUS, INSTEP, MARGIN
pub static OFFSET: f32 = 4.;

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

#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::CropBoxWidget)]
pub struct CropBoxWidget {
    #[property(get, set)]
    pub x: Cell<i32>,
    #[property(get, set)]
    pub y: Cell<i32>,
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
        let width = widget.width() as f32 - (OFFSET * 2.);
        let height = widget.height() as f32 - (OFFSET * 2.);

        let border_rect = graphene::Rect::new(OFFSET, OFFSET, width, height);

        let border = gsk::RoundedRect::from_rect(border_rect, 0.);
        let border_widths = [1.; 4];
        let border_colours = [gdk::RGBA::GREEN; 4];

        let thirds_box_stroke = gsk::Stroke::builder(1.).build();

        let horizontal_step = width / 3.;
        for step in 1..3 {
            let x = horizontal_step * step as f32;

            let path_builder = gsk::PathBuilder::new();
            path_builder.move_to(x, OFFSET);
            path_builder.line_to(x, height);

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &thirds_box_stroke, &gdk::RGBA::GREEN);
        }

        let vertical_step = height / 3.;
        for step in 1..3 {
            let y = vertical_step * step as f32;

            let path_builder = gsk::PathBuilder::new();
            path_builder.move_to(OFFSET, y);
            path_builder.line_to(width, y);

            let line = path_builder.to_path();
            snapshot.append_stroke(&line, &thirds_box_stroke, &gdk::RGBA::GREEN);
        }

        let circle_points = [
            graphene::Point::new(OFFSET, OFFSET),
            graphene::Point::new(OFFSET, height + OFFSET),
            graphene::Point::new(width + OFFSET, OFFSET),
            graphene::Point::new(width + OFFSET, height + OFFSET),
        ];

        for point in circle_points {
            let path_builder = gsk::PathBuilder::new();
            path_builder.add_circle(&point, OFFSET);
            let circle = path_builder.to_path();
            snapshot.append_fill(&circle, gsk::FillRule::Winding, &gdk::RGBA::GREEN);
        }

        snapshot.append_border(&border, &border_widths, &border_colours);
    }
}

impl Default for crate::ui::CropBoxWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0)
            .property("y", 0)
            .build()
    }
}
