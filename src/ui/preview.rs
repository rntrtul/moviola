mod bounding_box;
mod input;
mod orient;
mod pan;
mod preview;
pub mod preview_frame;
mod zoom;

use relm4::gtk::glib;

pub use crate::ui::preview::bounding_box::{BoundingBoxDimensions, CropMode};
pub use crate::ui::preview::orient::Orientation;

glib::wrapper! {
    pub struct Preview(ObjectSubclass<preview::Preview>)
        @extends relm4::gtk::Widget;
}
