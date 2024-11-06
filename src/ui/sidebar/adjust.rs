use crate::ui::preview::EffectParameters;
use crate::ui::sidebar::adjust::AdjustPageMsg::ContrastChange;
use crate::ui::slider::adjust_row::{AdjustRowModel, AdjustRowOutput};
use gtk4::prelude::WidgetExt;
use relm4::{
    adw, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

pub struct AdjustPageModel {
    parameters: EffectParameters,
    contrast_slider: Controller<AdjustRowModel>,
}

#[derive(Debug)]
pub enum AdjustPageMsg {
    ContrastChange(f64),
}

#[derive(Debug)]
pub enum AdjustPageOutput {
    EffectUpdate(EffectParameters),
}

#[relm4::component(pub)]
impl SimpleComponent for AdjustPageModel {
    type Input = AdjustPageMsg;
    type Output = AdjustPageOutput;
    type Init = ();

    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            adw::PreferencesGroup{
                model.contrast_slider.widget(){},
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let contrast_slider = AdjustRowModel::builder()
            .launch("Contrast".to_string())
            .forward(sender.input_sender(), |msg| match msg {
                AdjustRowOutput::ValueChanged(val) => AdjustPageMsg::ContrastChange(val),
            });

        let model = AdjustPageModel {
            parameters: EffectParameters::new(),
            contrast_slider,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ContrastChange(level) => {
                self.parameters.set_contrast(level as f32);
                sender
                    .output(AdjustPageOutput::EffectUpdate(self.parameters))
                    .unwrap()
            }
        }
    }
}
