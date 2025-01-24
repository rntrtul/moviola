use relm4::gtk::glib;

mod handle;
mod video_control;

pub use video_control::{VideoControlModel, VideoControlMsg, VideoControlOutput};

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends relm4::gtk::Widget;
}
