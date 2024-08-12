use gst_video::VideoOrientationMethod;
use relm4::{
    adw, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

use crate::ui::crop_box::CropMode;
use crate::ui::crop_controls::{CropControlsModel, CropControlsOutput};

pub struct ControlsModel {
    crop_page: Controller<CropControlsModel>,
}

#[derive(Debug)]
pub enum ControlsMsg {
    ExportFrame,
    Orient(VideoOrientationMethod),
    SetCropMode(CropMode),
}

#[derive(Debug)]
pub enum ControlsOutput {
    ExportFrame,
    HideCropBox,
    OrientVideo(VideoOrientationMethod),
    SetCropMode(CropMode),
    ShowCropBox,
}

#[relm4::component(pub)]
impl SimpleComponent for ControlsModel {
    type Input = ControlsMsg;
    type Output = ControlsOutput;
    type Init = ();

    view! {
        adw::ToolbarView{
            #[name="stack"]
            adw::ViewStack{
                connect_visible_child_name_notify[sender] => move |stack|{
                    println!("selected child: {}", stack.visible_child_name().unwrap());
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
                CropControlsOutput::ExportFrame => ControlsMsg::ExportFrame,
            });

        let model = ControlsModel { crop_page };

        let widgets = view_output!();

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
        }
    }
}

impl ControlsModel {}
