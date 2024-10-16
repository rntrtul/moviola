use crate::ui::preview::effects_pipeline::timer::Timer;
use crate::ui::preview::effects_pipeline::vertex::{INDICES, VERTICES};
use crate::ui::preview::effects_pipeline::{texture, vertex};
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
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    frame_bind_group_layout: wgpu::BindGroupLayout,
    output_texture_view: wgpu::TextureView,
    render_target: wgpu::Texture,
    output_staging_buffer: wgpu::Buffer,
    output_dimensions: (u32, u32),
    video_frame_texture: RefCell<texture::Texture>,
    timer: Timer,
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
                    required_features: wgpu::Features::TIMESTAMP_QUERY,
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

        // use bind groups for effects paramters?
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
                ],
                label: Some("texture_binding group layout"),
            });

        let texture =
            texture::Texture::new_for_size(1, 1, &device, &frame_bind_group_layout, "").unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let output_dimensions = (512, 288);
        let (render_target, output_staging_buffer, output_texture_view) =
            Self::create_render_target(output_dimensions.0, output_dimensions.1, &device);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(VERTICES),
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
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex::Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // maybe switch to line list for crop box
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

        Self {
            device,
            queue,
            render_target,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            frame_bind_group_layout,
            output_staging_buffer,
            output_texture_view,
            output_dimensions,
            timer,
            video_frame_texture: RefCell::new(texture),
            frame_count: Cell::new(0),
        }
    }

    fn padded_bytes_per_row(row_width: u32) -> u32 {
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            * (((row_width * *U32_SIZE) as f32 / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as f32).ceil()
                as u32)
    }

    pub fn create_render_target(
        width: u32,
        height: u32,
        device: &wgpu::Device,
    ) -> (wgpu::Texture, wgpu::Buffer, wgpu::TextureView) {
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });

        let padded_bytes_per_row = Self::padded_bytes_per_row(width);
        let output_texture_size = (padded_bytes_per_row * height) as wgpu::BufferAddress;

        let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output staging Buffer"),
            size: output_texture_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let output_texture_view =
            render_target.create_view(&wgpu::TextureViewDescriptor::default());

        (render_target, output_staging_buffer, output_texture_view)
    }

    pub fn update_input_texture_output_texture_size(
        &mut self,
        frame_width: u32,
        frame_height: u32,
        render_width: u32,
        render_height: u32,
    ) {
        let (render_target, output_staging_buffer, output_texture_view) =
            Self::create_render_target(render_width, render_height, &self.device);

        self.render_target = render_target;
        self.output_staging_buffer = output_staging_buffer;
        self.output_texture_view = output_texture_view;
        self.output_dimensions = (render_width, render_height);

        self.video_frame_texture.replace(
            texture::Texture::new_for_size(
                frame_width,
                frame_height,
                &self.device,
                &self.frame_bind_group_layout,
                "video frame texture",
            )
            .unwrap(),
        );
    }

    // todo: accept effect paramters
    pub fn prepare_video_frame_render_pass(&self, sample: gst::Sample) -> wgpu::CommandBuffer {
        let texture = self.video_frame_texture.borrow();
        texture.write_from_sample(&self.queue, sample);

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

        let padded_bytes_per_row = Self::padded_bytes_per_row(self.output_dimensions.0);
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.render_target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.output_staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.output_dimensions.1),
                },
            },
            wgpu::Extent3d {
                width: self.output_dimensions.0,
                height: self.output_dimensions.1,
                depth_or_array_layers: 1,
            },
        );

        encoder.resolve_query_set(&self.timer.query_set, 0..2, &self.timer.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            &self.timer.resolve_buffer,
            0,
            &self.timer.destination_buffer,
            0,
            self.timer.destination_buffer.size(),
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
            let buffer_slice = self.output_staging_buffer.slice(..);
            let (sender, receiver) = flume::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
            self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
            receiver.recv_async().await.unwrap().unwrap();

            self.timer.collect_results(&self.device, &self.queue);

            {
                let mut view = buffer_slice.get_mapped_range_mut();
                let padded_bytes_per_row =
                    Self::padded_bytes_per_row(self.output_dimensions.0) as usize;
                let bytes_per_row = (self.output_dimensions.0 * *U32_SIZE) as usize;

                // todo: find way to skip copying data (takes ~2ms for 4k frame)
                //          maybe do copy in compute shader to unpad?
                let mut padded_row_start = padded_bytes_per_row;
                let mut un_padded_row_start = bytes_per_row;
                for _ in 1..self.output_dimensions.1 {
                    view.copy_within(
                        padded_row_start..(padded_row_start + bytes_per_row),
                        un_padded_row_start,
                    );
                    padded_row_start += padded_bytes_per_row;
                    un_padded_row_start += bytes_per_row;
                }

                gdk_texture = gdk::MemoryTexture::new(
                    self.output_dimensions.0 as i32,
                    self.output_dimensions.1 as i32,
                    gdk::MemoryFormat::R8g8b8a8,
                    &glib::Bytes::from(&view.iter().as_slice()),
                    (self.output_dimensions.0 as i32 * 4) as usize,
                )
                .upcast::<gdk::Texture>();

                // if self.frame_count.get() % 48 == 0 {
                //     println!("SAVING IMG");
                //     let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                //         self.output_dimensions.0,
                //         self.output_dimensions.1,
                //         view,
                //     )
                //     .unwrap();
                //     image_buffer.save("test_image.png").unwrap();
                // }
                self.frame_count.set(self.frame_count.get() + 1);
            }

            self.output_staging_buffer.unmap();
        }

        Ok(gdk_texture)
    }
}
