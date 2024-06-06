use relm4::{adw, gtk, main_application, ComponentParts, ComponentSender, SimpleComponent};

use gtk::prelude::{
    ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt, BoxExt,
};
use gtk::{glib};
use gst::prelude::*;
use relm4::adw::gdk;

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
    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,
            set_default_height: 480,
            set_default_width: 640,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            #[name="ToolBarview"]
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                #[name = "content"]
                gtk::Box{
                    set_orientation: gtk::Orientation::Vertical,

                    #[name = "pic_frame"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                    },

                    #[name="stack"]
                    adw::ViewStack {
                        add_titled: (&adw::StatusPage::builder()
                            .title("TTT")
                            .description("noVideoHEre")
                            .build(), Some("TTT"), "t"),
                        
                        add_titled: (&adw::StatusPage::builder().title("FFF").description("HERE").build(), Some("FFF"), "f"),
                    },
                },

                #[name="switchBar"]
                add_bottom_bar = &adw::ViewSwitcherBar{},
            }

        }
    }

    type Widgets = AppWidgets;

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

        picture.set_paintable(Some(&paintable));

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
        widgets.pic_frame.append(&offload);

        playbin.set_state(gst::State::Playing).unwrap();

        widgets.switchBar.set_reveal(true);
        widgets.switchBar.set_stack(Some(&widgets.stack));
        widgets.ToolBarview.set_content(Some(&widgets.content));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => main_application().quit(),
        }
    }
}
