use gst::prelude::*;
use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
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
            inline_css: "margin: 15px",

            #[name = "vid_frame"]
            gtk::Box {
                #[watch]
                set_visible: model.video_is_selected,
                set_orientation: gtk::Orientation::Vertical,
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

        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(move |_, _, _, _| {
            sender.input(VideoPlayerMsg::TogglePlayPause);
        });
        widgets.vid_frame.add_controller(gesture);
        
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
            _ => panic!("Unknown message recived for video player")
        }
    }
}

impl VideoPlayerModel {
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

        self.playbin = Some(playbin);
        self.is_playing = true;
    }
}