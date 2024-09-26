use gtk4;
use gtk4::{gdk, glib};

mod handle;
pub mod preview;
pub(crate) mod sidebar;
pub mod timeline;
pub mod video_player;

// todo: seems too grey. More noticeable on crop overlay
pub(crate) static IGNORE_OVERLAY_COLOUR: gdk::RGBA = gdk::RGBA::new(0.612, 0.612, 0.612, 0.79);

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk4::Widget;
}
