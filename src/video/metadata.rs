use gst::caps::{Builder, NoFeature};
use gst::ClockTime;
use relm4::gtk;

// todo: handle multiple audio streams
// todo: include audio and video bitrate
#[derive(Debug, Clone, Copy)]
pub struct VideoCodecInfo {
    pub(crate) container: VideoContainer,
    pub(crate) video_codec: VideoCodec,
    pub(crate) audio_codec: AudioCodec,
}

// todo: rename to ContainerInfo, container -> container-format
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
    QUICKTIME,
    Unknown,
}

impl AudioCodec {
    pub fn display(&self) -> &str {
        match self {
            AudioCodec::AAC => "AAC",
            AudioCodec::OPUS => "Opus",
            AudioCodec::RAW => "Raw",
            AudioCodec::Unknown => "Unknown",
            AudioCodec::NoAudio => "No Audio",
        }
    }

    pub fn caps_builder(&self) -> Builder<NoFeature> {
        match self {
            AudioCodec::AAC => gst::Caps::builder("audio/mpeg"),
            AudioCodec::OPUS => gst::Caps::builder("audio/x-opus"),
            AudioCodec::RAW => gst::Caps::builder("audio/x-raw"),
            AudioCodec::Unknown => gst::Caps::builder(""),
            AudioCodec::NoAudio => gst::Caps::builder(""),
        }
    }

    pub fn string_list() -> gtk::StringList {
        gtk::StringList::new(&[
            AudioCodec::AAC.display(),
            AudioCodec::OPUS.display(),
            AudioCodec::RAW.display(),
        ])
    }

    pub fn from_string_list_index(idx: u32) -> Self {
        match idx {
            0 => AudioCodec::AAC,
            1 => AudioCodec::OPUS,
            2 => AudioCodec::RAW,
            _ => AudioCodec::Unknown,
        }
    }

    pub fn to_string_list_index(&self) -> u32 {
        match self {
            AudioCodec::AAC => 0,
            AudioCodec::OPUS => 1,
            AudioCodec::RAW => 2,
            AudioCodec::Unknown => 100,
            AudioCodec::NoAudio => 100,
        }
    }

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
    pub fn display(&self) -> &str {
        match self {
            VideoCodec::AV1 => "AV1",
            VideoCodec::VP8 => "VP8",
            VideoCodec::VP9 => "VP9",
            VideoCodec::X264 => "H264",
            VideoCodec::X265 => "H265",
            VideoCodec::Unknown => "Unknown",
        }
    }

    pub fn caps_builder(&self) -> Builder<NoFeature> {
        // todo: should this information be set on encoding profile or caps?
        // x-av1 needs stream-format: obu-stream, alignment: tu
        // x-265 needs stream-format: byte-stream, alignment: au, profile: alot of options (main or main 10)
        // x-264 needs stream-format: avc/byte-stream, alignment: au, profile: alot of options (high?)
        // x-vp9 needs profile 0
        // x-vp8 needs profile 0
        match self {
            VideoCodec::AV1 => gst::Caps::builder("video/x-av1"),
            VideoCodec::VP8 => gst::Caps::builder("video/x-vp8"),
            VideoCodec::VP9 => gst::Caps::builder("video/x-vp9"),
            VideoCodec::X264 => gst::Caps::builder("video/x-h264").field("profile", "high"),
            VideoCodec::X265 => gst::Caps::builder("video/x-h265"),
            VideoCodec::Unknown => gst::Caps::builder(""),
        }
    }

    pub fn string_list() -> gtk::StringList {
        gtk::StringList::new(&[
            VideoCodec::AV1.display(),
            VideoCodec::VP8.display(),
            VideoCodec::VP9.display(),
            VideoCodec::X264.display(),
            VideoCodec::X265.display(),
        ])
    }

    pub fn from_string_list_index(idx: u32) -> Self {
        match idx {
            0 => VideoCodec::AV1,
            1 => VideoCodec::VP8,
            2 => VideoCodec::VP9,
            3 => VideoCodec::X264,
            4 => VideoCodec::X265,
            _ => VideoCodec::Unknown,
        }
    }

    pub fn to_string_list_index(&self) -> u32 {
        match self {
            VideoCodec::AV1 => 0,
            VideoCodec::VP8 => 1,
            VideoCodec::VP9 => 2,
            VideoCodec::X264 => 3,
            VideoCodec::X265 => 4,
            VideoCodec::Unknown => 100,
        }
    }

    pub fn from_description(description: &str) -> Self {
        match description {
            desc if desc.contains("AV1") => VideoCodec::AV1,
            desc if desc.contains("VP8") => VideoCodec::VP8,
            desc if desc.contains("VP9") => VideoCodec::VP9,
            desc if desc.contains("H.264") => VideoCodec::X264,
            desc if desc.contains("H.265") => VideoCodec::X265,
            _ => VideoCodec::Unknown,
        }
    }
}

impl VideoContainer {
    pub fn display(&self) -> &str {
        match self {
            VideoContainer::MP4 => "MP4",
            VideoContainer::MKV => "MKV",
            VideoContainer::QUICKTIME => "Quicktime",
            VideoContainer::Unknown => "Unknown",
        }
    }

    // todo: use encoding profile file extension
    pub fn file_extension(&self) -> &str {
        match self {
            VideoContainer::MP4 => "mp4",
            VideoContainer::MKV => "mkv",
            VideoContainer::QUICKTIME => "mov",
            VideoContainer::Unknown => "",
        }
    }

    pub fn caps_builder(&self) -> Builder<NoFeature> {
        match self {
            VideoContainer::MP4 => gst::Caps::builder("video/quicktime").field("variant", "iso"),
            VideoContainer::MKV => gst::Caps::builder("video/x-matroska"),
            VideoContainer::QUICKTIME => {
                gst::Caps::builder("video/quicktime").field("variant", "apple")
            }
            VideoContainer::Unknown => gst::Caps::builder(""),
        }
    }

    pub fn string_list() -> gtk::StringList {
        gtk::StringList::new(&[
            VideoContainer::MP4.display(),
            VideoContainer::MKV.display(),
            VideoContainer::QUICKTIME.display(),
        ])
    }

    pub fn from_string_list_index(idx: u32) -> Self {
        match idx {
            0 => VideoContainer::MP4,
            1 => VideoContainer::MKV,
            2 => VideoContainer::QUICKTIME,
            _ => VideoContainer::Unknown,
        }
    }

    pub fn to_string_list_index(&self) -> u32 {
        match self {
            VideoContainer::MP4 => 0,
            VideoContainer::MKV => 1,
            VideoContainer::QUICKTIME => 2,
            VideoContainer::Unknown => 100,
        }
    }
    pub fn from_description(description: &str) -> Self {
        // see webm report as matroska?
        match description {
            desc if desc == "Matroska" => VideoContainer::MKV,
            desc if desc == "ISO MP4/M4A" => VideoContainer::MP4,
            desc if desc == "Quicktime" => VideoContainer::QUICKTIME,
            _ => VideoContainer::Unknown,
        }
    }
}
