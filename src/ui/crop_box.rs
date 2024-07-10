use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExtManual};
use gtk4::subclass::prelude::{
    DerivedObjectProperties, ObjectImpl, ObjectSubclass, ObjectSubclassExt,
};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, glib, graphene, gsk, Orientation, Snapshot};
use relm4::gtk;

#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::CropBoxWidget)]
pub struct CropBoxWidget {
    #[property(get, set)]
    pub x: Cell<i32>,
    #[property(get, set)]
    pub y: Cell<i32>,
    #[property(get, set)]
    pub width: Cell<i32>,
    #[property(get, set)]
    pub height: Cell<i32>,
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
    fn measure(&self, _orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        // do max fill on parent size
        (for_size, for_size, -1, -1)
    }

    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();

        let border_rect =
            graphene::Rect::new(0., 0., widget.width() as f32, widget.height() as f32);

        let border = gsk::RoundedRect::from_rect(border_rect, 0.);
        let border_widths = [5.; 4];
        let border_colours = [gdk::RGBA::GREEN; 4];

        snapshot.append_border(&border, &border_widths, &border_colours);
    }
}

impl Default for crate::ui::CropBoxWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0)
            .property("y", 0)
            .property("width", 50)
            .property("height", 50)
            .build()
    }
}
