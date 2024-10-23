use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;

use anyhow::Error;
use ges::glib;
use gst::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, ObjectExt};
use gst::{ClockTime, SeekFlags, State};
use gst_app::AppSink;
use gst_video::VideoFrameExt;
use gtk4::gdk;
use gtk4::gdk::MemoryTexture;

use crate::video;

static THUMBNAIL_WIDTH: u32 = 180;
static NUM_THUMBNAILS: u64 = 8;

pub struct Thumbnail;

impl Thumbnail {
    fn new_sample_callback(
        appsink: &AppSink,
        barrier: Arc<Barrier>,
        current_thumbnail_started: Arc<(Mutex<bool>, Condvar)>,
        thumbnail: Arc<Mutex<Vec<MemoryTexture>>>,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let (lock, cvar) = &*Arc::clone(&current_thumbnail_started);
        let mut got_current = lock.lock().unwrap();

        if *got_current {
            return Err(gst::FlowError::Eos);
        }
        *got_current = true;

        let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Error)?;

        thread::spawn(move || {
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
            let target_height = (THUMBNAIL_WIDTH as f64 / aspect_ratio).floor();

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
                THUMBNAIL_WIDTH,
                target_height as u32,
            );

            let gdk_texture = gdk::MemoryTexture::new(
                THUMBNAIL_WIDTH as i32,
                target_height as i32,
                gdk::MemoryFormat::R8g8b8,
                &glib::Bytes::from(&scaled_img.iter().as_slice()),
                (THUMBNAIL_WIDTH * 3) as usize,
            );

            {
                let thumbnail_lock = &*Arc::clone(&thumbnail);
                let mut thumnail_vec = thumbnail_lock.lock().unwrap();
                thumnail_vec.push(gdk_texture);
            }

            barrier.wait();
        });

        cvar.notify_one();
        Err(gst::FlowError::Eos)
    }

    fn create_thumbnail_pipeline(
        video_uri: String,
        barrier: Arc<Barrier>,
        current_thumbnail_started: Arc<(Mutex<bool>, Condvar)>,
        thumbnail: Arc<Mutex<Vec<MemoryTexture>>>,
    ) -> Result<gst::Pipeline, Error> {
        let pipeline = gst::parse::launch(&format!(
            "uridecodebin uri={video_uri} ! videoconvert ! appsink name=sink"
        ))?
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
                        Arc::clone(&thumbnail),
                    )
                })
                .build(),
        );
        Ok(pipeline)
    }

    fn launch_thumbnail_threads(
        video_uri: String,
        barrier: Arc<Barrier>,
        thumbnail: Arc<Mutex<Vec<MemoryTexture>>>,
    ) {
        let current_thumbnail_started: Arc<(Mutex<bool>, Condvar)> =
            Arc::new((Mutex::new(false), Condvar::new()));

        let pipeline = Self::create_thumbnail_pipeline(
            video_uri,
            Arc::clone(&barrier),
            Arc::clone(&current_thumbnail_started),
            Arc::clone(&thumbnail),
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

        barrier.wait();
        pipeline.set_state(State::Null).unwrap();
    }

    pub fn number_of_thumbnails() -> u64 {
        NUM_THUMBNAILS
    }

    pub async fn generate_thumbnails(video_uri: String) -> Vec<MemoryTexture> {
        let all_thumbnails_generated = Arc::new(Barrier::new((NUM_THUMBNAILS + 1) as usize));
        let thumbnails: Arc<Mutex<Vec<MemoryTexture>>> =
            Arc::new(Mutex::new(Vec::with_capacity(NUM_THUMBNAILS as usize)));
        Self::launch_thumbnail_threads(
            video_uri,
            Arc::clone(&all_thumbnails_generated),
            thumbnails.clone(),
        );

        Arc::into_inner(thumbnails).unwrap().into_inner().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_create() {
        gst::init().unwrap();

        // todo: read from env/config
        let uri = "file:///home/fareed/Videos/TheFallGuy.mkv";

        let thumbnails: Arc<Mutex<Vec<MemoryTexture>>> =
            Arc::new(Mutex::new(Vec::with_capacity(NUM_THUMBNAILS as usize)));
        let barrier = Arc::new(Barrier::new((NUM_THUMBNAILS + 1) as usize));

        Thumbnail::launch_thumbnail_threads(
            uri.parse().unwrap(),
            barrier.clone(),
            thumbnails.clone(),
        );
        barrier.wait();

        {
            let thumbnail_lock = &*Arc::clone(&thumbnails);
            let thumnail_vec = thumbnail_lock.lock().unwrap();
            assert_eq!(thumnail_vec.len(), NUM_THUMBNAILS as usize);
        }
    }
}
