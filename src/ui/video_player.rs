use gst::prelude::*;
use gtk4::prelude::{BoxExt, ButtonExt, EventControllerExt, GestureDragExt, OrientableExt, WidgetExt};
use relm4::*;
use relm4::adw::gdk;

pub struct VideoPlayerModel {
    video_is_selected: bool,
    is_playing: bool,
    gtk_sink: gst::Element,
    video_uri: Option<String>,
    playbin: Option<gst::Element>,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    Play,
    Pause,
    TogglePlayPause,
    Stop,
    SeekToPercent(f64),
    NewVideo(String),
}

#[relm4::component(pub)]
impl SimpleComponent for VideoPlayerModel {
    type Input = VideoPlayerMsg;
    type Output = ();
    type Init = u8;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 670,
            set_height_request: 390,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            inline_css: "margin: 15px",

            #[name = "vid_frame"]
            gtk::Box {
                #[watch]
                set_visible: model.video_is_selected,
                set_orientation: gtk::Orientation::Vertical,

                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |_,_,_,_| {
                        sender.input(VideoPlayerMsg::TogglePlayPause)
                    }
                }
            },

            gtk::Box {
                set_spacing: 10,
                add_css_class: "toolbar",

                gtk::Button {
                    set_icon_name: "play",
                },

                #[name = "timeline"]
                gtk::Box {
                    set_hexpand: true,
                    inline_css: "background-color: grey",

                    add_controller = gtk::GestureClick {
                        connect_pressed[sender] => move |click,_,x,_| {
                            let width = click.widget().width() as f64;
                            let percent = x / width;
                            sender.input(VideoPlayerMsg::SeekToPercent(percent));
                        }
                    },

                    add_controller = gtk::GestureDrag {
                        connect_drag_update[sender] => move |drag,x_offset,_| {
                            let (start_x, _) = drag.start_point().unwrap();
                            let width = drag.widget().width() as f64;
                            let percent_dragged = (start_x + x_offset) / width;

                            sender.input(VideoPlayerMsg::SeekToPercent(percent_dragged));
                        }
                    },
                },

                gtk::Button {
                     set_icon_name: "audio-volume-muted",
                },
            },
        }
    }

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

        let model = VideoPlayerModel {
            video_is_selected: false,
            is_playing: true,
            playbin: None,
            gtk_sink,
            video_uri: None,
        };

        let widgets = view_output!();

        widgets.vid_frame.append(&offload);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            VideoPlayerMsg::NewVideo(value) => {
                self.video_uri = Some(value);
                self.video_is_selected = true;
                self.play_new_video();
            }
            VideoPlayerMsg::TogglePlayPause => self.video_toggle_play_pause(),
            VideoPlayerMsg::SeekToPercent(percent) => self.seek_to_percent(percent),
            _ => panic!("Unknown message received for video player")
        }
    }
}

impl VideoPlayerModel {
    // todo: hookup with ui/keyboard. add support for stepping backwards
    fn step_next_frame(&mut self) {
        if let Some(video_sink) = self.playbin.as_ref().unwrap().property::<Option<gst::Element>>("video-sink") {
            let step = gst::event::Step::new(gst::format::Buffers::ONE, 1.0, true, false);
            video_sink.send_event(step);
        }
    }
    fn seek_to_percent(&mut self, percent: f64) {
        if self.playbin.is_none() {
            println!("early exit for seek");
            return;
        }

        let duration = self.playbin.as_ref().unwrap().query_duration::<gst::ClockTime>().unwrap();
        let seconds = (duration.seconds() as f64 * percent) as u64;

        let time = gst::GenericFormattedValue::from(gst::ClockTime::from_seconds(seconds));

        let seek = gst::event::Seek::new(
            1.0,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            time,
            gst::SeekType::End,
            gst::ClockTime::ZERO);

        self.playbin.as_ref().unwrap().send_event(seek);
    }

    fn video_toggle_play_pause(&mut self) {
        let (new_state, playbin_new_state) = if self.is_playing {
            (false, gst::State::Paused)
        } else {
            (true, gst::State::Playing)
        };

        self.is_playing = new_state;
        self.playbin.as_ref().unwrap().set_state(playbin_new_state).unwrap();
    }
    fn play_new_video(&mut self) {
        if self.playbin.is_some() {
            self.playbin.as_ref().unwrap().set_state(gst::State::Null).unwrap();
        }

        let playbin = gst::ElementFactory::make("playbin")
            .name("playbin")
            .property("uri", self.video_uri.as_ref().unwrap())
            .build()
            .unwrap();

        playbin.set_property("video-sink", &self.gtk_sink);
        playbin.set_state(gst::State::Playing).unwrap();

        //  todo: investigate to see if leaking memory here
        self.playbin = Some(playbin);
        self.is_playing = true;
    }
}