use crate::range::Range;
use gst::glib;
use gst::subclass::prelude::{ObjectImpl, ObjectSubclass};
use relm4::adw;
use relm4::gtk::prelude::{SnapshotExt, WidgetExt};
use relm4::gtk::subclass::prelude::{ObjectSubclassExt, ObjectSubclassIsExt};
use relm4::gtk::subclass::widget::WidgetImpl;
use relm4::gtk::{gdk, graphene, gsk, Orientation, Snapshot};
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
pub enum SliderFillMode {
    EdgeToEdge,
    CenterOut,
}

// todo: find better colours
static BACKGROUND_COLOUR: gdk::RGBA = gdk::RGBA::new(0.23, 0.23, 0.23, 1.0);
static FILL_COLOUR: gdk::RGBA = gdk::RGBA::new(0.3, 0.3, 0.3, 1.0);
static BAR_WIDTH: f32 = 6f32;
static BAR_OFFSET: f32 = BAR_WIDTH / 2.0;

pub struct Slider {
    value: Cell<f64>,
    value_range: Cell<Range>,
    value_step_size: Cell<f64>,
    show_ticks: Cell<bool>,
    show_bar: Cell<bool>,
    fill_mode: Cell<SliderFillMode>,
    fill_colour: Cell<gdk::RGBA>,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            value: Cell::new(0.0),
            value_range: Cell::new(Range::default()),
            value_step_size: Cell::new(0.005),
            show_ticks: Cell::new(true),
            show_bar: Cell::new(true),
            fill_mode: Cell::new(SliderFillMode::EdgeToEdge),
            fill_colour: Cell::new(FILL_COLOUR),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Slider {
    const NAME: &'static str = "Slider";
    type Type = super::Slider;
    type ParentType = relm4::gtk::Widget;
}

impl ObjectImpl for Slider {
    fn constructed(&self) {}
}

impl WidgetImpl for Slider {
    fn measure(&self, _orientation: Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        (0, 40, 0, 0)
    }
    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();
        snapshot.save();
        let range = self.value_range.get();
        let value = self.value.get();

        let widget_rect =
            graphene::Rect::new(0f32, 0f32, widget.width() as f32, widget.height() as f32);

        let rounded_size = graphene::Size::new(4.0, 6.0);
        snapshot.push_rounded_clip(&gsk::RoundedRect::new(
            widget_rect,
            rounded_size,
            rounded_size,
            rounded_size,
            rounded_size,
        ));

        let (fill_rect, bar_rect) = match self.fill_mode.get() {
            SliderFillMode::EdgeToEdge => {
                let x = self.value_to_width_percent() as f32;
                let bar_rect =
                    graphene::Rect::new(x - BAR_OFFSET, 0f32, BAR_WIDTH, widget.height() as f32);

                let fill_rect = graphene::Rect::new(
                    0f32,
                    0f32,
                    self.value_to_width_percent() as f32,
                    widget.height() as f32,
                );

                (fill_rect, bar_rect)
            }
            SliderFillMode::CenterOut => {
                let center_x = widget.width() as f32 / 2.0;
                let rel_percent = range.percent_from_value(value) - 0.50;
                let fill_width = (widget.width() as f64 * rel_percent).abs() as f32;

                let (start_x, width, far_edge_x) = if rel_percent < 0.0 {
                    (center_x - fill_width, fill_width, center_x - fill_width)
                } else {
                    (center_x, fill_width, center_x + fill_width)
                };

                let bar_rect = graphene::Rect::new(
                    far_edge_x - BAR_OFFSET,
                    0f32,
                    BAR_WIDTH,
                    widget.height() as f32,
                );

                let fill_rect = graphene::Rect::new(start_x, 0f32, width, widget.height() as f32);

                (fill_rect, bar_rect)
            }
        };

        snapshot.append_color(&BACKGROUND_COLOUR, &widget_rect);
        snapshot.append_color(&self.fill_colour.get(), &fill_rect);
        // todo: get actual accent colour
        if self.show_bar.get() {
            snapshot.append_color(&adw::AccentColor::Blue.to_rgba(), &bar_rect);
        }

        if self.show_ticks.get() {
            self.draw_tickmarks(snapshot);
        }

        snapshot.pop(); // popping clip

        snapshot.restore();
    }
}

impl Slider {
    fn draw_tickmarks(&self, snapshot: &Snapshot) {
        // todo: build path and store it. Avoid recomputation on every draw. Recompute when size changes.
        let ticks = 40;
        let short_length = 4;
        let long_length = 8;

        let path_builder = gsk::PathBuilder::new();
        let x_step = self.obj().width() as f32 / 40.0;

        for tick in 1..ticks {
            let length = if tick % 5 == 0 {
                long_length
            } else {
                short_length
            };

            path_builder.move_to(tick as f32 * x_step, 0f32);
            path_builder.line_to(tick as f32 * x_step, length as f32);
        }

        let line = path_builder.to_path();
        let stroke = gsk::Stroke::builder(1.).build();
        snapshot.append_stroke(&line, &stroke, &gdk::RGBA::WHITE);
    }

    fn percent_to_stepped_value(&self, percent: f64) -> f64 {
        let range = self.value_range.get();

        let value = (percent * range.distance()) + range.min;
        let remainder = value.abs() % self.value_step_size.get();

        if remainder == 0.0 {
            return value;
        }

        if value.is_sign_negative() {
            -(value.abs() - remainder)
        } else {
            value + self.value_step_size.get() - remainder
        }
    }

    fn value_to_width_percent(&self) -> f64 {
        self.obj().width() as f64 * self.value_range.get().percent_from_value(self.value.get())
    }
}

impl crate::ui::slider::Slider {
    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }

    pub(crate) fn new_with_range(range: Range, fill_mode: SliderFillMode) -> Self {
        let obj: Self = glib::Object::builder().build();

        obj.imp().value_range.set(range);
        obj.imp().value.set(range.default);
        obj.imp().fill_mode.set(fill_mode);
        obj
    }

    pub(crate) fn drag_update(&self, target: f64) {
        let percent = (target / self.width() as f64).clamp(0.0, 1.0);
        let value = self.imp().percent_to_stepped_value(percent);

        self.imp().value.set(value);
        self.queue_draw();
    }

    pub(crate) fn reset(&self) {
        self.imp().value.set(self.imp().value_range.get().default);
        self.queue_draw();
    }

    pub(crate) fn value(&self) -> f64 {
        self.imp().value.get()
    }

    pub(crate) fn value_as_range_percent(&self) -> f64 {
        self.imp()
            .value_range
            .get()
            .percent_from_value(self.imp().value.get())
    }

    pub(crate) fn map_value_to_range(&self, range: Range) -> f64 {
        range.map_value_from_range(self.imp().value_range.get(), self.imp().value.get())
    }
}
