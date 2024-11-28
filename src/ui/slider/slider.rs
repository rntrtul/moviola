use ges::glib;
use ges::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::prelude::{SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::{ObjectSubclassExt, ObjectSubclassIsExt};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, graphene, gsk, Orientation, Snapshot};
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

impl Default for Range {
    fn default() -> Self {
        Range {
            min: -1.0,
            max: 1.0,
        }
    }
}

impl Range {
    pub fn new(min: f64, max: f64) -> Self {
        Range { min, max }
    }

    pub fn distance(&self) -> f64 {
        self.max - self.min
    }

    pub fn percent_from_value(&self, value: f64) -> f64 {
        (value - self.min) / self.distance()
    }

    pub fn value_from_percent(&self, percent: f64) -> f64 {
        (self.distance() * percent) + self.min
    }

    pub fn map_value_from_range(&self, range: Range, value: f64) -> f64 {
        self.value_from_percent(range.percent_from_value(value))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SliderFillMode {
    EdgeToEdge,
    CenterOut,
}

pub struct Slider {
    value: Cell<f64>,
    value_range: Cell<Range>,
    default_value: Cell<f64>,
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
            default_value: Cell::new(0.0),
            value_step_size: Cell::new(0.01),
            show_ticks: Cell::new(true),
            show_bar: Cell::new(true),
            fill_mode: Cell::new(SliderFillMode::EdgeToEdge),
            fill_colour: Cell::new(gdk::RGBA::new(0.29, 0.29, 0.29, 1.0)),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Slider {
    const NAME: &'static str = "Slider";
    type Type = super::Slider;
    type ParentType = gtk4::Widget;
}

impl ObjectImpl for Slider {}

impl WidgetImpl for Slider {
    fn measure(&self, _orientation: Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        (0, 50, 0, 0)
    }
    fn snapshot(&self, snapshot: &Snapshot) {
        let widget = self.obj();
        snapshot.save();
        let range = self.value_range.get();
        let value = self.value.get();

        let fill_rect = match self.fill_mode.get() {
            SliderFillMode::EdgeToEdge => graphene::Rect::new(
                0f32,
                0f32,
                self.value_to_width_percent() as f32,
                widget.height() as f32,
            ),
            SliderFillMode::CenterOut => {
                let center_x = widget.width() as f32 / 2.0;
                let rel_percent = range.percent_from_value(value) - 0.50;
                let fill_width = (widget.width() as f64 * rel_percent).abs() as f32;

                let (start_x, width) = if rel_percent < 0.0 {
                    (center_x - fill_width, fill_width)
                } else {
                    (center_x, fill_width)
                };

                graphene::Rect::new(start_x, 0f32, width, widget.height() as f32)
            }
        };

        snapshot.append_color(&self.fill_colour.get(), &fill_rect);

        if self.show_ticks.get() {
            self.draw_tickmarks(snapshot);
        }

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

    pub(crate) fn new_with_range(
        range: Range,
        default_value: f64,
        fill_mode: SliderFillMode,
    ) -> Self {
        let obj: Self = glib::Object::builder().build();

        obj.imp().value_range.set(range);
        obj.imp().default_value.set(default_value);
        obj.imp().value.set(default_value);
        obj.imp().fill_mode.set(fill_mode);
        obj
    }

    pub(crate) fn drag_update(&self, target: f64) {
        let percent = (target / self.width() as f64).clamp(0.0, 1.0);
        let value = self.imp().percent_to_stepped_value(percent);

        self.imp().value.set(value);
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
