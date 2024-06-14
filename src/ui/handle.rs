use std::cell::Cell;

use gtk4::{gdk, glib, graphene, gsk, Orientation, Snapshot};
use gtk4::prelude::{ObjectExt, SnapshotExtManual, WidgetExt};
use gtk4::subclass::prelude::*;
use relm4::gtk;

#[derive(glib::Properties, Default)]
#[properties(wrapper_type = super::HandleWidget)]
pub struct HandleWidget {
    #[property(get, set)]
    pub x: Cell<f32>,
}

#[glib::object_subclass]
impl ObjectSubclass for HandleWidget {
    const NAME: &'static str = "HandleWidget";
    type Type = super::HandleWidget;
    type ParentType = gtk::Widget;
}

#[glib::derived_properties]
impl ObjectImpl for HandleWidget {}

impl WidgetImpl for HandleWidget {
    fn measure(&self, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        println!("{:?}, {}", orientation, for_size);
        if orientation == gtk::Orientation::Horizontal {
            //     calc min width
            (10, 10, -1, -1)
        } else {
            (20, 200, -1, -1)
        }
    }

    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();

        let target_height = (widget.height() as f32) * 0.75;
        println!("w: {}, h: {}", widget.width(), widget.height());
        let y_instep = widget.height() as f32 * 0.125;

        let blue_colour = gdk::RGBA::WHITE;
        let rect = graphene::Rect::new(self.x.get(), y_instep, widget.width() as f32, target_height);
        let round_rect = gsk::RoundedRect::from_rect(rect, 6f32);
        snapshot.append_border(&round_rect, &[2f32, 2f32, 2f32, 2f32], &[blue_colour, blue_colour, blue_colour, blue_colour]);
    }
}

impl HandleWidget {
    pub fn set_x(&mut self, pos: f32) {
        self.x.set(pos);
    }
}
