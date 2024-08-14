use gtk4::prelude::{ButtonExt, WidgetExt};
use relm4::adw::prelude::{ComboRowExt, ExpanderRowExt, PreferencesRowExt};
use relm4::{adw, gtk, Component, ComponentParts, ComponentSender, SimpleComponent};

use crate::video::metadata::{AudioCodec, VideoCodec, VideoCodecInfo, VideoContainer};

pub struct OutputControlsModel {}

#[derive(Debug)]
pub enum OutputControlsMsg {
    DefaultCodecs(VideoCodecInfo),
}

#[derive(Debug)]
pub enum OutputControlsOutput {
    ExportFrame,
}

#[relm4::component(pub)]
impl Component for OutputControlsModel {
    type Input = OutputControlsMsg;
    type Output = OutputControlsOutput;
    type CommandOutput = ();
    type Init = ();

    view! {
        adw::PreferencesPage{
            set_hexpand: true,

            adw::PreferencesGroup{
                adw::ExpanderRow {
                    set_title: "Codec",

                    add_row: video_row = &adw::ComboRow{
                        set_title: "Video Codec",
                        #[wrap(Some)]
                        set_model = &VideoCodec::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let codec = VideoCodec::from_string_list_index(dropdown.selected());
                        }
                    },

                    add_row: audio_row = &adw::ComboRow{
                        set_title: "Audio Codec",
                        #[wrap(Some)]
                        set_model = &AudioCodec::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let codec = AudioCodec::from_string_list_index(dropdown.selected());
                        }
                    },

                    add_row: container_row = &adw::ComboRow{
                        set_title: "Output Container",
                        #[wrap(Some)]
                        set_model = &VideoContainer::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let codec = VideoContainer::from_string_list_index(dropdown.selected());
                        }
                    },
                },
            },

            adw::PreferencesGroup{
                 gtk::Button {
                    set_label: "Export Frame",
                    connect_clicked[sender] => move |_| {
                        sender.output(OutputControlsOutput::ExportFrame).unwrap()
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
        let model = OutputControlsModel {};

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
            OutputControlsMsg::DefaultCodecs(defaults) => {
                let audio_idx = defaults.audio_codec.to_string_list_index();
                let video_idx = defaults.video_codec.to_string_list_index();
                let container_idx = defaults.container.to_string_list_index();

                widgets.audio_row.set_selected(audio_idx);
                widgets.video_row.set_selected(video_idx);
                widgets.container_row.set_selected(container_idx);
            }
        }
        self.update_view(widgets, sender);
    }
}

impl OutputControlsModel {}
