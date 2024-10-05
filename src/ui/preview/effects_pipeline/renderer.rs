use crate::ui::preview::effects_pipeline::vertex::{INDICES, VERTICES};
use crate::ui::preview::effects_pipeline::{texture, vertex};
use wgpu::util::DeviceExt;

const OUTPUT_TEXTURE_DIMS: (usize, usize) = (512, 288);

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
            .request_device(&Default::default(), None)
            .await
            .unwrap();

        // todo: will have to create texture and bind group every time? or just update bindgroup entries?
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Texture Descriptor"),
            size: wgpu::Extent3d {
                width: OUTPUT_TEXTURE_DIMS.0 as u32,
                height: OUTPUT_TEXTURE_DIMS.0 as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });

        // todo: use size_of to calc size
        let output_texture_data =
            Vec::<u8>::with_capacity(OUTPUT_TEXTURE_DIMS.0 * OUTPUT_TEXTURE_DIMS.1 * 4);

        let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output staging Buffer"),
            size: output_texture_data.capacity() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let output_texture_view =
            render_target.create_view(&wgpu::TextureViewDescriptor::default());

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
                alpha_to_coverage_enabled: false, // anti-aliasing related. probably needed for crop box
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
        }
    }

    // todo: accept effect paramters
    pub fn prepare_video_frame_render_pass(&self) -> wgpu::CommandBuffer {
        // todo: determine format/type of video frame sent. meanwhile using this test img
        let diffuse_bytes = include_bytes!("test_orb in field.jpg");
        let frame_texture =
            texture::Texture::from_bytes(&self.device, &self.queue, diffuse_bytes, "orb jbpg")
                .unwrap();

        let frame_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.frame_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&frame_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&frame_texture.sampler),
                },
            ],
            label: Some("diffuse bind group"),
        });

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
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &frame_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

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
                    bytes_per_row: Some((OUTPUT_TEXTURE_DIMS.0 * 4) as u32),
                    rows_per_image: Some(OUTPUT_TEXTURE_DIMS.1 as u32),
                },
            },
            wgpu::Extent3d {
                width: OUTPUT_TEXTURE_DIMS.0 as u32,
                height: OUTPUT_TEXTURE_DIMS.1 as u32,
                depth_or_array_layers: 1,
            },
        );

        encoder.finish()
    }

    // todo: accept multiple command buffers
    pub async fn render(
        &self,
        command_buffer: wgpu::CommandBuffer,
    ) -> Result<(), wgpu::SurfaceError> {
        self.queue.submit(Some(command_buffer));
        {
            let buffer_slice = self.output_staging_buffer.slice(..);
            let (sender, receiver) = flume::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
            self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
            receiver.recv_async().await.unwrap().unwrap();
            {
                let mut output_texture_data =
                    Vec::<u8>::with_capacity(OUTPUT_TEXTURE_DIMS.0 * OUTPUT_TEXTURE_DIMS.1 * 4);
                let view = buffer_slice.get_mapped_range();
                output_texture_data.extend_from_slice(&view[..]);

                let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                    OUTPUT_TEXTURE_DIMS.0 as u32,
                    OUTPUT_TEXTURE_DIMS.1 as u32,
                    view,
                )
                .unwrap();
                image_buffer.save("test_image.png").unwrap();
            }

            self.output_staging_buffer.unmap();
        }
        Ok(())
    }
}
