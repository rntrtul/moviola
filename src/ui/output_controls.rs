use gtk4::prelude::{ButtonExt, ListBoxRowExt, WidgetExt};
use relm4::adw::prelude::{ActionRowExt, ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use relm4::{adw, gtk, Component, ComponentParts, ComponentSender};

use crate::ui::output_controls::OutputControlsMsg::{
    AudioCodecChange, ContainerChange, CustomEncoding, VideoCodecChange,
};
use crate::video::metadata::{AudioCodec, ContainerFormat, VideoCodec, VideoContainerInfo};

pub struct OutputControlsModel {
    default_codec: VideoContainerInfo,
    selected_codec: VideoContainerInfo,
    custom_encoding: bool,
}

#[derive(Debug)]
pub enum OutputControlsMsg {
    DefaultCodecs(VideoContainerInfo),
    CustomEncoding(bool),
    VideoCodecChange(VideoCodec),
    AudioCodecChange(AudioCodec),
    ContainerChange(ContainerFormat),
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
        let model = OutputControlsModel {
            default_codec: VideoContainerInfo::default(),
            selected_codec: VideoContainerInfo::default(),
            custom_encoding: false,
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
            OutputControlsMsg::DefaultCodecs(defaults) => {
                self.default_codec = defaults;
                self.selected_codec = defaults;

                let audio_idx = defaults.audio_codec.to_string_list_index();
                let video_idx = defaults.video_codec.to_string_list_index();
                let container_idx = defaults.container.to_string_list_index();

                widgets.video_codec_row.set_selected(video_idx);
                widgets.container_row.set_selected(container_idx);

                match defaults.audio_codec {
                    AudioCodec::NoAudio => {
                        widgets.audio_codec_row.set_selectable(false);
                    }
                    _ => widgets.audio_codec_row.set_selected(audio_idx),
                }
            }
            // todo: some bookkeeping to keep selected_changed accurate
            VideoCodecChange(codec) => self.selected_codec.video_codec = codec,
            AudioCodecChange(codec) => self.selected_codec.audio_codec = codec,
            ContainerChange(container) => self.selected_codec.container = container,
            CustomEncoding(enabled) => self.custom_encoding = enabled,
        }
        self.update_view(widgets, sender);
    }
}

impl OutputControlsModel {
    pub fn export_settings(&self) -> VideoContainerInfo {
        // todo: pass container info regardless
        //  changing container shouldn't trigger a reencoding
        if self.custom_encoding {
            self.selected_codec
        } else {
            self.default_codec
        }
    }
}
