use crate::video;
use anyhow::Error;
use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions, Resizer};
use ges::glib;
use gst::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, ObjectExt};
use gst::{ClockTime, SeekFlags, State};
use gst_app::AppSink;
use gst_video::VideoFrameExt;
use gtk4::gdk;
use gtk4::gdk::MemoryTexture;
use std::sync::{Arc, Condvar, Mutex};

static THUMBNAIL_WIDTH: u32 = 180;
static NUM_THUMBNAILS: u64 = 8;

pub struct Thumbnail;

impl Thumbnail {
    fn new_sample_callback(
        appsink: &AppSink,
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

        let buffer = sample.buffer().ok_or_else(|| gst::FlowError::Error)?;

        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(&buffer, &info)
            .map_err(|_| gst::FlowError::Error)?;

        let aspect_ratio = (frame.width() as f64 * info.par().numer() as f64)
            / (frame.height() as f64 * info.par().denom() as f64);
        let target_height = (THUMBNAIL_WIDTH as f64 / aspect_ratio).floor();

        let rgba_image = image::RgbaImage::from_raw(
            frame.width(),
            frame.height(),
            Vec::from(frame.plane_data(0).unwrap()),
        )
        .unwrap();
        let dyn_img = image::DynamicImage::from(rgba_image);

        let mut thumbnail_pic = fast_image_resize::images::Image::new(
            THUMBNAIL_WIDTH,
            target_height as u32,
            PixelType::U8x4,
        );

        // todo: upgrade to resize 5.0 (has issue with building it right now)
        //  for multithreaded single img
        let mut resizer = Resizer::new();
        resizer
            .resize(
                &dyn_img,
                &mut thumbnail_pic,
                Some(
                    &ResizeOptions::new()
                        .resize_alg(ResizeAlg::Nearest)
                        .use_alpha(false),
                ),
            )
            .unwrap();

        let gdk_texture = MemoryTexture::new(
            THUMBNAIL_WIDTH as i32,
            target_height as i32,
            gdk::MemoryFormat::R8g8b8a8,
            &glib::Bytes::from(thumbnail_pic.buffer()),
            (THUMBNAIL_WIDTH * 4) as usize,
        );

        {
            let thumbnail_lock = &*Arc::clone(&thumbnail);
            let mut thumnail_vec = thumbnail_lock.lock().unwrap();
            thumnail_vec.push(gdk_texture);
        }

        cvar.notify_one();
        Err(gst::FlowError::Eos)
    }

    fn create_thumbnail_pipeline(
        video_uri: String,
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
                        Arc::clone(&current_thumbnail_started),
                        Arc::clone(&thumbnail),
                    )
                })
                .build(),
        );
        Ok(pipeline)
    }

    fn launch_thumbnail_threads(video_uri: String, thumbnail: Arc<Mutex<Vec<MemoryTexture>>>) {
        let current_thumbnail_started: Arc<(Mutex<bool>, Condvar)> =
            Arc::new((Mutex::new(false), Condvar::new()));

        let pipeline = Self::create_thumbnail_pipeline(
            video_uri,
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

        pipeline.set_state(State::Null).unwrap();
    }

    pub fn number_of_thumbnails() -> u64 {
        NUM_THUMBNAILS
    }

    pub async fn generate_thumbnails(video_uri: String) -> Vec<MemoryTexture> {
        let thumbnails: Arc<Mutex<Vec<MemoryTexture>>> =
            Arc::new(Mutex::new(Vec::with_capacity(NUM_THUMBNAILS as usize)));
        Self::launch_thumbnail_threads(video_uri, thumbnails.clone());

        Arc::into_inner(thumbnails).unwrap().into_inner().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_create() {
        gst::init().unwrap();

        let uri = crate::config::VIDEO_TEST_FILE;

        let thumbnails: Arc<Mutex<Vec<MemoryTexture>>> =
            Arc::new(Mutex::new(Vec::with_capacity(NUM_THUMBNAILS as usize)));

        Thumbnail::launch_thumbnail_threads(uri.parse().unwrap(), thumbnails.clone());

        {
            let thumbnail_lock = &*Arc::clone(&thumbnails);
            let thumnail_vec = thumbnail_lock.lock().unwrap();
            assert_eq!(thumnail_vec.len(), NUM_THUMBNAILS as usize);
        }
    }
}
