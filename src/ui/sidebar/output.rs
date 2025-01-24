use relm4::adw::prelude::{ActionRowExt, ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use relm4::gtk::prelude::{ListBoxRowExt, WidgetExt};
use relm4::{adw, Component, ComponentParts, ComponentSender};

use crate::ui::sidebar::output::OutputPageMsg::{
    AudioCodecChange, AudioStreamChange, ContainerChange, CustomEncoding, VideoCodecChange,
};
use crate::ui::sidebar::OutputContainerSettings;
use crate::video::metadata::{
    AudioCodec, ContainerFormat, VideoCodec, VideoContainerInfo, AUDIO_BITRATE_DEFAULT,
};

pub struct OutputPageModel {
    video_info: VideoContainerInfo,
    export_settings: OutputContainerSettings,
    selected_audio_stream_idx: u32,
    custom_encoding: bool,
}

#[derive(Debug)]
pub enum OutputPageMsg {
    VideoInfo(VideoContainerInfo),
    CustomEncoding(bool),
    VideoCodecChange(VideoCodec),
    AudioCodecChange(AudioCodec),
    AudioStreamChange(u32),
    ContainerChange(ContainerFormat),
}

#[derive(Debug)]
pub enum OutputPageOutput {
    ExportFrame,
}

#[relm4::component(pub)]
impl Component for OutputPageModel {
    type Input = OutputPageMsg;
    type Output = OutputPageOutput;
    type CommandOutput = ();
    type Init = ();

    view! {
        adw::PreferencesPage{
            set_hexpand: true,

            adw::PreferencesGroup {
                adw::SwitchRow {
                    set_title: "Custom encoding",
                    set_subtitle: "will lose lossless enconding",

                    connect_active_notify[sender] => move |row| {
                        sender.input(CustomEncoding(row.is_active()))
                    },
                }
            },

            adw::PreferencesGroup {
                #[watch]
                set_sensitive: model.custom_encoding,
                #[name= "container_row"]
                adw::ComboRow{
                        set_title: "Output Container",
                        #[wrap(Some)]
                        set_model = &ContainerFormat::string_list(),
                        connect_selected_item_notify [sender] => move |dropdown| {
                            let container = ContainerFormat::from_string_list_index(dropdown.selected());
                            sender.input(ContainerChange(container));
                        }
                    }
            },

            adw::PreferencesGroup {
                set_title: "Video",
                #[watch]
                set_sensitive: model.custom_encoding,
                #[name= "video_codec_row"]
                adw::ComboRow{
                    set_title: "Codec",
                    #[wrap(Some)]
                    set_model = &VideoCodec::string_list(),
                    connect_selected_item_notify [sender] => move |dropdown| {
                        let codec = VideoCodec::from_string_list_index(dropdown.selected());
                        sender.input(VideoCodecChange(codec));
                    }
                },
            },

             adw::PreferencesGroup {
                set_title: "Audio",
                #[watch]
                set_sensitive: model.custom_encoding,

                #[name= "audio_stream_row"]
                adw::ComboRow{
                    set_title: "Stream",
                    set_visible: false,
                    connect_selected_item_notify [sender] => move |dropdown| {
                        sender.input(AudioStreamChange(dropdown.selected()))
                    }
                },

                #[name= "audio_codec_row"]
                adw::ComboRow{
                    set_title: "Codec",
                    #[wrap(Some)]
                    set_model = &AudioCodec::string_list(),
                    connect_selected_item_notify [sender] => move |dropdown| {
                        let codec = AudioCodec::from_string_list_index(dropdown.selected());
                        sender.input(AudioCodecChange(codec));
                    }
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let settings = OutputContainerSettings {
            no_audio: false,
            audio_stream_idx: 0,
            audio_codec: AudioCodec::Unknown,
            audio_bitrate: 0,
            video_bitrate: 0,
            video_codec: VideoCodec::Unknown,
            container: ContainerFormat::Unknown,
        };

        let model = OutputPageModel {
            video_info: VideoContainerInfo::default(),
            export_settings: settings,
            custom_encoding: false,
            selected_audio_stream_idx: 0,
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
            OutputPageMsg::VideoInfo(video_info) => {
                self.video_info = video_info.clone();
                self.export_settings = self.export_settings_from_video_info();

                let video_idx = video_info.video_codec.to_string_list_index();
                let container_idx = video_info.container.to_string_list_index();

                widgets.video_codec_row.set_selected(video_idx);
                widgets.container_row.set_selected(container_idx);

                if self.video_info.audio_streams.len() >= 2 {
                    widgets.audio_stream_row.set_visible(true);
                    widgets
                        .audio_stream_row
                        .set_model(Some(&self.video_info.audio_streams_string_list()));
                }

                if !self.video_info.audio_streams.is_empty() {
                    let first_stream_codec = self.video_info.audio_streams.first().unwrap().codec;
                    let audio_idx = first_stream_codec.to_string_list_index();

                    match first_stream_codec {
                        AudioCodec::NoAudio => {
                            widgets.audio_codec_row.set_selectable(false);
                        }
                        _ => widgets.audio_codec_row.set_selected(audio_idx),
                    }
                }
            }
            VideoCodecChange(codec) => self.export_settings.video_codec = codec,
            AudioCodecChange(codec) => self.export_settings.audio_codec = codec,
            AudioStreamChange(stream_idx) => {
                self.export_settings.audio_stream_idx = stream_idx;

                let stream_codec = self.video_info.audio_streams[stream_idx as usize].codec;
                let audio_idx = stream_codec.to_string_list_index();

                match stream_codec {
                    AudioCodec::NoAudio => {
                        widgets.audio_codec_row.set_selectable(false);
                    }
                    _ => widgets.audio_codec_row.set_selected(audio_idx),
                }
            }
            ContainerChange(container) => self.export_settings.container = container,
            CustomEncoding(enabled) => {
                self.custom_encoding = enabled;
            }
        }
        self.update_view(widgets, sender);
    }
}

impl OutputPageModel {
    fn export_settings_from_video_info(&self) -> OutputContainerSettings {
        OutputContainerSettings {
            no_audio: false,
            audio_stream_idx: 0,
            video_bitrate: self.video_info.video_bitrate,
            video_codec: self.video_info.video_codec,
            container: self.video_info.container,
            audio_bitrate: if !self.video_info.audio_streams.is_empty() {
                self.video_info.audio_streams[0].bitrate
            } else {
                AUDIO_BITRATE_DEFAULT
            },
            audio_codec: if !self.video_info.audio_streams.is_empty() {
                self.video_info.audio_streams[0].codec
            } else {
                AudioCodec::NoAudio
            },
        }
    }

    pub fn export_settings(&self) -> OutputContainerSettings {
        if !self.custom_encoding {
            self.export_settings_from_video_info()
        } else {
            // todo: pass container info regardless
            //  changing container shouldn't trigger a reencoding
            self.export_settings
        }
    }
}
