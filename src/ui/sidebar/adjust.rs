use crate::renderer::EffectParameters;
use crate::ui::sidebar::adjust::AdjustPageMsg::{
    BrightnessChange, ContrastChange, SaturationChange,
};
use crate::ui::slider::adjust_row::{AdjustRowInit, AdjustRowModel, AdjustRowOutput};
use crate::ui::slider::SliderFillMode;
use crate::ui::Range;
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::component::Connector;
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent,
};

pub struct AdjustPageModel {
    parameters: EffectParameters,
    contrast_slider: Controller<AdjustRowModel>,
    brigtness_slider: Controller<AdjustRowModel>,
    saturation_slider: Controller<AdjustRowModel>,
}

#[derive(Debug)]
pub enum AdjustPageMsg {
    ContrastChange(f64),
    BrightnessChange(f64),
    SaturationChange(f64),
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
            adw::PreferencesGroup {
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,

                    model.contrast_slider.widget(){},
                    model.brigtness_slider.widget(){},
                    model.saturation_slider.widget(){},
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let contrast_slider = build_slider("Contrast", EffectParameters::contrast_range()).forward(
            sender.input_sender(),
            |msg| match msg {
                AdjustRowOutput::ValueChanged(val) => ContrastChange(val),
            },
        );

        let brigtness_slider = build_slider("Brightness", EffectParameters::brigntess_range())
            .forward(sender.input_sender(), |msg| match msg {
                AdjustRowOutput::ValueChanged(val) => BrightnessChange(val),
            });

        let saturation_slider = build_slider("Saturation", EffectParameters::saturation_range())
            .forward(sender.input_sender(), |msg| match msg {
                AdjustRowOutput::ValueChanged(val) => SaturationChange(val),
            });

        let model = AdjustPageModel {
            parameters: EffectParameters::new(),
            contrast_slider,
            brigtness_slider,
            saturation_slider,
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
            BrightnessChange(level) => {
                self.parameters.set_brightness(level as f32);
                sender
                    .output(AdjustPageOutput::EffectUpdate(self.parameters))
                    .unwrap()
            }
            SaturationChange(level) => {
                self.parameters.set_saturation(level as f32);
                sender
                    .output(AdjustPageOutput::EffectUpdate(self.parameters))
                    .unwrap()
            }
        }
    }
}

fn build_slider(
    label: &str,
    (val_range, display_range): (Range, Range),
) -> Connector<AdjustRowModel> {
    AdjustRowModel::builder().launch(AdjustRowInit::new(
        label,
        true,
        true,
        val_range,
        display_range,
        SliderFillMode::EdgeToEdge,
    ))
}
