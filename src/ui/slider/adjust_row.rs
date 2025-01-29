use crate::range::Range;
use crate::ui::slider::slider::SliderFillMode;
use crate::ui::slider::Slider;
use relm4::component::Connector;
use relm4::gtk::prelude::{GestureDragExt, WidgetExt};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

#[derive(Debug)]
pub struct AdjustRowModel {
    label: String,
    show_label: bool,
    show_value: bool,
    display_range: Range,
    value: f64,
}

#[derive(Debug)]
pub enum AdjustRowMsg {
    DragBegin,
    DragUpdate(f64),
    DragEnd,
    Reset,
    SilentReset,
}

#[derive(Debug)]
pub enum AdjustRowOutput {
    DragBegin,
    ValueChanged(f64),
    DragEnd,
}

#[derive(Debug)]
pub struct AdjustRowInit {
    label: String,
    show_label: bool,
    show_value: bool,
    value_range: Range,
    display_range: Range,
    fill_mode: SliderFillMode,
}

impl AdjustRowInit {
    pub fn new(
        label: &str,
        show_label: bool,
        show_value: bool,
        value_range: Range,
        display_range: Range,
        fill_mode: SliderFillMode,
    ) -> Self {
        Self {
            label: label.to_string(),
            show_label,
            show_value,
            value_range,
            display_range,
            fill_mode,
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
            set_child: slider = &Slider::new_with_range(init.value_range, init.fill_mode) {
                add_controller = gtk::GestureDrag {
                    connect_drag_begin[sender] => move |_,_,_| {
                        sender.input(AdjustRowMsg::DragBegin);
                    },
                    connect_drag_update[sender] => move |drag,x_offset,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let target = start_x + x_offset;
                        sender.input(AdjustRowMsg::DragUpdate(target))
                    },
                    connect_drag_end[sender] => move |_,_,_| {
                        sender.input(AdjustRowMsg::DragEnd);
                    },
                },
                add_controller = gtk::GestureClick {
                    connect_released[sender] => move |_, presses,_, _| {
                        if presses == 2 {
                            sender.input(AdjustRowMsg::Reset);
                        }
                    },
                },
            },

            add_overlay = &gtk::Box {
                set_can_target: false,
                set_hexpand: true,
                set_margin_start: 12,
                set_margin_end: 12,

                gtk::Label {
                    set_label: &model.label,
                    set_visible: model.show_label,
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                },
                #[name = "value_label"]
                gtk::Label {
                    set_label: model.format_init_display_value(init.value_range).as_str(),
                    set_visible: model.show_value,
                    set_halign: gtk::Align::End,
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
            value: init.value_range.default,
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
            AdjustRowMsg::DragBegin => sender.output(AdjustRowOutput::DragBegin).unwrap(),
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
            AdjustRowMsg::DragEnd => {
                self.value = widgets.slider.value();
                sender.output(AdjustRowOutput::DragEnd).unwrap();
            }
            AdjustRowMsg::Reset => {
                widgets.slider.reset();
                self.update_label_from_slider(&widgets.slider, &widgets.value_label);
                sender
                    .output(AdjustRowOutput::ValueChanged(widgets.slider.value()))
                    .unwrap();
            }
            AdjustRowMsg::SilentReset => {
                widgets.slider.reset();
                self.update_label_from_slider(&widgets.slider, &widgets.value_label);
            }
        }

        self.update_view(widgets, sender);
    }
}

impl AdjustRowModel {
    pub fn slider_value(&self) -> f64 {
        self.value
    }

    fn update_label_from_slider(&self, slider: &Slider, label: &relm4::gtk::Label) {
        let display_value = slider.map_value_to_range(self.display_range);
        let display_str = self.format_display_value(display_value);
        label.set_label(display_str.as_str());
    }

    fn format_display_value(&self, value: f64) -> String {
        format!("{:.0}", value)
    }

    fn format_init_display_value(&self, value_range: Range) -> String {
        self.format_display_value(
            self.display_range
                .map_value_from_range(value_range, value_range.default),
        )
    }

    pub fn build_slider(
        label: &str,
        (val_range, display_range): (Range, Range),
    ) -> Connector<AdjustRowModel> {
        AdjustRowModel::builder().launch(AdjustRowInit::new(
            label,
            true,
            true,
            val_range,
            display_range,
            SliderFillMode::CenterOut,
        ))
    }
}
