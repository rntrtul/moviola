mod bounding_box;
mod effects_pipeline;
mod pan;
mod preview;
mod zoom;

use gst::Sample;
use gtk4::glib;
use gtk4::prelude::{PaintableExt, WidgetExt};
use gtk4::subclass::prelude::ObjectSubclassIsExt;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CropMode {
    Free,
    Original,
    Square,
    _16To9,
    _4To5,
    _5To7,
    _4To3,
    _3To5,
    _3To2,
}

impl CropMode {
    fn value(&self) -> f32 {
        match *self {
            CropMode::Free => 0.,
            CropMode::Original => 0.,
            CropMode::Square => 1.,
            CropMode::_16To9 => 16. / 9.,
            CropMode::_4To3 => 4. / 3.,
            CropMode::_3To2 => 2. / 3.,
            CropMode::_4To5 => 4. / 5.,
            CropMode::_5To7 => 5. / 7.,
            CropMode::_3To5 => 3. / 5.,
        }
    }
}

pub struct BoundingBoxDimensions {
    pub(crate) left_x: f32,
    pub(crate) top_y: f32,
    pub(crate) right_x: f32,
    pub(crate) bottom_y: f32,
}

glib::wrapper! {
    pub struct Preview(ObjectSubclass<preview::Preview>)
        @extends gtk4::Widget;
}

impl Preview {
    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_crop_mode(&self, crop_modes: CropMode) {
        self.imp().crop_mode.set(crop_modes);
        self.imp().maintain_aspect_ratio();
        self.queue_draw();
    }

    pub fn show_crop_box(&self) {
        self.imp().show_crop_box.set(true);
        self.queue_draw();
    }

    pub fn hide_crop_box(&self) {
        self.imp().show_crop_box.set(false);
        self.queue_draw();
    }

    pub fn render_sample(&self, sample: Sample) {
        self.imp().render_sample(sample);
        self.queue_draw();
    }

    pub fn export_settings(&self) -> BoundingBoxDimensions {
        BoundingBoxDimensions {
            left_x: self.imp().left_x.get(),
            top_y: self.imp().top_y.get(),
            right_x: self.imp().right_x.get(),
            bottom_y: self.imp().bottom_y.get(),
        }
    }
}
