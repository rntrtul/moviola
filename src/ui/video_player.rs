use std::fmt::Debug;

use ges::gst_pbutils;
use ges::gst_pbutils::EncodingContainerProfile;
use ges::prelude::EncodingProfileBuilder;
use gst::prelude::*;
use gst::Element;
use gst_video::VideoOrientationMethod;
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::adw::gdk;
use relm4::*;

use crate::ui::crop_box::MARGIN;

// todo: dispose of stuff on quit
pub struct VideoPlayerModel {
    video_is_loaded: bool,
    is_playing: bool,
    is_mute: bool,
    gtk_sink: Element,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    TogglePlayPause,
    VideoLoaded,
}

#[derive(Debug)]
pub enum VideoPlayerOutput {
    ToggleVideoPlay,
}

impl VideoPlayerModel {
    pub fn sink(&self) -> &Element {
        &self.gtk_sink
    }
}

#[relm4::component(pub)]
impl Component for VideoPlayerModel {
    type CommandOutput = ();
    type Input = VideoPlayerMsg;
    type Output = VideoPlayerOutput;
    view! {
        #[name = "vid_container"]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 640,
            set_height_request: 360,

            gtk::Spinner {
                #[watch]
                set_spinning: !model.video_is_loaded,
                #[watch]
                set_visible: !model.video_is_loaded,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
            },

            add_controller = gtk::GestureClick {
                connect_pressed[sender] => move |_,_,_,_| {
                    sender.input(VideoPlayerMsg::TogglePlayPause)
                }
            },
        }
    }

    type Init = ();

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let gtk_sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .unwrap();

        let paintable = gtk_sink.property::<gdk::Paintable>("paintable");
        // todo: need gst-plugins-gtk4 13.0 to be able to use orientation property with paintable
        let picture = gtk::Picture::new();

        picture.set_paintable(Some(&paintable));
        picture.set_margin_all(MARGIN as i32);

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        offload.set_visible(false);

        let model = VideoPlayerModel {
            video_is_loaded: false,
            is_playing: false,
            is_mute: false,
            gtk_sink,
        };

        let widgets = view_output!();

        widgets.vid_container.append(&offload);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            VideoPlayerMsg::VideoLoaded => {
                self.is_playing = true;
                self.video_is_loaded = true;
                root.last_child().unwrap().set_visible(true);
            }
            VideoPlayerMsg::TogglePlayPause => {
                sender.output(VideoPlayerOutput::ToggleVideoPlay).unwrap();
            }
        }

        self.update_view(widgets, sender);
    }
}

impl VideoPlayerModel {
    // todo: hookup with ui/keyboard. add support for stepping backwards
    fn step_next_frame(&mut self) {
        let step = gst::event::Step::new(gst::format::Buffers::ONE, 1.0, true, false);
        self.gtk_sink.send_event(step);
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
    //
    // fn remove_effect(&self, effect_name: &str) {
    //     let info = self.playing_info.as_ref().unwrap();
    //
    //     let found_element = info
    //         .clip
    //         .children(false)
    //         .into_iter()
    //         .find(|child| child.name().unwrap() == effect_name);
    //
    //     if let Some(prev_effect) = found_element {
    //         info.clip
    //             .remove(&prev_effect)
    //             .expect("could not delete previous effect");
    //     }
    // }
    //
    // fn add_effect(&mut self, effect: &Effect) {
    //     let info = self.playing_info.as_mut().unwrap();
    //     info.clip.add(effect).unwrap();
    //     info.timeline.commit_sync();
    // }
    //
    // fn set_video_orientation(&mut self, orientation: VideoOrientationMethod) {
    //     // todo: split flip and rotates
    //     let val = Self::video_orientation_method_to_val(orientation);
    //
    //     let effect = format!("autovideoflip video-direction={val}");
    //     let flip = Effect::new(effect.as_str()).expect("could not make flip");
    //     flip.set_name(Some("orientation"))
    //         .expect("Unable to set name");
    //
    //     self.remove_effect("orientation");
    //
    //     let mut preview_width = 640;
    //     let mut preview_height = (preview_width as f64 / self.frame_info.aspect_ratio) as i32;
    //
    //     if orientation == VideoOrientationMethod::_90l
    //         || orientation == VideoOrientationMethod::_90r
    //     {
    //         let tmp = preview_width;
    //         preview_width = preview_height;
    //         preview_height = tmp;
    //         println!("switched resolution to {preview_width}x{preview_height}");
    //     }
    //
    //     let preview_caps = gst::Caps::builder("video/x-raw")
    //         .field("framerate", self.frame_info.framerate)
    //         .field("width", preview_width)
    //         .field("height", preview_height)
    //         .build();
    //     let tracks = self.playing_info.as_ref().unwrap().timeline.tracks();
    //     let track = tracks.first().unwrap();
    //     track.set_restriction_caps(&preview_caps);
    //     // self.playing_info.as_ref().unwrap().timeline.commit_sync();
    //     self.add_effect(&flip);
    // }
    //
    // fn set_video_crop(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
    //     println!("{top} {left} {right} {bottom}");
    //     let effect = format!("videocrop top={top} left={left} right={right} bottom={bottom}");
    //     let crop = Effect::new(effect.as_str()).expect("could not make crop");
    //     crop.set_name(Some("crop")).expect("unable to set name");
    //
    //     self.remove_effect("crop");
    //     self.add_effect(&crop);
    //
    //     // convert left, right, to displaying
    //
    //     let scale_factor = 640 as f64 / self.frame_info.width as f64;
    //
    //     let preview_width = (self.frame_info.width as i32 - left - right) as f64 * scale_factor;
    //     let preview_height = (self.frame_info.height as i32 - top - right) as f64 * scale_factor;
    //
    //     println!("preview: {preview_width}x{preview_height}");
    //
    //     let preview_caps = gst::Caps::builder("video/x-raw")
    //         .field("framerate", self.frame_info.framerate)
    //         .field("width", preview_width as i32)
    //         .field("height", preview_height as i32)
    //         .build();
    //     let tracks = self.playing_info.as_ref().unwrap().timeline.tracks();
    //     let track = tracks.first().unwrap();
    //     track.set_restriction_caps(&preview_caps);
    //     self.playing_info.as_ref().unwrap().timeline.commit_sync();
    // }
    //
    // fn export_video(&self) {
    //     let now = SystemTime::now();
    //     let info = self.playing_info.as_ref().unwrap();
    //     // todo: use toggle_play_pause for setting state to keep ui insync
    //     info.pipeline.set_state(State::Paused).unwrap();
    //
    //     let out_uri = "file:///home/fareed/Videos/out.mkv";
    //     let container_profile = Self::build_container_profile();
    //
    //     info.pipeline
    //         .set_render_settings(&out_uri, &container_profile)
    //         .expect("unable to set render settings");
    //     // todo: use smart_render?
    //     info.pipeline
    //         .set_mode(PipelineFlags::RENDER)
    //         .expect("failed to set to render");
    //
    //     let start_time = 1500;
    //     // self.frame_info.duration.mseconds() as f64 * self.timeline.model().get_target_start_percent();
    //     let end_time = 35000;
    //     // self.frame_info.duration.mseconds() as f64 * self.timeline.model().get_target_end_percent();
    //     let duration = (end_time - start_time) as u64;
    //
    //     info.clip
    //         .set_inpoint(ClockTime::from_mseconds(start_time as u64));
    //     info.clip.set_duration(ClockTime::from_mseconds(duration));
    //
    //     info.pipeline.set_state(State::Playing).unwrap();
    //
    //     let bus = info.pipeline.bus().unwrap();
    //
    //     thread::spawn(move || {
    //         for msg in bus.iter_timed(ClockTime::NONE) {
    //             use gst::MessageView;
    //
    //             match msg.view() {
    //                 MessageView::Eos(..) => {
    //                     println!("Done? in {:?}", now.elapsed());
    //                     break;
    //                 }
    //                 MessageView::Error(err) => {
    //                     println!(
    //                         "Error from {:?}: {} ({:?})",
    //                         err.src().map(|s| s.path_string()),
    //                         err.error(),
    //                         err.debug()
    //                     );
    //                     break;
    //                 }
    //                 _ => (),
    //             }
    //         }
    //     });
    // }
}
