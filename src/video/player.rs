use std::fmt::Debug;

use crate::video::metadata::{
    AudioCodec, AudioStreamInfo, ContainerFormat, VideoCodec, VideoContainerInfo, VideoInfo,
    AUDIO_BITRATE_DEFAULT, VIDEO_BITRATE_DEFAULT,
};
use gst::glib::FlagsClass;
use gst::prelude::{ElementExt, ElementExtManual, ObjectExt, PadExt};
use gst::{Bus, ClockTime, SeekFlags, State};
use gst_video::VideoFrameExt;

#[derive(Debug)]
pub struct Player {
    pub(crate) is_mute: bool,
    pub(crate) is_playing: bool,
    pub(crate) playbin: gst::Element,
    pub(crate) info: VideoInfo,
}

impl Player {
    pub fn new(sink: &gst::Element) -> Self {
        // todo: set to lower resolution for preview. might save more memory (higher cpu?)
        let playbin = gst::ElementFactory::make("playbin")
            .name("playbin")
            .build()
            .unwrap();

        let flags = playbin.property_value("flags");
        let flags_class = FlagsClass::with_type(flags.type_()).unwrap();

        let flags = flags_class
            .builder_with_value(flags)
            .unwrap()
            .set_by_nick("audio")
            .set_by_nick("video")
            .unset_by_nick("text")
            .build()
            .unwrap();
        playbin.set_property_from_value("flags", &flags);

        let app_sink = gst_app::AppSink::builder()
            .enable_last_sample(true)
            .max_buffers(1)
            .caps(
                &gst_video::VideoCapsBuilder::new()
                    .format(gst_video::VideoFormat::Rgba)
                    .build(),
            )
            .build();

        app_sink.set_callbacks(
            (gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().unwrap();
                    let info = sample
                        .caps()
                        .and_then(|caps| gst_video::VideoInfo::from_caps(caps).ok())
                        .unwrap();
                    let mut buffer = sample.buffer().unwrap();

                    // println!("got buffer size: {}, {}", buffer.size(), buffer.map_readable().unwrap().len());
                    // println!("got info {}, {}x{} {}", info.size(), info.width(), info.height(), info.format_info().to_string());

                    Ok(gst::FlowSuccess::Ok)
                })
                .build()),
        );

        // playbin.set_property("video-sink", &sink);
        playbin.set_property("video-sink", &app_sink);

        playbin.set_state(State::Ready).unwrap();

        Self {
            is_mute: false,
            is_playing: false,
            playbin,
            info: Default::default(),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn is_mute(&self) -> bool {
        self.is_mute
    }

    pub fn info(&self) -> VideoInfo {
        self.info.clone()
    }

    pub fn position(&self) -> ClockTime {
        let result = self.playbin.query_position::<ClockTime>();

        result.unwrap_or_else(|| ClockTime::ZERO)
    }

    pub fn reset_pipeline(&mut self) {
        self.playbin.set_state(State::Null).unwrap();
        self.is_playing = false;
        self.is_mute = false;
    }

    pub fn set_is_mute(&mut self, is_mute: bool) {
        self.is_mute = is_mute;
        self.playbin.set_property("mute", is_mute);
    }
    pub fn set_is_playing(&mut self, play: bool) {
        self.is_playing = play;

        let state = if play { State::Playing } else { State::Paused };
        self.playbin.set_state(state).unwrap();
    }

    pub fn toggle_mute(&mut self) {
        self.set_is_mute(!self.is_mute);
    }
    pub fn toggle_play_plause(&mut self) {
        self.set_is_playing(!self.is_playing);
    }

    pub fn seek(&self, timestamp: ClockTime) {
        let time = gst::GenericFormattedValue::from(timestamp);
        let seek = gst::event::Seek::new(
            1.0,
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT,
            gst::SeekType::Set,
            time,
            gst::SeekType::End,
            ClockTime::ZERO,
        );

        self.playbin.send_event(seek);
    }

    pub fn play_uri(&mut self, uri: String) {
        // fixme: why does this take 2 seconds
        self.playbin.set_state(State::Null).unwrap();
        self.playbin.set_property("uri", uri.as_str());
        self.playbin.set_state(State::Playing).unwrap();
        self.is_mute = false;
    }

    pub fn discover_metadata(&mut self) -> VideoInfo {
        let duration = self.playbin.query_duration::<ClockTime>().unwrap();

        let video_tags = self
            .playbin
            .emit_by_name::<Option<gst::TagList>>("get-video-tags", &[&0])
            .expect("no video stream present");
        let pad = self
            .playbin
            .emit_by_name::<Option<gst::Pad>>("get-video-pad", &[&0])
            .expect("no pad availble for video stream");
        let caps = pad.current_caps().unwrap();
        let cap_struct = caps.structure(0).unwrap();

        let width = cap_struct.get::<i32>("width").unwrap() as u32;
        let height = cap_struct.get::<i32>("height").unwrap() as u32;
        let framerate = cap_struct.get::<gst::Fraction>("framerate").unwrap();
        let aspect_ratio = width as f64 / height as f64;

        let video_codec = if let Some(tag) = video_tags.get::<gst::tags::VideoCodec>() {
            VideoCodec::from_description(tag.get())
        } else {
            VideoCodec::Unknown
        };

        let video_bitrate = if let Some(tag) = video_tags.get::<gst::tags::Bitrate>() {
            tag.get()
        } else {
            VIDEO_BITRATE_DEFAULT
        };

        let container = if let Some(tag) = video_tags.get::<gst::tags::ContainerFormat>() {
            ContainerFormat::from_description(tag.get())
        } else {
            ContainerFormat::Unknown
        };

        let num_audio_streams = self.playbin.property::<i32>("n-audio");
        let mut audio_streams_info: Vec<AudioStreamInfo> =
            Vec::with_capacity(num_audio_streams as usize);

        for i in 0..num_audio_streams {
            let audio_tags = self
                .playbin
                .emit_by_name::<Option<gst::TagList>>("get-audio-tags", &[&i])
                .expect("unable to get first audio stream");

            let audio_codec = if let Some(tag) = audio_tags.get::<gst::tags::AudioCodec>() {
                AudioCodec::from_description(tag.get())
            } else {
                AudioCodec::Unknown
            };

            let audio_bitrate = if let Some(tag) = audio_tags.get::<gst::tags::Bitrate>() {
                tag.get()
            } else {
                AUDIO_BITRATE_DEFAULT
            };

            let language = if let Some(tag) = audio_tags.get::<gst::tags::LanguageCode>() {
                tag.get().to_string()
            } else {
                "Unknown".to_string()
            };

            let stream_info = AudioStreamInfo {
                codec: audio_codec,
                bitrate: audio_bitrate,
                language,
            };

            audio_streams_info.push(stream_info);
        }

        let codec_info = VideoContainerInfo {
            container,
            video_codec,
            video_bitrate,
            audio_streams: audio_streams_info,
        };

        let video_info = VideoInfo {
            duration,
            framerate,
            width,
            height,
            aspect_ratio,
            container_info: codec_info,
        };
        self.info = video_info.clone();

        video_info
    }

    pub fn pipeline_bus(&self) -> Bus {
        self.playbin.bus().unwrap()
    }

    pub fn wait_for_pipeline_init(bus: Bus) {
        for msg in bus.iter_timed(ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::AsyncDone(..) => {
                    break;
                }
                _ => (),
            }
        }
    }
}
