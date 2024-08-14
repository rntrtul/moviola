use gst::ClockTime;

// todo: pull in codec info and rename file to metadata
// add e-ac-3 (atsc) ?
#[derive(Debug, Clone, Copy)]
pub enum AudioCodec {
    AAC,
    OPUS,
    RAW,
    Unknown,
    NoAudio,
}

#[derive(Debug, Clone, Copy)]
pub enum VideoCodec {
    AV1,
    MPEG,
    VP8,
    VP9,
    X264,
    X265,
    Unknown,
}
// maybe remove webm
#[derive(Debug, Clone, Copy)]
pub enum VideoContainer {
    MP4,
    MKV,
    MOV,
    WEBM,
    Unknown,
}

// todo: handle multiple audio streams
#[derive(Debug, Clone, Copy)]
pub struct VideoCodecInfo {
    pub(crate) container: VideoContainer,
    pub(crate) video_codec: VideoCodec,
    pub(crate) audio_codec: AudioCodec,
}

impl Default for VideoCodecInfo {
    fn default() -> Self {
        Self {
            container: VideoContainer::Unknown,
            video_codec: VideoCodec::Unknown,
            audio_codec: AudioCodec::Unknown,
        }
    }
}

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

impl AudioCodec {
    pub fn from_description(description: &str) -> Self {
        match description {
            desc if desc.starts_with("MPEG") => AudioCodec::AAC,
            desc if desc.starts_with("Opus") => AudioCodec::OPUS,
            desc if desc.starts_with("Raw") || desc.starts_with("Uncompressed") => AudioCodec::RAW,
            _ => AudioCodec::Unknown,
        }
    }
}

impl VideoCodec {
    pub fn from_description(description: &str) -> Self {
        match description {
            desc if desc.starts_with("AV1") => VideoCodec::AV1,
            desc if desc.starts_with("MPEG") => VideoCodec::MPEG,
            desc if desc.starts_with("VP8") => VideoCodec::VP8,
            desc if desc.starts_with("VP9") => VideoCodec::VP9,
            desc if desc.starts_with("H.264") => VideoCodec::X264,
            desc if desc.starts_with("H.265") => VideoCodec::X265,
            _ => VideoCodec::Unknown,
        }
    }
}

impl VideoContainer {
    pub fn from_description(description: &str) -> Self {
        // see webm report as matroska?
        match description {
            desc if desc == "Matroska" => VideoContainer::MKV,
            desc if desc == "ISO MP4/M4A" => VideoContainer::MP4,
            desc if desc == "Quicktime" => VideoContainer::MOV,
            _ => VideoContainer::Unknown,
        }
    }
}
