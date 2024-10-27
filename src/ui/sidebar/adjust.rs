use crate::ui::preview::EffectParameters;
use crate::ui::sidebar::adjust::AdjustPageMsg::ContrastChange;
use gtk4::prelude::{RangeExt, WidgetExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};

pub struct AdjustPageModel {
    parameters: EffectParameters,
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
                gtk::Scale::with_range(gtk::Orientation::Horizontal, -0f64, 2f64, 0.1f64 ){
                    connect_value_changed[sender] => move|scale| {
                                sender.input(ContrastChange(scale.value()));
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = AdjustPageModel {
            parameters: EffectParameters::new(),
        };

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
