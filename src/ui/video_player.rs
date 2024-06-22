use std::sync::{Arc, Condvar, mpsc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Error;
use gst::{ClockTime, Element, element_error, SeekFlags};
use gst::glib::FlagsClass;
use gst::prelude::*;
use gst_video::VideoFrameExt;
use gtk4::gio;
use gtk4::prelude::{BoxExt, ButtonExt, EventControllerExt, GestureDragExt, OrientableExt, WidgetExt};
use relm4::*;
use relm4::adw::gdk;

// todo: dispose of stuff on quit

static THUMBNAIL_PATH: &str = "/home/fareed/Videos";
static NUM_THUMBNAILS: u64 = 12;
static THUMBNAIL_HEIGHT: u32 = 90;

// todo: do i need is_loaded and is_playing?
pub struct VideoPlayerModel {
    video_is_selected: bool,
    video_is_loaded: bool,
    is_playing: bool,
    is_mute: bool,
    thumbnails_available: bool,
    gtk_sink: Element,
    video_uri: Option<String>,
    playbin: Option<Element>,
}

#[derive(Debug)]
pub enum VideoPlayerMsg {
    TogglePlayPause,
    ToggleMute,
    SeekToPercent(f64),
    NewVideo(String),
    AddThumbnails,
    MoveStartTo(i32),
    MoveStartEnd,
    MoveEndTo(i32),
    MoveEndEnd,
    UpdateSeekBar(f64),
}

#[derive(Debug)]
pub enum VideoPlayerCommandMsg {
    VideoInit(bool),
    GenerateThumbnails,
    UpdateSeekPos,
}

#[relm4::component(pub)]
impl Component for VideoPlayerModel {
    type CommandOutput = VideoPlayerCommandMsg;
    type Input = VideoPlayerMsg;
    type Output = ();
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 670,
            set_height_request: 390,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            inline_css: "margin: 15px",

            gtk::Spinner {
                #[watch]
                set_spinning: !model.video_is_loaded,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
            },

            #[name = "vid_frame"]
            gtk::Box {
                #[watch]
                set_visible: model.video_is_loaded,
                set_orientation: gtk::Orientation::Vertical,

                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |_,_,_,_| {
                        sender.input(VideoPlayerMsg::TogglePlayPause)
                    }
                }
            },

            gtk::Box {
                #[watch]
                set_visible: model.video_is_loaded,
                set_spacing: 10,
                add_css_class: "toolbar",

                gtk::Button {
                    #[watch]
                    set_icon_name: if model.is_playing {
                        "pause"
                    } else {
                        "play"
                    },

                    connect_clicked[sender] => move |_| {
                            sender.input(VideoPlayerMsg::TogglePlayPause)
                    }
                },

                #[name = "overlay"]
                gtk::Overlay {
                    #[wrap(Some)]
                    set_child: timeline = &gtk::Box {
                        set_hexpand: true,
                        inline_css: "background-color: grey",
                        set_margin_start: 5,
                        set_margin_end: 5,

                        add_controller = gtk::GestureClick {
                            connect_pressed[sender] => move |click,_,x,_| {
                                let width = click.widget().width() as f64;
                                let percent = x / width;
                                sender.input(VideoPlayerMsg::SeekToPercent(percent));
                            }
                        },

                        add_controller = gtk::GestureDrag {
                            connect_drag_update[sender] => move |drag,x_offset,_| {
                                let (start_x, _) = drag.start_point().unwrap();
                                let width = drag.widget().width() as f64;
                                let percent_dragged = (start_x + x_offset) / width;

                                sender.input(VideoPlayerMsg::SeekToPercent(percent_dragged));
                            },
                        }
                    },

                    add_overlay: start_handle = &super::HandleWidget::default() {
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Center,

                        add_controller = gtk::GestureDrag {
                            connect_drag_update[sender] => move |drag,offset_x,_| {
                                let (start_x, _) = drag.start_point().unwrap();
                                let targ_x = (start_x + offset_x) as i32;
                                sender.input(VideoPlayerMsg::MoveStartTo(targ_x))
                            },

                            connect_drag_end[sender] => move |_, _,_| {
                                sender.input(VideoPlayerMsg::MoveStartEnd);
                            },
                        }
                    },

                    add_overlay: end_handle = &super::HandleWidget::default() {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Center,

                        add_controller = gtk::GestureDrag {
                            connect_drag_update[sender] => move |drag,offset_x,_| {
                                let (start_x, _) = drag.start_point().unwrap();
                                let targ_x = (start_x + offset_x) as i32;
                                sender.input(VideoPlayerMsg::MoveEndTo(targ_x))
                            },

                            connect_drag_end[sender] => move |_, _,_| {
                                sender.input(VideoPlayerMsg::MoveEndEnd);
                            },
                        }
                    },

                    add_overlay: seek_bar = &super::HandleWidget::new(0, false) {
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Center,
                    },
                },

                gtk::Button {
                    #[watch]
                     set_icon_name: if model.is_mute {
                        "audio-volume-muted"
                    } else {
                        "audio-volume-high"
                    },
                    connect_clicked[sender] => move |_| {
                            sender.input(VideoPlayerMsg::ToggleMute)
                    }
                },
            },
        }
    }

    type Init = u8;

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self>
    {
        gst::init().unwrap();

        let gtk_sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .unwrap();

        let paintable = gtk_sink.property::<gdk::Paintable>("paintable");
        let picture = gtk::Picture::new();

        picture.set_paintable(Some(&paintable));

        let offload = gtk4::GraphicsOffload::new(Some(&picture));
        offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);


        let model = VideoPlayerModel {
            video_is_selected: false,
            video_is_loaded: false,
            is_playing: false,
            is_mute: false,
            thumbnails_available: false,
            playbin: None,
            gtk_sink,
            video_uri: None,
        };

        let widgets = view_output!();

        widgets.vid_frame.append(&offload);

        ComponentParts { model, widgets }
    }

    fn update_with_view(&mut self, widgets: &mut Self::Widgets, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            VideoPlayerMsg::NewVideo(value) => {
                self.video_uri = Some(value);
                self.video_is_loaded = false;
                self.is_playing = false;
                self.video_is_selected = true;
                self.play_new_video();
                VideoPlayerModel::remove_timeline_thumbnails(&widgets.timeline);

                let playbin_clone = self.playbin.as_ref().unwrap().clone();
                sender.oneshot_command(async move {
                    VideoPlayerModel::wait_for_playbin_done(&playbin_clone);
                    VideoPlayerCommandMsg::VideoInit(true)
                });

                let uri = self.video_uri.as_ref().unwrap().clone();
                sender.oneshot_command(async move {
                    let thumbnail_pair = Arc::new((Mutex::new(false), Condvar::new()));

                    VideoPlayerModel::thumbnail_thread(uri, Arc::clone(&thumbnail_pair));

                    let (num_thumbs, all_done) = &*thumbnail_pair;
                    let mut thumbnails_done = num_thumbs.lock().unwrap();
                    while !*thumbnails_done {
                        thumbnails_done = all_done.wait(thumbnails_done).unwrap();
                    }
                    VideoPlayerCommandMsg::GenerateThumbnails
                });
            }
            VideoPlayerMsg::TogglePlayPause => self.video_toggle_play_pause(),
            VideoPlayerMsg::SeekToPercent(percent) => {
                let seek_bar_pos = (widgets.timeline.width() as f64 * percent) as i32;
                if seek_bar_pos != widgets.seek_bar.margin_start() {
                    widgets.seek_bar.set_margin_start(seek_bar_pos);
                    self.seek_to_percent(percent);
                }
            }
            VideoPlayerMsg::ToggleMute => self.toggle_mute(),
            VideoPlayerMsg::AddThumbnails => {
                let timeline = &widgets.timeline;
                VideoPlayerModel::populate_timeline(timeline);
            }
            VideoPlayerMsg::MoveStartTo(pos) => {
                // todo: make MoveStartTo and MoveEndTo generic as MoveHandleTo(isStart, pos)
                let end_pos = widgets.timeline.width() - widgets.end_handle.x();
                let target_pos = widgets.start_handle.x() + pos;
                let seek_percent = target_pos as f64 / widgets.timeline.width() as f64;

                if end_pos > target_pos {
                    if target_pos >= 0 {
                        widgets.start_handle.set_rel_x(pos);
                        widgets.start_handle.queue_draw();
                        sender.input(VideoPlayerMsg::SeekToPercent(seek_percent));
                    } else if (target_pos < 0) && (widgets.start_handle.rel_x() != -widgets.start_handle.x()) {
                        widgets.start_handle.set_rel_x(-widgets.start_handle.x());
                        widgets.start_handle.queue_draw();
                        sender.input(VideoPlayerMsg::SeekToPercent(seek_percent));
                    }
                }
            }
            VideoPlayerMsg::MoveEndTo(pos) => {
                let target_instep = -widgets.end_handle.x() + pos;
                let target_pos = widgets.timeline.width() + target_instep;
                let seek_percent = target_pos as f64 / widgets.timeline.width() as f64;

                if target_pos > widgets.start_handle.x() {
                    if target_instep <= 0 {
                        widgets.end_handle.set_rel_x(pos);
                        widgets.end_handle.queue_draw();
                        sender.input(VideoPlayerMsg::SeekToPercent(seek_percent));
                    } else if (target_instep > 0) && (widgets.end_handle.rel_x() != widgets.end_handle.x()) {
                        widgets.end_handle.set_rel_x(widgets.end_handle.x());
                        widgets.end_handle.queue_draw();
                        sender.input(VideoPlayerMsg::SeekToPercent(seek_percent));
                    }
                }
            }
            VideoPlayerMsg::MoveStartEnd => {
                let curr_margin = widgets.start_handle.x();
                let new_margin = curr_margin + widgets.start_handle.rel_x();

                widgets.start_handle.set_x(new_margin);
                widgets.start_handle.set_margin_start(new_margin);
                widgets.start_handle.set_rel_x(0);
            }
            VideoPlayerMsg::MoveEndEnd => {
                let curr_margin = widgets.end_handle.x();
                let new_margin = (-curr_margin + widgets.end_handle.rel_x()).abs();

                widgets.end_handle.set_x(new_margin);
                widgets.end_handle.set_margin_end(new_margin);
                widgets.end_handle.set_rel_x(0);
            }
            VideoPlayerMsg::UpdateSeekBar(percent) => {
                let target_bar_pos = (widgets.timeline.width() as f64 * percent) as i32;
                if target_bar_pos != widgets.seek_bar.margin_start() {
                    widgets.seek_bar.set_margin_start(target_bar_pos);
                }
            }
        }

        self.update_view(widgets, sender);
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            VideoPlayerCommandMsg::VideoInit(_) => {
                self.is_playing = true;
                self.video_is_loaded = true;

                let playbin_clone = self.playbin.as_ref().unwrap().clone();
                sender.command(|out, shutdown| {
                    shutdown.register(async move {
                        loop {
                            // todo: determine good wait time to make smooth
                            // todo: test more with video switches
                            tokio::time::sleep(Duration::from_millis(30)).await;
                            if playbin_clone.state(Some(ClockTime::ZERO)).1 == gst::State::Playing {
                                out.send(VideoPlayerCommandMsg::UpdateSeekPos).unwrap();
                            }
                        }
                    }).drop_on_shutdown()
                })
            }
            VideoPlayerCommandMsg::GenerateThumbnails => {
                self.thumbnails_available = true;
                sender.input(VideoPlayerMsg::AddThumbnails);
            }
            VideoPlayerCommandMsg::UpdateSeekPos => {
                // fixme: on a lot of drags query_position failed. find way to reproduce better
                let duration = self.playbin.as_ref().unwrap().query_duration::<ClockTime>().unwrap();
                let pos = self.playbin.as_ref().unwrap().query_position::<ClockTime>().unwrap();
                let percent = pos.mseconds() as f64 / duration.mseconds() as f64;
                sender.input(VideoPlayerMsg::UpdateSeekBar(percent));
            }
        }
    }
}

impl VideoPlayerModel {
    // todo: hookup with ui/keyboard. add support for stepping backwards
    fn step_next_frame(&mut self) {
        if let Some(video_sink) = self.playbin.as_ref().unwrap().property::<Option<Element>>("video-sink") {
            let step = gst::event::Step::new(gst::format::Buffers::ONE, 1.0, true, false);
            video_sink.send_event(step);
        }
    }

    // todo: cleanup arguments needed
    fn create_thumbnail_pipeline(
        got_current_thumb: Arc<Mutex<bool>>,
        current_thumb_num: Arc<Mutex<u64>>,
        video_uri: String,
        senders: mpsc::Sender<u8>,
        thumbnails_done: Arc<(Mutex<bool>, Condvar)>) -> Result<gst::Pipeline, Error>
    {
        let pipeline = gst::parse::launch(&format!(
            "uridecodebin uri={video_uri} ! videoconvert ! appsink name=sink"
        )).unwrap()
            .downcast::<gst::Pipeline>()
            .expect("Expected a gst::pipeline");

        let appsink = pipeline
            .by_name("sink")
            .expect("sink element not found")
            .downcast::<gst_app::AppSink>()
            .expect("Sink element is expected to be appsink!");

        appsink.set_property("sync", false);

        appsink.set_caps(Some(
            &gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgbx)
                .build(),
        ));

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let mut got_current = got_current_thumb.lock().unwrap();

                    if *got_current {
                        return Err(gst::FlowError::Eos);
                    }

                    *got_current = true;
                    let current_thumb_num = Arc::clone(&current_thumb_num);
                    let thumbnails_done = Arc::clone(&thumbnails_done);

                    let appsink = appsink.clone();
                    thread::spawn(move || {
                        let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Error).unwrap();
                        let buffer = sample.buffer().ok_or_else(|| {
                            element_error!(appsink, gst::ResourceError::Failed, ("Failed"));
                            gst::FlowError::Error
                        }).unwrap();

                        let caps = sample.caps().expect("sample without caps");
                        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

                        let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                            .map_err(|_| {
                                element_error!(appsink, gst::ResourceError::Failed, ("Failed to map buff readable"));
                                gst::FlowError::Error
                            }).unwrap();

                        let aspect_ratio = (frame.width() as f64 * info.par().numer() as f64)
                            / (frame.height() as f64 * info.par().denom() as f64);
                        let target_height = THUMBNAIL_HEIGHT;
                        let target_width = target_height as f64 * aspect_ratio;

                        let img = image::FlatSamples::<&[u8]> {
                            samples: frame.plane_data(0).unwrap(),
                            layout: image::flat::SampleLayout {
                                channels: 3,
                                channel_stride: 1,
                                width: frame.width(),
                                width_stride: 4,
                                height: frame.height(),
                                height_stride: frame.plane_stride()[0] as usize,
                            },
                            color_hint: Some(image::ColorType::Rgb8),
                        };

                        let scaled_img = image::imageops::thumbnail(
                            &img.as_view::<image::Rgb<u8>>().expect("could not create image view"),
                            target_width as u32,
                            target_height,
                        );
                        let mut thumb_num = current_thumb_num.lock().unwrap();
                        let thumbnail_save_path = std::path::PathBuf::from(
                            format!("/{}/thumbnail_{}.jpg", THUMBNAIL_PATH, *thumb_num)
                        );

                        scaled_img.save(&thumbnail_save_path).map_err(|err| {
                            element_error!(appsink, gst::ResourceError::Write,
                            (
                                "Failed to write a preview file {}: {}",
                                &thumbnail_save_path.display(), err
                            ));
                            gst::FlowError::Error
                        }).unwrap();
                        *thumb_num += 1;

                        let (thumbs_done_lock, thumbs_done_cvar) = &*thumbnails_done;
                        let mut done_thumbnails = thumbs_done_lock.lock().unwrap();
                        if *thumb_num == NUM_THUMBNAILS {
                            *done_thumbnails = true;
                            thumbs_done_cvar.notify_one();
                        }
                    });

                    senders.send(0).unwrap();
                    Err(gst::FlowError::Eos)
                })
                .build()
        );

        Ok(pipeline)
    }

    fn seek_to_percent(&mut self, percent: f64) {
        if self.playbin.is_none() || !self.video_is_loaded {
            println!("early exit for seek");
            return;
        }

        let duration = self.playbin.as_ref().unwrap().query_duration::<ClockTime>().unwrap();
        let seconds = (duration.seconds() as f64 * percent) as u64;

        let time = gst::GenericFormattedValue::from(ClockTime::from_seconds(seconds));
        let seek = gst::event::Seek::new(
            1.0,
            SeekFlags::FLUSH | SeekFlags::KEY_UNIT,
            gst::SeekType::Set,
            time,
            gst::SeekType::End,
            ClockTime::ZERO);

        self.playbin.as_ref().unwrap().send_event(seek);
    }

    fn toggle_mute(&mut self) {
        self.is_mute = !self.is_mute;
        self.playbin.as_ref().unwrap().set_property("mute", self.is_mute);
    }

    fn video_toggle_play_pause(&mut self) {
        let (new_state, playbin_new_state) = if self.is_playing {
            (false, gst::State::Paused)
        } else {
            (true, gst::State::Playing)
        };

        self.is_playing = new_state;
        self.playbin.as_ref().unwrap().set_state(playbin_new_state).unwrap();
    }

    fn remove_timeline_thumbnails(timeline: &gtk::Box) {
        if timeline.first_child().is_some() {
            for _ in 0..NUM_THUMBNAILS {
                let child = timeline.first_child().unwrap();
                timeline.remove(&child);
            }
        }
    }

    fn populate_timeline(timeline: &gtk::Box) {
        // todo: see if can reuse picture widget instead of discarding. without storing ref to all of them
        // todo: try and cache thumbnails of 10 videos?
        Self::remove_timeline_thumbnails(timeline);

        for i in 0..NUM_THUMBNAILS {
            let file = gio::File::for_parse_name(format!("{}/thumbnail_{}.jpg", THUMBNAIL_PATH, i).as_str());
            let image = gtk::Picture::for_file(&file);

            image.set_hexpand(true);
            image.set_valign(gtk::Align::Fill);
            image.set_halign(gtk::Align::Fill);
            timeline.append(&image);
        }
    }

    // fixme: speed up
    // pipeline ready in ~1300ms, ~900ms for all thumbnail after
    // try to reuse existing pipeline
    fn thumbnail_thread(video_uri: String, thumbnails_done: Arc<(Mutex<bool>, Condvar)>) {
        let uri = video_uri.clone();

        thread::spawn(move || {
            let got_current_thumb = Arc::new(Mutex::new(false));
            let current_thumb_num = Arc::new(Mutex::new(0));
            let (senders, receiver) = mpsc::channel();

            let pipeline = VideoPlayerModel::create_thumbnail_pipeline(
                Arc::clone(&got_current_thumb),
                Arc::clone(&current_thumb_num),
                uri,
                senders.clone(),
                Arc::clone(&thumbnails_done),
            ).expect("could not create thumbnail pipeline");

            pipeline.set_state(gst::State::Paused).unwrap();
            let bus = pipeline.bus().expect("Pipeline without a bus.");

            for msg in bus.iter_timed(ClockTime::NONE) {
                use gst::MessageView;

                match msg.view() {
                    MessageView::AsyncDone(..) => {
                        break;
                    }
                    _ => ()
                }
            }

            let duration = pipeline.query_duration::<ClockTime>().unwrap();
            let step = duration.mseconds() / (NUM_THUMBNAILS + 2); // + 2 so first and last frame not chosen

            for i in 0..NUM_THUMBNAILS {
                let timestamp = gst::GenericFormattedValue::from(ClockTime::from_mseconds(step + (step * i)));
                if pipeline.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, timestamp).is_err()
                {
                    println!("Failed to seek");
                }
                pipeline.set_state(gst::State::Playing).unwrap();
                receiver.recv().unwrap();
                pipeline.set_state(gst::State::Paused).unwrap();

                let mut gen_new = got_current_thumb.lock().unwrap();
                *gen_new = false;
            }
        });
    }

    fn wait_for_playbin_done(playbin: &Element) {
        let bus = playbin.bus().unwrap();

        for msg in bus.iter_timed(ClockTime::NONE) {
            use gst::MessageView;

            match msg.view() {
                MessageView::AsyncDone(..) => {
                    break;
                }
                _ => ()
            }
        }
    }

    fn play_new_video(&mut self) {
        if self.playbin.is_some() {
            self.playbin.as_ref().unwrap().set_state(gst::State::Null).unwrap();
            self.playbin.as_ref().unwrap().set_property("uri", self.video_uri.as_ref().unwrap());
        } else {
            let playbin = gst::ElementFactory::make("playbin")
                .name("playbin")
                .property("uri", self.video_uri.as_ref().unwrap())
                .property("video-sink", &self.gtk_sink)
                .build()
                .unwrap();

            let flags = playbin.property_value("flags");
            let flags_class = FlagsClass::with_type(flags.type_()).unwrap();

            let flags = flags_class
                .builder_with_value(flags)
                .unwrap()
                .set_by_nick("audio")
                .set_by_nick("video")
                .unset_by_nick("text")
                .build()
                .unwrap();
            playbin.set_property_from_value("flags", &flags);

            self.playbin = Some(playbin);
        }

        self.playbin.as_ref().unwrap().set_property("mute", false);
        self.playbin.as_ref().unwrap().set_state(gst::State::Playing).unwrap();

        self.is_mute = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_create() {
        gst::init().unwrap();

        let uri = "file:///home/fareed/Videos/mp3e1.mkv";
        let thumbnail_pair = Arc::new((Mutex::new(false), Condvar::new()));

        VideoPlayerModel::thumbnail_thread(uri.parse().unwrap(), Arc::clone(&thumbnail_pair));

        let (num_thumbs, all_done) = &*thumbnail_pair;
        let mut thumbnails_done = num_thumbs.lock().unwrap();
        while !*thumbnails_done {
            thumbnails_done = all_done.wait(thumbnails_done).unwrap();
        }

        assert_eq!(true, true);
    }
}
