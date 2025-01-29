use crate::range::Range;
use crate::ui::preview::{CropMode, Orientation};
use crate::ui::slider::adjust_row::{AdjustRowModel, AdjustRowMsg, AdjustRowOutput};
use relm4::adw::prelude::{ComboRowExt, PreferencesRowExt};
use relm4::gtk::prelude::{OrientableExt, WidgetExt};
use relm4::{
    adw, gtk, Component, ComponentController, ComponentParts, ComponentSender, Controller,
};

pub struct CropPageModel {
    straighten_slider: Controller<AdjustRowModel>,
    crop_mode: CropMode,
    orientation: Orientation,
}

#[derive(Debug)]
pub enum CropPageMsg {
    SetCropMode(CropMode),
    SetBaseOrientation(Orientation),
    StraightenBegin,
    Straighten(f64),
    StraightenEnd,
    RotateRight90,
    FlipHorizontally,
    FlipVertically,
    Reset,
}

#[derive(Debug)]
pub enum CropPageOutput {
    OrientVideo(Orientation),
    SetCropMode(CropMode),
    StraigtenBegin,
    Straighten(f64),
    StraightenEnd,
}

#[relm4::component(pub)]
impl Component for CropPageModel {
    type CommandOutput = ();
    type Input = CropPageMsg;
    type Output = CropPageOutput;
    type Init = ();

    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            adw::PreferencesGroup{
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    model.straighten_slider.widget(){},
                },
            },

            adw::PreferencesGroup{
                adw::SwitchRow{
                    set_title: "Vertical Flip",
                    connect_active_notify => CropPageMsg::FlipVertically,
                },

                adw::SwitchRow{
                    set_title: "Horizontal Flip",
                    connect_active_notify => CropPageMsg::FlipHorizontally,
                },
            },

            adw::PreferencesGroup{
                #[name="crop_mode_row"]
                adw::ComboRow {
                    set_title: "Aspect Ratio",
                    #[wrap(Some)]
                    set_model = &gtk::StringList::new(
                        &["Free", "Original", "Square", "16:9", "4:5", "5:7", "4:3", "3:5", "3:2"]),
                    set_selected: 0,

                    connect_selected_item_notify [sender] => move |dropdown| {
                        let mode = match dropdown.selected() {
                            0 => CropMode::Free,
                            1 => CropMode::Original,
                            2 => CropMode::Square,
                            3 => CropMode::_16To9,
                            4 => CropMode::_4To5,
                            5 => CropMode::_5To7,
                            6 => CropMode::_4To3,
                            7 => CropMode::_3To5,
                            8 => CropMode::_3To2,
                            _ => panic!("Unknown crop mode selected")
                        };
                        sender.input(CropPageMsg::SetCropMode(mode));
                    }
                },
            },

            adw::PreferencesGroup {
                set_valign: gtk::Align::End,
                set_vexpand: true,

                adw::ButtonRow {
                    set_title: "Reset",
                    add_css_class: "destructive-action",
                    connect_activated => CropPageMsg::Reset,
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let straighten_slider = AdjustRowModel::build_slider(
            "Straighten",
            (Range::new(-45.0, 45.0), Range::new(-45.0, 45.0)),
        )
        .forward(sender.input_sender(), |msg| match msg {
            AdjustRowOutput::DragBegin => CropPageMsg::StraightenBegin,
            AdjustRowOutput::ValueChanged(val) => CropPageMsg::Straighten(val),
            AdjustRowOutput::DragEnd => CropPageMsg::StraightenEnd,
        });

        let model = CropPageModel {
            straighten_slider,
            crop_mode: CropMode::Free,
            orientation: Orientation::default(),
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
            CropPageMsg::Reset => {
                self.orientation.reset();
                self.crop_mode = CropMode::Free;

                widgets.crop_mode_row.set_selected(0);
                self.straighten_slider.emit(AdjustRowMsg::SilentReset);

                sender.output(CropPageOutput::Straighten(0f64)).unwrap();
                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap();
                sender
                    .output(CropPageOutput::SetCropMode(self.crop_mode))
                    .unwrap();
            }
            CropPageMsg::SetBaseOrientation(orientation) => {
                self.orientation = orientation;
                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropPageMsg::SetCropMode(mode) => {
                self.crop_mode = mode;
                sender.output(CropPageOutput::SetCropMode(mode)).unwrap();
            }
            CropPageMsg::StraightenBegin => sender.output(CropPageOutput::StraigtenBegin).unwrap(),
            CropPageMsg::Straighten(angle) => {
                sender.output(CropPageOutput::Straighten(angle)).unwrap();
            }
            CropPageMsg::StraightenEnd => sender.output(CropPageOutput::StraightenEnd).unwrap(),
            CropPageMsg::RotateRight90 => {
                self.orientation.rotate_90_clockwise();
                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropPageMsg::FlipHorizontally => {
                self.orientation.mirror_horizontally();
                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
            CropPageMsg::FlipVertically => {
                self.orientation.mirror_vertically();

                sender
                    .output(CropPageOutput::OrientVideo(self.orientation))
                    .unwrap()
            }
        }
    }
}

impl CropPageModel {
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn rotation_angle(&self) -> f32 {
        self.straighten_slider.model().slider_value() as f32
    }
}
