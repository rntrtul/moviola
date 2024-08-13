use std::path::PathBuf;
use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;
use std::time::SystemTime;

use anyhow::Error;
use gst::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, ObjectExt};
use gst::{ClockTime, SeekFlags, State};
use gst_app::AppSink;
use gst_video::VideoFrameExt;
use relm4::gtk;

use crate::video;

// todo: maybe make thumbnails fixed aspect ratio?
static THUMBNAIL_HEIGHT: u32 = 180;
static NUM_THUMBNAILS: u64 = 8;

pub struct Thumbnail;

impl Thumbnail {
    pub fn save_sample_as_image(sample: &gst::Sample, target_height: u32, save_path: PathBuf) {
        let buffer = sample
            .buffer()
            .ok_or_else(|| gst::FlowError::Error)
            .unwrap();

        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
            .map_err(|_| gst::FlowError::Error)
            .unwrap();

        let aspect_ratio = (frame.width() as f64 * info.par().numer() as f64)
            / (frame.height() as f64 * info.par().denom() as f64);
        let target_width = target_height as f64 * aspect_ratio;

        let width_stride = *info.format_info().pixel_stride().first().unwrap() as usize;

        let img = image::FlatSamples::<&[u8]> {
            samples: frame.plane_data(0).unwrap(),
            layout: image::flat::SampleLayout {
                channels: 3,
                channel_stride: 1,
                width: frame.width(),
                width_stride,
                height: frame.height(),
                height_stride: frame.plane_stride()[0] as usize,
            },
            color_hint: Some(image::ColorType::Rgb8),
        };

        let scaled_img = image::imageops::thumbnail(
            &img.as_view::<image::Rgb<u8>>()
                .expect("could not create image view"),
            target_width as u32,
            target_height,
        );

        scaled_img
            .save(&save_path)
            .map_err(|err| gst::FlowError::Error)
            .unwrap();
    }

    fn new_sample_callback(
        appsink: &AppSink,
        barrier: Arc<Barrier>,
        current_thumbnail_started: Arc<(Mutex<bool>, Condvar)>,
        num_started: Arc<Mutex<u64>>,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let (lock, cvar) = &*Arc::clone(&current_thumbnail_started);
        let mut got_current = lock.lock().unwrap();

        if *got_current {
            return Err(gst::FlowError::Eos);
        }
        *got_current = true;

        let mut thumbnails_started = num_started.lock().unwrap();

        let curr_thumbnail = *thumbnails_started;
        let appsink = appsink.clone();

        thread::spawn(move || {
            let sample = appsink
                .pull_sample()
                .map_err(|_| gst::FlowError::Error)
                .unwrap();
            let thumbnail_save_path = Self::thumbnail_save_path(curr_thumbnail);
            Self::save_sample_as_image(&sample, THUMBNAIL_HEIGHT, thumbnail_save_path);
            barrier.wait();
        });

        *thumbnails_started += 1;
        cvar.notify_one();
        Err(gst::FlowError::Eos)
    }

    fn create_thumbnail_pipeline(
        video_uri: String,
        barrier: Arc<Barrier>,
        current_thumbnail_started: Arc<(Mutex<bool>, Condvar)>,
        num_started: Arc<Mutex<u64>>,
    ) -> Result<gst::Pipeline, Error> {
        let pipeline = gst::parse::launch(&format!(
            "uridecodebin uri={video_uri} ! videoconvert ! appsink name=sink"
        ))
        .unwrap()
        .downcast::<gst::Pipeline>()
        .expect("Expected a gst::pipeline");

        let appsink = pipeline
            .by_name("sink")
            .expect("sink element not found")
            .downcast::<AppSink>()
            .expect("Sink element is expected to be appsink!");

        appsink.set_property("sync", false);

        appsink.set_caps(Some(
            &gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgbx)
                .build(),
        ));

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    Self::new_sample_callback(
                        appsink,
                        Arc::clone(&barrier),
                        Arc::clone(&current_thumbnail_started),
                        Arc::clone(&num_started),
                    )
                })
                .build(),
        );
        Ok(pipeline)
    }

    // fixme: speed up
    // try to reuse existing pipeline or thumbnail pipeline. would be ~1.3 sec quicker for
    // subsequent videos
    // after calling should wait on barrier otherwise thumbnails not done
    fn launch_thumbnail_threads(video_uri: String, barrier: Arc<Barrier>) -> gst::Pipeline {
        let current_thumbnail_started: Arc<(Mutex<bool>, Condvar)> =
            Arc::new((Mutex::new(false), Condvar::new()));
        let num_started = Arc::new(Mutex::new(0));

        let pipeline = Self::create_thumbnail_pipeline(
            video_uri,
            barrier,
            Arc::clone(&current_thumbnail_started),
            Arc::clone(&num_started),
        )
        .expect("could not create thumbnail pipeline");

        pipeline.set_state(State::Paused).unwrap();
        video::player::Player::wait_for_pipeline_init(pipeline.bus().unwrap());

        let duration = pipeline.query_duration::<ClockTime>().unwrap();
        // + 1 so first and last frame not chosen
        let step = duration.mseconds() / (NUM_THUMBNAILS + 1);

        for i in 0..NUM_THUMBNAILS {
            let timestamp =
                gst::GenericFormattedValue::from(ClockTime::from_mseconds(step + (step * i)));
            if pipeline
                .seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, timestamp)
                .is_err()
            {
                println!("Failed to seek");
            }
            pipeline.set_state(State::Playing).unwrap();
            let (lock, started_thumbnail) = &*current_thumbnail_started;
            let mut started = started_thumbnail
                .wait_while(lock.lock().unwrap(), |pending| !*pending)
                .unwrap();

            pipeline.set_state(State::Paused).unwrap();
            *started = false;
        }

        return pipeline;
    }

    fn thumbnail_save_path(thumnail_num: u64) -> PathBuf {
        let mut path = gtk::glib::user_cache_dir();
        path.push(format!("thumbnail_{}.jpg", thumnail_num));
        path
    }

    pub fn thumbnail_paths() -> Vec<PathBuf> {
        let mut file_names = Vec::new();
        for i in 0..NUM_THUMBNAILS {
            file_names.push(Self::thumbnail_save_path(i));
        }
        file_names
    }

    pub fn number_of_thumbnails() -> u64 {
        NUM_THUMBNAILS
    }

    pub async fn generate_thumbnails(video_uri: String) {
        let now = SystemTime::now();
        let all_thumbnails_generated = Arc::new(Barrier::new((NUM_THUMBNAILS + 1) as usize));
        let pipeline =
            Self::launch_thumbnail_threads(video_uri, Arc::clone(&all_thumbnails_generated));
        all_thumbnails_generated.wait();
        pipeline.set_state(State::Null).unwrap();
        println!("done thumbnails in {:?}", now.elapsed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_create() {
        gst::init().unwrap();

        let uri = "file:///home/fareed/Videos/mp3e1.mkv";
        let barrier = Arc::new(Barrier::new((NUM_THUMBNAILS + 1) as usize));
        Thumbnail::launch_thumbnail_threads(uri.parse().unwrap(), Arc::clone(&barrier));
        barrier.wait();

        assert_eq!(true, true);
    }
}
