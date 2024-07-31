use std::fmt::Debug;

use ges::prelude::{GESPipelineExt, GESTrackExt, LayerExt, TimelineExt};
use ges::PipelineFlags;
use gst::prelude::{ElementExt, ElementExtManual};
use gst::{ClockTime, SeekFlags, State};

use crate::video::metadata_discoverer::VideoInfo;

#[derive(Debug)]
pub struct Player {
    is_mute: bool,
    is_playing: bool,
    pipeline: ges::Pipeline,
    info: VideoInfo,
}

impl Player {
    pub fn new(sink: &gst::Element) -> Self {
        let timeline = ges::Timeline::new_audio_video();
        timeline.append_layer();
        let pipeline = ges::Pipeline::new();
        pipeline.set_timeline(&timeline).unwrap();

        pipeline
            .set_mode(PipelineFlags::FULL_PREVIEW)
            .expect("unable to preview");
        pipeline.preview_set_video_sink(Some(sink));

        pipeline.set_state(State::Ready).unwrap();

        Self {
            is_mute: false,
            is_playing: false,
            pipeline,
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
        self.info
    }

    pub fn position(&self) -> ClockTime {
        let result = self.pipeline.query_position::<ClockTime>();

        result.unwrap_or_else(|| ClockTime::ZERO)
    }

    pub fn set_info(&mut self, info: VideoInfo) {
        self.info = info;
    }
    pub fn set_is_mute(&mut self, is_mute: bool) {
        self.is_mute = is_mute;
        // self.clip.set_mute(is_mute);
    }
    pub fn set_is_playing(&mut self, play: bool) {
        self.is_playing = play;

        let state = if play { State::Playing } else { State::Paused };
        self.pipeline.set_state(state).unwrap();
    }

    pub fn toggle_mute(&mut self) {
        self.set_is_mute(!self.is_mute);
    }
    pub fn toggle_play_plause(&mut self) {
        self.set_is_playing(!self.is_playing);
    }

    pub fn export_frame(&self) {
        // todo: ask for file location and name
        // is it taking the 720p playback and screenshotting that?
        self.pipeline
            .save_thumbnail(
                self.info.width as i32,
                self.info.height as i32,
                "image/jpeg",
                "/home/fareed/Videos/export.jpg",
            )
            .expect("unable to save exported frame");
    }

    pub fn seek_to_percent(&self, percent: f64) {
        let seconds = (self.info.duration.seconds() as f64 * percent) as u64;

        let time = gst::GenericFormattedValue::from(ClockTime::from_seconds(seconds));
        let seek = gst::event::Seek::new(
            1.0,
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT,
            gst::SeekType::Set,
            time,
            gst::SeekType::End,
            ClockTime::ZERO,
        );

        self.pipeline.send_event(seek);
    }

    pub fn play_uri(&mut self, uri: String) {
        // fixme: why does this take 2 seconds
        self.pipeline.set_state(State::Null).unwrap();

        let timeline = self.pipeline.timeline().unwrap();
        let layers = timeline.layers();
        let layer = layers.first().unwrap();

        let clip = ges::UriClip::new(uri.as_str()).expect("failed to create clip");

        let clips = layer.clips();
        if !clips.is_empty() {
            let prev_clip = clips.first().unwrap();
            layer
                .remove_clip(prev_clip)
                .expect("unable to delet eprior clip");
        }
        layer.add_clip(&clip).expect("unable to add clip");

        // todo: swap for portrait video
        let preview_width = 640;
        let preview_height = preview_width as f64 / self.info.aspect_ratio;

        let preview_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", self.info.framerate)
            .field("width", preview_width)
            .field("height", preview_height as i32)
            .build();
        let tracks = timeline.tracks();
        let track = tracks.first().unwrap();
        track.set_restriction_caps(&preview_caps);

        self.pipeline.set_state(State::Playing).unwrap();

        self.is_mute = false;
    }

    pub fn pipeline_bus(&self) -> gst::Bus {
        self.pipeline.bus().unwrap()
    }

    pub fn wait_for_pipeline_init(bus: gst::Bus) {
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
