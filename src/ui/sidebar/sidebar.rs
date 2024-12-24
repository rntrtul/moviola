use crate::renderer::EffectParameters;
use crate::ui::preview::{CropMode, Orientation};
use crate::ui::sidebar::adjust::{AdjustPageModel, AdjustPageOutput};
use crate::ui::sidebar::crop::{CropPageModel, CropPageMsg, CropPageOutput};
use crate::ui::sidebar::output::{OutputPageModel, OutputPageMsg, OutputPageOutput};
use crate::ui::sidebar::ControlsExportSettings;
use crate::video::metadata::VideoContainerInfo;
use gtk4::prelude::ButtonExt;
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

pub struct ControlsModel {
    crop_page: Controller<CropPageModel>,
    output_page: Controller<OutputPageModel>,
    adjust_page: Controller<AdjustPageModel>,
    stack: adw::ViewStack,
}

#[derive(Debug)]
pub enum ControlsMsg {
    VideoLoaded((VideoContainerInfo, Orientation)),
    Rotate,
    ExportFrame,
    Orient(Orientation),
    StraightenBegin,
    Straighten(f64),
    StraightenEnd,
    SetCropMode(CropMode),
    CropPageSelected,
    OutputPageSelected,
    AdjustPageSelected,
    EffectsChanged(EffectParameters),
}

#[derive(Debug)]
pub enum ControlsOutput {
    ExportFrame,
    ShowCropBox,
    HideCropBox,
    TempResetZoom,
    RestoreZoom,
    OrientVideo(Orientation),
    StraightenBegin,
    Straigten(f64),
    StraightenEnd,
    SetCropMode(CropMode),
    EffectsChanged(EffectParameters),
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
                    match stack.visible_child_name().unwrap().as_str() {
                        "crop_page" => sender.input(ControlsMsg::CropPageSelected),
                        "output_page" => sender.input(ControlsMsg::OutputPageSelected),
                        "adjust_page" => sender.input(ControlsMsg::AdjustPageSelected),
                        _ => {},
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
                    CropPageOutput::StraigtenBegin => ControlsMsg::StraightenBegin,
                    CropPageOutput::Straighten(angle) => ControlsMsg::Straighten(angle),
                    CropPageOutput::StraightenEnd => ControlsMsg::StraightenEnd,
                });

        let output_page =
            OutputPageModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    OutputPageOutput::ExportFrame => ControlsMsg::ExportFrame,
                });

        let adjust_page =
            AdjustPageModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    AdjustPageOutput::EffectUpdate(params) => ControlsMsg::EffectsChanged(params),
                });

        let widgets = view_output!();

        let model = ControlsModel {
            crop_page,
            output_page,
            adjust_page,
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

        model.stack.add_titled_with_icon(
            model.adjust_page.widget(),
            Some("adjust_page"),
            "Adjust",
            "crop-symbolic",
        );

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ControlsMsg::VideoLoaded((default_codec, base_orientaiton)) => {
                self.output_page
                    .emit(OutputPageMsg::VideoInfo(default_codec));
                self.crop_page
                    .emit(CropPageMsg::SetBaseOrientation(base_orientaiton));
            }
            ControlsMsg::SetCropMode(mode) => {
                sender.output(ControlsOutput::SetCropMode(mode)).unwrap();
                sender.output(ControlsOutput::ShowCropBox).unwrap();
            }
            ControlsMsg::Orient(orientation) => sender
                .output(ControlsOutput::OrientVideo(orientation))
                .unwrap(),
            ControlsMsg::StraightenBegin => sender.output(ControlsOutput::StraightenBegin).unwrap(),
            ControlsMsg::Straighten(angle) => {
                sender.output(ControlsOutput::Straigten(angle)).unwrap()
            }
            ControlsMsg::StraightenEnd => sender.output(ControlsOutput::StraightenEnd).unwrap(),
            ControlsMsg::ExportFrame => sender.output(ControlsOutput::ExportFrame).unwrap(),
            ControlsMsg::Rotate => self.crop_page.emit(CropPageMsg::RotateRight90),
            ControlsMsg::CropPageSelected => {
                sender.output(ControlsOutput::ShowCropBox).unwrap();
                sender.output(ControlsOutput::TempResetZoom).unwrap();
            }
            ControlsMsg::OutputPageSelected => {
                sender.output(ControlsOutput::HideCropBox).unwrap();
                sender.output(ControlsOutput::RestoreZoom).unwrap();
            }
            ControlsMsg::AdjustPageSelected => {
                sender.output(ControlsOutput::HideCropBox).unwrap();
                sender.output(ControlsOutput::RestoreZoom).unwrap();
            }
            ControlsMsg::EffectsChanged(params) => sender
                .output(ControlsOutput::EffectsChanged(params))
                .unwrap(),
        }
    }
}

impl ControlsModel {
    pub fn export_settings(&self) -> ControlsExportSettings {
        let export_container = self.output_page.model().export_settings();
        let effect_parameters = self.adjust_page.model().export_settings();

        // fixme: actually get value, for if it is default
        ControlsExportSettings {
            container: export_container,
            container_is_default: true,
            effect_parameters,
        }
    }

    pub fn connect_switcher_to_stack(&self, switcher: &adw::ViewSwitcherBar) {
        switcher.set_stack(Some(&self.stack))
    }
}
