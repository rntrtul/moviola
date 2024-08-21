use relm4::RelmApp;
use crate::app::App;

mod app;
mod preview;

fn main() {
    let app = RelmApp::new("relm4.test.moviola");
    app.run::<App>(0);
}
