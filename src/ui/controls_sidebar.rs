use gst_plugin_gtk4::Orientation;
use gtk4::prelude::{ButtonExt, WidgetExt};
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

use crate::ui::crop_box::CropMode;
use crate::ui::crop_controls::{CropControlsModel, CropControlsMsg, CropControlsOutput};
use crate::ui::output_controls::{OutputControlsModel, OutputControlsMsg, OutputControlsOutput};
use crate::video::metadata::VideoCodecInfo;

pub struct ControlsExportSettings {
    pub container: VideoCodecInfo,
    pub container_is_default: bool,
}

pub struct ControlsModel {
    crop_page: Controller<CropControlsModel>,
    output_page: Controller<OutputControlsModel>,
}

#[derive(Debug)]
pub enum ControlsMsg {
    Rotate,
    ExportFrame,
    Orient(Orientation),
    SetCropMode(CropMode),
    DefaultCodec(VideoCodecInfo),
}

#[derive(Debug)]
pub enum ControlsOutput {
    ExportFrame,
    HideCropBox,
    OrientVideo(Orientation),
    SetCropMode(CropMode),
    ShowCropBox,
    SaveFile,
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

              pack_start = &gtk::Button {
                    set_label: "Save",
                    add_css_class: "suggested-action",
                    connect_clicked[sender] => move |_| {
                        sender.output(ControlsOutput::SaveFile).unwrap();
                    },
                },

                pack_end = & gtk::Button {
                    set_icon_name: "rotate-right",
                    connect_clicked => ControlsMsg::Rotate,
                },
            },

            #[name="stack"]
            adw::ViewStack{
                connect_visible_child_name_notify[sender] => move |stack|{
                    if stack.visible_child_name().unwrap() == "crop_page" {
                        sender.output(ControlsOutput::ShowCropBox).unwrap()
                    } else {
                        sender.output(ControlsOutput::HideCropBox).unwrap()
                    }
                },
            },

            #[name="switch_bar"]
            add_bottom_bar = &adw::ViewSwitcherBar{
                set_reveal: true,
            },
        },
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let crop_page: Controller<CropControlsModel> = CropControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                CropControlsOutput::SetCropMode(mode) => ControlsMsg::SetCropMode(mode),
                CropControlsOutput::OrientVideo(orientation) => ControlsMsg::Orient(orientation),
            });

        let output_page = OutputControlsModel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                OutputControlsOutput::ExportFrame => ControlsMsg::ExportFrame,
            },
        );

        let model = ControlsModel {
            crop_page,
            output_page,
        };

        let widgets = view_output!();

        // todo: figure out way to select none?
        // order matters
        widgets.stack.add_titled_with_icon(
            model.output_page.widget(),
            Some("output_page"),
            "Output",
            "video-encoder-symbolic",
        );

        widgets.stack.add_titled_with_icon(
            model.crop_page.widget(),
            Some("crop_page"),
            "Crop",
            "crop-symbolic",
        );

        widgets.switch_bar.set_reveal(true);
        widgets.switch_bar.set_stack(Some(&widgets.stack));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ControlsMsg::SetCropMode(mode) => {
                // todo: show and hide crop mode when crop_page selected
                sender.output(ControlsOutput::SetCropMode(mode)).unwrap();
                sender.output(ControlsOutput::ShowCropBox).unwrap();
            }
            ControlsMsg::Orient(orientation) => sender
                .output(ControlsOutput::OrientVideo(orientation))
                .unwrap(),
            ControlsMsg::ExportFrame => sender.output(ControlsOutput::ExportFrame).unwrap(),
            ControlsMsg::Rotate => self.crop_page.emit(CropControlsMsg::RotateRight90),
            ControlsMsg::DefaultCodec(defaults) => {
                self.output_page
                    .emit(OutputControlsMsg::DefaultCodecs(defaults));
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
}
