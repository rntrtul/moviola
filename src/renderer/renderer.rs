use crate::renderer::frame_position::FramePositionUniform;
use crate::renderer::handler::TimerCmd;
use crate::renderer::timer::{GpuTimer, QuerySet};
use crate::renderer::{texture, EffectParameters, TimerEvent};
use crate::ui::preview::Orientation;
use ges::glib;
use gst::Sample;
use gtk4::gdk;
use gtk4::prelude::Cast;
use std::cell::RefCell;
use std::default::Default;
use std::sync::mpsc;
use std::time::Instant;
use wgpu::include_wgsl;

pub static U32_SIZE: u32 = size_of::<u32>() as u32;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    output_dimensions: (u32, u32),
    orientation: Orientation,
    frame_position: FramePositionUniform,
    effect_parameters: EffectParameters,
    video_frame_texture: RefCell<texture::Texture>,
    frame_position_pipeline: wgpu::ComputePipeline,
    frame_position_bind_group_layout: wgpu::BindGroupLayout,
    frame_position_bind_group: wgpu::BindGroup,
    frame_position_buffer: wgpu::Buffer,
    positioned_frame: wgpu::Texture,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: wgpu::BindGroup,
    effect_buffer: wgpu::Buffer,
    compute_buffer: wgpu::Buffer,
    output_staging_buffer: wgpu::Buffer,
    pub(crate) timer: GpuTimer,
    timer_sender: mpsc::Sender<TimerCmd>,
}

// todo: rename compute shader to unpadding

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
                        ty: wgpu::BindingType::StorageTexture {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(256),
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
                ],
            });

        let effect_parameters = EffectParameters::new();
        let effect_buffer = effect_parameters.buffer(&device);

        let texture = texture::Texture::new_for_size(1, 1, &device, "").unwrap();

        let output_dimensions = (512, 288);

        let frame_position = FramePositionUniform::new();

        let (
            output_staging_buffer,
            compute_buffer,
            compute_bind_group,
            positioned_frame,
            frame_position_buffer,
            frame_position_bind_group,
        ) = Self::create_render_target(
            output_dimensions.0,
            output_dimensions.1,
            &effect_buffer,
            &compute_bind_group_layout,
            &frame_position_bind_group_layout,
            &frame_position,
            &texture,
            &device,
        );

        let compute_shader = device.create_shader_module(include_wgsl!("compute.wgsl"));

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute pipeline"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
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

        // todo: order these to follow struct definition
        Self {
            device,
            queue,
            compute_pipeline,
            compute_bind_group_layout,
            compute_bind_group,
            output_staging_buffer,
            compute_buffer,
            effect_buffer,
            effect_parameters,
            output_dimensions,
            timer,
            orientation: Orientation::default(),
            video_frame_texture: RefCell::new(texture),
            frame_position,
            frame_position_buffer,
            timer_sender,
            frame_position_bind_group_layout,
            frame_position_pipeline,
            frame_position_bind_group,
            positioned_frame,
        }
    }

    fn create_frame_positon_bind_groups(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        frame_position_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePositionUniform,
        texture: &texture::Texture,
    ) -> (wgpu::Texture, wgpu::Buffer, wgpu::BindGroup) {
        let positioned_frame = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Texture Descriptor"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });

        let frame_position_buffer = frame_position.buffer(&device);

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
                    resource: frame_position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &positioned_frame.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        (
            positioned_frame,
            frame_position_buffer,
            frame_position_bind_group,
        )
    }

    fn create_compute_bind_groups(
        device: &wgpu::Device,
        texture_size: u64,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        effect_buffer: &wgpu::Buffer,
        input_texture: &wgpu::Texture,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let compute_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute buffer"),
            size: texture_size,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &input_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: compute_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: effect_buffer.as_entire_binding(),
                },
            ],
        });

        (compute_buffer, compute_bind_group)
    }

    fn create_render_target(
        width: u32,
        height: u32,
        effect_buffer: &wgpu::Buffer,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePositionUniform,
        texture: &texture::Texture,
        device: &wgpu::Device,
    ) -> (
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Texture,
        wgpu::Buffer,
        wgpu::BindGroup,
    ) {
        let output_texture_size = texture_size(width, height);
        let (positioned_frame, frame_position_buffer, frame_position_bind_group) =
            Self::create_frame_positon_bind_groups(
                &device,
                width,
                height,
                &frame_position_bind_group_layout,
                &frame_position,
                &texture,
            );
        let (compute_buffer, compute_bind_group) = Self::create_compute_bind_groups(
            &device,
            output_texture_size,
            &compute_bind_group_layout,
            &effect_buffer,
            &positioned_frame,
        );

        let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output staging Buffer"),
            size: output_texture_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        (
            output_staging_buffer,
            compute_buffer,
            compute_bind_group,
            positioned_frame,
            frame_position_buffer,
            frame_position_bind_group,
        )
    }

    fn update_render_target(&mut self, width: u32, height: u32) {
        let (
            output_staging_buffer,
            compute_buffer,
            compute_bind_group,
            positoned_frame,
            frame_position_buffer,
            frame_postion_bind_group,
        ) = Self::create_render_target(
            width,
            height,
            &self.effect_buffer,
            &self.compute_bind_group_layout,
            &self.frame_position_bind_group_layout,
            &self.frame_position,
            &self.video_frame_texture.borrow(),
            &self.device,
        );

        self.output_staging_buffer = output_staging_buffer;
        self.compute_buffer = compute_buffer;
        self.compute_bind_group = compute_bind_group;
        self.frame_position_buffer = frame_position_buffer;
        self.frame_position_bind_group = frame_postion_bind_group;
        self.positioned_frame = positoned_frame;
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
    }

    fn update_output_texture_size(&mut self, width: u32, height: u32) {
        self.update_render_target(width, height);
        self.output_dimensions = (width, height);
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
                    timestamp_writes: self.timer.query_timestamp_writes(QuerySet::Position),
                });

            frame_position_pass.set_pipeline(&self.frame_position_pipeline);
            frame_position_pass.set_bind_group(0, &self.frame_position_bind_group, &[]);
            frame_position_pass.dispatch_workgroups(
                self.output_dimensions.0.div_ceil(256),
                self.output_dimensions.1,
                1,
            );
        }

        // compute pass
        if !self.effect_parameters.is_default() {
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: self.timer.query_timestamp_writes(QuerySet::Compute),
                });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
                compute_pass.dispatch_workgroups(
                    self.output_dimensions.0.div_ceil(256),
                    self.output_dimensions.1,
                    1,
                );
            }

            encoder.copy_buffer_to_buffer(
                &self.compute_buffer,
                0,
                &self.output_staging_buffer,
                0,
                self.output_staging_buffer.size(),
            );
        } else {
            let bytes_per_row = self.positioned_frame.width() * U32_SIZE;
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.positioned_frame,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.output_staging_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                },
                self.positioned_frame.size(),
            );
        }

        encoder.resolve_query_set(
            &self.timer.query_set,
            0..self.timer.queries(),
            &self.timer.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timer.resolve_buffer,
            0,
            &self.timer.result_buffer,
            0,
            self.timer.result_buffer.size(),
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
            let slice = self.output_staging_buffer.slice(..);

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
                    self.output_dimensions.0 as i32,
                    self.output_dimensions.1 as i32,
                    gdk::MemoryFormat::R8g8b8a8,
                    &glib::Bytes::from(&view.iter().as_slice()),
                    self.output_dimensions.0 as usize * 4,
                )
                .upcast::<gdk::Texture>();

                self.timer_sender
                    .send(TimerCmd::Stop(TimerEvent::TextureCreate, Instant::now()))
                    .unwrap();
            }
            self.timer.collect_query_results(&self.device, &self.queue);

            self.output_staging_buffer.unmap();
        }

        Ok(gdk_texture)
    }

    pub fn upload_new_sample(&mut self, sample: &Sample) {
        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        if !self.is_size_equal_to_curr_input_size(info.width(), info.height()) {
            self.update_input_texture_size(info.width(), info.height());
            self.timer.reset();
        }
        self.sample_to_texture(sample);
    }

    pub fn update_effects(&mut self, parameters: EffectParameters) {
        if parameters.is_default() {
            self.timer.disable_query_set(QuerySet::Compute);
        } else {
            self.timer.enable_query_set(QuerySet::Compute);
        }

        self.effect_parameters = parameters;

        let mut view = self
            .queue
            .write_buffer_with(
                &self.effect_buffer,
                0,
                wgpu::BufferSize::new(self.effect_buffer.size()).unwrap(),
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
        let (w, h) = self.orientation.oriented_size(width, height);
        self.update_output_texture_size(w, h);
    }

    pub fn orient(&mut self, orientation: Orientation) {
        // fixme: update the orientation uniform

        if self.orientation.is_width_flipped() != orientation.is_width_flipped() {
            self.update_output_texture_size(self.output_dimensions.1, self.output_dimensions.0);
        }

        self.orientation = orientation;
    }
}

fn texture_size(width: u32, height: u32) -> u64 {
    (width * height * U32_SIZE) as u64
}
