use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum TimerEvent {
    FrameTime,
    BuffMap,
    TextureCreate,
}

impl TimerEvent {
    pub fn label(&self) -> &str {
        match self {
            TimerEvent::FrameTime => "Frame time",
            TimerEvent::BuffMap => "buffer map",
            TimerEvent::TextureCreate => "gdk texture",
        }
    }
}

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
    avg: RollingAverage,
    start_time: Instant,
    started: bool,
}

impl InFlightTimer {
    pub fn new() -> Self {
        Self {
            avg: RollingAverage::new(SAMPLES_FOR_AVG),
            start_time: Instant::now(),
            started: false,
        }
    }

    pub fn start_time(&mut self, start_time: Instant) {
        if !self.started {
            self.start_time = start_time;
            self.started = true;
        }
    }

    pub fn stop_time(&mut self, end_time: Instant) {
        let elapsed = end_time
            .checked_duration_since(self.start_time)
            .unwrap_or(Duration::new(0, 0));
        self.started = false;
        self.avg.add_sample(elapsed.as_millis() as f64);
    }

    pub fn avg(&self) -> f64 {
        self.avg.avg()
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum QuerySet {
    Render,
    Compute,
}

pub(crate) struct GpuTimer {
    pub(crate) query_set: wgpu::QuerySet,
    pub(crate) resolve_buffer: wgpu::Buffer,
    pub(crate) result_buffer: wgpu::Buffer,
    gpu_render_times: RollingAverage,
    gpu_compute_times: RollingAverage,
    total_frames_recorded: u32,
    active_query_sets: Vec<QuerySet>,
}

static SAMPLES_FOR_AVG: u32 = 1000;

impl GpuTimer {
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
            gpu_render_times: RollingAverage::new(SAMPLES_FOR_AVG),
            gpu_compute_times: RollingAverage::new(SAMPLES_FOR_AVG),
            total_frames_recorded: 0,
            active_query_sets: vec![QuerySet::Render],
        }
    }

    pub fn reset(&mut self) {
        self.gpu_render_times.clear();
        self.gpu_compute_times.clear();
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
    }

    pub fn frame_time_msg(&self) -> String {
        let render_time = self.gpu_render_times.avg();
        let compute_time = self.gpu_compute_times.avg();
        let total_time = render_time + compute_time;

        format!(
            "GPU: {:.2}μs (r {:.2} + c {:.2})",
            total_time, render_time, compute_time
        )
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

pub struct Timer {
    in_flight_times: HashMap<TimerEvent, InFlightTimer>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            in_flight_times: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.in_flight_times.clear();
    }

    pub fn start_time(&mut self, event: TimerEvent, start: Instant) {
        let timer = self
            .in_flight_times
            .entry(event)
            .or_insert(InFlightTimer::new());
        timer.start_time(start);
    }

    pub fn stop_time(&mut self, event: TimerEvent, stop: Instant) {
        self.in_flight_times
            .entry(event)
            .and_modify(|timer| timer.stop_time(stop));
    }

    fn append_event_to_msg(&self, msg: &mut String, event: TimerEvent) {
        if let Some(timer) = self.in_flight_times.get(&event) {
            msg.push_str(&format!("{}: {:.2}ms | ", event.label(), timer.avg()));
        }
    }

    pub fn timings(&self, gpu_time: Option<String>) -> String {
        let mut msg = "".to_string();

        self.append_event_to_msg(&mut msg, TimerEvent::FrameTime);
        self.append_event_to_msg(&mut msg, TimerEvent::BuffMap);
        self.append_event_to_msg(&mut msg, TimerEvent::TextureCreate);

        if let Some(gpu_time) = gpu_time {
            msg.push_str(&format!("[{}]", gpu_time));
        }

        msg
    }
}
