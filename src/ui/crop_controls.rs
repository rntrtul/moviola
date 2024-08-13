use gst_video::VideoOrientationMethod;
use gtk4::prelude::WidgetExt;
use relm4::adw::prelude::{ComboRowExt, ExpanderRowExt, PreferencesRowExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};

use crate::ui::crop_box::CropMode;

pub struct CropControlsModel {
    crop_mode: CropMode,
    orientation: VideoOrientationMethod,
    rotation_angle: i32,
    show_crop_box: bool,
    is_flip_vertical: bool,
    is_flip_horizontal: bool,
}

#[derive(Debug)]
pub enum CropControlsMsg {
    SetCropMode(CropMode),
    RotateRight90,
    RotateLeft90,
    FlipHorizontally,
    FlipVertically,
}

#[derive(Debug)]
pub enum CropControlsOutput {
    OrientVideo(VideoOrientationMethod),
    SetCropMode(CropMode),
}

#[relm4::component(pub)]
impl SimpleComponent for CropControlsModel {
    type Input = CropControlsMsg;
    type Output = CropControlsOutput;
    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            adw::PreferencesGroup{
                adw::ExpanderRow {
                    set_title: "Flip",

                    add_row= &adw::SwitchRow{
                        set_title: "Vertical Flip",
                        connect_active_notify => CropControlsMsg::FlipVertically,
                    },

                    add_row= &adw::SwitchRow{
                        set_title: "Horizontal Flip",
                        connect_active_notify => CropControlsMsg::FlipHorizontally,
                    }
                },

                adw::ComboRow {
                    set_title: "Aspect Ratio",
                    #[wrap(Some)]
                    set_model = &gtk::StringList::new(
                        &["Free", "Original", "Square", "16:9", "4:5", "5:7", "4:3", "3:5", "3:2"]),
                    set_selected: 1,

                    connect_selected_item_notify [sender] => move |dropdown| {
                        let mode = match dropdown.selected() {
                            0 => CropMode::Original,
                            1 => CropMode::Free,
                            2 => CropMode::Square,
                            3 => CropMode::_16To9,
                            4 => CropMode::_4To5,
                            5 => CropMode::_5To7,
                            6 => CropMode::_4To3,
                            7 => CropMode::_3To5,
                            8 => CropMode::_3To2,
                            _ => panic!("Unknown crop mode selected")
                        };
                        sender.input(CropControlsMsg::SetCropMode(mode));
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
        let model = CropControlsModel {
            crop_mode: CropMode::Free,
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
            CropControlsMsg::SetCropMode(mode) => {
                self.crop_mode = mode;
                self.show_crop_box = true;
                sender
                    .output(CropControlsOutput::SetCropMode(mode))
                    .unwrap();
            }
            CropControlsMsg::RotateRight90 => {
                self.rotation_angle = (self.rotation_angle + 90) % 360;
                self.update_video_orientation_val();
                sender
                    .output(CropControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropControlsMsg::RotateLeft90 => {
                self.rotation_angle = if self.rotation_angle == 0 {
                    270
                } else {
                    self.rotation_angle - 90
                };
                self.update_video_orientation_val();

                sender
                    .output(CropControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropControlsMsg::FlipHorizontally => {
                self.is_flip_horizontal = !self.is_flip_horizontal;
                self.update_video_orientation_val();
                sender
                    .output(CropControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropControlsMsg::FlipVertically => {
                self.is_flip_vertical = !self.is_flip_vertical;
                self.update_video_orientation_val();
                sender
                    .output(CropControlsOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
        }
    }
}

impl CropControlsModel {
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
