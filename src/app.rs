use gtk::glib;
use gtk::prelude::{ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt};
use gtk4::gio;
use gtk4::prelude::{ButtonExt, FileExt, GtkApplicationExt};
use relm4::{
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, SimpleComponent,
};

use super::ui::edit_controls::{EditControlsModel, EditControlsOutput};
use super::ui::video_player::{VideoPlayerModel, VideoPlayerMsg};

pub(super) struct App {
    video_player: Controller<VideoPlayerModel>,
    edit_controls: Controller<EditControlsModel>,
    video_is_open: bool,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    ExportFrame,
    OpenFile,
    SetVideo(String),
    Quit,
}

impl App {
    fn launch_file_opener(sender: ComponentSender<Self>) {
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

        file_dialog.open(Some(&window), Some(&cancelable), move |result| {
            let file = match result {
                Ok(f) => f,
                Err(_) => return,
            };
            sender.input(AppMsg::SetVideo(file.uri().to_string()));
        });
    }
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Input = AppMsg;
    type Output = ();
    type Init = u8;
    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,
            set_default_height: 480,
            set_default_width: 640,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            #[name="tool_bar_view"]
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                     pack_end = &gtk::Button {
                        set_icon_name: "document-open-symbolic",
                        #[watch]
                        set_visible: model.video_is_open,
                        connect_clicked => AppMsg::OpenFile,
                    }
                },

                #[name = "content"]
                gtk::Box{
                    set_orientation: gtk::Orientation::Vertical,

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
                        set_visible: !model.video_is_open,
                    },

                    model.video_player.widget(){
                        set_visible: model.video_is_open,
                    },

                    #[name="stack"]
                    adw::ViewStack {
                        add_titled_with_icon: (model.edit_controls.widget(), Some("Edit"), "Edit", "cut"),
                        add_titled: (&adw::StatusPage::builder().title("FFF").description("HERE").build(), Some("Convert"), "Convert"),
                    },
                },

                #[name="switch_bar"]
                add_bottom_bar = &adw::ViewSwitcherBar{},
            }

        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let video_player: Controller<VideoPlayerModel> =
            VideoPlayerModel::builder().launch(2).detach();

        let edit_controls: Controller<EditControlsModel> = EditControlsModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                EditControlsOutput::ExportFrame => AppMsg::ExportFrame,
            });

        let model = Self {
            video_player,
            edit_controls,
            video_is_open: false,
        };

        let widgets = view_output!();

        widgets.switch_bar.set_reveal(true);
        widgets.switch_bar.set_stack(Some(&widgets.stack));
        widgets.tool_bar_view.set_content(Some(&widgets.content));

        widgets.open_file_btn.connect_clicked(move |_| {
            sender.input(AppMsg::OpenFile);
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => {
                println!("QUIT");
                main_application().quit()
            }
            AppMsg::OpenFile => App::launch_file_opener(_sender),
            AppMsg::SetVideo(file_name) => {
                self.video_player.emit(VideoPlayerMsg::NewVideo(file_name));
                self.video_player.widget().set_visible(true);
                self.video_is_open = true;
            }
            AppMsg::ExportFrame => {
                self.video_player.emit(VideoPlayerMsg::ExportFrame);
            }
        }
    }
}
