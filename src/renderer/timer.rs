use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub static FRAME_TIME_IDX: &str = "Frame time";
pub static BUFF_MAP_IDX: &str = "buffer map";
pub static GDK_TEX_IDX: &str = "mem text";

struct RollingAverage {
    total: f64,
    samples: VecDeque<f64>,
    max_samples: u32,
}

impl RollingAverage {
    pub fn new(max_samples: u32) -> Self {
        Self {
            total: 0f64,
            samples: VecDeque::with_capacity(max_samples as usize),
            max_samples,
        }
    }

    pub fn clear(&mut self) {
        self.total = 0f64;
        self.samples.clear();
    }

    pub fn add_sample(&mut self, sample: f64) {
        self.total += sample;
        self.samples.push_back(sample);
        if self.samples.len() as u32 > self.max_samples {
            self.total -= self.samples.pop_front().unwrap();
        }
    }

    pub fn avg(&self) -> f64 {
        self.total / self.samples.len() as f64
    }
}

struct InFlightTimer {
    start_time: Instant,
    started: bool,
}

impl InFlightTimer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            started: false,
        }
    }

    pub fn start_time(&mut self) {
        let now = Instant::now();
        if !self.started {
            self.start_time = now;
            self.started = true;
        }
    }

    pub fn stop_time(&mut self) -> Duration {
        let elapsed = self.start_time.elapsed();
        self.started = false;
        elapsed
    }
}

pub(crate) struct Timer {
    pub(crate) query_set: wgpu::QuerySet,
    pub(crate) resolve_buffer: wgpu::Buffer,
    pub(crate) destination_buffer: wgpu::Buffer,
    in_flight_times: HashMap<String, (InFlightTimer, RollingAverage)>,
    gpu_render_times: RollingAverage,
    gpu_compute_times: RollingAverage,
    total_frames_recorded: u32,
}

static SAMPLES_FOR_AVG: u32 = 1000;

impl Timer {
    pub fn new(device: &wgpu::Device) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("timestamp query set"),
            count: 4,
            ty: wgpu::QueryType::Timestamp,
        });

        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp resolve buffer"),
            size: 4 * 8,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp result buffer"),
            size: resolve_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            query_set,
            resolve_buffer,
            destination_buffer: result_buffer,
            in_flight_times: HashMap::new(),
            gpu_render_times: RollingAverage::new(SAMPLES_FOR_AVG),
            gpu_compute_times: RollingAverage::new(SAMPLES_FOR_AVG),
            total_frames_recorded: 0,
        }
    }

    pub fn reset(&mut self) {
        self.in_flight_times.clear();
        self.gpu_render_times.clear();
        self.gpu_compute_times.clear();
    }

    pub fn start_time(&mut self, label: &str) {
        if !self.in_flight_times.contains_key(label) {
            self.in_flight_times.insert(
                label.to_string(),
                (InFlightTimer::new(), RollingAverage::new(SAMPLES_FOR_AVG)),
            );
        }
        self.in_flight_times.get_mut(label).unwrap().0.start_time();
    }

    pub fn stop_time(&mut self, label: &str) {
        if !self.in_flight_times.contains_key(label) {
            return;
        }
        let elapsed = self.in_flight_times.get_mut(label).unwrap().0.stop_time();
        self.in_flight_times
            .get_mut(label)
            .unwrap()
            .1
            .add_sample(elapsed.as_millis() as f64);
    }

    pub fn collect_query_results(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.total_frames_recorded += 1;
        self.destination_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| ());
        device.poll(wgpu::Maintain::wait()).panic_on_timeout();

        let timestamps: Vec<u64> = {
            let timestamp_view = self.destination_buffer.slice(..).get_mapped_range();
            bytemuck::cast_slice(&timestamp_view).to_vec()
        };

        self.destination_buffer.unmap();

        let period = queue.get_timestamp_period();
        let elapsed_micro_seconds =
            |start, end: u64| end.wrapping_sub(start) as f64 * (period as f64) / 1000.0;

        self.gpu_render_times
            .add_sample(elapsed_micro_seconds(timestamps[0], timestamps[1]));
        self.gpu_compute_times
            .add_sample(elapsed_micro_seconds(timestamps[2], timestamps[3]));

        if self.total_frames_recorded % 30 == 0 {
            let render_time = self.gpu_render_times.avg();
            let compute_time = self.gpu_compute_times.avg();
            let total_time = render_time + compute_time;

            let mut msg = format!(
                "gpu: {:.2}μs (r {:.2} + c {:.2})",
                total_time, render_time, compute_time
            );
            for (label, (_, avg)) in self.in_flight_times.iter() {
                msg.push_str(&format!(" {label}: {:.2}ms", avg.avg()));
            }
            tracing::trace!(msg);
        }
    }
}
