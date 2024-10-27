use std::collections::VecDeque;
use std::io::Write;
use std::time::{Duration, SystemTime};

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
    start_time: SystemTime,
    started: bool,
}

impl InFlightTimer {
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
            started: false,
        }
    }

    pub fn start_time(&mut self) {
        let now = SystemTime::now();
        if !self.started {
            self.start_time = now;
            self.started = true;
        }
    }

    pub fn stop_time(&mut self) -> Duration {
        let elapsed = self.start_time.elapsed().unwrap();
        self.started = false;
        elapsed
    }
}

pub(crate) struct Timer {
    pub(crate) query_set: wgpu::QuerySet,
    pub(crate) resolve_buffer: wgpu::Buffer,
    pub(crate) destination_buffer: wgpu::Buffer,
    curr_frame_time: InFlightTimer,
    frame_times: RollingAverage,
    curr_gdk_tex_time: InFlightTimer,
    gdk_tex_times: RollingAverage,
    curr_buff_map_time: InFlightTimer,
    buff_map_times: RollingAverage,
    gpu_render_times: RollingAverage,
    gpu_compute_times: RollingAverage,
}

static SAMPLES_FOR_AVG: u32 = 30;

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
            curr_frame_time: InFlightTimer::new(),
            curr_buff_map_time: InFlightTimer::new(),
            curr_gdk_tex_time: InFlightTimer::new(),
            frame_times: RollingAverage::new(SAMPLES_FOR_AVG),
            gdk_tex_times: RollingAverage::new(SAMPLES_FOR_AVG),
            buff_map_times: RollingAverage::new(SAMPLES_FOR_AVG),
            gpu_render_times: RollingAverage::new(SAMPLES_FOR_AVG),
            gpu_compute_times: RollingAverage::new(SAMPLES_FOR_AVG),
        }
    }

    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.gpu_render_times.clear();
        self.gpu_compute_times.clear();
    }

    pub fn start_frame_time(&mut self) {
        self.curr_frame_time.start_time();
    }

    pub fn stop_frame_time(&mut self) {
        let elapsed = self.curr_frame_time.stop_time();
        self.frame_times.add_sample(elapsed.as_millis() as f64);
    }

    pub fn start_gdk_mem_time(&mut self) {
        self.curr_gdk_tex_time.start_time();
    }

    pub fn stop_gdk_mem_time(&mut self) {
        let elapsed = self.curr_gdk_tex_time.stop_time();
        self.gdk_tex_times.add_sample(elapsed.as_millis() as f64);
    }

    pub fn start_buff_map_time(&mut self) {
        self.curr_buff_map_time.start_time();
    }

    pub fn stop_buff_map_time(&mut self) {
        let elapsed = self.curr_buff_map_time.stop_time();
        self.buff_map_times.add_sample(elapsed.as_millis() as f64);
    }

    pub fn collect_query_results(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
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
    }

    pub fn print_results(&self) {
        let render_time = self.gpu_render_times.avg();
        let compute_time = self.gpu_compute_times.avg();
        let total_time = render_time + compute_time;

        print!(
            "\rFrame time: {:.2} ms (gpu: {:.2} μs [r {:.2} + c {:.2}], map_buff: {:.2} ms, gdk_tex: {:.2} ms)",
            self.frame_times.avg(),
            total_time,
            render_time,
            compute_time,
            self.buff_map_times.avg(),
            self.gdk_tex_times.avg(),
        );

        std::io::stdout().flush().unwrap();
    }
}
