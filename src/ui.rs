use gtk4;
use gtk4::gdk;

pub mod preview;
pub(crate) mod sidebar;
pub mod timeline;
pub mod video_player;

pub(crate) static IGNORE_OVERLAY_COLOUR: gdk::RGBA = gdk::RGBA::new(0.0, 0.0, 0.0, 0.7);
