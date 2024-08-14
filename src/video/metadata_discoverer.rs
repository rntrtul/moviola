use gst::ClockTime;

use crate::video::codecs::VideoCodecInfo;

// todo: pull in codec info and rename file to metadata

#[derive(Debug, Clone, Copy)]
pub struct VideoInfo {
    pub(crate) duration: ClockTime,
    pub(crate) framerate: gst::Fraction,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) aspect_ratio: f64,
    pub(crate) codec_info: VideoCodecInfo,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            duration: ClockTime::ZERO,
            framerate: gst::Fraction::from(0),
            width: 0,
            height: 0,
            aspect_ratio: 0.,
            codec_info: VideoCodecInfo::default(),
        }
    }
}
