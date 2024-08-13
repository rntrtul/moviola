use std::path::PathBuf;
use std::thread;
use std::time::SystemTime;

use ges::gst_pbutils::EncodingContainerProfile;
use ges::prelude::{EncodingProfileBuilder, LayerExt};
use ges::prelude::{GESPipelineExt, TimelineElementExt, TimelineExt};
use ges::{gst_pbutils, PipelineFlags};
use gst::prelude::{ElementExt, GObjectExtManualGst, GstObjectExt, ObjectExt};
use gst::{ClockTime, State};
use gst_video::VideoOrientationMethod;
use gtk4::gdk;
use relm4::ComponentSender;

use crate::app::{App, AppMsg};
use crate::video::player::Player;
use crate::video::thumbnail::Thumbnail;

#[derive(Debug)]
pub struct TimelineExportSettings {
    pub start: ClockTime,
    pub duration: ClockTime,
}

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
// todo: move export out of player. set effects to be preview_crop etc.
impl Player {
    pub fn set_video_orientation(&mut self, orientation: VideoOrientationMethod) {
        // todo: split flip and rotates
        let val = video_orientation_method_to_val(orientation);

        self.playbin
            .property::<gst::Element>("video-sink")
            .property::<gdk::Paintable>("paintable")
            .set_property_from_str("orientation", "90");
    }

    pub fn set_video_crop(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let crop = self.playbin.property::<gst::Element>("video-filter");

        crop.set_property("top", top);
        crop.set_property("left", left);
        crop.set_property("right", right);
        crop.set_property("bottom", bottom);
    }

    pub fn remove_crop(&mut self) {
        self.set_video_crop(0, 0, 0, 0);
    }

    pub fn export_frame(&self) {
        // todo: ask for file location and name
        // todo: get sample height caps.structure(0).get(height)
        let sample = self.playbin.property::<gst::Sample>("sample");
        let mut output = PathBuf::new();
        output.push("/home/fareed/Videos/export.jpg");
        Thumbnail::save_sample_as_image(&sample, self.info.height, output);
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

    pub fn export_video(
        &self,
        source_uri: String,
        save_uri: String,
        timeline_export_settings: TimelineExportSettings,
        app_sender: ComponentSender<App>,
    ) {
        let now = SystemTime::now();
        // todo: use toggle_play_pause for setting state to keep ui insync
        // todo: go back to original resolution.
        self.playbin.set_state(State::Null).unwrap();

        thread::spawn(move || {
            let timeline = ges::Timeline::new_audio_video();
            let layer = timeline.append_layer();
            let pipeline = ges::Pipeline::new();
            pipeline.set_timeline(&timeline).unwrap();

            let clip = ges::UriClip::new(source_uri.as_str()).expect("Failed to create clip");
            layer.add_clip(&clip).unwrap();

            // todo: add crop + rotate effects now.

            let container_profile = Self::build_container_profile();

            pipeline
                .set_render_settings(&save_uri.as_str(), &container_profile)
                .expect("unable to set render settings");
            // // todo: use smart_render? (only when using original container info?)
            pipeline
                .set_mode(PipelineFlags::RENDER)
                .expect("failed to set to render");

            clip.set_inpoint(timeline_export_settings.start);
            clip.set_duration(timeline_export_settings.duration);

            pipeline.set_state(State::Playing).unwrap();

            let bus = pipeline.bus().unwrap();

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
            app_sender.input(AppMsg::ExportDone);
            pipeline.set_state(State::Null).unwrap();
        });
    }
}
