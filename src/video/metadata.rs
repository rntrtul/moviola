use gst::ClockTime;
use relm4::gtk;

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
    QUICKTIME,
    WEBM,
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

    pub fn caps_name(&self) -> &str {
        match self {
            AudioCodec::AAC => "audio/mpeg",
            AudioCodec::OPUS => "audio/x-opus",
            AudioCodec::RAW => "audio/x-raw",
            AudioCodec::Unknown => "",
            AudioCodec::NoAudio => "",
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
            VideoCodec::MPEG => "MPEG",
            VideoCodec::VP8 => "VP8",
            VideoCodec::VP9 => "VP9",
            VideoCodec::X264 => "H264",
            VideoCodec::X265 => "H265",
            VideoCodec::Unknown => "Unknown",
        }
    }

    pub fn caps_name(&self) -> &str {
        match self {
            VideoCodec::AV1 => "video/x-av1",
            VideoCodec::MPEG => "video/mpeg",
            VideoCodec::VP8 => "video/x-vp8",
            VideoCodec::VP9 => "video/x-vp9",
            VideoCodec::X264 => "video/x-h264",
            VideoCodec::X265 => "video/x-h265",
            VideoCodec::Unknown => "",
        }
    }

    pub fn string_list() -> gtk::StringList {
        gtk::StringList::new(&[
            VideoCodec::AV1.display(),
            VideoCodec::MPEG.display(),
            VideoCodec::VP8.display(),
            VideoCodec::VP9.display(),
            VideoCodec::X264.display(),
            VideoCodec::X265.display(),
        ])
    }

    pub fn from_string_list_index(idx: u32) -> Self {
        match idx {
            0 => VideoCodec::AV1,
            1 => VideoCodec::MPEG,
            2 => VideoCodec::VP8,
            3 => VideoCodec::VP9,
            4 => VideoCodec::X264,
            5 => VideoCodec::X265,
            _ => VideoCodec::Unknown,
        }
    }

    pub fn to_string_list_index(&self) -> u32 {
        match self {
            VideoCodec::AV1 => 0,
            VideoCodec::MPEG => 1,
            VideoCodec::VP8 => 2,
            VideoCodec::VP9 => 3,
            VideoCodec::X264 => 4,
            VideoCodec::X265 => 5,
            VideoCodec::Unknown => 100,
        }
    }

    pub fn from_description(description: &str) -> Self {
        match description {
            desc if desc.contains("AV1") => VideoCodec::AV1,
            desc if desc.contains("MPEG") => VideoCodec::MPEG,
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
            VideoContainer::WEBM => "WEBM",
            VideoContainer::Unknown => "Unknown",
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            VideoContainer::MP4 => "mp4",
            VideoContainer::MKV => "mkv",
            VideoContainer::QUICKTIME => "mov",
            VideoContainer::WEBM => "webm",
            VideoContainer::Unknown => "",
        }
    }

    pub fn caps_name(&self) -> &str {
        match self {
            VideoContainer::MP4 => "video/mpeg",
            VideoContainer::MKV => "video/x-matroska",
            VideoContainer::QUICKTIME => "video/quicktime",
            VideoContainer::WEBM => "video/x-webm",
            VideoContainer::Unknown => "",
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
            VideoContainer::WEBM => 3,
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
