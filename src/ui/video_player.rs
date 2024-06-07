use gst::prelude::*;
use gtk4::prelude::{OrientableExt, BoxExt};
use relm4::*;
use relm4::adw::gdk;

pub struct VideoPlayerModel {
    playbin: gst::Element,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    Play,
    Pause,
    Stop,
}

#[relm4::component(pub)]
impl SimpleComponent for VideoPlayerModel {
    type Input = VideoPlayerMsg;
    type Output = ();
    type Init = u8;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            inline_css: "margin: 20px",

            #[name = "vid_frame"]
            gtk::Box {
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

        let widgets = view_output!();

        let gtk_sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .unwrap();

        let playbin = gst::ElementFactory::make("playbin")
            .name("playbin")
            .property("uri", "file:///home/fareed/Videos/mp3e1.mkv")
            .build()
            .unwrap();

        playbin.set_property("video-sink", &gtk_sink);

        let paintable = gtk_sink.property::<gdk::Paintable>("paintable");
        let picture = gtk::Picture::new();

        picture.set_paintable(Some(&paintable));

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        widgets.vid_frame.append(&offload);

        playbin.set_state(gst::State::Playing).unwrap();
        playbin.set_state(gst::State::Paused).unwrap();

        let model = VideoPlayerModel {
            playbin,
        };

        ComponentParts { model, widgets }
    }
}