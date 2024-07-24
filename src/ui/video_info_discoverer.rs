use ges::gst_pbutils::Discoverer;
use gst::ClockTime;

#[derive(Debug, Clone, Copy)]
pub struct VideoInfo {
    pub(crate) duration: ClockTime,
    pub(crate) framerate: gst::Fraction,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) aspect_ratio: f64,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            duration: ClockTime::ZERO,
            framerate: gst::Fraction::from(0),
            width: 0,
            height: 0,
            aspect_ratio: 0.,
        }
    }
}

pub struct VideoInfoDiscoverer {
    discoverer: Discoverer,
    pub video_info: VideoInfo,
}

impl VideoInfoDiscoverer {
    pub fn discover_uri(&mut self, uri: &str) {
        let info = self
            .discoverer
            .discover_uri(uri)
            .expect("could not discover uri");

        let video_streams = info.video_streams();
        let audio_streams = info.audio_streams();
        let vid_stream = video_streams.first().unwrap();

        let width = vid_stream.width();
        let height = vid_stream.height();

        let video_info = VideoInfo {
            duration: info.duration().unwrap(),
            framerate: vid_stream.framerate(),
            width,
            height,
            aspect_ratio: width as f64 / height as f64,
        };

        self.video_info = video_info;

        for audio in audio_streams {
            println!("audio lang: {:?}", audio.language());
        }
    }

    pub fn new() -> Self {
        // fixme: why does it take longer when called before other first
        Self {
            discoverer: Discoverer::new(5 * ClockTime::SECOND).unwrap(),
            video_info: VideoInfo::default(),
        }
    }
}
