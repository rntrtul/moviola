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

#[relm4::component(pub)]
impl Component for AdjustRowModel {
    type Input = AdjustRowMsg;
    type Output = AdjustRowOutput;
    type Init = String;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Overlay{
            #[wrap(Some)]
            set_child: slider = &Slider::new() {
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
                    set_label: "0.00",
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
            label: init,
            show_label: true,
            show_value: true,
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
