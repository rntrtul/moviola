use relm4::{adw, gtk, main_application, ComponentParts, ComponentSender, SimpleComponent};

use gtk::prelude::{
    ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt, BoxExt,
};
use gtk::{glib};
use gst::prelude::*;
use relm4::adw::gdk;
use relm4::adw::gdk::pango::ffi::pango_read_line;
use crate::preview::Preview;

pub(super) struct App {}

#[derive(Debug)]
pub(super) enum AppMsg {
    Quit,
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = u8;
    type Input = AppMsg;
    type Output = ();
    type Widgets = AppWidgets;

    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,
            set_default_height: 360,
            set_default_width: 640,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },


            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                adw::HeaderBar {
                    pack_end = &gtk::MenuButton {
                        set_icon_name: "open-menu-symbolic",
                    }
                },

                #[name = "pic_frame"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};
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

        let preview = Preview::new();
        preview.set_paintable(paintable);
        // picture.set_paintable(Some(&paintable));

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        // widgets.pic_frame.append(&offload);
        widgets.pic_frame.append(&preview);

        playbin.set_state(gst::State::Playing).unwrap();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => main_application().quit(),
        }
    }
}
