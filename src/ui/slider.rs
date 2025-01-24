use relm4::gtk::glib;

pub mod adjust_row;
mod slider;

glib::wrapper! {
    pub struct Slider(ObjectSubclass<slider::Slider>)
        @extends relm4::gtk::Widget;
}
