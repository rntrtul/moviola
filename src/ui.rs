use gtk4;
use gtk4::{gdk, glib};

pub mod controls_sidebar;
pub mod crop_box;
pub mod crop_controls;
mod handle;
pub mod timeline;
pub mod video_player;

// todo: seems too grey. More noticeable on crop overlay
pub(crate) static IGNORE_OVERLAY_COLOUR: gdk::RGBA = gdk::RGBA::new(0.612, 0.612, 0.612, 0.79);

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk4::Widget;
}

glib::wrapper! {
    pub struct CropBoxWidget(ObjectSubclass<crop_box::CropBoxWidget>)
        @extends gtk4::Widget;
}
