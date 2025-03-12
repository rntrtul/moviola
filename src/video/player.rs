use crate::app::{App, AppMsg};
use crate::renderer::{RenderCmd, TimerCmd, TimerEvent};
use crate::ui::preview::Orientation;
use crate::video::metadata::{
    AudioCodec, AudioStreamInfo, ContainerFormat, VideoCodec, VideoContainerInfo, VideoInfo,
    AUDIO_BITRATE_DEFAULT, VIDEO_BITRATE_DEFAULT,
};
use gst::glib::FlagsClass;
use gst::prelude::{ElementExt, ElementExtManual, GstObjectExt, ObjectExt, PadExt};
use gst::{Bus, ClockTime, FlowSuccess, SeekFlags, State};
use gst_app::AppSink;
use relm4::ComponentSender;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::sync::mpsc;
use std::time::Instant;

#[derive(Debug)]
pub enum PlayerError {
    NoVideoLoaded,
}

#[derive(Debug)]
pub struct Player {
    pub(crate) pipeline_ready: bool,
    pub(crate) is_mute: bool,
    pub(crate) is_playing: bool,
    pub(crate) playbin: gst::Element,
    pub(crate) info: VideoInfo,
    is_finished: bool,
    pub(crate) app_sink: AppSink,
}

impl Player {
    pub fn new(
        app_sender: ComponentSender<App>,
        sample_sender: mpsc::Sender<RenderCmd>,
        timer_sender: mpsc::Sender<TimerCmd>,
    ) -> Self {
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

        let app_sink = video_appsink(
            app_sender,
            sample_sender,
            timer_sender,
            AppSinkUsage::Preview,
        );

        playbin.set_property("video-sink", &app_sink);

        playbin.set_state(State::Ready).unwrap();

        Self {
            pipeline_ready: false,
            is_mute: false,
            is_playing: false,
            is_finished: false,
            playbin,
            info: Default::default(),
            app_sink,
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

    pub fn position(&self) -> Result<ClockTime, PlayerError> {
        if !self.pipeline_ready || !self.is_playing {
            return Err(PlayerError::NoVideoLoaded);
        }

        let result = self.playbin.query_position::<ClockTime>();

        if result.is_none() {
            Err(PlayerError::NoVideoLoaded)
        } else {
            Ok(result.unwrap())
        }
    }

    pub fn reset_pipeline(&mut self) {
        self.playbin.set_state(State::Null).unwrap();
        self.is_playing = false;
        self.set_is_mute(false);
        self.pipeline_ready = false;
    }

    pub fn set_is_mute(&mut self, is_mute: bool) {
        self.is_mute = is_mute;
        self.playbin.set_property("mute", is_mute);
    }
    pub fn set_is_playing(&mut self, play: bool) {
        if !self.pipeline_ready {
            return;
        }

        self.is_playing = play;

        if play && self.is_finished {
            self.seek(ClockTime::ZERO);
            self.is_finished = false;
        }

        let state = if play { State::Playing } else { State::Paused };
        self.playbin.set_state(state).unwrap();
    }

    pub fn set_is_finished(&mut self) {
        if !self.pipeline_ready {
            return;
        }

        self.set_is_playing(false);

        self.is_finished = true;
    }

    pub fn toggle_mute(&mut self) {
        self.set_is_mute(!self.is_mute);
    }
    pub fn toggle_play_plause(&mut self) {
        self.set_is_playing(!self.is_playing);
    }

    pub fn seek(&self, timestamp: ClockTime) {
        self.playbin
            .seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, timestamp)
            .unwrap();
    }

    pub fn play_uri(&mut self, uri: String) {
        // fixme: why does this take 2 seconds.
        //  1.5 seconds spend on loading/initing nvcodec plugin
        self.playbin.set_state(State::Null).unwrap();
        self.pipeline_ready = false;
        self.playbin.set_property("uri", uri.as_str());
        self.pipeline_ready = true;
        self.set_is_playing(true);
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

        let orientation = if let Some(orientaiton) = video_tags.get::<gst::tags::ImageOrientation>()
        {
            // todo: generate orientation from tag
            Orientation::new_with_base(90f32)
        } else {
            Orientation::default()
        };

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
                title: "".to_string(),
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
            orientation,
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

#[derive(PartialEq)]
pub(crate) enum AppSinkUsage {
    Export,
    Preview,
}

pub(crate) fn video_appsink(
    app_sender: ComponentSender<App>,
    sample_sender: mpsc::Sender<RenderCmd>,
    timer_sender: mpsc::Sender<TimerCmd>,
    usage: AppSinkUsage,
) -> AppSink {
    // todo: way to customize eos message
    //  (for preview want app_sendr, for export want to just send none to stop appsrc)
    //  if export set sync to false?

    let preroll_sender = sample_sender.clone();
    let preroll_timer_sender = timer_sender.clone();
    AppSink::builder()
        .enable_last_sample(true)
        .max_buffers(1)
        .sync(usage == AppSinkUsage::Preview)
        .caps(
            &gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgba)
                .build(),
        )
        .callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().unwrap();
                    sample_sender.send(RenderCmd::RenderSample(sample)).unwrap();
                    timer_sender
                        .send(TimerCmd::Start(TimerEvent::FrameTime, Instant::now()))
                        .unwrap();
                    Ok(FlowSuccess::Ok)
                })
                .new_preroll(move |appsink| {
                    let sample = appsink.pull_preroll().unwrap();
                    preroll_sender
                        .send(RenderCmd::RenderSample(sample))
                        .unwrap();
                    preroll_timer_sender
                        .send(TimerCmd::Start(TimerEvent::FrameTime, Instant::now()))
                        .unwrap();
                    Ok(FlowSuccess::Ok)
                })
                .eos(move |_| {
                    // todo: implement a ExportingVideoFinished
                    let msg = match usage {
                        AppSinkUsage::Export => AppMsg::VideoFinished,
                        AppSinkUsage::Preview => AppMsg::VideoFinished,
                    };
                    app_sender.input(msg);
                })
                .build(),
        )
        .build()
}
