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
    fn new(x: i32) -> Self {
        glib::Object::builder()
            .property("x", x)
            .property("rel_x", 0)
            .build()
    }
}

impl Default for HandleWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0)
            .property("rel_x", 0)
            .build()
    }
}
