use crate::app::{App, AppMsg};
use crate::renderer::renderer::RenderedFrame;
use crate::renderer::{FrameSize, RenderCmd, TimerCmd};
use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
use crate::video::metadata::VideoInfo;
use crate::video::player::{video_appsink, AppSinkUsage, Player};
use anyhow::Error;
use gst::prelude::{
    BufferPoolExt, BufferPoolExtManual, Cast, ElementExt, ElementExtManual, GstBinExt,
    GstBinExtManual, GstObjectExt, ObjectExt, PadExt,
};
use gst::ClockTime;
use gst_app::{AppSink, AppSrc};
use gst_pbutils::prelude::EncodingProfileBuilder;
use gst_pbutils::EncodingContainerProfile;
use gst_video::VideoBufferPoolConfig;
use relm4::ComponentSender;
use std::ops::Deref;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::SystemTime;

#[derive(Debug, Copy, Clone)]
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

        let (decode, encode) = start_export_video(
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
            wait_export_done_and_cleanup(decode, encode);
            app_sender.input(AppMsg::ExportDone);
        });
    }
}

fn wait_for_eos(bus: gst::Bus) {
    let now = SystemTime::now();

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

fn start_export_video(
    source_uri: String,
    save_uri: String,
    info: VideoInfo,
    output_size: FrameSize,
    encoding_settings: ControlsExportSettings,
    timeline_settings: TimelineExportSettings,
    video_appsink: AppSink,
    frame_receiver: mpsc::Receiver<RenderedFrame>,
) -> (gst::Pipeline, gst::Pipeline) {
    let (audio_sender, audio_recv) = mpsc::channel();

    let decode = launch_decode_pipeline(
        !encoding_settings.container.no_audio,
        audio_sender,
        timeline_settings,
        source_uri,
        video_appsink,
    )
    .expect("could not launch decode pipeline");
    let encode = launch_encode_pipeline(
        frame_receiver,
        audio_recv,
        info,
        output_size,
        encoding_settings,
        save_uri,
        timeline_settings.start,
    )
    .expect("could not launch encode pipeline");
    (decode, encode)
}

fn launch_decode_pipeline(
    audio_enabled: bool,
    audio_sender: mpsc::Sender<Option<gst::Sample>>,
    timeline_settings: TimelineExportSettings,
    source_uri: String,
    video_appsink: AppSink,
) -> Result<gst::Pipeline, Error> {
    let pipeline = gst::Pipeline::default();
    let decode_bin = gst::ElementFactory::make("uridecodebin")
        .property("uri", source_uri.as_str())
        .build()?;

    pipeline
        .add(&decode_bin)
        .expect("failed to add elements to pipeline");

    let pipeline_weak = pipeline.downgrade();
    let c = Arc::new((Mutex::new(0), Condvar::new()));
    let c2 = Arc::clone(&c);

    decode_bin.connect_pad_added(move |_dbin, dbin_src_pad| {
        let Some(pipeline) = pipeline_weak.upgrade() else {
            return;
        };

        let (is_audio, is_video) = {
            let media_type = dbin_src_pad.current_caps().and_then(|caps| {
                caps.structure(0).map(|s| {
                    let name = s.name();
                    (name.starts_with("audio/"), name.starts_with("video/"))
                })
            });

            match media_type {
                None => {
                    println!("Failed to get media type from pad {}", dbin_src_pad.name());
                    return;
                }
                Some(media_type) => media_type,
            }
        };

        let audio_sample_sender = audio_sender.clone();
        let audio_eos_sender = audio_sender.clone();
        let link_to_encode_bin = |is_audio, is_video| -> Result<(), Error> {
            if is_audio && audio_enabled {
                // todo: figure out how to connect to a specific audio stream
                let queue = gst::ElementFactory::make("queue").build()?;
                let app_sink = AppSink::builder()
                    .enable_last_sample(true)
                    .max_buffers(10)
                    .sync(false)
                    .callbacks(
                        gst_app::AppSinkCallbacks::builder()
                            .new_sample(move |appsink| {
                                let sample = appsink.pull_sample().unwrap();
                                audio_sample_sender.send(Some(sample)).unwrap();
                                Ok(gst::FlowSuccess::Ok)
                            })
                            .eos(move |_| {
                                audio_eos_sender.send(None).unwrap();
                            })
                            .build(),
                    )
                    .build();

                let elements = &[&queue, app_sink.upcast_ref()];
                pipeline
                    .add_many(elements)
                    .expect("failed to add audio elements to pipeline");
                gst::Element::link_many(elements)?;

                for e in elements {
                    e.sync_state_with_parent()?;
                }

                let sink_pad = queue.static_pad("sink").expect("queue has no sinkpad");
                dbin_src_pad.link(&sink_pad)?;
            } else if is_video {
                let queue = gst::ElementFactory::make("queue").build()?;
                let convert = gst::ElementFactory::make("videoconvert").build()?;

                let elements = &[&queue, &convert, video_appsink.upcast_ref()];
                pipeline
                    .add_many(elements)
                    .expect("failed to add video elements to pipeline");
                gst::Element::link_many(elements)?;

                for e in elements {
                    e.sync_state_with_parent()?
                }

                let sink_pad = queue.static_pad("sink").expect("queue has no sinkpad");
                dbin_src_pad.link(&sink_pad)?;
            }

            Ok(())
        };

        if let Err(err) = link_to_encode_bin(is_audio, is_video) {
            println!("failed to insert sink {err}");
        }

        if is_audio || is_video {
            //todo: see how it works with multiple audio tracks
            let (lock, cvar) = &*c2;
            let mut pads_connected = lock.lock().unwrap();
            *pads_connected += 1;
            cvar.notify_one();
        }
    });

    pipeline.set_state(gst::State::Paused)?;
    {
        let (lock, cvar) = &*c;
        let mut pads_connected = lock.lock().unwrap();
        while *pads_connected < 2 {
            pads_connected = cvar.wait(pads_connected).unwrap();
        }
    }

    pipeline
        .seek(
            1.0,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            timeline_settings.start,
            gst::SeekType::Set,
            timeline_settings.end,
        )
        .expect("Could NOT Seek");

    pipeline.set_state(gst::State::Playing)?;
    Ok(pipeline)
}

fn launch_encode_pipeline(
    frame_receiver: mpsc::Receiver<RenderedFrame>,
    audio_recv: mpsc::Receiver<Option<gst::Sample>>,
    info: VideoInfo,
    output_size: FrameSize,
    encoding_settings: ControlsExportSettings,
    save_uri: String,
    audio_start_offset: gst::ClockTime,
) -> Result<gst::Pipeline, Error> {
    //  encoders don't accept DMABUF so not used right now. They might be downloading the current dmabuf
    //  which is stored linearly and in RGBA so output fine, if slow.
    let _dma_caps = gst_video::VideoCapsBuilder::new()
        .format(gst_video::VideoFormat::DmaDrm)
        .features([
            gst_allocator::CAPS_FEATURE_MEMORY_DMABUF,
            gst_video::CAPS_FEATURE_META_GST_VIDEO_META,
        ])
        .field("drm-format", "RA24")
        .framerate(info.framerate.clone())
        .width(output_size.width as i32)
        .height(output_size.height as i32)
        .pixel_aspect_ratio(gst::Fraction::new(1, 1))
        .build();

    let gst_video_info = gst_video::VideoInfo::builder(
        gst_video::VideoFormat::Rgba,
        output_size.width,
        output_size.height,
    )
    .fps(info.framerate.clone())
    .build()
    .expect("Couldn't build video info");

    let pipeline = gst::Pipeline::default();

    let container_profile = build_container_profile(&info, encoding_settings.container);
    let encode_bin = gst::ElementFactory::make("encodebin")
        .property("profile", &container_profile)
        .build()?;
    let file_sink = gst::ElementFactory::make("filesink")
        .property("location", save_uri.as_str())
        .build()?;

    // todo: get padding required by hardware passed in
    let row_stride = (info.width as f32 / 32.0).ceil() as i32 * 128;
    let alloc = gst_allocator::DmaBufAllocator::new();

    let mut frame_count = 0;
    let frame_spacing = 1.0 / (info.framerate.numer() as f64 / info.framerate.denom() as f64);
    let video_appsrc = AppSrc::builder()
        .name("video appsrc")
        .format(gst::Format::Time)
        .caps(&gst_video_info.to_caps().unwrap())
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
                                (row_stride as u32 * output_size.height) as usize,
                                gst_allocator::FdMemoryFlags::NONE,
                            )
                            .expect("Failed to allocate buffer")
                    };

                    {
                        let buffer = buffer.get_mut().unwrap();
                        buffer.append_memory(mem);
                        gst_video::VideoMeta::add_full(
                            buffer,
                            gst_video::VideoFrameFlags::empty(),
                            gst_video::VideoFormat::Rgba,
                            output_size.width,
                            output_size.height,
                            &[0],
                            &[row_stride],
                        )
                        .unwrap();
                        buffer.set_pts(ClockTime::from_seconds_f64(
                            frame_count as f64 * frame_spacing,
                        ));
                    }

                    frame_count += 1;
                    let _ = appsrc.push_buffer(buffer);
                    println!("did vframe #{frame_count} in {:?}", timer.elapsed());
                })
                .build(),
        )
        .build();

    let audio_appsrc = AppSrc::builder()
        .name("audio appsrc")
        .format(gst::Format::Time)
        .callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    let Ok(Some(mut audio_sample)) = audio_recv.recv() else {
                        let _ = appsrc.end_of_stream();
                        return;
                    };

                    let audio_sample_ref = audio_sample.make_mut();
                    let mut buffer = audio_sample_ref.buffer_owned().unwrap();
                    audio_sample_ref.set_buffer(None);
                    {
                        let buffer_ref = buffer.make_mut();
                        let new_pts = buffer_ref.pts().unwrap() - audio_start_offset;
                        buffer_ref.set_pts(new_pts);
                    }
                    audio_sample_ref.set_buffer(Some(&buffer));

                    let _ = appsrc.push_sample(&audio_sample);
                })
                .build(),
        )
        .build();

    let mut elements = vec![video_appsrc.upcast_ref(), &encode_bin, &file_sink];

    if !encoding_settings.container.no_audio {
        elements.push(audio_appsrc.upcast_ref());
    }

    pipeline
        .add_many(elements)
        .expect("failed to add to encode pipeline");
    gst::Element::link_many([&encode_bin, &file_sink])?;

    let encode_video_sink_pad = encode_bin
        .request_pad_simple("video_%u")
        .expect("Could not get video pad from encodebin");
    let video_src_pad = video_appsrc
        .static_pad("src")
        .expect("video appsrc has no srcpad");
    video_src_pad.link(&encode_video_sink_pad)?;

    if !encoding_settings.container.no_audio {
        let encode_audio_sink_pad = encode_bin
            .request_pad_simple("audio_%u")
            .expect("Could not get audio pad from encodebin");
        let audio_src_pad = audio_appsrc
            .static_pad("src")
            .expect("videoconvert has no srcpad");
        audio_src_pad.link(&encode_audio_sink_pad)?;
    }

    pipeline.set_state(gst::State::Playing)?;
    Ok(pipeline)
}

fn wait_export_done_and_cleanup(decode: gst::Pipeline, encode: gst::Pipeline) {
    wait_for_eos(decode.bus().unwrap());
    wait_for_eos(encode.bus().unwrap());

    decode.set_state(gst::State::Null).unwrap();
    encode.set_state(gst::State::Null).unwrap();
}

#[cfg(test)]
mod tests {
    use crate::config::*;
    use crate::renderer::renderer::RenderedFrame;
    use crate::renderer::{FrameSize, RenderMode, RenderResopnse, RendererHandler};
    use crate::ui::sidebar::{ControlsExportSettings, OutputContainerSettings};
    use crate::video::export::{
        start_export_video, wait_export_done_and_cleanup, TimelineExportSettings,
    };
    use crate::video::metadata::{AudioCodec, ContainerFormat, VideoCodec, VideoInfo};
    use gst::ClockTime;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::sync::{mpsc, Arc};
    use tokio::task::JoinHandle;

    fn render_listner(
        render_response: mpsc::Receiver<RenderResopnse>,
        frame_sender: mpsc::Sender<RenderedFrame>,
        no_more_frames_incoming: Arc<AtomicBool>,
        frames_to_render: Arc<AtomicU64>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut frames_done = 0;
            loop {
                let Ok(response) = render_response.recv() else {
                    break;
                };

                frames_done += 1;
                match response {
                    RenderResopnse::FrameRendered(frame) => {
                        frame_sender.send(frame).unwrap();
                    }
                }

                if no_more_frames_incoming.load(std::sync::atomic::Ordering::Relaxed)
                    && frames_to_render.load(std::sync::atomic::Ordering::Relaxed) == frames_done
                {
                    break;
                }
            }
        })
    }

    fn discover_metadata(uri: &String) -> VideoInfo {
        let discoverer = gst_pbutils::Discoverer::new(ClockTime::from_seconds_f64(2.0))
            .expect("unable to make discoverer");
        let info = discoverer.discover_uri(uri.as_str()).unwrap();
        VideoInfo::from(info.clone())
    }

    // fixme: why do appsrc allocs fail due to fd.
    #[tokio::test(flavor = "multi_thread")]
    async fn export_basic_video() {
        gst::init().unwrap();

        let source_uri = VIDEO_TEST_FILE_SHORT.to_string();
        let video_info = discover_metadata(&source_uri);
        let frame_size = FrameSize::new(video_info.width, video_info.height);

        let (handler, render_response) = RendererHandler::new(RenderMode::AllFrames);
        let (frame_sender, frame_recv) = mpsc::channel();
        let frames_to_render = Arc::new(AtomicU64::new(0));
        let export_done = Arc::new(AtomicBool::new(false));

        let listener = render_listner(
            render_response,
            frame_sender,
            export_done.clone(),
            frames_to_render.clone(),
        );

        let render_sender = handler.render_cmd_sender();
        let app_sink = gst_app::AppSink::builder()
            .enable_last_sample(true)
            .max_buffers(1)
            .sync(false)
            .caps(
                &gst_video::VideoCapsBuilder::new()
                    .format(gst_video::VideoFormat::Rgba)
                    .build(),
            )
            .callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |appsink| {
                        let sample = appsink.pull_sample().unwrap();
                        let a = frames_to_render.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        render_sender
                            .send(crate::renderer::RenderCmd::RenderSample(sample))
                            .unwrap();

                        println!("sending video sample {}", a + 1);
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .eos(move |_| {
                        println!("video EOS");
                        export_done.store(true, std::sync::atomic::Ordering::Relaxed);
                    })
                    .build(),
            )
            .build();

        let (decode, encode) = start_export_video(
            source_uri,
            VIDEO_EXPORT_DST.to_string(),
            video_info,
            frame_size,
            ControlsExportSettings {
                container: OutputContainerSettings {
                    no_audio: false,
                    audio_stream_idx: 0,
                    audio_codec: AudioCodec::AAC,
                    audio_bitrate: 0,
                    container: ContainerFormat::MP4,
                    video_codec: VideoCodec::X265,
                    video_bitrate: 0,
                },
                container_is_default: true,
                effect_parameters: Default::default(),
            },
            TimelineExportSettings {
                start: ClockTime::from_seconds_f64(0.5f64),
                end: ClockTime::from_seconds_f64(3f64),
            },
            app_sink,
            frame_recv,
        );

        listener.await.expect("could not await on renderer listne");
        wait_export_done_and_cleanup(decode, encode);
    }
}
