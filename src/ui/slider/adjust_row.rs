use crate::ui::slider::Slider;
use gtk4::prelude::{GestureDragExt, WidgetExt};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

#[derive(Debug)]
pub struct AdjustRowModel {
    label: String,
    show_label: bool,
    show_value: bool,
}

#[derive(Debug)]
pub enum AdjustRowMsg {
    DragUpdate(f64),
}

#[derive(Debug)]
pub enum AdjustRowOutput {
    ValueChanged(f64),
}

#[derive(Debug)]
pub struct AdjustRowInit {
    label: String,
    show_label: bool,
    show_value: bool,
    min_value: f32,
    max_value: f32,
    default_value: f32,
}

impl AdjustRowInit {
    pub fn default_with_label(label: &str) -> Self {
        Self {
            label: label.to_string(),
            show_label: true,
            show_value: true,
            min_value: -1.0,
            max_value: 1.0,
            default_value: 0.0,
        }
    }

    pub fn default_with_label_values(label: &str, min: f32, max: f32, default: f32) -> Self {
        Self {
            label: label.to_string(),
            show_label: true,
            show_value: true,
            min_value: min,
            max_value: max,
            default_value: default,
        }
    }
}

#[relm4::component(pub)]
impl Component for AdjustRowModel {
    type Input = AdjustRowMsg;
    type Output = AdjustRowOutput;
    type Init = AdjustRowInit;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Overlay{
            #[wrap(Some)]
            set_child: slider = &Slider::new_with_val(init.min_value, init.max_value, init.default_value) {
                add_controller = gtk::GestureDrag {
                    connect_drag_update[sender] => move |drag,x_offset,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let target = start_x + x_offset;
                        sender.input(AdjustRowMsg::DragUpdate(target))
                    }
                }
            },
            add_overlay = &gtk::Box {
                set_can_target: false,
                set_hexpand: true,

                gtk::Label {
                    set_label: &model.label,
                    #[watch]
                    set_visible: model.show_label,
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },
                #[name = "value_label"]
                gtk::Label {
                    set_label: format!("{:.2}", init.default_value).as_str(),
                    #[watch]
                    set_visible: model.show_value,
                    set_halign: gtk::Align::End,
                    set_hexpand: true,
                    set_css_classes: &["monospace", "dim-label"],
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AdjustRowModel {
            label: init.label,
            show_label: init.show_label,
            show_value: init.show_value,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AdjustRowMsg::DragUpdate(target) => {
                let old_value = widgets.slider.value();
                widgets.slider.drag_update(target);
                let new_value = widgets.slider.value();

                if old_value != new_value {
                    widgets
                        .value_label
                        .set_label(format!("{:.2}", new_value).as_str());
                    sender
                        .output(AdjustRowOutput::ValueChanged(new_value as f64))
                        .unwrap();
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
