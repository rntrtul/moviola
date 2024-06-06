use relm4::RelmApp;
use crate::app::App;
use relm4_icons::icon_names;

mod app;
mod ui;

fn main() {
    relm4_icons::initialize_icons();
    let app = RelmApp::new("relm4.test.videditor");
    app.run::<App>(0);
}
