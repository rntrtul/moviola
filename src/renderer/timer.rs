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
        if self.samples.is_empty() {
            return 0f64;
        }
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

pub(crate) struct SingleTimer {
    avg: RollingAverage,
    timer: InFlightTimer,
}

impl SingleTimer {
    pub fn new() -> Self {
        Self {
            avg: RollingAverage::new(SAMPLES_FOR_AVG),
            timer: InFlightTimer::new(),
        }
    }

    pub fn avg(&self) -> f64 {
        self.avg.avg()
    }

    pub fn start_time(&mut self) {
        self.timer.start_time();
    }

    pub fn stop_time(&mut self) {
        let elapsed = self.timer.stop_time();
        self.avg.add_sample(elapsed.as_millis() as f64);
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum QuerySet {
    Render,
    Compute,
}

pub(crate) struct Timer {
    pub(crate) query_set: wgpu::QuerySet,
    pub(crate) resolve_buffer: wgpu::Buffer,
    pub(crate) result_buffer: wgpu::Buffer,
    in_flight_times: HashMap<String, SingleTimer>,
    gpu_render_times: RollingAverage,
    gpu_compute_times: RollingAverage,
    total_frames_recorded: u32,
    active_query_sets: Vec<QuerySet>,
}

static SAMPLES_FOR_AVG: u32 = 1000;

impl Timer {
    pub fn new(device: &wgpu::Device) -> Self {
        let max_query_count = 4;

        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("timestamp query set"),
            count: max_query_count,
            ty: wgpu::QueryType::Timestamp,
        });

        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp resolve buffer"),
            size: (max_query_count * 8) as wgpu::BufferAddress,
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
            result_buffer,
            in_flight_times: HashMap::new(),
            gpu_render_times: RollingAverage::new(SAMPLES_FOR_AVG),
            gpu_compute_times: RollingAverage::new(SAMPLES_FOR_AVG),
            total_frames_recorded: 0,
            active_query_sets: vec![QuerySet::Render],
        }
    }

    pub fn reset(&mut self) {
        self.in_flight_times.clear();
        self.gpu_render_times.clear();
        self.gpu_compute_times.clear();
    }

    pub fn start_time(&mut self, label: &str) {
        if !self.in_flight_times.contains_key(label) {
            self.in_flight_times
                .insert(label.to_string(), SingleTimer::new());
        }
        self.in_flight_times.get_mut(label).unwrap().start_time();
    }

    pub fn stop_time(&mut self, label: &str) {
        if !self.in_flight_times.contains_key(label) {
            return;
        }
        self.in_flight_times.get_mut(label).unwrap().stop_time();
    }

    pub fn collect_query_results(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.total_frames_recorded += 1;
        self.result_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| ());
        device.poll(wgpu::Maintain::wait()).panic_on_timeout();

        let timestamps: Vec<u64> = {
            let timestamp_view = self.result_buffer.slice(..).get_mapped_range();
            bytemuck::cast_slice(&timestamp_view).to_vec()
        };

        self.result_buffer.unmap();

        let period = queue.get_timestamp_period();
        let elapsed_micro_seconds =
            |start, end: u64| end.wrapping_sub(start) as f64 * (period as f64) / 1000.0;

        if let Some(render_start_idx) = self.query_set_start_index(QuerySet::Render) {
            let index = render_start_idx as usize;
            self.gpu_render_times.add_sample(elapsed_micro_seconds(
                timestamps[index],
                timestamps[index + 1],
            ));
        };

        if let Some(compute_start_idx) = self.query_set_start_index(QuerySet::Compute) {
            let index = compute_start_idx as usize;
            self.gpu_compute_times.add_sample(elapsed_micro_seconds(
                timestamps[index],
                timestamps[index + 1],
            ));
        };

        if self.total_frames_recorded % 30 == 0 {
            let render_time = self.gpu_render_times.avg();
            let compute_time = self.gpu_compute_times.avg();
            let total_time = render_time + compute_time;

            let mut msg = format!(
                "gpu: {:.2}μs (r {:.2} + c {:.2})",
                total_time, render_time, compute_time
            );
            for (label, timer) in self.in_flight_times.iter() {
                msg.push_str(&format!(" {label}: {:.2}ms", timer.avg()));
            }
            tracing::trace!(msg);
        }
    }

    fn query_set_start_index(&self, query_set: QuerySet) -> Option<u32> {
        if let Some(query_set_num) = self
            .active_query_sets
            .iter()
            .position(|set| *set == query_set)
        {
            Some((query_set_num * 2) as u32)
        } else {
            None
        }
    }

    pub fn queries(&self) -> u32 {
        self.active_query_sets.len() as u32 * 2
    }

    pub fn enable_query_set(&mut self, query_set: QuerySet) {
        if !self.is_query_enabled(query_set) {
            self.active_query_sets.push(query_set);
        }
    }

    pub fn disable_query_set(&mut self, to_disable: QuerySet) {
        if self.is_query_enabled(to_disable) {
            self.active_query_sets
                .retain(|query_set| *query_set != to_disable);
        }
    }

    fn is_query_enabled(&self, query_set: QuerySet) -> bool {
        self.active_query_sets.contains(&query_set)
    }

    pub fn render_pass_timestamp_writes(&self) -> Option<wgpu::RenderPassTimestampWrites> {
        if let Some(query_start) = self.query_set_start_index(QuerySet::Render) {
            Some(wgpu::RenderPassTimestampWrites {
                query_set: &self.query_set,
                beginning_of_pass_write_index: Some(query_start),
                end_of_pass_write_index: Some(query_start + 1),
            })
        } else {
            None
        }
    }

    pub fn compute_pass_timestamp_writes(&self) -> Option<wgpu::ComputePassTimestampWrites> {
        if let Some(query_start) = self.query_set_start_index(QuerySet::Compute) {
            Some(wgpu::ComputePassTimestampWrites {
                query_set: &self.query_set,
                beginning_of_pass_write_index: Some(query_start),
                end_of_pass_write_index: Some(query_start + 1),
            })
        } else {
            None
        }
    }
}
