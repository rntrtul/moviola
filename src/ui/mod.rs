use gst::glib;
use relm4::gtk;

pub mod video_player;
pub mod edit_controls;
mod handle;

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk::Widget;
}

impl HandleWidget {
    fn new(x: f32) -> Self {
        glib::Object::builder()
            .property("x", x)
            .build()
    }
}

impl Default for HandleWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0f32)
            .build()
    }
}
