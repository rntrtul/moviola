use relm4::{adw, gtk, main_application, ComponentParts, ComponentSender, SimpleComponent, Controller, Component, ComponentController};

use gtk::prelude::{
    ApplicationExt, GtkWindowExt, OrientableExt, WidgetExt};
use gtk::{glib};
use super::ui::video_player::VideoPlayerModel;

pub(super) struct App {
    video_player: Controller<VideoPlayerModel>,
}

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

                    model.video_player.widget(),

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
        let video_player: Controller<VideoPlayerModel> =
            VideoPlayerModel::builder()
                .launch(2)
                .detach();

        let model = Self { video_player };

        let widgets = view_output!();

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
