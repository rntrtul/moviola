use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

use anyhow::Error;
use gst::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, ObjectExt};
use gst::{element_error, ClockTime, SeekFlags};
use gst_video::VideoFrameExt;

use crate::ui::video_player::VideoPlayerModel;

static THUMBNAIL_PATH: &str = "/home/fareed/Videos";
static THUMBNAIL_HEIGHT: u32 = 90;
static NUM_THUMBNAILS: u64 = 12;

pub struct ThumbnailManager;

impl ThumbnailManager {
    fn create_thumbnail_pipeline(
        got_current_thumb: Arc<Mutex<bool>>,
        video_uri: String,
        senders: mpsc::Sender<u8>,
        thumbnails_done: Arc<(Mutex<u64>, Condvar)>,
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
            .downcast::<gst_app::AppSink>()
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
                    let mut got_current = got_current_thumb.lock().unwrap();

                    if *got_current {
                        return Err(gst::FlowError::Eos);
                    }

                    *got_current = true;
                    let thumbnails_done = Arc::clone(&thumbnails_done);

                    let appsink = appsink.clone();
                    thread::spawn(move || {
                        let sample = appsink
                            .pull_sample()
                            .map_err(|_| gst::FlowError::Error)
                            .unwrap();
                        let buffer = sample
                            .buffer()
                            .ok_or_else(|| {
                                element_error!(appsink, gst::ResourceError::Failed, ("Failed"));
                                gst::FlowError::Error
                            })
                            .unwrap();

                        let caps = sample.caps().expect("sample without caps");
                        let info =
                            gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

                        let frame =
                            gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                                .map_err(|_| {
                                    element_error!(
                                        appsink,
                                        gst::ResourceError::Failed,
                                        ("Failed to map buff readable")
                                    );
                                    gst::FlowError::Error
                                })
                                .unwrap();

                        let aspect_ratio = (frame.width() as f64 * info.par().numer() as f64)
                            / (frame.height() as f64 * info.par().denom() as f64);
                        let target_height = THUMBNAIL_HEIGHT;
                        let target_width = target_height as f64 * aspect_ratio;

                        let img = image::FlatSamples::<&[u8]> {
                            samples: frame.plane_data(0).unwrap(),
                            layout: image::flat::SampleLayout {
                                channels: 3,
                                channel_stride: 1,
                                width: frame.width(),
                                width_stride: 4,
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
                        let (thumbs_num_lock, thumbs_done_cvar) = &*thumbnails_done;
                        let mut thumb_num = thumbs_num_lock.lock().unwrap();

                        let thumbnail_save_path = std::path::PathBuf::from(format!(
                            "/{}/thumbnail_{}.jpg",
                            THUMBNAIL_PATH, *thumb_num
                        ));

                        scaled_img
                            .save(&thumbnail_save_path)
                            .map_err(|err| {
                                element_error!(
                                    appsink,
                                    gst::ResourceError::Write,
                                    (
                                        "Failed to write a preview file {}: {}",
                                        &thumbnail_save_path.display(),
                                        err
                                    )
                                );
                                gst::FlowError::Error
                            })
                            .unwrap();
                        *thumb_num += 1;

                        if *thumb_num == NUM_THUMBNAILS {
                            thumbs_done_cvar.notify_one();
                        }
                    });

                    senders.send(0).unwrap();
                    Err(gst::FlowError::Eos)
                })
                .build(),
        );
        Ok(pipeline)
    }

    // fixme: speed up
    // try to reuse existing pipeline or thumbnail pipeline. would be ~1.3 sec quicker for
    // subsequent videos
    fn thumbnail_thread(video_uri: String, thumbnails_done: Arc<(Mutex<u64>, Condvar)>) {
        let uri = video_uri.clone();

        // todo: figure way to return pipeline or use static pipeline to dispose of or null this pipeline
        thread::spawn(move || {
            let got_current_thumb = Arc::new(Mutex::new(false));
            let (senders, receiver) = mpsc::channel();

            let pipeline = Self::create_thumbnail_pipeline(
                Arc::clone(&got_current_thumb),
                uri,
                senders.clone(),
                Arc::clone(&thumbnails_done),
            )
                .expect("could not create thumbnail pipeline");

            pipeline.set_state(gst::State::Paused).unwrap();

            let pipe_clone = pipeline.clone();
            VideoPlayerModel::wait_for_playbin_done(&gst::Element::from(pipe_clone));

            let duration = pipeline.query_duration::<ClockTime>().unwrap();
            let step = duration.mseconds() / (NUM_THUMBNAILS + 2); // + 2 so first and last frame not chosen

            for i in 0..NUM_THUMBNAILS {
                let timestamp =
                    gst::GenericFormattedValue::from(ClockTime::from_mseconds(step + (step * i)));
                if pipeline
                    .seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, timestamp)
                    .is_err()
                {
                    println!("Failed to seek");
                }
                pipeline.set_state(gst::State::Playing).unwrap();
                receiver.recv().unwrap();
                pipeline.set_state(gst::State::Paused).unwrap();

                let mut gen_new = got_current_thumb.lock().unwrap();
                *gen_new = false;
            }
        });
    }

    pub fn get_thumbnail_paths() -> Vec<String> {
        let mut file_names = Vec::new();
        for i in 0..NUM_THUMBNAILS {
            file_names.push(format!("{}/thumbnail_{}.jpg", THUMBNAIL_PATH, i));
        }
        file_names
    }

    pub fn get_number_of_thumbnails() -> u64 {
        NUM_THUMBNAILS
    }

    pub async fn generate_thumbnails(video_uri: String) {
        let thumbnail_pair = Arc::new((Mutex::new(0), Condvar::new()));
        Self::thumbnail_thread(video_uri, Arc::clone(&thumbnail_pair));
        let (num_thumbs, all_done) = &*thumbnail_pair;
        let mut thumbnails_done = num_thumbs.lock().unwrap();

        while *thumbnails_done != NUM_THUMBNAILS {
            thumbnails_done = all_done.wait(thumbnails_done).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_create() {
        gst::init().unwrap();

        let uri = "file:///home/fareed/Videos/mp3e1.mkv";
        let thumbnail_pair = Arc::new((Mutex::new(0), Condvar::new()));

        ThumbnailManager::thumbnail_thread(uri.parse().unwrap(), Arc::clone(&thumbnail_pair));

        let (num_thumbs, all_done) = &*thumbnail_pair;
        let mut thumbnails_done = num_thumbs.lock().unwrap();
        while *thumbnails_done != NUM_THUMBNAILS {
            thumbnails_done = all_done.wait(thumbnails_done).unwrap();
        }

        assert_eq!(true, true);
    }
}
