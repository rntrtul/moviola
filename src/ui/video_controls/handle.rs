use crate::ui::IGNORE_OVERLAY_COLOUR;
use relm4::gtk;
use relm4::gtk::prelude::{ObjectExt, SnapshotExt, SnapshotExtManual, WidgetExt};
use relm4::gtk::subclass::prelude::*;
use relm4::gtk::{gdk, glib, graphene, gsk, Snapshot};
use std::cell::Cell;
use std::sync::LazyLock;

static FILL_RULE: gsk::FillRule = gsk::FillRule::Winding;
pub static HANDLE_WIDTH: f32 = 20f32;
pub static HANDLE_HEIGHT: f32 = 5f32;
static HANDLE_CURVE: LazyLock<graphene::Size> = LazyLock::new(|| graphene::Size::new(3f32, 6f32));
static ARROW_WIDTH: f32 = 6f32;
static ARROW_INSET: f32 = (HANDLE_WIDTH - ARROW_WIDTH) / 2.0;
static ARROW_HEIGHT_OFFSET: LazyLock<f32> = LazyLock::new(|| 1.0f32.tan() * ARROW_WIDTH);
static SEEK_BAR_WIDTH: f32 = 4f32;
static SEEK_BAR_OFFSET: f32 = HANDLE_WIDTH - (SEEK_BAR_WIDTH / 2f32);
static SEEK_COLOUR: gdk::RGBA = gdk::RGBA::WHITE;
static BORDER_COLOUR: LazyLock<gdk::RGBA> = LazyLock::new(|| gdk::RGBA::parse("#565656").unwrap());

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
        let widget = self.obj();

        if self.start_x.get() != 0f32 {
            let start_not_playing_rect = graphene::Rect::new(
                HANDLE_WIDTH,
                HANDLE_HEIGHT,
                self.start_left_x(),
                widget.height() as f32 - (2.0 * HANDLE_HEIGHT),
            );

            snapshot.append_color(&IGNORE_OVERLAY_COLOUR, &start_not_playing_rect);
        }

        if self.end_x.get() != 1f32 {
            let end_left_x = self.end_left_x();
            let end_not_playing_rect = graphene::Rect::new(
                end_left_x + HANDLE_WIDTH,
                HANDLE_HEIGHT,
                (widget.width() as f32 - end_left_x) - (HANDLE_WIDTH * 2f32),
                widget.height() as f32 - (2.0 * HANDLE_HEIGHT),
            );

            snapshot.append_color(&IGNORE_OVERLAY_COLOUR, &end_not_playing_rect);
        }

        let border = graphene::Rect::new(
            self.start_left_x() + HANDLE_WIDTH,
            0.0,
            self.end_left_x() - self.start_left_x() - HANDLE_WIDTH,
            widget.height() as f32,
        );
        snapshot.append_border(
            &gsk::RoundedRect::from_rect(border, 0f32),
            &[HANDLE_HEIGHT, 0f32, HANDLE_HEIGHT, 0f32],
            &[*BORDER_COLOUR; 4],
        );

        snapshot.append_fill(&self.seek_bar_path(), FILL_RULE, &SEEK_COLOUR);
        snapshot.append_fill(&self.start_handle_path(), FILL_RULE, &*BORDER_COLOUR);
        snapshot.append_fill(&self.end_handle_path(), FILL_RULE, &*BORDER_COLOUR);

        let arrow_stroke = gsk::Stroke::builder(4f32)
            .line_cap(gsk::LineCap::Round)
            .build();
        snapshot.append_stroke(&self.left_arrow_path(), &arrow_stroke, &SEEK_COLOUR);
        snapshot.append_stroke(&self.right_arrow_path(), &arrow_stroke, &SEEK_COLOUR);
    }
}

impl HandleWidget {
    fn marginless_width(&self) -> f32 {
        self.obj().width() as f32 - (HANDLE_WIDTH * 2f32)
    }

    fn start_left_x(&self) -> f32 {
        self.start_x.get() * self.marginless_width()
    }

    fn end_left_x(&self) -> f32 {
        (self.end_x.get() * self.marginless_width()) + HANDLE_WIDTH
    }

    fn start_handle_path(&self) -> gsk::Path {
        let left_x = self.start_left_x();
        let handle_rect =
            graphene::Rect::new(left_x, 0.0, HANDLE_WIDTH, self.obj().height() as f32);

        let handle_outline = gsk::RoundedRect::new(
            handle_rect,
            *HANDLE_CURVE,
            graphene::Size::zero(),
            graphene::Size::zero(),
            *HANDLE_CURVE,
        );

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&handle_outline);
        path_builder.to_path()
    }

    fn end_handle_path(&self) -> gsk::Path {
        let handle_rect = graphene::Rect::new(
            self.end_left_x(),
            0.0,
            HANDLE_WIDTH,
            self.obj().height() as f32,
        );
        let handle_outline = gsk::RoundedRect::new(
            handle_rect,
            graphene::Size::zero(),
            *HANDLE_CURVE,
            *HANDLE_CURVE,
            graphene::Size::zero(),
        );

        let path_builder = gsk::PathBuilder::new();
        path_builder.add_rounded_rect(&handle_outline);
        path_builder.to_path()
    }

    fn left_arrow_path(&self) -> gsk::Path {
        let path = gsk::PathBuilder::new();

        let x = self.start_left_x();
        let y = self.obj().height() as f32 / 2.0;

        path.move_to(x + ARROW_WIDTH + ARROW_INSET, y - *ARROW_HEIGHT_OFFSET);
        path.line_to(x + ARROW_INSET, y);
        path.line_to(x + ARROW_WIDTH + ARROW_INSET, y + *ARROW_HEIGHT_OFFSET);

        path.to_path()
    }

    fn right_arrow_path(&self) -> gsk::Path {
        let path = gsk::PathBuilder::new();

        let x = self.end_left_x();
        let y = self.obj().height() as f32 / 2.0;

        path.move_to(x + ARROW_INSET, y - *ARROW_HEIGHT_OFFSET);
        path.line_to(x + ARROW_WIDTH + ARROW_INSET, y);
        path.line_to(x + ARROW_INSET, y + *ARROW_HEIGHT_OFFSET);

        path.to_path()
    }

    fn seek_bar_path(&self) -> gsk::Path {
        let bar_rect = graphene::Rect::new(
            self.seek_x.get() * self.marginless_width() + SEEK_BAR_OFFSET,
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

impl super::HandleWidget {
    pub fn drag_start(&self, x: f64, y: f64) {
        let point = graphene::Point::new(x as f32, y as f32);

        let start_path = self.imp().start_handle_path();
        let end_path = self.imp().end_handle_path();

        self.set_is_start_dragging(start_path.in_fill(&point, FILL_RULE));
        self.set_is_end_dragging(end_path.in_fill(&point, FILL_RULE));
    }

    pub fn drag_update(&self, x: f32) {
        let x_adj = x - HANDLE_WIDTH;
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

impl Default for super::HandleWidget {
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
