use gtk4::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, gtk, RelmWidgetExt, SimpleComponent};

pub struct EditControlsModel {}

#[derive(Debug)]
pub enum EditControlsMsg {}

#[relm4::component(pub)]
impl SimpleComponent for EditControlsModel {
    type Input = EditControlsMsg;
    type Output = ();
    type Init = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            set_spacing: 20,

            gtk::Box {
                set_spacing: 10,

                gtk::Button {
                    set_label: "Play"
                },

                gtk::Box{
                    set_width_request: 200,
                    inline_css: "background-color: grey"
                },

                gtk::Button {
                    set_label: "Mute Video"
                },
            },

            gtk::Box {
                set_spacing: 10,

                gtk::Button {
                    set_label: "Crop",
                },

                gtk::Button {
                    set_label: "Rotate CW",
                },

                gtk::Button {
                    set_label: "Rotate CCW",
                },

                gtk::Button {
                    set_label: "Flip Horizontally"
                },

                gtk::Button {
                    set_label: "Flip Vertical"
                },
            },

            gtk::Box{
                set_halign: gtk::Align::End,
                gtk::Button {
                    set_label: "Export Frame",
                },
            },


        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = EditControlsModel {};

        ComponentParts { model, widgets }
    }
}