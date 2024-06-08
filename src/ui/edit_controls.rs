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
                add_css_class: "toolbar",

                gtk::Button {
                    set_icon_name: "play",
                },

                gtk::Box{
                    set_width_request: 300,
                    set_hexpand: true,
                    inline_css: "background-color: grey"
                },

                gtk::Button {
                     set_icon_name: "audio-volume-muted",
                },
            },

            gtk::Box {
                set_spacing: 10,

                gtk::Button {
                    set_icon_name: "crop",
                },
                // todo: make icons in buttons bigger
                gtk::Button {
                    set_icon_name: "rotate-right",
                    set_height_request: 32,
                    add_css_class: "circular",
                },

                gtk::Button {
                     set_icon_name: "rotate-left",
                     add_css_class: "flat",
                },

                gtk::Button {
                    set_icon_name: "panorama-horizontal",
                },

                gtk::Button {
                    set_icon_name: "panorama-vertical",

                },
            },

            gtk::Box{
                set_halign: gtk::Align::End,
                gtk::Button {
                    set_label: "Export Frame",
                    add_css_class: "pill",
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