use ges::glib;
use ges::subclass::prelude::{ObjectImpl, ObjectSubclass};
use gtk4::prelude::{SnapshotExt, WidgetExt};
use gtk4::subclass::prelude::{ObjectSubclassExt, ObjectSubclassIsExt};
use gtk4::subclass::widget::WidgetImpl;
use gtk4::{gdk, graphene, gsk, Orientation, Snapshot};
use std::cell::Cell;

pub struct Slider {
    value: Cell<f32>,
    min_value: Cell<f32>,
    max_value: Cell<f32>,
    default_value: Cell<f32>,
    value_step_size: Cell<f32>,
    show_ticks: Cell<bool>,
    show_bar: Cell<bool>,
    fill_colour: Cell<gdk::RGBA>,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            value: Cell::new(0.0),
            min_value: Cell::new(-1.0),
            max_value: Cell::new(1.0),
            default_value: Cell::new(0.0),
            value_step_size: Cell::new(0.01),
            show_ticks: Cell::new(false),
            show_bar: Cell::new(true),
            fill_colour: Cell::new(gdk::RGBA::new(0.5, 0.5, 0.5, 0.5)),
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
        snapshot.append_color(
            &gdk::RGBA::new(0.294, 0.294, 0.294, 1.0),
            &graphene::Rect::new(
                0f32,
                0f32,
                self.value_to_width_percent(),
                widget.height() as f32,
            ),
        );
        self.draw_tickmarks(snapshot);
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

    fn percent_to_stepped_value(&self, percent: f32) -> f32 {
        let value = (percent * self.value_range()) + self.min_value.get();
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

    fn value_range(&self) -> f32 {
        self.max_value.get() - self.min_value.get()
    }

    fn value_as_range_percent(&self) -> f32 {
        // fixme: handle negative values as valid (0 could be 50%)
        (self.value.get() - self.min_value.get()) / self.value_range()
    }

    fn value_to_width_percent(&self) -> f32 {
        self.obj().width() as f32 * self.value_as_range_percent()
    }
}

impl crate::ui::slider::Slider {
    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }

    pub(crate) fn drag_update(&self, target: f64) {
        let percent = (target / self.width() as f64).clamp(0.0, 1.0);
        let value = self.imp().percent_to_stepped_value(percent as f32);

        self.imp().value.set(value);
        self.queue_draw();
    }

    pub(crate) fn value(&self) -> f32 {
        self.imp().value.get()
    }
}
