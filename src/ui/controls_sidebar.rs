use gst_plugin_gtk4::Orientation;
use gtk4::prelude::ButtonExt;
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

use crate::ui::crop_page::{CropPageModel, CropPageMsg, CropPageOutput};
use crate::ui::output_page::{OutputPageModel, OutputPageMsg, OutputPageOutput};
use crate::ui::preview::CropMode;
use crate::video::metadata::{AudioCodec, ContainerFormat, VideoCodec, VideoContainerInfo};

// fixme: too similar to videoContainerInfo
#[derive(Debug, Clone, Copy)]
pub struct OutputContainerSettings {
    pub(crate) no_audio: bool,
    pub(crate) audio_stream_idx: u32,
    pub(crate) audio_codec: AudioCodec,
    pub(crate) audio_bitrate: u32,
    pub(crate) container: ContainerFormat,
    pub(crate) video_codec: VideoCodec,
    pub(crate) video_bitrate: u32,
}

pub struct ControlsExportSettings {
    pub container: OutputContainerSettings,
    pub container_is_default: bool,
}

pub struct ControlsModel {
    crop_page: Controller<CropPageModel>,
    output_page: Controller<OutputPageModel>,
    stack: adw::ViewStack,
}

#[derive(Debug)]
pub enum ControlsMsg {
    Rotate,
    ExportFrame,
    Orient(Orientation),
    SetCropMode(CropMode),
    DefaultCodec(VideoContainerInfo),
    CropPageSelected,
    OutputPageSelected,
}

#[derive(Debug)]
pub enum ControlsOutput {
    ExportFrame,
    ShowCropBox,
    HideCropBox,
    TempResetZoom,
    RestoreZoom,
    OrientVideo(Orientation),
    SetCropMode(CropMode),
}

#[relm4::component(pub)]
impl SimpleComponent for ControlsModel {
    type Input = ControlsMsg;
    type Output = ControlsOutput;
    type Init = ();

    view! {
        adw::ToolbarView{
            add_top_bar = &adw::HeaderBar {
                set_show_title: false,
                pack_start = & gtk::Button {
                    set_icon_name: "rotate-right",
                    connect_clicked => ControlsMsg::Rotate,
                },
            },

            #[name="stack"]
            adw::ViewStack{
                connect_visible_child_name_notify[sender] => move |stack|{
                    if stack.visible_child_name().unwrap() == "crop_page" {
                        sender.input(ControlsMsg::CropPageSelected)
                    } else {
                        sender.input(ControlsMsg::OutputPageSelected)
                    }
                },
            },
        },
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let crop_page: Controller<CropPageModel> =
            CropPageModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    CropPageOutput::SetCropMode(mode) => ControlsMsg::SetCropMode(mode),
                    CropPageOutput::OrientVideo(orientation) => ControlsMsg::Orient(orientation),
                });

        let output_page =
            OutputPageModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    OutputPageOutput::ExportFrame => ControlsMsg::ExportFrame,
                });

        let widgets = view_output!();

        let model = ControlsModel {
            crop_page,
            output_page,
            stack: widgets.stack.clone(),
        };

        // todo: figure out way to select none?
        // order matters
        model.stack.add_titled_with_icon(
            model.output_page.widget(),
            Some("output_page"),
            "Output",
            "video-encoder-symbolic",
        );

        model.stack.add_titled_with_icon(
            model.crop_page.widget(),
            Some("crop_page"),
            "Crop",
            "crop-symbolic",
        );

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ControlsMsg::SetCropMode(mode) => {
                sender.output(ControlsOutput::SetCropMode(mode)).unwrap();
                sender.output(ControlsOutput::ShowCropBox).unwrap();
            }
            ControlsMsg::Orient(orientation) => sender
                .output(ControlsOutput::OrientVideo(orientation))
                .unwrap(),
            ControlsMsg::ExportFrame => sender.output(ControlsOutput::ExportFrame).unwrap(),
            ControlsMsg::Rotate => self.crop_page.emit(CropPageMsg::RotateRight90),
            ControlsMsg::DefaultCodec(defaults) => {
                self.output_page.emit(OutputPageMsg::VideoInfo(defaults));
            }
            ControlsMsg::CropPageSelected => {
                sender.output(ControlsOutput::ShowCropBox).unwrap();
                sender.output(ControlsOutput::TempResetZoom).unwrap();
            }
            ControlsMsg::OutputPageSelected => {
                sender.output(ControlsOutput::HideCropBox).unwrap();
                sender.output(ControlsOutput::RestoreZoom).unwrap();
            }
        }
    }
}

impl ControlsModel {
    pub fn export_settings(&self) -> ControlsExportSettings {
        let export_container = self.output_page.model().export_settings();

        // fixme: actually get value, for if it is default
        ControlsExportSettings {
            container: export_container,
            container_is_default: true,
        }
    }

    pub fn connect_switcher_to_stack(&self, switcher: &adw::ViewSwitcherBar) {
        switcher.set_stack(Some(&self.stack))
    }
}
