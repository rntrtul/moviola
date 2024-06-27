use std::rc::Rc;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

use anyhow::Error;
use gst::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, ObjectExt};
use gst::{element_error, ClockTime, SeekFlags};
use gst_video::VideoFrameExt;
use gtk4::gio;
use gtk4::prelude::{BoxExt, EventControllerExt, GestureDragExt, WidgetExt};
use relm4::*;
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::ui::handle_manager::HandleManager;
use crate::ui::video_player::VideoPlayerModel;

static THUMBNAIL_PATH: &str = "/home/fareed/Videos";
static NUM_THUMBNAILS: u64 = 12;
static THUMBNAIL_HEIGHT: u32 = 90;

#[derive(Debug)]
pub struct TimelineModel {
    thumbnails_available: bool,
    handle_manager: Option<HandleManager>,
}

#[derive(Debug)]
pub enum TimelineMsg {
    GenerateThumnails(String),
    PopulateTimeline,
    MoveStartTo(i32),
    MoveStartEnd,
    MoveEndTo(i32),
    MoveEndEnd,
    UpdateSeekBarPos(f64),
    SeekToPercent(f64),
}

#[derive(Debug)]
pub enum TimelineCmdMsg {
    ThumbnailsGenerated,
}

#[derive(Debug)]
pub enum TimelineOutput {
    SeekToPercent(f64),
}

#[relm4::component(pub)]
impl Component for TimelineModel {
    type CommandOutput = TimelineCmdMsg;
    type Input = TimelineMsg;
    type Output = TimelineOutput;
    type Init = ();

    view! {
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
                        sender.input(TimelineMsg::SeekToPercent(percent));
                    }
                },

                add_controller = gtk::GestureDrag {
                    connect_drag_update[sender] => move |drag,x_offset,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let width = drag.widget().width() as f64;
                        let percent_dragged = (start_x + x_offset) / width;

                        sender.input(TimelineMsg::SeekToPercent(percent_dragged));
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
                        sender.input(TimelineMsg::MoveStartTo(targ_x))
                    },

                    connect_drag_end[sender] => move |_, _,_| {
                        sender.input(TimelineMsg::MoveStartEnd);
                    },
                }
            },

            add_overlay: end_handle = &super::HandleWidget::new(0, true, false) {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,

                add_controller = gtk::GestureDrag {
                    connect_drag_update[sender] => move |drag,offset_x,_| {
                        let (start_x, _) = drag.start_point().unwrap();
                        let targ_x = (start_x + offset_x) as i32;
                        sender.input(TimelineMsg::MoveEndTo(targ_x))
                    },

                    connect_drag_end[sender] => move |_, _,_| {
                        sender.input(TimelineMsg::MoveEndEnd);
                    },
                }
            },

            add_overlay: seek_bar = &super::HandleWidget::new(0, false, false) {
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Center,
            },
        },
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = TimelineModel {
            thumbnails_available: false,
            handle_manager: None,
        };

        let widgets = view_output!();

        let handle_manager = HandleManager {
            start_handle: Rc::new(widgets.start_handle.clone()),
            end_handle: Rc::new(widgets.end_handle.clone()),
        };

        model.handle_manager = Some(handle_manager);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            TimelineMsg::GenerateThumnails(uri) => {
                Self::remove_timeline_thumbnails(&widgets.timeline);
                self.thumbnails_available = false;

                sender.oneshot_command(async move {
                    let thumbnail_pair = Arc::new((Mutex::new(0), Condvar::new()));
                    TimelineModel::thumbnail_thread(uri, Arc::clone(&thumbnail_pair));
                    let (num_thumbs, all_done) = &*thumbnail_pair;
                    let mut thumbnails_done = num_thumbs.lock().unwrap();

                    while *thumbnails_done != NUM_THUMBNAILS {
                        thumbnails_done = all_done.wait(thumbnails_done).unwrap();
                    }
                    TimelineCmdMsg::ThumbnailsGenerated
                });
            }
            TimelineMsg::PopulateTimeline => {
                let timeline = &widgets.timeline;
                TimelineModel::populate_timeline(timeline);
            }
            TimelineMsg::SeekToPercent(percent) => {
                let seek_bar_pos = (widgets.timeline.width() as f64 * percent) as i32;
                if seek_bar_pos != widgets.seek_bar.margin_start() {
                    widgets.seek_bar.set_margin_start(seek_bar_pos);
                    sender
                        .output(TimelineOutput::SeekToPercent(percent))
                        .unwrap();
                }
            }
            TimelineMsg::UpdateSeekBarPos(percent) => {
                let target_bar_pos = (widgets.timeline.width() as f64 * percent) as i32;
                if target_bar_pos != widgets.seek_bar.margin_start() {
                    widgets.seek_bar.set_margin_start(target_bar_pos);
                }
            }
            TimelineMsg::MoveStartTo(pos) => {
                if self
                    .handle_manager
                    .as_ref()
                    .unwrap()
                    .try_set_start_rel_x(pos, widgets.timeline.width())
                {
                    let seek_percent =
                        widgets.start_handle.target_x() as f64 / widgets.timeline.width() as f64;
                    sender.input(TimelineMsg::SeekToPercent(seek_percent));
                }
            }
            TimelineMsg::MoveEndTo(pos) => {
                if self
                    .handle_manager
                    .as_ref()
                    .unwrap()
                    .try_set_end_rel_x(pos, widgets.timeline.width())
                {
                    let seek_percent =
                        widgets.end_handle.target_x() as f64 / widgets.timeline.width() as f64;
                    sender.input(TimelineMsg::SeekToPercent(seek_percent));
                }
            }
            TimelineMsg::MoveStartEnd => self.handle_manager.as_ref().unwrap().set_start_margin(),
            TimelineMsg::MoveEndEnd => self.handle_manager.as_ref().unwrap().set_end_margin(),
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            TimelineCmdMsg::ThumbnailsGenerated => {
                self.thumbnails_available = true;
                sender.input(TimelineMsg::PopulateTimeline);
            }
        }
    }
}

impl TimelineModel {
    fn create_thumbnail_pipeline(
        got_current_thumb: Arc<Mutex<bool>>,
        video_uri: String,
        senders: mpsc::Sender<u8>,
        thumbnails_done: Arc<(Mutex<u64>, Condvar)>,
    ) -> Result<gst::Pipeline, Error> {
        let pipeline = gst::parse::launch(&format!(
            "uridecodebin uri={video_uri} ! videoconvert ! appsink name=sink"
        ))
        .unwrap()
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
                    let thumbnails_done = Arc::clone(&thumbnails_done);

                    let appsink = appsink.clone();
                    thread::spawn(move || {
                        let sample = appsink
                            .pull_sample()
                            .map_err(|_| gst::FlowError::Error)
                            .unwrap();
                        let buffer = sample
                            .buffer()
                            .ok_or_else(|| {
                                element_error!(appsink, gst::ResourceError::Failed, ("Failed"));
                                gst::FlowError::Error
                            })
                            .unwrap();

                        let caps = sample.caps().expect("sample without caps");
                        let info =
                            gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

                        let frame =
                            gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
                                .map_err(|_| {
                                    element_error!(
                                        appsink,
                                        gst::ResourceError::Failed,
                                        ("Failed to map buff readable")
                                    );
                                    gst::FlowError::Error
                                })
                                .unwrap();

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
                            &img.as_view::<image::Rgb<u8>>()
                                .expect("could not create image view"),
                            target_width as u32,
                            target_height,
                        );
                        let (thumbs_num_lock, thumbs_done_cvar) = &*thumbnails_done;
                        let mut thumb_num = thumbs_num_lock.lock().unwrap();

                        let thumbnail_save_path = std::path::PathBuf::from(format!(
                            "/{}/thumbnail_{}.jpg",
                            THUMBNAIL_PATH, *thumb_num
                        ));

                        scaled_img
                            .save(&thumbnail_save_path)
                            .map_err(|err| {
                                element_error!(
                                    appsink,
                                    gst::ResourceError::Write,
                                    (
                                        "Failed to write a preview file {}: {}",
                                        &thumbnail_save_path.display(),
                                        err
                                    )
                                );
                                gst::FlowError::Error
                            })
                            .unwrap();
                        *thumb_num += 1;

                        if *thumb_num == NUM_THUMBNAILS {
                            thumbs_done_cvar.notify_one();
                        }
                    });

                    senders.send(0).unwrap();
                    Err(gst::FlowError::Eos)
                })
                .build(),
        );
        Ok(pipeline)
    }

    // fixme: speed up
    // try to reuse existing pipeline or thumbnail pipeline. would be ~1.3 sec quicker for
    // subsequent videos
    fn thumbnail_thread(video_uri: String, thumbnails_done: Arc<(Mutex<u64>, Condvar)>) {
        let uri = video_uri.clone();

        // todo: figure way to return pipeline or use static pipeline to dispose of or null this pipeline
        thread::spawn(move || {
            let got_current_thumb = Arc::new(Mutex::new(false));
            let (senders, receiver) = mpsc::channel();

            let pipeline = TimelineModel::create_thumbnail_pipeline(
                Arc::clone(&got_current_thumb),
                uri,
                senders.clone(),
                Arc::clone(&thumbnails_done),
            )
            .expect("could not create thumbnail pipeline");

            pipeline.set_state(gst::State::Paused).unwrap();

            let pipe_clone = pipeline.clone();
            VideoPlayerModel::wait_for_playbin_done(&gst::Element::from(pipe_clone));

            let duration = pipeline.query_duration::<ClockTime>().unwrap();
            let step = duration.mseconds() / (NUM_THUMBNAILS + 2); // + 2 so first and last frame not chosen

            for i in 0..NUM_THUMBNAILS {
                let timestamp =
                    gst::GenericFormattedValue::from(ClockTime::from_mseconds(step + (step * i)));
                if pipeline
                    .seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, timestamp)
                    .is_err()
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
        Self::remove_timeline_thumbnails(timeline);

        for i in 0..NUM_THUMBNAILS {
            let file = gio::File::for_parse_name(
                format!("{}/thumbnail_{}.jpg", THUMBNAIL_PATH, i).as_str(),
            );
            let image = gtk::Picture::for_file(&file);

            image.set_hexpand(true);
            image.set_valign(gtk::Align::Fill);
            image.set_halign(gtk::Align::Fill);
            timeline.append(&image);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb_create() {
        gst::init().unwrap();

        let uri = "file:///home/fareed/Videos/mp3e1.mkv";
        let thumbnail_pair = Arc::new((Mutex::new(0), Condvar::new()));

        TimelineModel::thumbnail_thread(uri.parse().unwrap(), Arc::clone(&thumbnail_pair));

        let (num_thumbs, all_done) = &*thumbnail_pair;
        let mut thumbnails_done = num_thumbs.lock().unwrap();
        while *thumbnails_done != NUM_THUMBNAILS {
            thumbnails_done = all_done.wait(thumbnails_done).unwrap();
        }

        assert_eq!(true, true);
    }
}
