use std::thread;
use std::time::{Duration, SystemTime};

use ges::gst_pbutils;
use ges::prelude::{
    EncodingProfileBuilder, ExtractableExt, GESPipelineExt, LayerExt, TimelineElementExt,
    TimelineExt, UriClipAssetExt,
};
use gst::prelude::*;
use gst::{ClockTime, Element, SeekFlags, State};
use gtk4::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::adw::gdk;
use relm4::*;

use crate::ui::timeline::{TimelineModel, TimelineMsg, TimelineOutput};

struct PlayingInfo {
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
    timeline: Controller<TimelineModel>,
    video_duration: Option<ClockTime>,
    video_uri: Option<String>,
    playing_info: Option<PlayingInfo>,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    ExportFrame,
    TogglePlayPause,
    ToggleMute,
    SeekToPercent(f64),
    NewVideo(String),
    ExportVideo,
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
    type Output = TimelineMsg;
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 670,
            set_height_request: 390,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            inline_css: "margin: 15px",

            gtk::Spinner {
                #[watch]
                set_spinning: !model.video_is_loaded,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
            },

            #[name = "vid_frame"]
            gtk::Box {
                #[watch]
                set_visible: model.video_is_loaded,
                set_orientation: gtk::Orientation::Vertical,

                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |_,_,_,_| {
                        sender.input(VideoPlayerMsg::TogglePlayPause)
                    }
                }
            },

            gtk::Box {
                #[watch]
                set_visible: model.video_is_loaded,
                set_spacing: 10,
                add_css_class: "toolbar",

                gtk::Button {
                    #[watch]
                    set_icon_name: if model.is_playing {
                        "pause"
                    } else {
                        "play"
                    },

                    connect_clicked[sender] => move |_| {
                            sender.input(VideoPlayerMsg::TogglePlayPause)
                    }
                },

                model.timeline.widget(){},

                gtk::Button {
                    #[watch]
                     set_icon_name: if model.is_mute {
                        "audio-volume-muted"
                    } else {
                        "audio-volume-high"
                    },
                    connect_clicked[sender] => move |_| {
                            sender.input(VideoPlayerMsg::ToggleMute)
                    }
                },
            },
        }
    }

    // todo: remove init u8, not used
    type Init = u8;

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

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);

        let timeline: Controller<TimelineModel> =
            TimelineModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    TimelineOutput::SeekToPercent(percent) => {
                        VideoPlayerMsg::SeekToPercent(percent)
                    }
                });

        let model = VideoPlayerModel {
            video_is_selected: false,
            video_is_loaded: false,
            is_playing: false,
            is_mute: false,
            gtk_sink,
            timeline,
            video_duration: None,
            video_uri: None,
            playing_info: None,
        };

        let widgets = view_output!();

        widgets.vid_frame.append(&offload);

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
            VideoPlayerMsg::NewVideo(value) => {
                self.video_is_selected = true;
                self.video_uri = Some(value);
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

                let uri = self.video_uri.as_ref().unwrap().clone();
                self.timeline
                    .sender()
                    .send(TimelineMsg::GenerateThumbnails(uri))
                    .unwrap();
            }
            VideoPlayerMsg::SeekToPercent(percent) => self.seek_to_percent(percent),
            VideoPlayerMsg::TogglePlayPause => self.video_toggle_play_pause(),
            VideoPlayerMsg::ToggleMute => self.toggle_mute(),
            VideoPlayerMsg::ExportFrame => {
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
                let now = SystemTime::now();
                let info = self.playing_info.as_ref().unwrap();
                info.pipeline.set_state(State::Paused).unwrap();

                let audio_profile = gst_pbutils::EncodingAudioProfile::builder(
                    &gst::Caps::builder("audio/mpeg").build(),
                )
                    .build();
                let video_profile = gst_pbutils::EncodingVideoProfile::builder(
                    &gst::Caps::builder("video/x-h265").build(),
                )
                    .build();

                let container_profile = gst_pbutils::EncodingContainerProfile::builder(
                    &gst::Caps::builder("video/x-matroska").build(),
                )
                    .name("Container")
                    .add_profile(video_profile)
                    .add_profile(audio_profile)
                    .build();

                let out_uri = "file:///home/fareed/Videos/out.mkv";

                info.pipeline
                    .set_render_settings(&out_uri, &container_profile)
                    .expect("unable to set render settings");
                info.pipeline
                    .set_mode(ges::PipelineFlags::RENDER)
                    .expect("failed to set to render");

                let start_time = self.video_duration.unwrap().mseconds() as f64
                    * self.timeline.model().get_target_start_percent();
                let end_time = self.video_duration.unwrap().mseconds() as f64
                    * self.timeline.model().get_target_end_percent();
                let duration = (end_time - start_time) as u64;

                info.clip
                    .set_inpoint(ClockTime::from_mseconds(start_time as u64));
                info.clip.set_duration(ClockTime::from_mseconds(duration));

                info.pipeline.set_state(State::Playing).unwrap();

                let bus = info.pipeline.bus().unwrap();

                thread::spawn(move || {
                    for msg in bus.iter_timed(gst::ClockTime::NONE) {
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

                let asset = self
                    .playing_info
                    .as_ref()
                    .unwrap()
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
                        self.timeline
                            .sender()
                            .send(TimelineMsg::UpdateSeekBarPos(percent))
                            .unwrap();
                    }
                    None => {}
                }
            }
        }
    }
}

impl VideoPlayerModel {
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
            let mut play_info = self.playing_info.as_mut().unwrap();
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
            layer.add_clip(&clip).unwrap();

            pipeline.set_video_sink(Some(&self.gtk_sink));
            // fixme: audio does not play
            let audio_sink = gst::ElementFactory::make("autoaudiosink").build().unwrap();
            pipeline.preview_set_audio_sink(Some(&audio_sink));

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
}
