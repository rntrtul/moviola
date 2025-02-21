use crate::app::{App, AppMsg};
use crate::renderer::renderer::RenderedFrame;
use crate::renderer::{FrameSize, RenderCmd, TimerCmd};
use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::metadata::VideoInfo;
use crate::video::player::{video_appsink, AppSinkUsage, Player};
use gst::prelude::{
    Cast, ElementExt, ElementExtManual, GstBinExt, GstBinExtManual, GstObjectExt, ObjectExt,
    PadExt, PadExtManual,
};
use gst::ClockTime;
use gst_app::{AppSink, AppSrc};
use gst_pbutils::prelude::{EncodingProfileBuilder, EncodingProfileExt};
use gst_pbutils::EncodingContainerProfile;
use gst_video::VideoFrameExt;
use relm4::gtk::prelude::{TextureExt, TextureExtManual};
use relm4::ComponentSender;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

#[derive(Debug)]
pub struct TimelineExportSettings {
    pub start: ClockTime,
    pub end: ClockTime,
}

impl TimelineExportSettings {
    pub fn duration(&self) -> ClockTime {
        self.end - self.start
    }
}

impl Player {
    pub fn export_video(
        &mut self,
        source_uri: String,
        save_uri: String,
        timeline_settings: TimelineExportSettings,
        controls_export_settings: ControlsExportSettings,
        output_size: FrameSize,
        frame_receiver: mpsc::Receiver<RenderedFrame>,
        app_sender: ComponentSender<App>,
        sample_sender: mpsc::Sender<RenderCmd>,
        timer_sender: mpsc::Sender<TimerCmd>,
    ) {
        self.reset_pipeline();

        let video_app_sink = video_appsink(
            app_sender.clone(),
            sample_sender,
            timer_sender,
            AppSinkUsage::Export,
        );

        let pipeline = export_video(
            source_uri,
            save_uri,
            self.info.clone(),
            output_size,
            controls_export_settings,
            timeline_settings,
            video_app_sink,
            frame_receiver,
        );

        thread::spawn(move || {
            let now = SystemTime::now();
            let bus = pipeline.bus().unwrap();

            for msg in bus.iter_timed(ClockTime::NONE) {
                use gst::MessageView;

                match msg.view() {
                    MessageView::Eos(..) => {
                        println!("Done? in {:?}", now.elapsed());
                        break;
                    }
                    MessageView::Error(err) => {
                        println!(
                            "Error from {:?}: {} ({:?})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        );
                        break;
                    }
                    _ => (),
                }
            }
            pipeline.set_state(gst::State::Null).unwrap();
            app_sender.input(AppMsg::ExportDone);
        });
    }
}

fn build_container_profile(
    info: &VideoInfo,
    container: OutputContainerSettings,
) -> EncodingContainerProfile {
    let container_caps = container.container.caps_builder().build();
    let video_caps = container.video_codec.caps_builder().build();

    let video_profile = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
        .name("video_profile")
        .build();
    let container_builder = EncodingContainerProfile::builder(&container_caps)
        .name("Container")
        .add_profile(video_profile);

    if container.no_audio {
        container_builder.build()
    } else {
        let audio_stream = &info.container_info.audio_streams[container.audio_stream_idx as usize];

        let audio_caps = audio_stream.codec.caps_builder().build();
        let audio_profile = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
            .name("audio_profile")
            .build();

        container_builder.add_profile(audio_profile).build()
    }
}

fn export_video(
    source_uri: String,
    save_uri: String,
    info: VideoInfo,
    output_size: FrameSize,
    encoding_settings: ControlsExportSettings,
    timeline_settings: TimelineExportSettings,
    video_appsink: AppSink,
    frame_receiver: mpsc::Receiver<RenderedFrame>,
) -> gst::Pipeline {
    let gst_video_info = gst_video::VideoInfo::builder(
        gst_video::VideoFormat::Rgba,
        output_size.width,
        output_size.height,
    )
    .fps(info.framerate.clone())
    .build()
    .expect("Couldn't build video info");
    let container_profile = build_container_profile(&info, encoding_settings.container);

    let pipeline = gst::Pipeline::with_name("export_pipeline");

    let decodebin = gst::ElementFactory::make("uridecodebin3")
        .property("instant-uri", true)
        .property("uri", source_uri.as_str())
        .build()
        .unwrap();
    let encode_bin = gst::ElementFactory::make("encodebin")
        .property("profile", &container_profile)
        .build()
        .unwrap();
    let file_sink = gst::ElementFactory::make("filesink")
        .property("location", save_uri.as_str())
        .property("sync", false)
        .build()
        .unwrap();

    let mut frame_count = 0;
    let frame_spacing = 1.0 / (info.framerate.numer() as f64 / info.framerate.denom() as f64);
    let alloc = gst_allocator::DmaBufAllocator::new();
    let video_app_src = AppSrc::builder()
        .caps(&gst_video_info.to_caps().unwrap())
        .format(gst::Format::Time)
        .callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    let Ok(frame) = frame_receiver.recv() else {
                        let _ = appsrc.end_of_stream();
                        return;
                    };

                    let timer = SystemTime::now();

                    let mut buffer = gst::Buffer::new();
                    let mem = unsafe {
                        alloc
                            .alloc_with_flags(
                                frame.fd,
                                (output_size.width * output_size.height * 4) as usize,
                                gst_allocator::FdMemoryFlags::DONT_CLOSE,
                            )
                            .expect("Failed to allocate buffer")
                    };

                    {
                        let buffer = buffer.get_mut().unwrap();
                        buffer.append_memory(mem);
                        buffer.set_pts(ClockTime::from_seconds_f64(
                            frame_count as f64 * frame_spacing,
                        ));
                    }

                    frame_count += 1;
                    let _ = appsrc.push_buffer(buffer);
                    // println!("did frame #{frame_count} in {:?}", timer.elapsed().unwrap());
                })
                .build(),
        )
        .build();

    pipeline
        .add_many(&[
            &decodebin,
            &encode_bin,
            &file_sink,
            video_app_src.upcast_ref(),
        ])
        .unwrap();
    gst::Element::link_many([&encode_bin, &file_sink]).unwrap();

    let encode_video_sink = encode_bin.request_pad_simple("video_%u").unwrap();
    let video_src = &video_app_src
        .static_pad("src")
        .expect("no src pad for video appsrc");
    video_src.link(&encode_video_sink).unwrap();

    let pipeline_weak = pipeline.downgrade();

    decodebin.connect_pad_added(move |decode, src_pad| {
        let Some(pipeline) = pipeline_weak.upgrade() else {
            return;
        };

        let (is_audio, is_video) = (
            src_pad.name().starts_with("audio"),
            src_pad.name().starts_with("video"),
        );

        if is_audio {
            let encode_audio_sink = encode_bin.request_pad_simple("audio_%u").unwrap();
            src_pad.link(&encode_audio_sink).unwrap();
        } else if is_video {
            let queue = gst::ElementFactory::make("queue").build().unwrap();
            let video_convert = gst::ElementFactory::make("videoconvert").build().unwrap();

            let elements = &[&queue, &video_convert, video_appsink.upcast_ref()];
            pipeline.add_many(elements).unwrap();
            gst::Element::link_many(elements).unwrap();

            for e in elements {
                e.sync_state_with_parent().unwrap();
            }

            let video_sink = queue.static_pad("sink").unwrap();
            src_pad.link(&video_sink).unwrap();
        }
    });

    pipeline.set_state(gst::State::Paused).unwrap();

    // pipeline
    //     .seek(
    //         1.0,
    //         gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
    //         gst::SeekType::Set,
    //         timeline_settings.start,
    //         gst::SeekType::Set,
    //         timeline_settings.end,
    //     )
    //     .unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();
    pipeline
}

#[cfg(test)]
mod tests {
    // todo: figure out how to get around needing relm4 sender for app for player
    //  create an appsink manually and for eos have custom way of handler

    #[test]
    fn export_basic_video() {
        // let (handler, texture_receiver) = RendererHandler::new(RenderMode::MostRecentFrame);
        // let player = Rc::new(RefCell::new(Player::new(
        //     sender.clone(),
        //     handler.render_cmd_sender(),
        //     handler.timer_cmd_sender(),
        // )));
    }
}
