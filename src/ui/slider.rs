use gtk4::glib;

pub mod adjust_row;
mod slider;

glib::wrapper! {
    pub struct Slider(ObjectSubclass<slider::Slider>)
        @extends gtk4::Widget;
}
