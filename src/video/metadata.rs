use crate::ui::preview::Orientation;
use gst::caps::{Builder, NoFeature};
use gst::ClockTime;
use gst_pbutils::prelude::DiscovererStreamInfoExt;
use gst_pbutils::{DiscovererAudioInfo, DiscovererInfo};
use relm4::gtk;

pub static VIDEO_BITRATE_DEFAULT: u32 = 3000000;
pub static AUDIO_BITRATE_DEFAULT: u32 = 128000;

// todo: add stream title
#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    pub(crate) codec: AudioCodec,
    pub(crate) bitrate: u32,
    pub(crate) language: String,
    pub(crate) title: String,
}

impl From<DiscovererAudioInfo> for AudioStreamInfo {
    fn from(info: DiscovererAudioInfo) -> Self {
        let mut codec = AudioCodec::Unknown;
        let mut title = "".to_string();

        if let Some(tags) = info.tags() {
            for (tag, val) in tags.iter() {
                // println!("{tag:?}: {val:?}");
                match tag.as_str() {
                    "audio-codec" => {
                        codec = AudioCodec::from_description(val.get::<&str>().unwrap());
                    }
                    "title" => {
                        title = val.get::<&str>().unwrap().to_string();
                    }
                    _ => {}
                }
            }
        }

        AudioStreamInfo {
            codec,
            title,
            bitrate: info.bitrate(),
            language: if let Some(lang) = info.language() {
                lang.to_string()
            } else {
                "".to_string()
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoContainerInfo {
    pub(crate) container: ContainerFormat,
    pub(crate) video_codec: VideoCodec,
    pub(crate) video_bitrate: u32,
    pub(crate) audio_streams: Vec<AudioStreamInfo>,
}

impl From<DiscovererInfo> for VideoContainerInfo {
    fn from(info: DiscovererInfo) -> Self {
        let mut audio_streams = vec![];
        for audio_info in info.audio_streams() {
            audio_streams.push(AudioStreamInfo::from(audio_info));
        }

        let mut video_codec = VideoCodec::Unknown;
        let mut container = ContainerFormat::Unknown;

        if let Some(tags) = info.tags() {
            for (tag, val) in tags.iter() {
                match tag.as_str() {
                    "video-codec" => {
                        video_codec = VideoCodec::from_description(val.get::<&str>().unwrap())
                    }
                    "container-format" => {
                        container = ContainerFormat::from_description(val.get::<&str>().unwrap())
                    }
                    _ => {}
                }
            }
        }

        let mut video_bitrate = 0;
        for vidinfo in info.video_streams() {
            video_bitrate = vidinfo.bitrate();
        }

        Self {
            container,
            video_codec,
            video_bitrate,
            audio_streams,
        }
    }
}

impl Default for VideoContainerInfo {
    fn default() -> Self {
        Self {
            container: ContainerFormat::Unknown,
            video_codec: VideoCodec::Unknown,
            video_bitrate: 0,
            audio_streams: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub(crate) duration: ClockTime,
    pub(crate) framerate: gst::Fraction,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) aspect_ratio: f64,
    pub(crate) container_info: VideoContainerInfo,
    pub(crate) orientation: Orientation,
}

impl From<DiscovererInfo> for VideoInfo {
    fn from(info: DiscovererInfo) -> Self {
        let mut width = 0;
        let mut height = 0;
        let mut framerate = gst::Fraction::new(0, 1);
        let mut orientation = Orientation::default();

        // todo: handle multiple video streams (need to update struct to match audiostream)
        for vidinfo in info.video_streams() {
            width = vidinfo.width();
            height = vidinfo.height();
            framerate = vidinfo.framerate();

            if let Some(tags) = vidinfo.tags() {
                for (tag, val) in tags.iter() {
                    match tag.as_str() {
                        "image-orientation" => {
                            // 7 is length of "rotate-" string
                            let degrees = val.get::<&str>().unwrap()[7..]
                                .to_string()
                                .parse::<u32>()
                                .unwrap();
                            println!("degress: {degrees}");
                            orientation.base_angle = degrees as f32;
                        }
                        _ => {}
                    }
                }
            }
        }

        let aspect_ratio = width as f64 / height as f64;
        let duration = info.duration().unwrap();
        let container_info = VideoContainerInfo::from(info);

        Self {
            duration,
            framerate,
            width,
            height,
            aspect_ratio,
            container_info,
            orientation,
        }
    }
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            duration: ClockTime::ZERO,
            framerate: gst::Fraction::from(0),
            width: 0,
            height: 0,
            aspect_ratio: 0.,
            container_info: VideoContainerInfo::default(),
            orientation: Orientation::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AudioCodec {
    AAC,
    AC3,
    DTS,
    EAC3,
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

#[derive(Debug, Clone, Copy)]
pub enum ContainerFormat {
    MP4,
    MKV,
    QUICKTIME,
    Unknown,
}

impl VideoContainerInfo {
    pub fn audio_streams_string_list(&self) -> gtk::StringList {
        let list = gtk::StringList::new(&[]);

        for stream in self.audio_streams.iter() {
            list.append(stream.language.as_str());
        }

        list
    }
}

// todo: use trait to make string_list, from_string_list_index, to_string_list_index generic
// fixme: add e-ac3 support
impl AudioCodec {
    pub fn display(&self) -> &str {
        match self {
            AudioCodec::AAC => "AAC",
            AudioCodec::AC3 => "AC-3",
            AudioCodec::DTS => "DTS",
            AudioCodec::EAC3 => "E-AC-3",
            AudioCodec::OPUS => "Opus",
            AudioCodec::RAW => "Raw",
            AudioCodec::Unknown => "Unknown",
            AudioCodec::NoAudio => "No Audio",
        }
    }

    pub fn caps_builder(&self) -> Builder<NoFeature> {
        match self {
            AudioCodec::AAC => gst::Caps::builder("audio/mpeg"),
            AudioCodec::AC3 => gst::Caps::builder("audio/x-ac3"),
            AudioCodec::DTS => gst::Caps::builder("audio/x-dts"),
            AudioCodec::EAC3 => gst::Caps::builder("audio/x-eac3"),
            AudioCodec::OPUS => gst::Caps::builder("audio/x-opus"),
            AudioCodec::RAW => gst::Caps::builder("audio/x-raw"),
            AudioCodec::Unknown => gst::Caps::builder(""),
            AudioCodec::NoAudio => gst::Caps::builder(""),
        }
    }

    pub fn string_list() -> gtk::StringList {
        gtk::StringList::new(&[
            AudioCodec::AAC.display(),
            AudioCodec::AC3.display(),
            AudioCodec::DTS.display(),
            AudioCodec::EAC3.display(),
            AudioCodec::OPUS.display(),
            AudioCodec::RAW.display(),
        ])
    }

    pub fn from_string_list_index(idx: u32) -> Self {
        match idx {
            0 => AudioCodec::AAC,
            1 => AudioCodec::AC3,
            2 => AudioCodec::DTS,
            3 => AudioCodec::EAC3,
            4 => AudioCodec::OPUS,
            5 => AudioCodec::RAW,
            _ => AudioCodec::Unknown,
        }
    }

    pub fn to_string_list_index(&self) -> u32 {
        match self {
            AudioCodec::AAC => 0,
            AudioCodec::AC3 => 1,
            AudioCodec::DTS => 2,
            AudioCodec::EAC3 => 3,
            AudioCodec::OPUS => 4,
            AudioCodec::RAW => 5,
            AudioCodec::Unknown => 100,
            AudioCodec::NoAudio => 100,
        }
    }

    pub fn from_description(description: &str) -> Self {
        match description {
            desc if desc.starts_with("MPEG") => AudioCodec::AAC,
            desc if desc.starts_with("Opus") => AudioCodec::OPUS,
            desc if desc.starts_with("AC-3") => AudioCodec::AC3,
            desc if desc.starts_with("E-AC-3") => AudioCodec::EAC3,
            desc if desc.starts_with("DTS") => AudioCodec::DTS,
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
            desc if desc.contains("H.265") || desc.contains("HEVC") => VideoCodec::X265,
            _ => VideoCodec::Unknown,
        }
    }
}

impl ContainerFormat {
    pub fn display(&self) -> &str {
        match self {
            ContainerFormat::MP4 => "MP4",
            ContainerFormat::MKV => "MKV",
            ContainerFormat::QUICKTIME => "Quicktime",
            ContainerFormat::Unknown => "Unknown",
        }
    }

    // todo: use encoding profile file extension
    pub fn file_extension(&self) -> &str {
        match self {
            ContainerFormat::MP4 => "mp4",
            ContainerFormat::MKV => "mkv",
            ContainerFormat::QUICKTIME => "mov",
            ContainerFormat::Unknown => "",
        }
    }

    pub fn caps_builder(&self) -> Builder<NoFeature> {
        match self {
            ContainerFormat::MP4 => gst::Caps::builder("video/quicktime").field("variant", "iso"),
            ContainerFormat::MKV => gst::Caps::builder("video/x-matroska"),
            ContainerFormat::QUICKTIME => {
                gst::Caps::builder("video/quicktime").field("variant", "apple")
            }
            ContainerFormat::Unknown => gst::Caps::builder(""),
        }
    }

    pub fn string_list() -> gtk::StringList {
        gtk::StringList::new(&[
            ContainerFormat::MP4.display(),
            ContainerFormat::MKV.display(),
            ContainerFormat::QUICKTIME.display(),
        ])
    }

    pub fn from_string_list_index(idx: u32) -> Self {
        match idx {
            0 => ContainerFormat::MP4,
            1 => ContainerFormat::MKV,
            2 => ContainerFormat::QUICKTIME,
            _ => ContainerFormat::Unknown,
        }
    }

    pub fn to_string_list_index(&self) -> u32 {
        match self {
            ContainerFormat::MP4 => 0,
            ContainerFormat::MKV => 1,
            ContainerFormat::QUICKTIME => 2,
            ContainerFormat::Unknown => 100,
        }
    }
    pub fn from_description(description: &str) -> Self {
        // see webm report as matroska?
        match description {
            "Matroska" => ContainerFormat::MKV,
            "ISO MP4/M4A" => ContainerFormat::MP4,
            "Quicktime" => ContainerFormat::QUICKTIME,
            _ => ContainerFormat::Unknown,
        }
    }
}
