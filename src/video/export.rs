use std::thread;
use std::time::SystemTime;

use ges::gst_pbutils::EncodingContainerProfile;
use ges::prelude::EncodingProfileBuilder;
use ges::prelude::{GESContainerExt, GESPipelineExt, GESTrackExt, TimelineElementExt, TimelineExt};
use ges::{gst_pbutils, Effect, PipelineFlags};
use gst::prelude::{ElementExt, GstObjectExt};
use gst::{ClockTime, Fraction, State};
use gst_video::VideoOrientationMethod;

use crate::video::player::Player;

fn video_orientation_method_to_val(method: VideoOrientationMethod) -> u8 {
    match method {
        VideoOrientationMethod::Identity => 0,
        VideoOrientationMethod::_90r => 1,
        VideoOrientationMethod::_180 => 2,
        VideoOrientationMethod::_90l => 3,
        VideoOrientationMethod::Horiz => 4,
        VideoOrientationMethod::Vert => 5,
        VideoOrientationMethod::UlLr => 6,
        VideoOrientationMethod::UrLl => 7,
        VideoOrientationMethod::Auto => 8,
        VideoOrientationMethod::Custom => 9,
        _ => panic!("unknown value given"),
    }
}

impl Player {
    // todo: handle crop and rotates. need to store this info in player first
    fn preview_size(&self) -> (i32, i32) {
        let preview_width = 640;
        let preview_height = (preview_width as f64 / self.info.aspect_ratio) as i32;

        (preview_width, preview_height)
    }

    fn build_container_profile() -> EncodingContainerProfile {
        // todo: pass in audio and video targets and target resolution/aspect ratio
        let audio_profile =
            gst_pbutils::EncodingAudioProfile::builder(&gst::Caps::builder("audio/mpeg").build())
                .build();
        let video_profile =
            gst_pbutils::EncodingVideoProfile::builder(&gst::Caps::builder("video/x-h264").build())
                .build();

        EncodingContainerProfile::builder(&gst::Caps::builder("video/x-matroska").build())
            .name("Container")
            .add_profile(video_profile)
            .add_profile(audio_profile)
            .build()
    }
    fn remove_effect(&self, effect_name: &str) {
        let clip = self.clip();

        let found_element = clip
            .children(false)
            .into_iter()
            .find(|child| child.name().unwrap() == effect_name);

        if let Some(prev_effect) = found_element {
            clip.remove(&prev_effect)
                .expect("could not delete previous effect");
        }
    }

    fn add_effect(&mut self, effect: &Effect) {
        let clip = self.clip();

        clip.add(effect).unwrap();
        self.pipeline.timeline().unwrap().commit_sync();
    }

    fn set_restriction_caps(&mut self, framerate: Fraction, width: i32, height: i32) {
        let preview_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", framerate)
            .field("width", width)
            .field("height", height)
            .build();

        let tracks = self.pipeline.timeline().unwrap().tracks();
        let track = tracks.first().unwrap();
        track.set_restriction_caps(&preview_caps);
    }

    // set caps based on original video aspect ratio
    pub(crate) fn set_preview_aspect_ratio_original(&mut self) {
        let (width, height) = self.preview_size();

        self.set_restriction_caps(self.info.framerate, width, height);
    }

    pub fn set_video_orientation(&mut self, orientation: VideoOrientationMethod) {
        // todo: split flip and rotates
        let val = video_orientation_method_to_val(orientation);

        let effect = format!("autovideoflip video-direction={val}");
        let flip = Effect::new(effect.as_str()).expect("could not make flip");
        flip.set_name(Some("orientation"))
            .expect("Unable to set name");

        self.remove_effect("orientation");

        let (mut preview_width, mut preview_height) = self.preview_size();

        if orientation == VideoOrientationMethod::_90l
            || orientation == VideoOrientationMethod::_90r
        {
            let tmp = preview_width;
            preview_width = preview_height;
            preview_height = tmp;
            println!("switched resolution to {preview_width}x{preview_height}");
        }
        self.set_restriction_caps(self.info.framerate, preview_width, preview_height);

        self.add_effect(&flip);
        self.pipeline.timeline().unwrap().commit_sync();
    }

    // fixme: video getting squished, when adjusting preview aspect ratio.
    pub fn set_video_crop(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let effect = format!("videocrop top={top} left={left} right={right} bottom={bottom}");
        let crop = Effect::new(effect.as_str()).expect("could not make crop");
        crop.set_name(Some("crop")).expect("unable to set name");

        self.remove_effect("crop");
        self.add_effect(&crop);

        // convert left, right, to displaying
        let crop_width = self.info.width as i32 - left - right;
        let crop_height = self.info.height as i32 - top - bottom;

        let crop_aspect_ratio = crop_width as f64 / crop_height as f64;

        let preview_width = 640;
        let preview_height = (preview_width as f64 / crop_aspect_ratio) as i32;

        // self.set_restriction_caps(self.info.framerate, preview_width, preview_height);
        self.pipeline.timeline().unwrap().commit_sync();
    }

    pub fn remove_crop(&mut self) {
        self.remove_effect("crop");
        self.set_preview_aspect_ratio_original();
        self.pipeline.timeline().unwrap().commit_sync();
    }

    pub fn export_frame(&self) {
        // todo: ask for file location and name
        // is it taking the 720p playback and screenshotting that? set caps to native res first?
        self.pipeline
            .save_thumbnail(
                self.info.width as i32,
                self.info.height as i32,
                "image/jpeg",
                "/home/fareed/Videos/export.jpg",
            )
            .expect("unable to save exported frame");
    }

    pub fn export_video(&self) {
        let now = SystemTime::now();
        // todo: use toggle_play_pause for setting state to keep ui insync
        // todo: go back to original resolution.
        self.pipeline.set_state(State::Paused).unwrap();

        let out_uri = "file:///home/fareed/Videos/out2.mkv";
        let container_profile = Self::build_container_profile();

        self.pipeline
            .set_render_settings(&out_uri, &container_profile)
            .expect("unable to set render settings");
        // todo: use smart_render?
        self.pipeline
            .set_mode(PipelineFlags::RENDER)
            .expect("failed to set to render");

        let start_time = 1500;
        // self.frame_info.duration.mseconds() as f64 * self.timeline.model().get_target_start_percent();
        let end_time = 35000;
        // self.frame_info.duration.mseconds() as f64 * self.timeline.model().get_target_end_percent();
        let duration = (end_time - start_time) as u64;

        let clip = self.clip();

        clip.set_inpoint(ClockTime::from_mseconds(start_time as u64));
        clip.set_duration(ClockTime::from_mseconds(duration));

        self.pipeline.set_state(State::Playing).unwrap();

        let bus = self.pipeline.bus().unwrap();

        thread::spawn(move || {
            for msg in bus.iter_timed(ClockTime::NONE) {
                use gst::MessageView;

                match msg.view() {
                    MessageView::Eos(..) => {
                        println!("Done? in {:?}", now.elapsed());
                        break;
                    }
                    MessageView::Error(err) => {
                        println!(
                            "Error from {:?}: {} ({:?})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        );
                        break;
                    }
                    _ => (),
                }
            }
        });

        //     todo: when done, retrun to select video screen, and maybe open file explorer to location.
    }
}
