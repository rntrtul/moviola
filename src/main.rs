use gtk4::gio;
use relm4::{adw, gtk, RelmApp, RELM_THREADS};

use crate::app::App;

use self::config::RESOURCES_FILE;

mod app;
mod config;
mod renderer;
mod ui;
mod video;

use clap::Parser;
use url::Url;

#[derive(Parser)]
#[clap(about, version, author, long_about = None)]
struct Cli {
    #[arg(short, long)]
    file_path: Option<std::path::PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    env_logger::init();
    gst::init().unwrap();
    gtk::init().unwrap();

    let res = gio::Resource::load(RESOURCES_FILE).unwrap();
    gio::resources_register(&res);

    let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    theme.add_resource_path("/org/fareedsh/Moviola/icons");

    RELM_THREADS.set(2).unwrap();
    let style_manger = adw::StyleManager::default();
    style_manger.set_color_scheme(adw::ColorScheme::ForceDark);

    let uri = if let Some(path) = cli.file_path {
        let uri = Url::from_file_path(path.canonicalize().unwrap())
            .unwrap()
            .to_string();
        Some(uri)
    } else {
        None
    };

    let app = RelmApp::new("org.fareedsh.Moviola").with_args(vec![]);
    app.run::<App>(uri);
}
