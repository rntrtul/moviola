use std::time;

use gst::{element_error, SeekFlags};
use gst::prelude::*;
use gst_app::prelude::BaseSrcExt;
use gst_video::VideoFrameExt;
use gtk4::prelude::{BoxExt, ButtonExt, EventControllerExt, GestureDragExt, OrientableExt, WidgetExt};
use relm4::*;
use relm4::adw::gdk;

pub struct VideoPlayerModel {
    video_is_selected: bool,
    is_playing: bool,
    is_mute: bool,
    gtk_sink: gst::Element,
    video_uri: Option<String>,
    playbin: Option<gst::Element>,
    thumbnail_pipeline: Option<gst::Pipeline>,
    got_snapshot: bool,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    Play,
    Pause,
    TogglePlayPause,
    ToggleMute,
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
                            // todo: worry about seek only working on drag being still?
                            let (start_x, _) = drag.start_point().unwrap();
                            let width = drag.widget().width() as f64;
                            let percent_dragged = (start_x + x_offset) / width;

                            sender.input(VideoPlayerMsg::SeekToPercent(percent_dragged));
                        }
                    },
                },

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
            is_mute: false,
            playbin: None,
            gtk_sink,
            video_uri: None,
            thumbnail_pipeline: None,
            got_snapshot: false,
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
            VideoPlayerMsg::ToggleMute => self.toggle_mute(),
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

    fn create_thumbnail(&mut self, thumbnail_save_path: std::path::PathBuf, timestamp: u64) {
        let uri = &self.video_uri.as_ref().unwrap();
        let pipeline = gst::parse::launch(&format!(
            "uridecodebin uri={uri} ! videoconvert ! appsink name=sink"
        )).unwrap()
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

        let mut got_snapshot = false;

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Error).unwrap();
                    let buffer = sample.buffer().ok_or_else(|| {
                        element_error!(appsink, gst::ResourceError::Failed, ("Failed"));
                        gst::FlowError::Error
                    }).unwrap();

                    // fixme: does this limit to 1 snapshot
                    if got_snapshot {
                        return Err(gst::FlowError::Eos);
                    }

                    got_snapshot = true;

                    let caps = sample.caps().expect("sample without caps");
                    let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");
                    println!("name: {}, {:?}", info.format_info().has_alpha(), info.format_info().flags());

                    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                        .map_err(|_| {
                            element_error!(appsink, gst::ResourceError::Failed, ("Failed to map buff readable"));
                            gst::FlowError::Error
                        }).unwrap();


                    let aspect_ratio = (frame.width() as f64 * info.par().numer() as f64)
                        / (frame.height() as f64 * info.par().denom() as f64);
                    let target_height = 180;
                    let target_width = target_height as f64 * aspect_ratio;
                    println!("w: {}, h: {}", target_width, target_height);

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
                        &img.as_view::<image::Rgb<u8>>().expect("could not create image view"),
                        target_width as u32,
                        target_height as u32,
                    );

                    scaled_img.save(&thumbnail_save_path).map_err(|err| {
                        element_error!(appsink, gst::ResourceError::Write,
                        (
                            "Failed to write a preview file {}: {}",
                            &thumbnail_save_path.display(), err
                        ));
                        gst::FlowError::Error
                    }).unwrap();

                    Err(gst::FlowError::Eos)
                })
                .build()
        );

        pipeline.set_state(gst::State::Paused).unwrap();
        std::thread::sleep(time::Duration::from_secs(3));

        let time = gst::GenericFormattedValue::from(gst::ClockTime::from_seconds(timestamp));
        let _ = pipeline.seek_simple(SeekFlags::FLUSH, time);
        pipeline.set_state(gst::State::Playing).unwrap();

        self.thumbnail_pipeline = Some(pipeline);
    }

    fn seek_to_percent(&mut self, percent: f64) {
        if self.playbin.is_none() || !self.is_playing {
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

    fn toggle_mute(&mut self) {
        self.is_mute = !self.is_mute;
        self.playbin.as_ref().unwrap().set_property("mute", self.is_mute);
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
            self.playbin.as_ref().unwrap().set_property("uri", self.video_uri.as_ref().unwrap());
        } else {
            let playbin = gst::ElementFactory::make("playbin")
                .name("playbin")
                .property("uri", self.video_uri.as_ref().unwrap())
                .property("video-sink", &self.gtk_sink)
                .build()
                .unwrap();

            self.playbin = Some(playbin);
        }

        self.playbin.as_ref().unwrap().set_property("mute", false);
        self.playbin.as_ref().unwrap().set_state(gst::State::Playing).unwrap();

        self.create_thumbnail(std::path::PathBuf::from("/home/fareed/Videos/test.jpg"), 35);
        // todo: pause until playbin setup right to pervent seeking from happening too early
        self.is_playing = true;
        self.is_mute = false;
    }
}