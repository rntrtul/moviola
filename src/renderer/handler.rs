use crate::renderer::renderer::Renderer;
use crate::renderer::timer::Timer;
use crate::renderer::{EffectParameters, TimerEvent};
use crate::ui::preview::Orientation;
use gtk4::gdk;
use std::cmp::PartialEq;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;
use tokio::sync::Mutex;

pub enum RenderCmd {
    ChangeRenderMode(RenderMode),
    RenderFrame,
    RenderSample(gst::Sample),
    UpdateEffects(EffectParameters),
    UpdateOutputResolution(u32, u32),
    UpdateOrientation(Orientation),
}

// todo: rename outputresult
pub enum TimerCmd {
    Start(TimerEvent, Instant),
    Stop(TimerEvent, Instant),
    OutputResult(Option<String>),
    Quit,
}

#[derive(PartialEq)]
pub enum RenderMode {
    MostRecentFrame,
    AllFrames,
}

fn render_frame(
    sender: mpsc::Sender<gdk::Texture>,
    renderer: Arc<Mutex<Renderer>>,
    render_queued: Arc<AtomicBool>,
    render_cmd_sender: mpsc::Sender<RenderCmd>,
) {
    tokio::spawn(async move {
        let tex;
        {
            let mut renderer = renderer.lock().await;
            tex = renderer.render_frame().await;
        }
        sender.send(tex).unwrap();

        if render_queued.load(std::sync::atomic::Ordering::Relaxed) {
            render_cmd_sender.send(RenderCmd::RenderFrame).unwrap();
        }
    });
}

async fn update_queued(
    renderer: Arc<Mutex<Renderer>>,
    sample: &mut Option<gst::Sample>,
    effect_parms: &mut Option<EffectParameters>,
    orientation: &mut Option<Orientation>,
    output_res: &mut Option<(u32, u32)>,
) {
    let mut renderer = renderer.lock().await;

    if let Some((width, height)) = output_res {
        renderer.update_output_resolution(*width, *height);
    }

    if let Some(orientation) = orientation.take() {
        renderer.orient(orientation);
    }

    if let Some(sample) = sample.take() {
        renderer.upload_new_sample(&sample);
    }

    if let Some(params) = effect_parms.take() {
        renderer.update_effects(params);
    }
}

async fn render_loop(
    texture_sender: mpsc::Sender<gdk::Texture>,
    timer_sender: mpsc::Sender<TimerCmd>,
    cmd_recv: mpsc::Receiver<RenderCmd>,
    renderer_cmd_sender: mpsc::Sender<RenderCmd>,
    render_mode: RenderMode,
) {
    let renderer = Arc::new(Mutex::new(Renderer::new(timer_sender.clone()).await));

    // todo: make a struct to hold these
    let mut queued_effect_params: Option<EffectParameters> = None;
    let mut queued_output_resolution: Option<(u32, u32)> = None;
    let mut queued_orientation: Option<Orientation> = None;
    let mut render_mode = render_mode;

    let mut samples: VecDeque<gst::Sample> = VecDeque::with_capacity(1);
    let render_queued = Arc::new(AtomicBool::new(false));

    let mut frames_rendered: u32 = 0;

    loop {
        let Ok(cmd) = cmd_recv.recv() else {
            break;
        };

        if frames_rendered != 0 && frames_rendered % 30 == 0 {
            let gpu_timing = renderer.lock().await.gpu_timer.frame_time_msg();
            timer_sender
                .send(TimerCmd::OutputResult(Some(gpu_timing)))
                .unwrap();
        }

        match cmd {
            RenderCmd::UpdateEffects(params) => {
                queued_effect_params.replace(params);

                if let Ok(mut renderer) = renderer.try_lock() {
                    renderer.update_effects(queued_effect_params.take().unwrap());
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::RenderSample(sample) => {
                if render_mode == RenderMode::MostRecentFrame {
                    samples.clear();
                }
                samples.push_back(sample);

                if let Ok(mut guarded_renderer) = renderer.try_lock() {
                    guarded_renderer.upload_new_sample(&samples.pop_front().unwrap());
                    drop(guarded_renderer);

                    render_frame(
                        texture_sender.clone(),
                        renderer.clone(),
                        render_queued.clone(),
                        renderer_cmd_sender.clone(),
                    );
                    frames_rendered += 1;
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::RenderFrame => {
                // fixme: find better way of testing if lockable other than getting lock
                //      and dropping it.
                if let Ok(guarded_renderer) = renderer.try_lock() {
                    drop(guarded_renderer);

                    if render_queued.load(std::sync::atomic::Ordering::Relaxed) {
                        let are_more_renders_queued = match render_mode {
                            RenderMode::MostRecentFrame => false,
                            RenderMode::AllFrames => samples.len() != 0,
                        };
                        render_queued.store(
                            are_more_renders_queued,
                            std::sync::atomic::Ordering::Relaxed,
                        );

                        update_queued(
                            renderer.clone(),
                            &mut samples.pop_front(),
                            &mut queued_effect_params,
                            &mut queued_orientation,
                            &mut queued_output_resolution,
                        )
                        .await;
                    }

                    render_frame(
                        texture_sender.clone(),
                        renderer.clone(),
                        render_queued.clone(),
                        renderer_cmd_sender.clone(),
                    );
                    frames_rendered += 1;
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::UpdateOutputResolution(width, height) => {
                queued_output_resolution.replace((width, height));

                if let Ok(mut renderer) = renderer.try_lock() {
                    let (width, height) = queued_output_resolution.take().unwrap();
                    renderer.update_output_resolution(width, height);
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::UpdateOrientation(orientation) => {
                queued_orientation.replace(orientation);

                if let Ok(mut renderer) = renderer.try_lock() {
                    renderer.orient(queued_orientation.take().unwrap());
                } else {
                    render_queued.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            RenderCmd::ChangeRenderMode(mode) => {
                render_mode = mode;
            }
        }
    }
}

async fn timer_loop(timer_reciver: mpsc::Receiver<TimerCmd>) {
    let mut timer = Timer::new();

    loop {
        let Ok(cmd) = timer_reciver.recv() else {
            break;
        };

        match cmd {
            TimerCmd::Start(label, time) => {
                timer.start_time(label, time);
            }
            TimerCmd::Stop(label, time) => {
                timer.stop_time(label, time);
            }
            TimerCmd::OutputResult(gpu_time) => {
                tracing::trace!("{}", timer.timings(gpu_time));
            }
            TimerCmd::Quit => {
                break;
            }
        }
    }
}

pub struct RendererHandler {
    _thread: thread::JoinHandle<()>,
    cmd_sender: mpsc::Sender<RenderCmd>,
    timer_sender: mpsc::Sender<TimerCmd>,
}

impl RendererHandler {
    pub fn new(mode: RenderMode) -> (Self, mpsc::Receiver<gdk::Texture>) {
        let (cmd_sender, cmd_recv) = mpsc::channel::<RenderCmd>();
        let (output_sender, output_receiver) = mpsc::channel::<gdk::Texture>();
        let (timer_sender, timer_receiver) = mpsc::channel::<TimerCmd>();

        let renderer_cmd_sender = cmd_sender.clone();
        let timer_cmd_sender = timer_sender.clone();

        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            runtime.spawn(timer_loop(timer_receiver));

            runtime.block_on(render_loop(
                output_sender,
                timer_cmd_sender.clone(),
                cmd_recv,
                renderer_cmd_sender,
                mode,
            ));

            timer_cmd_sender.send(TimerCmd::Quit).unwrap();
        });

        let handler = Self {
            _thread: thread,
            cmd_sender,
            timer_sender,
        };

        (handler, output_receiver)
    }

    pub fn render_cmd_sender(&self) -> mpsc::Sender<RenderCmd> {
        self.cmd_sender.clone()
    }

    pub fn timer_cmd_sender(&self) -> mpsc::Sender<TimerCmd> {
        self.timer_sender.clone()
    }

    pub fn send_render_cmd(&self, cmd: RenderCmd) {
        self.cmd_sender.send(cmd).unwrap();
    }
}
