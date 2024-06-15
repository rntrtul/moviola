use std::cell::Cell;

use gtk4::{gdk, glib, graphene, gsk, Orientation, Snapshot};
use gtk4::prelude::{ObjectExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::*;
use relm4::gtk;

#[derive(glib::Properties, Default)]
#[properties(wrapper_type = super::HandleWidget)]
pub struct HandleWidget {
    #[property(get, set)]
    pub x: Cell<i32>,
    #[property(get, set)]
    pub rel_x: Cell<i32>,
}

//     todo: have setting for thickness and height percent of parent?
//              or just do type and have those settings pre determined?


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
        // println!("{:?}, {}", orientation, for_size);
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
        let y_instep = widget.height() as f32 * 0.125;
        // fixme: limit handle to bounds of the timeline (snaps to it afterwards)

        let rect = graphene::Rect::new(self.rel_x.get() as f32, y_instep, widget.width() as f32, target_height);
        let round_rect = gsk::RoundedRect::from_rect(rect, 6f32);

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&round_rect);
        let path = path_builder.to_path();

        snapshot.append_fill(&path, gsk::FillRule::Winding, &gdk::RGBA::WHITE);
    }
}

impl HandleWidget {
    pub fn set_x(&mut self, pos: i32) {
        self.x.set(pos);
    }

    pub fn set_rel_x(&mut self, pos: i32) {
        self.rel_x.set(pos);
    }
}
