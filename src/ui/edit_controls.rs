use gst_video::VideoOrientationMethod;
use gtk4::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{gtk, ComponentParts, ComponentSender, SimpleComponent};

use crate::ui::edit_controls::CropType::{
    Crop16To9, Crop3To2, Crop4To3, Crop5To4, CropFree, CropOriginal, CropSquare,
};

pub struct EditControlsModel {
    crop_mode: EditControlsMsg,
    orientation: VideoOrientationMethod,
    rotation_angle: u32,
    is_flip_vertical: bool,
    is_flip_horizontal: bool,
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
    ExportFrame,
    ExportVideo,
    CropMode(CropType),
    RotateRight90,
}

#[derive(Debug)]
pub enum EditControlsOutput {
    ExportFrame,
    ExportVideo,
    OrientVideo(VideoOrientationMethod),
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

                    connect_clicked => EditControlsMsg::RotateRight90,
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

                        connect_clicked => EditControlsMsg::ExportFrame,
                    },
                },

                gtk::Button {
                    set_label: "save",

                    connect_clicked => EditControlsMsg::ExportVideo
                }
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = EditControlsModel {
            crop_mode: EditControlsMsg::CropMode(CropFree),
            orientation: VideoOrientationMethod::Identity,
            rotation_angle: 0,
            is_flip_vertical: false,
            is_flip_horizontal: false,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            EditControlsMsg::CropMode(_) => {
                self.crop_mode = message;
            }
            EditControlsMsg::ExportFrame => {
                sender.output(EditControlsOutput::ExportFrame).unwrap();
            }
            EditControlsMsg::ExportVideo => sender.output(EditControlsOutput::ExportVideo).unwrap(),
            EditControlsMsg::RotateRight90 => {
                self.rotation_angle += 90;
                if self.rotation_angle > 360 {
                    self.rotation_angle = 0;
                }
                self.update_video_orientation_val();
                sender
                    .output(EditControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
        }
    }
}

impl EditControlsModel {
    fn update_video_orientation_val(&mut self) {
        self.orientation = if self.is_flip_horizontal {
            VideoOrientationMethod::Horiz
        } else if self.is_flip_vertical {
            if self.rotation_angle == 270 {
                VideoOrientationMethod::UlLr
            } else if self.rotation_angle == 90 {
                VideoOrientationMethod::UrLl
            } else {
                VideoOrientationMethod::Vert
            }
        } else if self.rotation_angle == 90 {
            VideoOrientationMethod::_90r
        } else if self.rotation_angle == 270 {
            VideoOrientationMethod::_90l
        } else if self.rotation_angle == 180 {
            VideoOrientationMethod::_180
        } else {
            VideoOrientationMethod::Identity
        }
    }
}
