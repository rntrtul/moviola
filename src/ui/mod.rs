use gst::glib;
use relm4::gtk;

pub mod video_player;
pub mod edit_controls;
mod handle;
mod timeline;

glib::wrapper! {
    pub struct HandleWidget(ObjectSubclass<handle::HandleWidget>)
        @extends gtk::Widget;
}

impl HandleWidget {
    fn new(x: i32, is_handle: bool) -> Self {
        glib::Object::builder()
            .property("x", x)
            .property("rel_x", 0)
            .property("is_handle", is_handle)
            .build()
    }
}

impl Default for HandleWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0)
            .property("rel_x", 0)
            .property("is_handle", true)
            .build()
    }
}
