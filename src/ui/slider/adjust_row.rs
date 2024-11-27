use crate::ui::slider::slider::Range;
use crate::ui::slider::Slider;
use gtk4::prelude::{GestureDragExt, WidgetExt};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

#[derive(Debug)]
pub struct AdjustRowModel {
    label: String,
    show_label: bool,
    show_value: bool,
    display_range: Range,
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
    value_range: Range,
    display_range: Range,
    default_value: f64,
}

impl AdjustRowInit {
    pub fn default_with_label(label: &str) -> Self {
        Self {
            label: label.to_string(),
            show_label: true,
            show_value: true,
            value_range: Range::default(),
            display_range: Range::new(-100.0, 100.0),
            default_value: 0.0,
        }
    }

    pub fn default_with_label_values(label: &str, min: f64, max: f64, default: f64) -> Self {
        Self {
            label: label.to_string(),
            show_label: true,
            show_value: true,
            value_range: Range::new(min, max),
            display_range: Range::new(-100.0, 100.0),
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
            set_child: slider = &Slider::new_with_range(init.value_range, init.default_value) {
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
                    set_label: model.format_init_display_value(init.value_range, init.default_value).as_str(),
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
            display_range: init.display_range,
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
                let old_value = widgets.slider.value_as_range_percent();
                widgets.slider.drag_update(target);
                let new_value = widgets.slider.value_as_range_percent();

                if old_value != new_value {
                    let display_value = widgets.slider.map_value_to_range(self.display_range);
                    let display_str = self.format_display_value(display_value);
                    widgets.value_label.set_label(display_str.as_str());
                    sender
                        .output(AdjustRowOutput::ValueChanged(widgets.slider.value()))
                        .unwrap();
                }
            }
        }

        self.update_view(widgets, sender);
    }
}

impl AdjustRowModel {
    fn format_display_value(&self, value: f64) -> String {
        format!("{:.0}", value)
    }

    fn format_init_display_value(&self, value_range: Range, default_value: f64) -> String {
        self.format_display_value(
            self.display_range
                .map_value_from_range(value_range, default_value),
        )
    }
}
