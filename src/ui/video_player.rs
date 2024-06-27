use std::thread;
use std::time::Duration;

use gst::glib::FlagsClass;
use gst::prelude::*;
use gst::{ClockTime, Element, SeekFlags};
use gst_video::VideoFrameExt;
use gtk4::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use image::RgbImage;
use relm4::adw::gdk;
use relm4::*;

use crate::ui::timeline::{TimelineModel, TimelineMsg, TimelineOutput};

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
    playbin: Option<Element>,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    ExportFrame,
    TogglePlayPause,
    ToggleMute,
    SeekToPercent(f64),
    NewVideo(String),
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
            playbin: None,
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

                let playbin_clone = self.playbin.as_ref().unwrap().clone();
                sender.oneshot_command(async move {
                    VideoPlayerModel::wait_for_playbin_done(&playbin_clone);
                    VideoPlayerCommandMsg::VideoInit(true)
                });

                let uri = self.video_uri.as_ref().unwrap().clone();
                self.timeline
                    .sender()
                    .send(TimelineMsg::GenerateThumnails(uri))
                    .unwrap();
            }
            VideoPlayerMsg::SeekToPercent(percent) => self.seek_to_percent(percent),
            VideoPlayerMsg::TogglePlayPause => self.video_toggle_play_pause(),
            VideoPlayerMsg::ToggleMute => self.toggle_mute(),
            VideoPlayerMsg::ExportFrame => {
                if self.video_is_loaded {
                    let sample = self
                        .playbin
                        .as_ref()
                        .unwrap()
                        .property::<gst::Sample>("sample");
                    VideoPlayerModel::save_sample(&sample);
                }
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
                self.video_duration = Some(
                    self.playbin
                        .as_ref()
                        .unwrap()
                        .query_duration::<ClockTime>()
                        .unwrap(),
                );

                let playbin_clone = self.playbin.as_ref().unwrap().clone();
                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            loop {
                                tokio::time::sleep(Duration::from_millis(30)).await;
                                if playbin_clone.state(Some(ClockTime::ZERO)).1
                                    == gst::State::Playing
                                {
                                    out.send(VideoPlayerCommandMsg::UpdateSeekPos).unwrap();
                                }
                            }
                        })
                        .drop_on_shutdown()
                })
            }
            VideoPlayerCommandMsg::UpdateSeekPos => {
                if self.video_duration == None {
                    return;
                }

                let query_val = self.playbin.as_ref().unwrap().query_position::<ClockTime>();
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
        if let Some(video_sink) = self
            .playbin
            .as_ref()
            .unwrap()
            .property::<Option<Element>>("video-sink")
        {
            let step = gst::event::Step::new(gst::format::Buffers::ONE, 1.0, true, false);
            video_sink.send_event(step);
        }
    }

    fn seek_to_percent(&mut self, percent: f64) {
        if self.playbin.is_none() || !self.video_is_loaded {
            println!("early exit for seek");
            return;
        }

        let duration = self
            .playbin
            .as_ref()
            .unwrap()
            .query_duration::<ClockTime>()
            .unwrap();
        let seconds = (duration.seconds() as f64 * percent) as u64;

        let time = gst::GenericFormattedValue::from(ClockTime::from_seconds(seconds));
        let seek = gst::event::Seek::new(
            1.0,
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT,
            gst::SeekType::Set,
            time,
            gst::SeekType::End,
            ClockTime::ZERO,
        );

        self.playbin.as_ref().unwrap().send_event(seek);
    }

    fn toggle_mute(&mut self) {
        self.is_mute = !self.is_mute;
        self.playbin
            .as_ref()
            .unwrap()
            .set_property("mute", self.is_mute);
    }

    fn video_toggle_play_pause(&mut self) {
        let (new_state, playbin_new_state) = if self.is_playing {
            (false, gst::State::Paused)
        } else {
            (true, gst::State::Playing)
        };

        self.is_playing = new_state;
        self.playbin
            .as_ref()
            .unwrap()
            .set_state(playbin_new_state)
            .unwrap();
    }

    fn play_new_video(&mut self) {
        if self.playbin.is_some() {
            self.playbin
                .as_ref()
                .unwrap()
                .set_state(gst::State::Null)
                .unwrap();
            self.playbin
                .as_ref()
                .unwrap()
                .set_property("uri", self.video_uri.as_ref().unwrap());
        } else {
            let playbin = gst::ElementFactory::make("playbin")
                .name("playbin")
                .property("uri", self.video_uri.as_ref().unwrap())
                .property("video-sink", &self.gtk_sink)
                .build()
                .unwrap();

            let flags = playbin.property_value("flags");
            let flags_class = FlagsClass::with_type(flags.type_()).unwrap();

            let flags = flags_class
                .builder_with_value(flags)
                .unwrap()
                .set_by_nick("audio")
                .set_by_nick("video")
                .unset_by_nick("text")
                .build()
                .unwrap();
            playbin.set_property_from_value("flags", &flags);

            self.playbin = Some(playbin);
        }

        self.playbin.as_ref().unwrap().set_property("mute", false);
        self.playbin
            .as_ref()
            .unwrap()
            .set_state(gst::State::Playing)
            .unwrap();

        self.is_mute = false;
    }

    fn save_sample(sample: &gst::Sample) {
        let buffer = sample.buffer().unwrap();

        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info).unwrap();

        let img = image::FlatSamples::<&[u8]> {
            samples: frame.plane_data(0).unwrap(),
            layout: image::flat::SampleLayout {
                channels: 3,
                channel_stride: 1,
                width: frame.width(),
                width_stride: 3,
                height: frame.height(),
                height_stride: frame.plane_stride()[0] as usize,
            },
            color_hint: Some(image::ColorType::Rgb8),
        };

        let save_img =
            RgbImage::from_raw(frame.width(), frame.height(), Vec::from(img.samples)).unwrap();

        thread::spawn(move || {
            let thumbnail_save_path =
                std::path::PathBuf::from("/home/fareed/Videos/export_frame.jpg");
            save_img.save(&thumbnail_save_path).unwrap();
        });
    }
}
