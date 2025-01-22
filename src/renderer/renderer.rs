use crate::renderer::frame_position::FramePositionUniform;
use crate::renderer::handler::TimerCmd;
use crate::renderer::timer::{GpuTimer, QuerySet};
use crate::renderer::vertex::{FrameRect, INDICES};
use crate::renderer::{texture, vertex, EffectParameters, TimerEvent};
use crate::ui::preview::Orientation;
use ges::glib;
use gst::Sample;
use gtk4::gdk;
use gtk4::prelude::Cast;
use std::cell::{Cell, RefCell};
use std::default::Default;
use std::sync::mpsc;
use std::time::Instant;
use wgpu::include_wgsl;
use wgpu::util::DeviceExt;

pub static U32_SIZE: u32 = size_of::<u32>() as u32;

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    frame_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group: wgpu::BindGroup,
    output_texture_view: wgpu::TextureView,
    render_target: wgpu::Texture,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    effect_buffer: wgpu::Buffer,
    output_staging_buffer: wgpu::Buffer,
    compute_buffer: wgpu::Buffer,
    effect_parameters: EffectParameters,
    output_dimensions: (u32, u32),
    video_frame_texture: RefCell<texture::Texture>,
    video_frame_rect: FrameRect,
    frame_position: FramePositionUniform,
    frame_position_buffer: wgpu::Buffer,
    orientation: Orientation,
    pub timer: GpuTimer,
    timer_sender: mpsc::Sender<TimerCmd>,
    frame_count: Cell<u32>,
}

// todo: add pipeline step for frame positioning as first step
//   (subsequent steps will only deal with smaller frames)
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

        let frame_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_binding group layout"),
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

        let texture = texture::Texture::new_for_size(
            1,
            1,
            &device,
            &frame_bind_group_layout,
            &effect_buffer,
            "",
        )
        .unwrap();

        let draw_shader = device.create_shader_module(include_wgsl!("draw.wgsl"));

        let output_dimensions = (512, 288);

        let frame_position = FramePositionUniform::new();

        let (
            render_target,
            output_staging_buffer,
            output_texture_view,
            compute_buffer,
            compute_bind_group,
            frame_position_buffer,
        ) = Self::create_render_target(
            output_dimensions.0,
            output_dimensions.1,
            &compute_bind_group_layout,
            &frame_position,
            &device,
        );

        let frame_rect = FrameRect::new();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&frame_rect.vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indices = INDICES.len() as u32;

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layour"),
                bind_group_layouts: &[&frame_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex::Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

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

        Self {
            device,
            queue,
            render_target,
            compute_pipeline,
            render_pipeline,
            num_indices,
            frame_bind_group_layout,
            compute_bind_group_layout,
            compute_bind_group,
            vertex_buffer,
            index_buffer,
            output_staging_buffer,
            compute_buffer,
            effect_buffer,
            effect_parameters,
            output_texture_view,
            output_dimensions,
            timer,
            orientation: Orientation::default(),
            video_frame_texture: RefCell::new(texture),
            video_frame_rect: frame_rect,
            frame_count: Cell::new(0),
            frame_position,
            frame_position_buffer,
            timer_sender,
        }
    }

    fn create_render_target(
        width: u32,
        height: u32,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePositionUniform,
        device: &wgpu::Device,
    ) -> (
        wgpu::Texture,
        wgpu::Buffer,
        wgpu::TextureView,
        wgpu::Buffer,
        wgpu::BindGroup,
        wgpu::Buffer,
    ) {
        let render_target = device.create_texture(&wgpu::TextureDescriptor {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });

        let output_texture_size = (width * height * U32_SIZE) as u64;

        let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output staging Buffer"),
            size: output_texture_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let compute_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute buffer"),
            size: output_texture_size,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let output_texture_view =
            render_target.create_view(&wgpu::TextureViewDescriptor::default());

        let frame_position_buffer = frame_position.buffer(&device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: compute_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: frame_position_buffer.as_entire_binding(),
                },
            ],
        });

        (
            render_target,
            output_staging_buffer,
            output_texture_view,
            compute_buffer,
            bind_group,
            frame_position_buffer,
        )
    }

    fn update_render_target(&mut self, width: u32, height: u32) {
        let (
            render_target,
            output_staging_buffer,
            output_texture_view,
            compute_buffer,
            compute_bind_group,
            frame_position_buffer,
        ) = Self::create_render_target(
            width,
            height,
            &self.compute_bind_group_layout,
            &self.frame_position,
            &self.device,
        );

        self.render_target = render_target;
        self.output_staging_buffer = output_staging_buffer;
        self.compute_buffer = compute_buffer;
        self.output_texture_view = output_texture_view;
        self.compute_bind_group = compute_bind_group;
        self.frame_position_buffer = frame_position_buffer;
    }

    pub fn is_size_equal_to_curr_input_size(&self, width: u32, height: u32) -> bool {
        let texture = &self.video_frame_texture.borrow().texture;
        width == texture.width() && height == texture.height()
    }

    fn update_input_texture_size(&mut self, width: u32, height: u32) {
        self.video_frame_texture.replace(
            texture::Texture::new_for_size(
                width,
                height,
                &self.device,
                &self.frame_bind_group_layout,
                &self.effect_buffer,
                "video frame texture",
            )
            .unwrap(),
        );

        if self.video_frame_texture.borrow().is_padded {
            self.timer.enable_query_set(QuerySet::Compute);
        } else {
            self.timer.disable_query_set(QuerySet::Compute);
        }
    }

    fn update_output_texture_size(&mut self, width: u32, height: u32) {
        self.update_render_target(width, height);
        self.output_dimensions = (width, height);
    }

    fn update_vertex_buffer(&mut self) {
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(&self.video_frame_rect.vertices()),
                usage: wgpu::BufferUsages::VERTEX,
            });
    }

    fn sample_to_texture(&self, sample: &Sample) {
        self.video_frame_texture
            .borrow()
            .write_from_sample(&self.queue, sample);
        self.queue.submit([]);
    }

    fn prepare_video_frame_render_pass(&mut self) -> wgpu::CommandBuffer {
        let texture = self.video_frame_texture.borrow();
        let frame_is_padded = self.video_frame_texture.borrow().is_padded;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.output_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: self.timer.render_pass_timestamp_writes(),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // compute pass
        if frame_is_padded {
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: self.timer.compute_pass_timestamp_writes(),
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
            let bytes_per_row = self.render_target.width() * U32_SIZE;
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.render_target,
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
                self.render_target.size(),
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

                self.frame_count.set(self.frame_count.get() + 1);
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
        self.video_frame_rect.orient(orientation);
        self.update_vertex_buffer();

        if self.orientation.is_width_flipped() != orientation.is_width_flipped() {
            self.update_output_texture_size(self.output_dimensions.1, self.output_dimensions.0);
        }

        self.orientation = orientation;
    }
}
