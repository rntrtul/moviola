use gtk4::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, gtk, RelmWidgetExt, SimpleComponent};

use crate::ui::edit_controls::CropType::{Crop16To9, Crop3To2, Crop4To3, Crop5To4, CropFree, CropOriginal, CropSquare};

pub struct EditControlsModel {
    crop_mode: EditControlsMsg,
}

#[derive(Debug)]
enum CropType {
    CropFree = 0,
    CropOriginal,
    CropSquare,
    Crop5To4,
    Crop4To3,
    Crop3To2,
    Crop16To9,
}

#[derive(Debug)]
pub enum EditControlsMsg {
    CropMode(CropType),
}

#[derive(Debug)]
pub enum EditControlsOutput {
    SeekToPercent(f64),
}

#[relm4::component(pub)]
impl SimpleComponent for EditControlsModel {
    type Input = EditControlsMsg;
    type Output = EditControlsOutput;
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

                #[name = "timeline"]
                gtk::Box {
                    set_width_request: 300,
                    inline_css: "background-color: grey"
                },

                gtk::Button {
                     set_icon_name: "audio-volume-muted",
                },
            },

            gtk::Box {
                set_spacing: 10,

                gtk::Box {
                add_css_class: "linked",

                    gtk::Button {
                        set_icon_name: "crop",
                    },

                    #[name = "crop_mode_dropdown"]
                    gtk::DropDown::from_strings(&["Free", "Original", "Square", "5:4", "4:3", "3:2", "16:9"]) {
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let mode = match dropdown.selected() {
                                0 => CropFree,
                                1 => CropOriginal,
                                2 => CropSquare,
                                3 => Crop5To4,
                                4 => Crop4To3,
                                5 => Crop3To2,
                                6 => Crop16To9,
                                _ => panic!("Unknown crop mode selected")
                            };
                            sender.input(EditControlsMsg::CropMode(mode));
                        }
                    },
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

                gtk::Box{
                    set_halign: gtk::Align::End,
                    gtk::Button {
                        set_label: "Export Frame",
                        add_css_class: "pill",
                    },
                },
            },
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = EditControlsModel {
            crop_mode: EditControlsMsg::CropMode(CropFree),
        };

        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(move |_, _, x, y| {
            let percent = x / 300f64;
            sender.output(EditControlsOutput::SeekToPercent(percent)).expect("could not send message");
        });
        widgets.timeline.add_controller(gesture);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            EditControlsMsg::CropMode(_) => {
                self.crop_mode = message;
            }
        }
    }
}