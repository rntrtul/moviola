use crate::ui::preview::effects_pipeline::effects::EffectParameters;
use crate::ui::preview::effects_pipeline::timer::Timer;
use crate::ui::preview::effects_pipeline::vertex::{FrameRect, INDICES};
use crate::ui::preview::effects_pipeline::{texture, vertex};
use crate::ui::preview::Orientation;
use ges::glib;
use gtk4::gdk;
use gtk4::prelude::Cast;
use lazy_static::lazy_static;
use std::cell::{Cell, RefCell};
use std::default::Default;
use wgpu::util::DeviceExt;

lazy_static! {
    static ref U32_SIZE: u32 = std::mem::size_of::<u32>() as u32;
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    frame_bind_group_layout: wgpu::BindGroupLayout,
    compute_bind_group_layout: wgpu::BindGroupLayout,
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
    orientation: Orientation,
    pub timer: Timer,
    frame_count: Cell<u32>,
}

impl Renderer {
    pub async fn new() -> Renderer {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
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
        let timer = Timer::new(&device);

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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: Some(
                                wgpu::BufferSize::new(EffectParameters::buffer_size()).unwrap(),
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("texture_binding group layout"),
            });

        let effect_parameters = EffectParameters::new();

        let effect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Effect Parameter Buffer"),
            size: EffectParameters::buffer_size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self::populate_effect_buffer(effect_parameters, &queue, &effect_buffer);

        let texture = texture::Texture::new_for_size(
            1,
            1,
            &device,
            &frame_bind_group_layout,
            &effect_buffer,
            "",
        )
        .unwrap();

        let draw_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into()),
        });

        let output_dimensions = (512, 288);
        let (render_target, output_staging_buffer, output_texture_view, compute_buffer) =
            Self::create_render_target(output_dimensions.0, output_dimensions.1, &device);

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
                entry_point: "vs_main",
                buffers: &[vertex::Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "fs_main",
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
                alpha_to_coverage_enabled: false, // antialiasing related. probably needed for crop box
            },
            multiview: None,
            cache: None,
        });

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
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
                ],
            });

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
            entry_point: "main",
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
        }
    }

    fn create_render_target(
        width: u32,
        height: u32,
        device: &wgpu::Device,
    ) -> (wgpu::Texture, wgpu::Buffer, wgpu::TextureView, wgpu::Buffer) {
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

        let output_texture_size = (width * height * *U32_SIZE) as u64;

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

        (
            render_target,
            output_staging_buffer,
            output_texture_view,
            compute_buffer,
        )
    }

    fn update_render_target(&mut self, width: u32, height: u32) {
        let (render_target, output_staging_buffer, output_texture_view, compute_buffer) =
            Self::create_render_target(width, height, &self.device);

        self.render_target = render_target;
        self.output_staging_buffer = output_staging_buffer;
        self.compute_buffer = compute_buffer;
        self.output_texture_view = output_texture_view;
    }

    pub fn orient(&mut self, orientation: Orientation) {
        self.video_frame_rect.orient(orientation);

        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(&self.video_frame_rect.vertices()),
                usage: wgpu::BufferUsages::VERTEX,
            });

        if self.orientation.is_vertical() != orientation.is_vertical() {
            self.output_dimensions = (self.output_dimensions.1, self.output_dimensions.0);
            self.update_render_target(self.output_dimensions.0, self.output_dimensions.1);
        }

        self.orientation = orientation;
    }

    pub fn update_input_texture_output_texture_size(
        &mut self,
        frame_width: u32,
        frame_height: u32,
        render_width: u32,
        render_height: u32,
    ) {
        self.update_render_target(render_width, render_height);
        self.output_dimensions = (render_width, render_height);
        self.timer.reset();

        self.video_frame_texture.replace(
            texture::Texture::new_for_size(
                frame_width,
                frame_height,
                &self.device,
                &self.frame_bind_group_layout,
                &self.effect_buffer,
                "video frame texture",
            )
            .unwrap(),
        );
    }

    pub fn sample_to_texture(&self, sample: gst::Sample) {
        self.video_frame_texture
            .borrow()
            .write_from_sample(&self.queue, sample);
        self.queue.submit([]);
    }

    pub fn populate_effect_buffer(
        effect_parameters: EffectParameters,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
    ) {
        let mut view = queue
            .write_buffer_with(
                &buffer,
                0,
                wgpu::BufferSize::new(EffectParameters::buffer_size()).unwrap(),
            )
            .unwrap();

        let buffer = bytemuck::cast_slice_mut(&mut view);
        effect_parameters.populate_buffer(buffer);
    }
    pub async fn update_effects(&mut self, parameters: EffectParameters) {
        self.effect_parameters = parameters;
        Self::populate_effect_buffer(self.effect_parameters, &self.queue, &self.effect_buffer);
        self.queue.submit([]);
    }

    pub fn prepare_video_frame_render_pass(&mut self) -> wgpu::CommandBuffer {
        let texture = self.video_frame_texture.borrow();

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
                timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                    query_set: &self.timer.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timer.query_set,
                    beginning_of_pass_write_index: Some(2),
                    end_of_pass_write_index: Some(3),
                }),
            });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compute Bind Group"),
                layout: &self.compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.output_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.compute_buffer.as_entire_binding(),
                    },
                ],
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.output_dimensions.0.div_ceil(256),
                self.output_dimensions.1,
                1,
            );
        }

        encoder.resolve_query_set(&self.timer.query_set, 0..4, &self.timer.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            &self.timer.resolve_buffer,
            0,
            &self.timer.destination_buffer,
            0,
            self.timer.destination_buffer.size(),
        );

        encoder.copy_buffer_to_buffer(
            &self.compute_buffer,
            0,
            &self.output_staging_buffer,
            0,
            self.output_staging_buffer.size(),
        );

        encoder.finish()
    }

    // todo: accept multiple command buffers
    pub async fn render(
        &mut self,
        command_buffer: wgpu::CommandBuffer,
    ) -> Result<gdk::Texture, wgpu::SurfaceError> {
        self.queue.submit(Some(command_buffer));
        let gdk_texture: gdk::Texture;

        {
            let (sender, receiver) = flume::bounded(1);
            let slice = self.output_staging_buffer.slice(..);

            self.timer.start_buff_map_time();

            slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
            self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
            receiver.recv_async().await.unwrap().unwrap();

            self.timer.stop_buff_map_time();
            self.timer.collect_query_results(&self.device, &self.queue);

            {
                self.timer.start_gdk_mem_time();
                let view = slice.get_mapped_range();

                gdk_texture = gdk::MemoryTexture::new(
                    self.output_dimensions.0 as i32,
                    self.output_dimensions.1 as i32,
                    gdk::MemoryFormat::R8g8b8a8,
                    &glib::Bytes::from(&view.iter().as_slice()),
                    (self.output_dimensions.0 as i32 * 4) as usize,
                )
                .upcast::<gdk::Texture>();

                self.timer.stop_gdk_mem_time();
                self.frame_count.set(self.frame_count.get() + 1);
            }

            self.output_staging_buffer.unmap();
        }

        Ok(gdk_texture)
    }
}
