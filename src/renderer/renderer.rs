use crate::renderer::frame_position::{FramePosition, FrameSize};
use crate::renderer::handler::TimerCmd;
use crate::renderer::timer::{GpuTimer, QuerySet};
use crate::renderer::{texture, EffectParameters, TimerEvent};
use crate::ui::preview::Orientation;
use gst::Sample;
use image::DynamicImage;
use relm4::gtk::prelude::Cast;
use relm4::gtk::{gdk, glib};
use std::cell::RefCell;
use std::default::Default;
use std::sync::mpsc;
use std::time::Instant;
use wgpu::include_wgsl;

pub static U32_SIZE: u32 = size_of::<u32>() as u32;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    output_size: FrameSize,
    frame_position: FramePosition,
    effect_parameters: EffectParameters,
    video_frame_texture: RefCell<texture::Texture>,
    frame_position_pipeline: wgpu::ComputePipeline,
    frame_position_bind_group_layout: wgpu::BindGroupLayout,
    frame_position_bind_group: wgpu::BindGroup,
    frame_position_buffer: wgpu::Buffer,
    positioned_frame_buffer: wgpu::Buffer,
    effects_pipeline: wgpu::ComputePipeline,
    effects_bind_group_layout: wgpu::BindGroupLayout,
    effects_bind_group: wgpu::BindGroup,
    effects_buffer: wgpu::Buffer,
    effects_output_buffer: wgpu::Buffer,
    final_output_buffer: wgpu::Buffer,
    pub(crate) gpu_timer: GpuTimer,
    timer_sender: mpsc::Sender<TimerCmd>,
}

impl Renderer {
    pub async fn new(timer_sender: mpsc::Sender<TimerCmd>) -> Renderer {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::TIMESTAMP_QUERY
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        // assuming timestamp feature available always
        let timer = GpuTimer::new(&device);

        let frame_position_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Frame Position bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let effects_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Effects bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(256),
                        },
                        count: None,
                    },
                ],
            });

        let effect_parameters = EffectParameters::new();
        let effects_buffer = effect_parameters.buffer(&device);

        let texture = texture::Texture::new_for_size(1, 1, &device, "").unwrap();

        let output_size = FrameSize::new(512, 288);
        let frame_position = FramePosition::new(output_size);

        let (
            positioned_frame_buffer,
            frame_position_buffer,
            frame_position_bind_group,
            effects_output_buffer,
            effects_bind_group,
            final_output_buffer,
        ) = Self::create_render_target(
            output_size,
            &effects_buffer,
            &effects_bind_group_layout,
            &frame_position_bind_group_layout,
            &frame_position,
            &texture,
            &device,
        );

        let effects_shader = device.create_shader_module(include_wgsl!("effects.wgsl"));

        let effects_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("effects pipeline"),
                bind_group_layouts: &[&effects_bind_group_layout],
                push_constant_ranges: &[],
            });

        let effects_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("effects pipeline"),
            layout: Some(&effects_pipeline_layout),
            module: &effects_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let position_shader = device.create_shader_module(include_wgsl!("position.wgsl"));

        let frame_position_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("frame poistion pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("frame postion pipeline layout"),
                        bind_group_layouts: &[&frame_position_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &position_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            device,
            queue,
            output_size,
            frame_position,
            effect_parameters,
            video_frame_texture: RefCell::new(texture),
            frame_position_pipeline,
            frame_position_bind_group_layout,
            frame_position_bind_group,
            frame_position_buffer,
            positioned_frame_buffer,
            effects_pipeline,
            effects_bind_group_layout,
            effects_bind_group,
            effects_buffer,
            effects_output_buffer,
            final_output_buffer,
            gpu_timer: timer,
            timer_sender,
        }
    }

    fn create_frame_positon_bind_groups(
        device: &wgpu::Device,
        frame_size: &FrameSize,
        frame_position_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePosition,
        texture: &texture::Texture,
        frame_size_buffer: &wgpu::Buffer,
    ) -> (wgpu::Buffer, wgpu::Buffer, wgpu::BindGroup) {
        let frame_position_buffer = frame_position.buffer(&device);

        let positioned_frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effects output buffer"),
            size: frame_size.texture_size(),
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let frame_position_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame Position Bind Group"),
            layout: &frame_position_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: frame_position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: frame_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: positioned_frame_buffer.as_entire_binding(),
                },
            ],
        });

        (
            positioned_frame_buffer,
            frame_position_buffer,
            frame_position_bind_group,
        )
    }

    fn create_effects_bind_groups(
        device: &wgpu::Device,
        texture_size: u64,
        effects_bind_group_layout: &wgpu::BindGroupLayout,
        effect_parameters_buffer: &wgpu::Buffer,
        input_buffer: &wgpu::Buffer,
        frame_size_buffer: &wgpu::Buffer,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let effects_output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effects output buffer"),
            size: texture_size,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let effects_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Effects Bind Group"),
            layout: &effects_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: frame_size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: effect_parameters_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: effects_output_buffer.as_entire_binding(),
                },
            ],
        });

        (effects_output_buffer, effects_bind_group)
    }

    fn create_render_target(
        output_frame_size: FrameSize,
        effect_buffer: &wgpu::Buffer,
        effects_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePosition,
        texture: &texture::Texture,
        device: &wgpu::Device,
    ) -> (
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Buffer,
    ) {
        let frame_size_buffer = output_frame_size.buffer(&device);

        let (positioned_frame_buffer, frame_position_buffer, frame_position_bind_group) =
            Self::create_frame_positon_bind_groups(
                &device,
                &output_frame_size,
                &frame_position_bind_group_layout,
                &frame_position,
                &texture,
                &frame_size_buffer,
            );

        let (effects_output_buffer, effects_bind_group) = Self::create_effects_bind_groups(
            &device,
            output_frame_size.texture_size(),
            &effects_bind_group_layout,
            &effect_buffer,
            &positioned_frame_buffer,
            &frame_size_buffer,
        );

        let final_output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output staging Buffer"),
            size: output_frame_size.texture_size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        (
            positioned_frame_buffer,
            frame_position_buffer,
            frame_position_bind_group,
            effects_output_buffer,
            effects_bind_group,
            final_output_buffer,
        )
    }

    fn update_render_target(&mut self, output_frame_size: FrameSize) {
        let (
            positioned_frame_buffer,
            frame_position_buffer,
            frame_postion_bind_group,
            effects_output_buffer,
            effects_bind_group,
            final_output_buffer,
        ) = Self::create_render_target(
            output_frame_size,
            &self.effects_buffer,
            &self.effects_bind_group_layout,
            &self.frame_position_bind_group_layout,
            &self.frame_position,
            &self.video_frame_texture.borrow(),
            &self.device,
        );

        self.final_output_buffer = final_output_buffer;
        self.effects_output_buffer = effects_output_buffer;
        self.effects_bind_group = effects_bind_group;
        self.frame_position_buffer = frame_position_buffer;
        self.frame_position_bind_group = frame_postion_bind_group;
        self.positioned_frame_buffer = positioned_frame_buffer;
    }

    pub fn is_size_equal_to_curr_input_size(&self, width: u32, height: u32) -> bool {
        let texture = &self.video_frame_texture.borrow().texture;
        width == texture.width() && height == texture.height()
    }

    fn update_input_texture_size(&mut self, width: u32, height: u32) {
        self.video_frame_texture.replace(
            texture::Texture::new_for_size(width, height, &self.device, "video frame texture")
                .unwrap(),
        );
        self.frame_position.original_frame_size = FrameSize::new(width, height);
    }

    fn update_buffers_for_output_size(&mut self, output_frame_size: FrameSize) {
        self.update_render_target(output_frame_size);
        self.output_size = output_frame_size;
    }

    fn sample_to_texture(&self, sample: &Sample) {
        self.video_frame_texture
            .borrow()
            .write_from_sample(&self.queue, sample);
        self.queue.submit([]);
    }

    fn prepare_video_frame_render_pass(&mut self) -> wgpu::CommandBuffer {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut frame_position_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("frame position pass"),
                    timestamp_writes: self.gpu_timer.query_timestamp_writes(QuerySet::Position),
                });

            frame_position_pass.set_pipeline(&self.frame_position_pipeline);
            frame_position_pass.set_bind_group(0, &self.frame_position_bind_group, &[]);
            frame_position_pass.dispatch_workgroups(
                self.output_size.width.div_ceil(256),
                self.output_size.height,
                1,
            );
        }

        let mut output_source_buffer = &self.positioned_frame_buffer;

        if !self.effect_parameters.is_default() {
            {
                let mut effects_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Effects Pass"),
                    timestamp_writes: self.gpu_timer.query_timestamp_writes(QuerySet::Effects),
                });

                effects_pass.set_pipeline(&self.effects_pipeline);
                effects_pass.set_bind_group(0, &self.effects_bind_group, &[]);
                effects_pass.dispatch_workgroups(
                    self.output_size.width.div_ceil(256),
                    self.output_size.height,
                    1,
                );
            }

            output_source_buffer = &self.effects_output_buffer;
        }

        encoder.copy_buffer_to_buffer(
            &output_source_buffer,
            0,
            &self.final_output_buffer,
            0,
            self.final_output_buffer.size(),
        );

        encoder.resolve_query_set(
            &self.gpu_timer.query_set,
            0..self.gpu_timer.queries(),
            &self.gpu_timer.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu_timer.resolve_buffer,
            0,
            &self.gpu_timer.result_buffer,
            0,
            self.gpu_timer.result_buffer.size(),
        );

        encoder.finish()
    }

    // todo: accept multiple command buffers
    async fn render(
        &mut self,
        command_buffer: wgpu::CommandBuffer,
    ) -> Result<gdk::Texture, wgpu::SurfaceError> {
        self.queue.submit(Some(command_buffer));
        let gdk_texture: gdk::Texture;

        {
            let (sender, receiver) = mpsc::channel();
            let slice = self.final_output_buffer.slice(..);

            self.timer_sender
                .send(TimerCmd::Start(TimerEvent::BuffMap, Instant::now()))
                .unwrap();

            slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
            self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
            receiver.recv().unwrap().unwrap();

            self.timer_sender
                .send(TimerCmd::Stop(TimerEvent::BuffMap, Instant::now()))
                .unwrap();

            {
                self.timer_sender
                    .send(TimerCmd::Start(TimerEvent::TextureCreate, Instant::now()))
                    .unwrap();
                let view = slice.get_mapped_range();

                gdk_texture = gdk::MemoryTexture::new(
                    self.output_size.width as i32,
                    self.output_size.height as i32,
                    gdk::MemoryFormat::R8g8b8a8,
                    &glib::Bytes::from(&view.iter().as_slice()),
                    self.output_size.width as usize * 4,
                )
                .upcast::<gdk::Texture>();

                self.timer_sender
                    .send(TimerCmd::Stop(TimerEvent::TextureCreate, Instant::now()))
                    .unwrap();
            }
            self.gpu_timer
                .collect_query_results(&self.device, &self.queue);

            self.final_output_buffer.unmap();
        }

        Ok(gdk_texture)
    }

    pub fn upload_new_sample(&mut self, sample: &Sample) {
        self.timer_sender
            .send(TimerCmd::Start(TimerEvent::Renderer, Instant::now()))
            .unwrap();
        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        if !self.is_size_equal_to_curr_input_size(info.width(), info.height()) {
            self.update_input_texture_size(info.width(), info.height());
            self.gpu_timer.reset();
        }
        self.sample_to_texture(sample);
    }

    pub fn upload_new_image(&mut self, img: &DynamicImage) {
        if !self.is_size_equal_to_curr_input_size(img.width(), img.height()) {
            self.update_input_texture_size(img.width(), img.height());
        }
        self.video_frame_texture
            .borrow()
            .write_from_image(&self.queue, img);
        self.queue.submit([]);
    }

    pub fn update_effects(&mut self, parameters: EffectParameters) {
        if parameters.is_default() {
            self.gpu_timer.disable_query_set(QuerySet::Effects);
        } else {
            self.gpu_timer.enable_query_set(QuerySet::Effects);
        }

        self.effect_parameters = parameters;

        let mut view = self
            .queue
            .write_buffer_with(
                &self.effects_buffer,
                0,
                wgpu::BufferSize::new(self.effects_buffer.size()).unwrap(),
            )
            .unwrap();

        let buffer: &mut [f32] = bytemuck::cast_slice_mut(&mut view);
        buffer.clone_from_slice(bytemuck::cast_slice(&[self.effect_parameters]));
    }

    pub async fn render_frame(&mut self) -> gdk::Texture {
        let command_buffer = self.prepare_video_frame_render_pass();
        self.render(command_buffer).await.expect("Could not render")
    }

    pub fn update_output_resolution(&mut self, width: u32, height: u32) {
        // need to handle width and height when base orientation is non-zero as input width + heights
        // are relative to the sample/frame which is always 0deg.
        let (w, h) = self.frame_position.orientation.oriented_size(width, height);
        let size = FrameSize::new(w, h);
        self.frame_position.scale_for_output_size(size);
        self.update_buffers_for_output_size(size);
    }

    pub fn orient(&mut self, orientation: Orientation) {
        self.frame_position.orientation = orientation;
        let size = self.frame_position.output_frame_size();
        self.update_buffers_for_output_size(size);
    }

    pub fn position_frame(&mut self, frame_position: FramePosition) {
        self.frame_position = frame_position;
        let output_size = self.frame_position.output_frame_size();
        self.update_buffers_for_output_size(output_size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IMG_TEST_LANDSCAPE;
    use relm4::gtk::prelude::TextureExt;

    #[tokio::test]
    async fn render_to_image() {
        let (sender, _recv) = mpsc::channel();
        let mut r = Renderer::new(sender).await;

        let img = image::open(IMG_TEST_LANDSCAPE).unwrap();
        let mut frame_position = FramePosition::new(FrameSize::new(img.width(), img.height()));
        frame_position.scale_for_output_size(FrameSize::new(img.width() / 2, img.height() / 2));
        frame_position.orientation = Orientation {
            angle: 90.0,
            base_angle: 0.0,
            mirrored: false,
        };
        frame_position.straigthen_angle = 31f32.to_radians();

        let mut effects = EffectParameters::new();
        effects.saturation = 1.2;

        r.upload_new_image(&img);
        r.position_frame(frame_position);
        // r.update_effects(effects);

        let texture = r.render_frame().await;
        println!("time to render: {}", r.gpu_timer.frame_time_msg());
        texture.save_to_png("test_image.png").unwrap();
    }
}
