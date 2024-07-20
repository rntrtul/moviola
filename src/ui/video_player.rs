use std::thread;
use std::time::{Duration, SystemTime};

use ges::gst_pbutils::EncodingContainerProfile;
use ges::prelude::{
    EncodingProfileBuilder, ExtractableExt, GESContainerExt, GESPipelineExt, LayerExt,
    TimelineElementExt, TimelineExt, UriClipAssetExt, UriClipExt,
};
use ges::{gst_pbutils, Effect, PipelineFlags};
use gst::prelude::*;
use gst::{ClockTime, Element, SeekFlags, State};
use gst_video::{VideoFrameExt, VideoOrientationMethod};
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::adw::gdk;
use relm4::*;

use crate::ui::crop_box::MARGIN;

#[derive(Debug)]
pub struct FrameInfo {
    width: u32,
    height: u32,
    pub(crate) aspect_ratio: f64,
}

pub struct PlayingInfo {
    pipeline: ges::Pipeline,
    clip: ges::UriClip,
    layer: ges::Layer,
    timeline: ges::Timeline,
}

// todo: dispose of stuff on quit
// todo: do i need is_loaded and is_playing?
pub struct VideoPlayerModel {
    video_is_selected: bool,
    video_is_loaded: bool,
    is_playing: bool,
    is_mute: bool,
    gtk_sink: Element,
    video_duration: Option<ClockTime>,
    video_uri: Option<String>,
    playing_info: Option<PlayingInfo>,
    frame_info: FrameInfo,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    ExportFrame,
    FrameInfo(FrameInfo),
    TogglePlayPause,
    ToggleMute,
    SeekToPercent(f64),
    OrientVideo(VideoOrientationMethod),
    NewVideo(String),
    ExportVideo,
}

#[derive(Debug)]
pub enum VideoPlayerOutput {
    AudioMute,
    AudioPlaying,
    UpdateSeekBarPos(f64),
    VideoLoaded,
    VideoPaused,
    VideoPlaying,
}

#[derive(Debug)]
pub enum VideoPlayerCommandMsg {
    VideoInit(bool),
    UpdateSeekPos,
}

#[relm4::component(pub)]
impl Component for VideoPlayerModel {
    type CommandOutput = VideoPlayerCommandMsg;
    type Input = VideoPlayerMsg;
    type Output = VideoPlayerOutput;
    view! {
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

            #[name = "vid_container"]
            gtk::Box{
                set_width_request: 640,
                set_height_request: 360,
                #[watch]
                set_visible: model.video_is_loaded,
                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |_,_,_,_| {
                        sender.input(VideoPlayerMsg::TogglePlayPause)
                    }
                },
            },
        }
    }

    type Init = ();

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        gst::init().unwrap();

        let gtk_sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .unwrap();

        let paintable = gtk_sink.property::<gdk::Paintable>("paintable");
        let picture = gtk::Picture::new();

        picture.set_paintable(Some(&paintable));
        picture.set_margin_all(MARGIN as i32);

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);

        let model = VideoPlayerModel {
            video_is_selected: false,
            video_is_loaded: false,
            is_playing: false,
            is_mute: false,
            gtk_sink,
            video_duration: None,
            video_uri: None,
            playing_info: None,
            frame_info: FrameInfo {
                width: 0,
                height: 0,
                aspect_ratio: 0.,
            },
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
        _root: &Self::Root,
    ) {
        match message {
            VideoPlayerMsg::NewVideo(uri) => {
                self.video_is_selected = true;
                self.video_uri = Some(uri);
                self.video_is_loaded = false;
                self.is_playing = false;
                self.video_duration = None;
                self.play_new_video();

                let bus = self.playing_info.as_ref().unwrap().pipeline.bus().unwrap();
                sender.oneshot_command(async move {
                    for msg in bus.iter_timed(ClockTime::NONE) {
                        use gst::MessageView;

                        match msg.view() {
                            MessageView::AsyncDone(..) => {
                                break;
                            }
                            _ => (),
                        }
                    }

                    VideoPlayerCommandMsg::VideoInit(true)
                });
                sender.output(VideoPlayerOutput::VideoLoaded).unwrap();
            }
            VideoPlayerMsg::SeekToPercent(percent) => self.seek_to_percent(percent),
            VideoPlayerMsg::TogglePlayPause => {
                // if widgets.crop_box.drag_active() {
                //     return;
                // }

                self.video_toggle_play_pause();
                if self.is_playing {
                    sender.output(VideoPlayerOutput::VideoPlaying).unwrap();
                } else {
                    sender.output(VideoPlayerOutput::VideoPaused).unwrap();
                }
            }
            VideoPlayerMsg::ToggleMute => {
                self.toggle_mute();
                if self.is_mute {
                    sender.output(VideoPlayerOutput::AudioMute).unwrap();
                } else {
                    sender.output(VideoPlayerOutput::AudioPlaying).unwrap();
                }
            }
            VideoPlayerMsg::ExportFrame => {
                // todo: get actual video width and height
                // todo: ask for file location and name
                if self.video_is_loaded {
                    self.playing_info
                        .as_ref()
                        .unwrap()
                        .pipeline
                        .save_thumbnail(1920, 1080, "image/jpeg", "/home/fareed/Videos/export.jpg")
                        .expect("unable to save exported frame");
                }
            }
            VideoPlayerMsg::ExportVideo => {
                self.export_video();
            }
            VideoPlayerMsg::OrientVideo(orientation) => {
                self.add_orientation(orientation);
            }
            VideoPlayerMsg::FrameInfo(info) => self.frame_info = info,
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            VideoPlayerCommandMsg::VideoInit(_) => {
                self.is_playing = true;
                self.video_is_loaded = true;

                let info = self.playing_info.as_ref().unwrap();
                let asset = info
                    .clip
                    .asset()
                    .unwrap()
                    .downcast::<ges::UriClipAsset>()
                    .unwrap();

                self.video_duration = Some(asset.duration().expect("could not get duration"));

                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            loop {
                                tokio::time::sleep(Duration::from_millis(125)).await;
                                out.send(VideoPlayerCommandMsg::UpdateSeekPos).unwrap();
                            }
                        })
                        .drop_on_shutdown()
                });
                sender.output(VideoPlayerOutput::VideoPlaying).unwrap();
            }
            VideoPlayerCommandMsg::UpdateSeekPos => {
                let info = self.playing_info.as_ref().unwrap();

                if self.video_duration == None
                    || info.pipeline.state(Some(ClockTime::ZERO)).1 != State::Playing
                {
                    return;
                }
                let query_val = info.pipeline.query_position::<ClockTime>();
                match query_val {
                    Some(curr_position) => {
                        let percent = curr_position.mseconds() as f64
                            / self.video_duration.unwrap().mseconds() as f64;
                        sender
                            .output(VideoPlayerOutput::UpdateSeekBarPos(percent))
                            .unwrap();
                    }
                    None => {}
                }
            }
        }
    }
}

impl VideoPlayerModel {
    pub fn get_sample_frame_info(sample: gst::Sample) -> FrameInfo {
        let buffer = sample.buffer().unwrap();

        let caps = sample.caps().expect("Sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info).unwrap();
        let display_aspect_ratio = (frame.width() as f64 * info.par().numer() as f64)
            / (frame.height() as f64 * info.par().denom() as f64);

        FrameInfo {
            width: frame.width(),
            height: frame.height(),
            aspect_ratio: display_aspect_ratio,
        }
    }

    // fixme: sometimes new video just hangs
    pub fn wait_for_playbin_done(playbin: &Element) {
        let bus = playbin.bus().unwrap();

        for msg in bus.iter_timed(ClockTime::NONE) {
            use gst::MessageView;

            match msg.view() {
                MessageView::AsyncDone(..) => {
                    break;
                }
                _ => (),
            }
        }
    }

    // todo: hookup with ui/keyboard. add support for stepping backwards
    fn step_next_frame(&mut self) {
        let step = gst::event::Step::new(gst::format::Buffers::ONE, 1.0, true, false);
        self.gtk_sink.send_event(step);
    }

    fn seek_to_percent(&mut self, percent: f64) {
        if self.playing_info.is_none() || !self.video_is_loaded {
            println!("early exit for seek");
            return;
        }

        let seconds = (self.video_duration.unwrap().seconds() as f64 * percent) as u64;

        let time = gst::GenericFormattedValue::from(ClockTime::from_seconds(seconds));
        let seek = gst::event::Seek::new(
            1.0,
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT,
            gst::SeekType::Set,
            time,
            gst::SeekType::End,
            ClockTime::ZERO,
        );

        self.playing_info
            .as_ref()
            .unwrap()
            .pipeline
            .send_event(seek);
    }

    fn toggle_mute(&mut self) {
        // fixme: work with gesPipeline
        self.is_mute = !self.is_mute;
        self.playing_info
            .as_ref()
            .unwrap()
            .clip
            .set_mute(self.is_mute);
    }

    fn video_toggle_play_pause(&mut self) {
        let (new_state, playbin_new_state) = if self.is_playing {
            (false, State::Paused)
        } else {
            (true, State::Playing)
        };

        self.is_playing = new_state;
        self.playing_info
            .as_ref()
            .unwrap()
            .pipeline
            .set_state(playbin_new_state)
            .unwrap();
    }

    fn play_new_video(&mut self) {
        if self.playing_info.is_some() {
            let play_info = self.playing_info.as_mut().unwrap();
            play_info.pipeline.set_state(State::Null).unwrap();

            let clip =
                ges::UriClip::new(self.video_uri.as_ref().unwrap()).expect("failed to create clip");

            play_info
                .layer
                .remove_clip(&play_info.clip)
                .expect("could not delete");

            play_info.layer.add_clip(&clip).expect("unable to add clip");
            play_info.clip = clip;
        } else {
            let timeline = ges::Timeline::new_audio_video();
            let layer = timeline.append_layer();
            let pipeline = ges::Pipeline::new();
            pipeline.set_timeline(&timeline).unwrap();

            let clip =
                ges::UriClip::new(self.video_uri.as_ref().unwrap()).expect("failed to create clip");
            clip.set_mute(false);
            layer.add_clip(&clip).unwrap();

            pipeline
                .set_mode(PipelineFlags::FULL_PREVIEW)
                .expect("unable to preview");
            pipeline.set_video_sink(Some(&self.gtk_sink));
            // fixme: audio does not play (maybe need to choose audio stream)
            let audio_sink = gst::ElementFactory::make("autoaudiosink").build().unwrap();
            pipeline.set_audio_sink(Some(&audio_sink));

            let info = PlayingInfo {
                pipeline,
                layer,
                clip,
                timeline,
            };

            self.playing_info = Some(info);
        }

        self.playing_info
            .as_ref()
            .unwrap()
            .pipeline
            .set_state(State::Playing)
            .unwrap();

        self.is_mute = false;
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

    fn replace_or_add_effect(&mut self, effect: &Effect, effect_name: &str) {
        let info = self.playing_info.as_mut().unwrap();

        let found_element = info
            .clip
            .children(false)
            .into_iter()
            .find(|child| child.name().unwrap() == effect_name);

        if let Some(prev_effect) = found_element {
            info.clip
                .remove(&prev_effect)
                .expect("could not delete previous effect");
        }

        info.clip.add(effect).unwrap();
        info.timeline.commit_sync();
    }

    fn add_orientation(&mut self, orientation: VideoOrientationMethod) {
        // todo: split flip and rotates
        let val = Self::video_orientation_method_to_val(orientation);

        let effect = format!("autovideoflip video-direction={val}");
        let flip = Effect::new(effect.as_str()).expect("could not make flip");
        flip.set_name(Some("orientation"))
            .expect("Unable to set name");

        self.replace_or_add_effect(&flip, "orientation");
    }

    fn export_video(&self) {
        let now = SystemTime::now();
        let info = self.playing_info.as_ref().unwrap();
        // todo: use toggle_play_pause for setting state to keep ui insync
        info.pipeline.set_state(State::Paused).unwrap();

        let out_uri = "file:///home/fareed/Videos/out.mkv";
        let container_profile = Self::build_container_profile();

        info.pipeline
            .set_render_settings(&out_uri, &container_profile)
            .expect("unable to set render settings");
        // todo: use smart_render?
        info.pipeline
            .set_mode(PipelineFlags::RENDER)
            .expect("failed to set to render");

        let start_time = 1500;
        // self.video_duration.unwrap().mseconds() as f64 * self.timeline.model().get_target_start_percent();
        let end_time = 35000;
        // self.video_duration.unwrap().mseconds() as f64 * self.timeline.model().get_target_end_percent();
        let duration = (end_time - start_time) as u64;

        info.clip
            .set_inpoint(ClockTime::from_mseconds(start_time as u64));
        info.clip.set_duration(ClockTime::from_mseconds(duration));

        info.pipeline.set_state(State::Playing).unwrap();

        let bus = info.pipeline.bus().unwrap();

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
    }
}
