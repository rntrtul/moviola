mod bounding_box;
mod effects_pipeline;
mod orient;
mod pan;
mod preview;
mod zoom;

use crate::ui::sidebar::CropExportSettings;
use gst::Sample;
use gtk4::glib;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

pub use crate::ui::preview::bounding_box::{BoundingBoxDimensions, CropMode};
pub use crate::ui::preview::orient::Orientation;

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
        self.imp().render_new_sample(sample);
        self.queue_draw();
    }

    pub fn export_settings(&self) -> CropExportSettings {
        CropExportSettings {
            bounding_box: BoundingBoxDimensions {
                left_x: self.imp().left_x.get(),
                top_y: self.imp().top_y.get(),
                right_x: self.imp().right_x.get(),
                bottom_y: self.imp().bottom_y.get(),
            },
            orientation: self.imp().orientation.get(),
        }
    }

    pub fn reset_preview(&self) {
        self.imp().left_x.set(0.0);
        self.imp().top_y.set(0.0);
        self.imp().right_x.set(1.0);
        self.imp().bottom_y.set(1.0);

        self.imp().zoom.set(1.0);
        self.imp().orientation.set(Orientation {
            angle: 0.0,
            mirrored: false,
        });
    }
}
