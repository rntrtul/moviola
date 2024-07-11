use gst_video::VideoOrientationMethod;
use gtk4::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{gtk, ComponentParts, ComponentSender, SimpleComponent};

use crate::ui::crop_box::CropType;
use crate::ui::edit_controls::EditControlsOutput::{HideCropBox, ShowCropBox};

pub struct EditControlsModel {
    crop_mode: CropType,
    orientation: VideoOrientationMethod,
    rotation_angle: i32,
    show_crop_box: bool,
    is_flip_vertical: bool,
    is_flip_horizontal: bool,
}

#[derive(Debug)]
pub enum EditControlsMsg {
    ExportFrame,
    ExportVideo,
    SetCropMode(CropType),
    ToggleCropBox,
    RotateRight90,
    RotateLeft90,
    FlipHorizontally,
    FlipVertically,
}

#[derive(Debug)]
pub enum EditControlsOutput {
    ExportFrame,
    ExportVideo,
    OrientVideo(VideoOrientationMethod),
    ShowCropBox,
    HideCropBox,
    SetCropMode(CropType),
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
                        connect_clicked => EditControlsMsg::ToggleCropBox,
                    },

                    #[name = "crop_mode_dropdown"]
                    gtk::DropDown::from_strings(&["Free", "Original", "Square", "5:4", "4:3", "3:2", "16:9"]) {
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let mode = match dropdown.selected() {
                                0 => CropType::CropFree,
                                1 => CropType::CropOriginal,
                                2 => CropType::CropSquare,
                                3 => CropType::Crop5To4,
                                4 => CropType::Crop4To3,
                                5 => CropType::Crop3To2,
                                6 => CropType::Crop16To9,
                                _ => panic!("Unknown crop mode selected")
                            };
                            sender.input(EditControlsMsg::SetCropMode(mode));
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

                    connect_clicked => EditControlsMsg::RotateLeft90,
                },

                gtk::Button {
                    set_icon_name: "panorama-horizontal",
                    connect_clicked => EditControlsMsg::FlipHorizontally,
                },

                gtk::Button {
                    set_icon_name: "panorama-vertical",
                    connect_clicked => EditControlsMsg::FlipVertically,
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
            crop_mode: CropType::CropFree,
            orientation: VideoOrientationMethod::Identity,
            rotation_angle: 0,
            show_crop_box: false,
            is_flip_vertical: false,
            is_flip_horizontal: false,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            EditControlsMsg::SetCropMode(mode) => {
                self.crop_mode = mode;
                self.show_crop_box = true;
                sender
                    .output(EditControlsOutput::SetCropMode(mode))
                    .unwrap();
                sender.output(ShowCropBox).unwrap();
            }
            EditControlsMsg::ToggleCropBox => {
                self.show_crop_box = !self.show_crop_box;

                let msg = if self.show_crop_box {
                    ShowCropBox
                } else {
                    HideCropBox
                };

                sender.output(msg).unwrap()
            }
            EditControlsMsg::ExportFrame => {
                sender.output(EditControlsOutput::ExportFrame).unwrap();
            }
            EditControlsMsg::ExportVideo => sender.output(EditControlsOutput::ExportVideo).unwrap(),
            EditControlsMsg::RotateRight90 => {
                self.rotation_angle += 90;
                if self.rotation_angle == 360 {
                    self.rotation_angle = 0;
                }
                self.update_video_orientation_val();
                sender
                    .output(EditControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            EditControlsMsg::RotateLeft90 => {
                self.rotation_angle = if self.rotation_angle == 0 {
                    270
                } else {
                    self.rotation_angle - 90
                };
                self.update_video_orientation_val();

                sender
                    .output(EditControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            EditControlsMsg::FlipHorizontally => {
                self.is_flip_horizontal = !self.is_flip_horizontal;
                self.update_video_orientation_val();
                sender
                    .output(EditControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            EditControlsMsg::FlipVertically => {
                self.is_flip_vertical = !self.is_flip_vertical;
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
        // todo: check scenarios for horizontal and rotations
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
