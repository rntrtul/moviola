use relm4::{RELM_THREADS, RelmApp};

use crate::app::App;

mod app;
mod ui;

fn main() {
    RELM_THREADS.set(2).unwrap();
    relm4_icons::initialize_icons();
    let app = RelmApp::new("relm4.test.videditor");
    app.run::<App>(0);
}
