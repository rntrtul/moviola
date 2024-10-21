use gtk4::glib;

mod handle;
mod video_control;

pub use video_control::{VideoControlModel, VideoControlMsg, VideoControlOutput};

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk4::Widget;
}
