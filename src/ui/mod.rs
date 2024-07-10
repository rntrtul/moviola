use gst::glib;
use relm4::gtk;

mod crop_box;
pub mod edit_controls;
mod handle;
mod handle_manager;
mod thumbnail_manager;
mod timeline;
pub mod video_player;

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk::Widget;
}

glib::wrapper! {
    pub struct CropBoxWidget(ObjectSubclass<crop_box::CropBoxWidget>)
        @extends gtk::Widget;
}
