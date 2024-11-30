use gtk4;
use gtk4::gdk;

pub mod preview;
pub(crate) mod sidebar;
mod slider;
pub mod video_controls;

pub(crate) static IGNORE_OVERLAY_COLOUR: gdk::RGBA = gdk::RGBA::new(0.0, 0.0, 0.0, 0.7);

pub use slider::Range;
