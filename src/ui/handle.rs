use std::cell::Cell;

use gtk4::prelude::{ObjectExt, SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{gdk, glib, graphene, gsk, Snapshot};
use relm4::gtk;

use crate::ui::IGNORE_OVERLAY_COLOUR;

static FILL_RULE: gsk::FillRule = gsk::FillRule::Winding;
pub static HANDLE_WIDTH: f32 = 10f32;
static SEEK_BAR_WIDTH: f32 = 5f32;

#[derive(glib::Properties, Default, Debug)]
#[properties(wrapper_type = super::HandleWidget)]
pub struct HandleWidget {
    #[property(get, set)]
    start_x: Cell<f32>,
    #[property(get, set)]
    end_x: Cell<f32>,
    #[property(get, set)]
    seek_x: Cell<f32>,
    #[property(get, set)]
    is_start_dragging: Cell<bool>,
    #[property(get, set)]
    is_end_dragging: Cell<bool>,
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
    fn snapshot(&self, snapshot: &Snapshot) {
        // todo: have shadow on handle?
        let widget = self.obj();

        if self.start_x.get() != 0f32 {
            let start_not_playing_rect = graphene::Rect::new(
                HANDLE_WIDTH,
                0.,
                self.start_left_x(),
                widget.height() as f32,
            );

            snapshot.append_color(&IGNORE_OVERLAY_COLOUR, &start_not_playing_rect);
        }

        if self.end_x.get() != 1f32 {
            let end_left_x = self.end_left_x();
            let end_not_playing_rect = graphene::Rect::new(
                end_left_x + HANDLE_WIDTH,
                0.,
                (widget.width() as f32 - end_left_x) - (HANDLE_WIDTH * 2f32),
                widget.height() as f32,
            );

            snapshot.append_color(&IGNORE_OVERLAY_COLOUR, &end_not_playing_rect);
        }

        snapshot.append_fill(&self.seek_bar_path(), FILL_RULE, &gdk::RGBA::WHITE);
        snapshot.append_fill(&self.start_handle_path(), FILL_RULE, &gdk::RGBA::WHITE);
        snapshot.append_fill(&self.end_handle_path(), FILL_RULE, &gdk::RGBA::WHITE);
    }
}

impl HandleWidget {
    fn marginless_width(&self) -> f32 {
        self.obj().width() as f32 - (HANDLE_WIDTH * 2f32)
    }

    fn start_left_x(&self) -> f32 {
        let width = self.marginless_width();
        self.start_x.get() * width
    }

    fn end_left_x(&self) -> f32 {
        // fixme: left edge not where it should be
        let width = self.marginless_width();
        (self.end_x.get() * width) + HANDLE_WIDTH
    }

    fn start_handle_path(&self) -> gsk::Path {
        let left_x = self.start_left_x();
        let handle_rect =
            graphene::Rect::new(left_x, 0f32, HANDLE_WIDTH, self.obj().height() as f32);
        let handle_outline = gsk::RoundedRect::from_rect(handle_rect, 6f32);

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&handle_outline);
        path_builder.to_path()
    }

    fn end_handle_path(&self) -> gsk::Path {
        let handle_rect = graphene::Rect::new(
            self.end_left_x(),
            0f32,
            HANDLE_WIDTH,
            self.obj().height() as f32,
        );
        let handle_outline = gsk::RoundedRect::from_rect(handle_rect, 6f32);

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&handle_outline);
        path_builder.to_path()
    }

    fn seek_bar_path(&self) -> gsk::Path {
        let width = self.marginless_width();

        let bar_rect = graphene::Rect::new(
            self.seek_x.get() * width,
            0f32,
            SEEK_BAR_WIDTH,
            self.obj().height() as f32,
        );
        let bar_outline = gsk::RoundedRect::from_rect(bar_rect, 6f32);

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&bar_outline);
        path_builder.to_path()
    }
}

impl crate::ui::HandleWidget {
    pub fn drag_start(&self, x: f64, y: f64) {
        let point = graphene::Point::new(x as f32, y as f32);

        let start_path = self.imp().start_handle_path();
        let end_path = self.imp().end_handle_path();

        self.set_is_start_dragging(start_path.in_fill(&point, FILL_RULE));
        self.set_is_end_dragging(end_path.in_fill(&point, FILL_RULE));
    }

    pub fn drag_update(&self, x: f32) {
        let x_adj = if self.is_start_dragging() || self.is_end_dragging() {
            x - HANDLE_WIDTH
        } else {
            x - SEEK_BAR_WIDTH
        };
        let percent = x_adj / (self.width() as f32 - (HANDLE_WIDTH * 2f32));

        if self.is_start_dragging() {
            self.set_start_x(percent.clamp(0f32, self.end_x()));
        } else if self.is_end_dragging() {
            self.set_end_x(percent.clamp(self.start_x(), 1f32));
        }

        self.set_seek_x(percent.clamp(0f32, 1f32));
    }

    pub fn drag_end(&self) {
        self.set_is_end_dragging(false);
        self.set_is_start_dragging(false);
    }

    pub fn reset(&self) {
        self.set_start_x(0f32);
        self.set_end_x(1f32);
        self.set_seek_x(0f32);
        self.set_is_end_dragging(false);
        self.set_is_start_dragging(false);
    }
}

impl Default for crate::ui::HandleWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("start_x", 0f32)
            .property("end_x", 1f32)
            .property("seek_x", 0f32)
            .property("is_start_dragging", false)
            .property("is_end_dragging", false)
            .build()
    }
}
