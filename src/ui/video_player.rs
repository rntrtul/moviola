use gst::glib;
use gst::prelude::*;
use gtk4::{gio};
use gtk4::prelude::{OrientableExt, BoxExt, WidgetExt, ButtonExt, FileExt, GtkApplicationExt};
use relm4::*;
use relm4::adw::gdk;

pub struct VideoPlayerModel {
    video_is_selected: bool,
    gtk_sink: gst::Element,
    video_uri: Option<String>,
    playbin: Option<gst::Element>,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    Play,
    Pause,
    Stop,
    NewVideo(String),
}

fn launch_file_opener(sender: ComponentSender<VideoPlayerModel>) {
    let filters = gio::ListStore::new::<gtk::FileFilter>();

    let video_filter = gtk::FileFilter::new();
    video_filter.add_mime_type("video/*");
    video_filter.set_name(Some("Video"));
    filters.append(&video_filter);

    let audio_filter = gtk::FileFilter::new();
    audio_filter.add_mime_type("audio/*");
    audio_filter.set_name(Some("Audio"));
    filters.append(&audio_filter);

    let file_dialog = gtk::FileDialog::builder()
        .title("Open Video")
        .accept_label("Open")
        .modal(true)
        .filters(&filters)
        .build();

    let cancelable = gio::Cancellable::new();
    let window = relm4::main_adw_application().active_window().unwrap();
    // let window = root.root().unwrap().downcast::<Window>().unwrap();

    file_dialog.open(Some(&window), Some(&cancelable), move |result| {
        let file = match result {
            Ok(f) => f,
            Err(_) => return,
        };
        let file_uri = file.uri().as_str().to_string();
        sender.input(VideoPlayerMsg::NewVideo(file_uri));
    })
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
            adw::StatusPage {
                set_title: "Select Video",
                set_description: Some("select a video file to edit"),

                #[name = "open_file_btn"]
                gtk::Button {
                    set_label: "Open File",
                    set_hexpand: false,
                    add_css_class: "suggested-action",
                    add_css_class: "pill",
                },

                #[watch]
                set_visible: !model.video_is_selected,
            },

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
            playbin: None,
            gtk_sink,
            video_uri: None,
        };

        let widgets = view_output!();

        widgets.vid_frame.append(&offload);

        widgets.open_file_btn.connect_clicked(glib::clone!(@weak root =>  move |_| {
            launch_file_opener(sender.clone());
        }));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            VideoPlayerMsg::NewVideo(value) => {
                self.video_uri = Some(value);
                self.video_is_selected = true;
                self.play_new_video();
            }
            _ => panic!("Unknown message recived for video player")
        }
    }
}

impl VideoPlayerModel {
    fn play_new_video(&mut self) {
        let playbin = gst::ElementFactory::make("playbin")
            .name("playbin")
            .property("uri", self.video_uri.as_ref().unwrap())
            .build()
            .unwrap();

        playbin.set_property("video-sink", &self.gtk_sink);
        playbin.set_state(gst::State::Playing).unwrap();

        self.playbin = Some(playbin);
    }
}