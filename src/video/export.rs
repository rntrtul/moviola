use std::thread;
use std::time::SystemTime;

use ges::gst_pbutils::EncodingContainerProfile;
use ges::prelude::{EncodingProfileBuilder, GESTrackExt, LayerExt};
use ges::prelude::{GESPipelineExt, TimelineElementExt, TimelineExt};
use ges::{gst_pbutils, PipelineFlags};
use gst::prelude::{ElementExt, GstObjectExt};
use gst::{ClockTime, State};
use gtk4::prelude::ToValue;
use relm4::ComponentSender;

use crate::app::{App, AppMsg};
use crate::ui::sidebar::{ControlsExportSettings, CropExportSettings, OutputContainerSettings};
use crate::video::player::Player;

#[derive(Debug)]
pub struct TimelineExportSettings {
    pub start: ClockTime,
    pub duration: ClockTime,
}

// todo: move export out of player. set effects to be preview_crop etc.
impl Player {
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
        crop_export_settings: CropExportSettings,
        app_sender: ComponentSender<App>,
    ) {
        let now = SystemTime::now();
        // todo: use toggle_play_pause for setting state to keep ui insync
        // todo: go back to original resolution.
        // todo: set bitrate to original video, to keep file size smaller at min
        self.playbin.set_state(State::Null).unwrap();

        let container_profile = self.build_container_profile(controls_export_settings.container);
        let orientation = crop_export_settings.orientation;
        let bounding_box = crop_export_settings.bounding_box;

        let video_direction = orientation.to_direction();

        let (source_width, source_height) = if orientation.is_vertical() {
            (self.info.height as i32, self.info.width as i32)
        } else {
            (self.info.width as i32, self.info.height as i32)
        };

        // offset is to place coordinate at 0,0. So use negative values
        let pos_x = -(bounding_box.left_x * source_width as f32) as i32;
        let pos_y = -(bounding_box.top_y * source_height as f32) as i32;

        let output_width =
            (source_width as f32 * (bounding_box.right_x - bounding_box.left_x)) as i32;
        let output_height =
            (source_height as f32 * (bounding_box.bottom_y - bounding_box.top_y)) as i32;

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
                .field("width", output_width)
                .field("height", output_height)
                .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
                .build();
            track.set_restriction_caps(&caps);

            track.elements().into_iter().for_each(|track_element| {
                track_element
                    .set_child_property("video-direction", &(video_direction.to_value()))
                    .unwrap();
                track_element
                    .set_child_property("width", &(source_width.to_value()))
                    .unwrap();
                track_element
                    .set_child_property("height", &(source_height.to_value()))
                    .unwrap();
                track_element
                    .set_child_property("posx", &(pos_x.to_value()))
                    .unwrap();
                track_element
                    .set_child_property("posy", &(pos_y.to_value()))
                    .unwrap();
            });

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
            // fixme: squished video, use gesVideoUriSource properties (video-direction)
            // let rotate = format!(
            //     "autovideoflip video-direction={}",
            //     sink_orientation_to_effect(orientation)
            // );
            // let rotate_effect = ges::Effect::new(&*rotate).unwrap();
            // clip.add(&rotate_effect).unwrap();

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
