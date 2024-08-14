use gtk4::prelude::{ButtonExt, ListBoxRowExt, WidgetExt};
use relm4::adw::prelude::{ComboRowExt, ExpanderRowExt, PreferencesRowExt};
use relm4::{adw, gtk, Component, ComponentParts, ComponentSender};

use crate::ui::output_controls::OutputControlsMsg::{
    AudioCodecChange, ContainerChange, CustomCodecSelected, VideoCodecChange,
};
use crate::video::metadata::{AudioCodec, VideoCodec, VideoCodecInfo, VideoContainer};

pub struct OutputControlsModel {
    default_codec: VideoCodecInfo,
    selected_codec: VideoCodecInfo,
    non_default_selected: bool,
}

#[derive(Debug)]
pub enum OutputControlsMsg {
    DefaultCodecs(VideoCodecInfo),
    CustomCodecSelected(bool),
    VideoCodecChange(VideoCodec),
    AudioCodecChange(AudioCodec),
    ContainerChange(VideoContainer),
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
                #[name= "codec_row"]
                adw::ExpanderRow {
                    set_title: "Custom output format",
                    set_show_enable_switch: true,
                    set_enable_expansion: false,

                    add_row: video_row = &adw::ComboRow{
                        set_title: "Video Codec",
                        #[wrap(Some)]
                        set_model = &VideoCodec::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let codec = VideoCodec::from_string_list_index(dropdown.selected());
                            sender.input(VideoCodecChange(codec));
                        }
                    },

                    add_row: audio_row = &adw::ComboRow{
                        set_title: "Audio Codec",
                        #[wrap(Some)]
                        set_model = &AudioCodec::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let codec = AudioCodec::from_string_list_index(dropdown.selected());
                            sender.input(AudioCodecChange(codec));
                        }
                    },

                    add_row: container_row = &adw::ComboRow{
                        set_title: "Output Container",
                        #[wrap(Some)]
                        set_model = &VideoContainer::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let container = VideoContainer::from_string_list_index(dropdown.selected());
                            sender.input(ContainerChange(container));
                        }
                    },

                    connect_enable_expansion_notify[sender] => move |row| {
                        sender.input(CustomCodecSelected(row.enables_expansion()))
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
        let model = OutputControlsModel {
            default_codec: VideoCodecInfo::default(),
            selected_codec: VideoCodecInfo::default(),
            non_default_selected: false,
        };

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
                self.default_codec = defaults;
                self.selected_codec = defaults;

                let audio_idx = defaults.audio_codec.to_string_list_index();
                let video_idx = defaults.video_codec.to_string_list_index();
                let container_idx = defaults.container.to_string_list_index();

                widgets.video_row.set_selected(video_idx);
                widgets.container_row.set_selected(container_idx);

                println!("defaults: {:?}", defaults);

                match defaults.audio_codec {
                    AudioCodec::NoAudio => {
                        widgets.audio_row.set_selectable(false);
                    }
                    _ => widgets.audio_row.set_selected(audio_idx),
                }
            }
            // todo: some bookkeeping to keep selected_changed accurate
            VideoCodecChange(codec) => self.selected_codec.video_codec = codec,
            AudioCodecChange(codec) => self.selected_codec.audio_codec = codec,
            ContainerChange(container) => self.selected_codec.container = container,
            CustomCodecSelected(enabled) => self.non_default_selected = enabled,
        }
        self.update_view(widgets, sender);
    }
}

impl OutputControlsModel {
    pub fn export_settings(&self) -> VideoCodecInfo {
        if self.non_default_selected {
            self.selected_codec
        } else {
            self.default_codec
        }
    }
}
