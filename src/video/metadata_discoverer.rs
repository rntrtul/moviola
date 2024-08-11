use ges::gst_pbutils::Discoverer;
use ges::prelude::DiscovererStreamInfoExt;
use gst::ClockTime;

use crate::video::codecs::{AudioCodec, VideoCodec, VideoCodecInfo, VideoContainer};

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub(crate) title: String,
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
            title: "".to_string(),
            duration: ClockTime::ZERO,
            framerate: gst::Fraction::from(0),
            width: 0,
            height: 0,
            aspect_ratio: 0.,
            codec_info: VideoCodecInfo::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetadataDiscoverer {
    discoverer: Discoverer,
    pub video_info: VideoInfo,
}

impl MetadataDiscoverer {
    pub fn discover_uri(&mut self, uri: &str) {
        let info = self
            .discoverer
            .discover_uri(uri)
            .expect("could not discover uri");

        let title = uri.split("/").last().unwrap();

        let video_streams = info.video_streams();
        let audio_streams = info.audio_streams();
        let vid_stream = video_streams.first().unwrap();

        let width = vid_stream.width();
        let height = vid_stream.height();

        let tags = info.tags().unwrap();
        let mut container = VideoContainer::Unknown;
        for tag in tags.iter() {
            if tag.0 == "container-format" {
                container =
                    VideoContainer::from_description(tag.1.get::<String>().unwrap().as_str());
                break;
            }
        }

        let video_caps = vid_stream.caps().unwrap();
        let video_description = ges::gst_pbutils::pb_utils_get_codec_description(&video_caps);
        let video_codec = VideoCodec::from_description(video_description.as_str());

        let audio_codec = if !audio_streams.is_empty() {
            let descripttion = ges::gst_pbutils::pb_utils_get_codec_description(
                &audio_streams.first().unwrap().caps().unwrap(),
            );
            AudioCodec::from_description(descripttion.as_str())
        } else {
            AudioCodec::NoAudio
        };

        for audio in audio_streams {
            println!("audio lang: {:?},", audio.language());
        }

        let codec_info = VideoCodecInfo {
            container,
            video_codec,
            audio_codec,
        };

        let video_info = VideoInfo {
            title: title.to_string(),
            duration: info.duration().unwrap(),
            framerate: vid_stream.framerate(),
            width,
            height,
            aspect_ratio: width as f64 / height as f64,
            codec_info,
        };

        self.video_info = video_info;
    }

    pub fn new() -> Self {
        // fixme: why does it take longer when called before other first
        Self {
            discoverer: Discoverer::new(5 * ClockTime::SECOND).unwrap(),
            video_info: VideoInfo::default(),
        }
    }
}
