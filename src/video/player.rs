use std::fmt::Debug;

use gst::glib::FlagsClass;
use gst::prelude::{ElementExt, ElementExtManual, ObjectExt};
use gst::{Bus, ClockTime, SeekFlags, State};

use crate::video::metadata_discoverer::VideoInfo;

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

        let crop = gst::ElementFactory::make("videocrop")
            .name("crop")
            .build()
            .unwrap();

        playbin.set_property("video-sink", &sink);
        playbin.set_property("video-filter", &crop);

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

    pub fn set_info(&mut self, info: VideoInfo) {
        self.info = info;
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
