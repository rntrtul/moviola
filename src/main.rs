use relm4::RelmApp;
use crate::app::App;

mod app;
mod ui;

fn main() {
    let app = RelmApp::new("relm4.test.videditor");
    app.run::<App>(0);
}
