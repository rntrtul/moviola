use gst::glib;
use relm4::gtk;

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
// todo: put in handle file?
impl HandleWidget {
    fn new(x: i32, is_handle: bool, is_start: bool) -> Self {
        glib::Object::builder()
            .property("x", x)
            .property("rel_x", 0)
            .property("is_handle", is_handle)
            .property("is_start", is_start)
            .build()
    }
}

impl Default for HandleWidget {
    fn default() -> Self {
        glib::Object::builder()
            .property("x", 0)
            .property("rel_x", 0)
            .property("is_handle", true)
            .property("is_start", true)
            .build()
    }
}
