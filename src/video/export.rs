use std::path::PathBuf;
use std::thread;
use std::time::SystemTime;

use ges::gst_pbutils::EncodingContainerProfile;
use ges::prelude::{EncodingProfileBuilder, GESContainerExt, GESTrackExt, LayerExt};
use ges::prelude::{GESPipelineExt, TimelineElementExt, TimelineExt};
use ges::{gst_pbutils, PipelineFlags};
use gst::prelude::{ElementExt, GstObjectExt, ObjectExt};
use gst::{ClockTime, State};
use gst_plugin_gtk4::Orientation;
use gtk4::gdk;
use relm4::ComponentSender;

use crate::app::{App, AppMsg};
use crate::ui::controls_sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::player::Player;
use crate::video::thumbnail::Thumbnail;

#[derive(Debug)]
pub struct TimelineExportSettings {
    pub start: ClockTime,
    pub duration: ClockTime,
}

fn sink_orientation_to_effect(method: Orientation) -> String {
    match method {
        Orientation::Auto => "auto".to_string(),
        Orientation::Rotate0 => "identity".to_string(),
        Orientation::Rotate90 => "90r".to_string(),
        Orientation::Rotate180 => "180".to_string(),
        Orientation::Rotate270 => "90l".to_string(),
        Orientation::FlipRotate0 => "horiz".to_string(),
        Orientation::FlipRotate90 => "ur-ll".to_string(),
        Orientation::FlipRotate180 => "vert".to_string(),
        Orientation::FlipRotate270 => "ul-lr".to_string(),
    }
}

// todo: move export out of player. set effects to be preview_crop etc.
impl Player {
    pub fn set_video_orientation(&mut self, orientation: Orientation) {
        self.playbin
            .property::<gst::Element>("video-sink")
            .property::<gdk::Paintable>("paintable")
            .set_property("orientation", orientation);
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

    fn build_container_profile(
        &self,
        container: OutputContainerSettings,
    ) -> EncodingContainerProfile {
        // todo: pass in resolution/aspect ratio target + bitrate to keep file size in check
        let container_caps = container.container.caps_builder().build();
        let video_caps = container.video_codec.caps_builder().build();

        let video_profile = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
            .name("video_profile")
            .build();
        let profile_builder = EncodingContainerProfile::builder(&container_caps)
            .name("Container")
            .add_profile(video_profile);

        if container.no_audio {
            profile_builder.build()
        } else {
            let audio_stream =
                &self.info.container_info.audio_streams[container.audio_stream_idx as usize];

            let audio_caps = audio_stream.codec.caps_builder().build();
            let audio_profile = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("audio_profile")
                .build();

            profile_builder.add_profile(audio_profile).build()
        }
    }

    pub fn export_video(
        &self,
        source_uri: String,
        save_uri: String,
        timeline_export_settings: TimelineExportSettings,
        controls_export_settings: ControlsExportSettings,
        app_sender: ComponentSender<App>,
    ) {
        let now = SystemTime::now();
        // todo: use toggle_play_pause for setting state to keep ui insync
        // todo: go back to original resolution.
        // todo: set bitrate to original video, to keep file size smaller at min
        self.playbin.set_state(State::Null).unwrap();

        let container_profile = self.build_container_profile(controls_export_settings.container);

        let orientation = self
            .playbin
            .property::<gst::Element>("video-sink")
            .property::<gdk::Paintable>("paintable")
            .property::<Orientation>("orientation");

        let (width, height) = match orientation {
            Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::FlipRotate270
            | Orientation::FlipRotate90 => (self.info.height, self.info.width),
            _ => (self.info.width, self.info.height),
        };

        thread::spawn(move || {
            let timeline = ges::Timeline::new_audio_video();
            let layer = timeline.append_layer();
            let pipeline = ges::Pipeline::new();
            pipeline.set_timeline(&timeline).unwrap();

            // clip needs to be aquired in seperate thread from playbin
            // todo: select audio stream (ges does not support selection)
            let clip = ges::UriClip::new(source_uri.as_str()).expect("Failed to create clip");
            layer.add_clip(&clip).unwrap();

            let tracks = timeline.tracks();
            let track = tracks.first().expect("No first track");
            let caps = gst::Caps::builder("video/x-raw")
                .field("width", width as i32)
                .field("height", height as i32)
                .build();
            track.set_restriction_caps(&caps);

            pipeline
                .set_render_settings(&save_uri.as_str(), &container_profile)
                .expect("unable to set render settings");

            //todo: use smart_render? (only when using original container info?)
            let render_mode = PipelineFlags::RENDER;

            pipeline
                .set_mode(render_mode)
                .expect("failed to set to render");

            clip.set_inpoint(timeline_export_settings.start);
            clip.set_duration(timeline_export_settings.duration);
            // todo: add crop + rotate effects now.
            // todo: should resolution be set in encoding profile or clip caps?
            let rotate = format!(
                "autovideoflip video-direction={}",
                sink_orientation_to_effect(orientation)
            );
            let rotate_effect = ges::Effect::new(&*rotate).unwrap();
            clip.add(&rotate_effect).unwrap();

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
