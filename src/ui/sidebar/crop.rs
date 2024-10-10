use gtk4::prelude::WidgetExt;
use relm4::adw::prelude::{ComboRowExt, PreferencesRowExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};

use crate::ui::preview::{CropMode, Orientation};

pub struct CropPageModel {
    crop_mode: CropMode,
    orientation: Orientation,
    show_crop_box: bool,
}

#[derive(Debug)]
pub enum CropPageMsg {
    SetCropMode(CropMode),
    RotateRight90,
    FlipHorizontally,
    FlipVertically,
}

#[derive(Debug)]
pub enum CropPageOutput {
    OrientVideo(Orientation),
    SetCropMode(CropMode),
}

#[relm4::component(pub)]
impl SimpleComponent for CropPageModel {
    type Input = CropPageMsg;
    type Output = CropPageOutput;
    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            adw::PreferencesGroup{
                adw::SwitchRow{
                    set_title: "Vertical Flip",
                    connect_active_notify => CropPageMsg::FlipVertically,
                },

                adw::SwitchRow{
                    set_title: "Horizontal Flip",
                    connect_active_notify => CropPageMsg::FlipHorizontally,
                },
            },

            adw::PreferencesGroup{
                adw::ComboRow {
                    set_title: "Aspect Ratio",
                    #[wrap(Some)]
                    set_model = &gtk::StringList::new(
                        &["Free", "Original", "Square", "16:9", "4:5", "5:7", "4:3", "3:5", "3:2"]),
                    set_selected: 0,

                    connect_selected_item_notify [sender] => move |dropdown| {
                        let mode = match dropdown.selected() {
                            0 => CropMode::Free,
                            1 => CropMode::Original,
                            2 => CropMode::Square,
                            3 => CropMode::_16To9,
                            4 => CropMode::_4To5,
                            5 => CropMode::_5To7,
                            6 => CropMode::_4To3,
                            7 => CropMode::_3To5,
                            8 => CropMode::_3To2,
                            _ => panic!("Unknown crop mode selected")
                        };
                        sender.input(CropPageMsg::SetCropMode(mode));
                    }
                },
            },
        }
    }

    type Init = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = CropPageModel {
            crop_mode: CropMode::Free,
            orientation: Orientation {
                angle: 0.0,
                mirrored: false,
            },
            show_crop_box: false,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            CropPageMsg::SetCropMode(mode) => {
                self.crop_mode = mode;
                self.show_crop_box = true;
                sender.output(CropPageOutput::SetCropMode(mode)).unwrap();
            }
            CropPageMsg::RotateRight90 => {
                self.orientation.angle = (self.orientation.angle + 90.0) % 360.0;
                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropPageMsg::FlipHorizontally => {
                self.orientation.mirrored = !self.orientation.mirrored;
                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropPageMsg::FlipVertically => {
                self.orientation.angle = (self.orientation.angle + 180.0) % 360.0;
                self.orientation.mirrored = !self.orientation.mirrored;

                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
        }
    }
}
