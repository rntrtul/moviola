use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{gdk, glib, graphene, gsk, Orientation, Snapshot};
use relm4::gtk;

#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::HandleWidget)]
pub struct HandleWidget {
    #[property(get, set)]
    pub x: Cell<i32>,
    #[property(get, set)]
    pub rel_x: Cell<i32>,
    #[property(get, set)]
    pub target_x: Cell<i32>,
    #[property(get, set)]
    pub percent_pos: Cell<f64>,
    #[property(get, set)]
    is_handle: Cell<bool>,
    #[property(get, set)]
    is_start: Cell<bool>,
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
    fn measure(&self, orientation: Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        if orientation == Orientation::Horizontal {
            // calc width range
            if self.is_handle.get() {
                (10, 10, -1, -1)
            } else {
                (5, 5, -1, -1)
            }
        } else {
            // height range
            (20, 200, -1, -1)
        }
    }

    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();

        let height_percent = if self.is_handle.get() { 0.75 } else { 1.0 };
        let instep_percent = if self.is_handle.get() { 0.125 } else { 0. };

        let target_height = (widget.height() as f32) * height_percent;
        let y_instep = widget.height() as f32 * instep_percent;
        // todo: have shadow on handle?
        // todo: have center of widget be considered 0

        let handle_rect = graphene::Rect::new(
            self.rel_x.get() as f32,
            y_instep,
            widget.width() as f32,
            target_height,
        );
        let handle_outline = gsk::RoundedRect::from_rect(handle_rect, 6f32);

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&handle_outline);
        let path = path_builder.to_path();

        let colour = if self.is_handle.get() {
            &gdk::RGBA::WHITE
        } else {
            &gdk::RGBA::BLUE
        };

        snapshot.append_fill(&path, gsk::FillRule::Winding, colour);

        if self.is_handle.get() {
            let is_moving = self.rel_x.get() != 0;

            let (overlay_start, overlay_width) = if self.is_start.get() {
                (-self.x.get() as f32, self.target_x.get() as f32)
            } else {
                if is_moving {
                    (
                        (self.x.get() + widget.width()) as f32,
                        (-self.x.get() + self.rel_x.get()) as f32,
                    )
                } else {
                    (widget.width() as f32, self.x.get() as f32)
                }
            };

            let not_playing_rect =
                graphene::Rect::new(overlay_start, 0., overlay_width, widget.height() as f32);
            let grey = gdk::RGBA::new(0.612, 0.612, 0.612, 0.89);
            snapshot.append_color(&grey, &not_playing_rect);
        }
    }
}

impl HandleWidget {
    pub fn set_rel_x(&self, pos: i32) {
        self.rel_x.set(pos);
    }

    pub fn set_target_x(&self, pos: i32) {
        self.target_x.set(pos);
    }
}

impl crate::ui::HandleWidget {
    fn new(x: i32, is_handle: bool, is_start: bool) -> Self {
        glib::Object::builder()
            .property("x", x)
            .property("rel_x", 0)
            .property("is_handle", is_handle)
            .property("is_start", is_start)
            .build()
    }
}

impl Default for crate::ui::HandleWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0)
            .property("rel_x", 0)
            .property("is_handle", true)
            .property("is_start", true)
            .build()
    }
}
