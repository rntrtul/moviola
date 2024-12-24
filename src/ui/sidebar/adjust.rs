use crate::renderer::EffectParameters;
use crate::ui::sidebar::adjust::AdjustPageMsg::{
    BrightnessChange, ContrastChange, SaturationChange,
};
use crate::ui::slider::adjust_row::{AdjustRowModel, AdjustRowMsg, AdjustRowOutput};
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::adw::prelude::PreferencesRowExt;
use relm4::{
    adw, gtk, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
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
    Reset,
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

            adw::PreferencesGroup {
                set_valign: gtk::Align::End,
                set_vexpand: true,

                adw::ButtonRow {
                    set_title: "Reset",
                    add_css_class: "destructive-action",

                    connect_activated => AdjustPageMsg::Reset,
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let contrast_slider = AdjustRowModel::build_slider(
            "Contrast",
            EffectParameters::contrast_range(),
        )
        .forward(sender.input_sender(), |msg| match msg {
            AdjustRowOutput::ValueChanged(val) => ContrastChange(val),
        });

        let brigtness_slider =
            AdjustRowModel::build_slider("Brightness", EffectParameters::brigntess_range())
                .forward(sender.input_sender(), |msg| match msg {
                    AdjustRowOutput::ValueChanged(val) => BrightnessChange(val),
                });

        let saturation_slider =
            AdjustRowModel::build_slider("Saturation", EffectParameters::saturation_range())
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
            AdjustPageMsg::Reset => {
                self.parameters.reset();
                self.saturation_slider.emit(AdjustRowMsg::SilentReset);
                self.contrast_slider.emit(AdjustRowMsg::SilentReset);
                self.brigtness_slider.emit(AdjustRowMsg::SilentReset);
                sender
                    .output(AdjustPageOutput::EffectUpdate(self.parameters))
                    .unwrap()
            }
        }
    }
}

impl AdjustPageModel {
    pub fn export_settings(&self) -> EffectParameters {
        self.parameters.clone()
    }
}
