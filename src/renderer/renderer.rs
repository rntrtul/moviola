use crate::renderer::frame_position::{FramePosition, FrameSize};
use crate::renderer::handler::TimerCmd;
use crate::renderer::presenter::Presenter;
use crate::renderer::texture::Texture;
use crate::renderer::timer::{GpuTimer, QuerySet};
use crate::renderer::{texture, EffectParameters, TimerEvent};
use crate::ui::preview::Orientation;
use ash::vk;
use gst::Sample;
use image::DynamicImage;
use relm4::gtk::gdk;
use std::cell::RefCell;
use std::default::Default;
use std::sync::{mpsc, LazyLock};
use std::time::Instant;
use wgpu::{hal, include_wgsl};

static INPUT_TEXTURE_USAGE: LazyLock<wgpu::TextureUsages> =
    LazyLock::new(|| wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING);

pub struct Renderer {
    output_size: FrameSize,
    frame_position: FramePosition,
    effect_parameters: EffectParameters,
    video_frame_texture: RefCell<texture::Texture>,
    frame_position_pipeline: wgpu::ComputePipeline,
    frame_position_bind_group_layout: wgpu::BindGroupLayout,
    frame_position_bind_group: wgpu::BindGroup,
    frame_position_buffer: wgpu::Buffer,
    positioned_frame: Texture,
    effects_pipeline: wgpu::ComputePipeline,
    effects_bind_group_layout: wgpu::BindGroupLayout,
    effects_bind_group: wgpu::BindGroup,
    effects_buffer: wgpu::Buffer,
    post_effects_frame: Texture,
    presenter: Presenter,
    pub(crate) gpu_timer: GpuTimer,
    timer: mpsc::Sender<TimerCmd>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,
    instance: wgpu::Instance,
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

        let (device, queue) = create_device_queue(
            &instance,
            &adapter,
            wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        );

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
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
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
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
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
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let effect_parameters = EffectParameters::new();
        let effects_buffer = effect_parameters.buffer(&device);

        let output_size = FrameSize::new(512, 288);
        let input_texture =
            Texture::new(&device, output_size.into(), *INPUT_TEXTURE_USAGE, None).unwrap();

        let frame_position = FramePosition::new(output_size);
        let presenter = Presenter::new(2, &device, &instance, output_size.into());

        let (
            positioned_frame_buffer,
            frame_position_buffer,
            frame_position_bind_group,
            effects_output_buffer,
            effects_bind_group,
        ) = Self::create_render_target(
            output_size,
            &effects_buffer,
            &effects_bind_group_layout,
            &frame_position_bind_group_layout,
            &frame_position,
            &input_texture,
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
            instance,
            adapter,
            device,
            queue,
            output_size,
            frame_position,
            effect_parameters,
            video_frame_texture: RefCell::new(input_texture),
            frame_position_pipeline,
            frame_position_bind_group_layout,
            frame_position_bind_group,
            frame_position_buffer,
            positioned_frame: positioned_frame_buffer,
            effects_pipeline,
            effects_bind_group_layout,
            effects_bind_group,
            effects_buffer,
            post_effects_frame: effects_output_buffer,
            presenter,
            gpu_timer: timer,
            timer: timer_sender,
        }
    }

    fn create_frame_positon_bind_groups(
        device: &wgpu::Device,
        frame_size: FrameSize,
        frame_position_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePosition,
        source_texture: &Texture,
    ) -> (Texture, wgpu::Buffer, wgpu::BindGroup) {
        let frame_position_buffer = frame_position.buffer(&device);

        let positioned_frame = Texture::new(
            device,
            frame_size.into(),
            wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
            Some("positioned frame"),
        )
        .unwrap();

        let frame_position_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame Position Bind Group"),
            layout: &frame_position_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        &source_texture.sampler.as_ref().unwrap(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: frame_position_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&positioned_frame.view),
                },
            ],
        });

        (
            positioned_frame,
            frame_position_buffer,
            frame_position_bind_group,
        )
    }

    fn create_effects_bind_groups(
        device: &wgpu::Device,
        frame_size: FrameSize,
        effects_bind_group_layout: &wgpu::BindGroupLayout,
        effect_parameters_buffer: &wgpu::Buffer,
        input_texture: &Texture,
    ) -> (Texture, wgpu::BindGroup) {
        let post_effects_frame = Texture::new(
            device,
            frame_size.into(),
            wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::STORAGE_BINDING,
            Some("post effects frame"),
        )
        .unwrap();

        let effects_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Effects Bind Group"),
            layout: &effects_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: effect_parameters_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&post_effects_frame.view),
                },
            ],
        });

        (post_effects_frame, effects_bind_group)
    }

    fn create_render_target(
        output_frame_size: FrameSize,
        effect_buffer: &wgpu::Buffer,
        effects_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position_bind_group_layout: &wgpu::BindGroupLayout,
        frame_position: &FramePosition,
        texture: &Texture,
        device: &wgpu::Device,
    ) -> (
        Texture,
        wgpu::Buffer,
        wgpu::BindGroup,
        Texture,
        wgpu::BindGroup,
    ) {
        let (positioned_frame, frame_position_buffer, frame_position_bind_group) =
            Self::create_frame_positon_bind_groups(
                &device,
                output_frame_size,
                &frame_position_bind_group_layout,
                &frame_position,
                &texture,
            );

        let (post_effects_frame, effects_bind_group) = Self::create_effects_bind_groups(
            &device,
            output_frame_size,
            &effects_bind_group_layout,
            &effect_buffer,
            &positioned_frame,
        );

        (
            positioned_frame,
            frame_position_buffer,
            frame_position_bind_group,
            post_effects_frame,
            effects_bind_group,
        )
    }

    fn update_render_target(&mut self, output_frame_size: FrameSize) {
        let (
            positioned_frame,
            frame_position_buffer,
            frame_postion_bind_group,
            post_effects_frame,
            effects_bind_group,
        ) = Self::create_render_target(
            output_frame_size,
            &self.effects_buffer,
            &self.effects_bind_group_layout,
            &self.frame_position_bind_group_layout,
            &self.frame_position,
            &self.video_frame_texture.borrow(),
            &self.device,
        );

        self.presenter
            .resize_outputs(&self.device, &self.instance, output_frame_size.into());
        self.positioned_frame = positioned_frame;
        self.frame_position_buffer = frame_position_buffer;
        self.frame_position_bind_group = frame_postion_bind_group;
        self.post_effects_frame = post_effects_frame;
        self.effects_bind_group = effects_bind_group;
    }

    pub fn is_size_equal_to_curr_input_size(&self, width: u32, height: u32) -> bool {
        let texture = &self.video_frame_texture.borrow().texture;
        width == texture.width() && height == texture.height()
    }

    fn update_input_texture_size(&mut self, width: u32, height: u32) {
        self.video_frame_texture.replace(
            Texture::new(
                &self.device,
                self.output_size.into(),
                *INPUT_TEXTURE_USAGE,
                Some("video frame texture"),
            )
            .unwrap(),
        );
        self.frame_position.original_frame_size = FrameSize::new(width, height);

        // need to update positioning bind group for new texture view.
        // todo: just update position/step (effects not changed by output size change)
        self.update_render_target(self.output_size);
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

        let mut rendered_frame = &self.positioned_frame;

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

            rendered_frame = &self.post_effects_frame;
        }

        let final_output = self.presenter.next_presentation_texture();

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &rendered_frame.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &final_output.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            final_output.texture.size(),
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

        self.timer
            .send(TimerCmd::Start(TimerEvent::TextureCreate, Instant::now()))
            .unwrap();

        let output = self.presenter.current_presentation_texture();

        let builder = gdk::DmabufTextureBuilder::new();
        builder.set_display(&gdk::Display::default().unwrap());
        builder.set_fourcc(875709016); // fixme: don't hardcode xbgr
        builder.set_modifier(0);
        builder.set_width(self.output_size.width);
        builder.set_height(self.output_size.height);
        builder.set_n_planes(1);
        builder.set_fd(0, output.fd);
        builder.set_offset(0, 0);
        builder.set_stride(0, 4);

        let gdk_texture = unsafe {
            // first build is very slow ~100ms
            builder.build().expect("unable to build texture")
        };

        self.timer
            .send(TimerCmd::Stop(TimerEvent::TextureCreate, Instant::now()))
            .unwrap();

        self.timer
            .send(TimerCmd::Start(TimerEvent::QueueEmpty, Instant::now()))
            .unwrap();
        self.device.poll(wgpu::Maintain::wait()).panic_on_timeout();
        self.timer
            .send(TimerCmd::Stop(TimerEvent::QueueEmpty, Instant::now()))
            .unwrap();

        self.gpu_timer
            .collect_query_results(&self.device, &self.queue);

        Ok(gdk_texture)
    }

    pub fn upload_new_sample(&mut self, sample: &Sample) {
        self.timer
            .send(TimerCmd::Start(TimerEvent::Renderer, Instant::now()))
            .unwrap();
        self.timer
            .send(TimerCmd::Start(TimerEvent::SampleImport, Instant::now()))
            .unwrap();
        let caps = sample.caps().expect("sample without caps");
        let info = gst_video::VideoInfo::from_caps(caps).expect("Failed to parse caps");

        if !self.is_size_equal_to_curr_input_size(info.width(), info.height()) {
            self.update_input_texture_size(info.width(), info.height());
            self.gpu_timer.reset();
        }
        self.sample_to_texture(sample);
        self.timer
            .send(TimerCmd::Stop(TimerEvent::SampleImport, Instant::now()))
            .unwrap();
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

fn create_device_queue(
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
    required_features: wgpu::Features,
) -> (wgpu::Device, wgpu::Queue) {
    let instance = unsafe {
        if let Some(instance) = instance.as_hal::<hal::api::Vulkan>() {
            instance.shared_instance().raw_instance()
        } else {
            panic!("Failed to get vulakn hal instance");
        }
    };

    let mut open_device = None;
    let all_features = adapter.features() | required_features;
    unsafe {
        adapter.as_hal::<hal::api::Vulkan, _, _>(|adapter| {
            if let Some(adapter) = adapter {
                let raw = adapter.raw_physical_device();

                let mut enabled_extensions = adapter.required_device_extensions(all_features);
                enabled_extensions.push(vk::EXT_EXTERNAL_MEMORY_DMA_BUF_NAME);
                enabled_extensions.push(vk::KHR_EXTERNAL_MEMORY_FD_NAME);
                enabled_extensions.push(vk::KHR_EXTERNAL_MEMORY_NAME);
                enabled_extensions.push(vk::EXT_IMAGE_DRM_FORMAT_MODIFIER_NAME);

                let mut enabled_phd_features =
                    adapter.physical_device_features(&enabled_extensions, all_features);

                let queue_create_info = vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(0)
                    .queue_priorities(&[1.0]);
                let queue_family_infos = [queue_create_info];

                let str_pointers = enabled_extensions
                    .iter()
                    .map(|&s| s.as_ptr())
                    .collect::<Vec<_>>();

                let pre_info = vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_family_infos)
                    .enabled_extension_names(&str_pointers);

                let device_create_info = enabled_phd_features.add_to_device_create(pre_info);

                let raw_device = instance
                    .create_device(raw, &device_create_info, None)
                    .expect("Failed to create device");

                open_device = Some(
                    adapter
                        .device_from_raw(
                            raw_device,
                            None,
                            &enabled_extensions,
                            required_features,
                            &wgpu::MemoryHints::Performance,
                            0,
                            0,
                        )
                        .expect("Failed to create adapter"),
                );
            }
        })
    };

    let (device, queue) = unsafe {
        adapter
            .create_device_from_hal(
                open_device.unwrap(),
                &wgpu::DeviceDescriptor {
                    required_features,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .expect("Failed to create device and queue from hal")
    };

    (device, queue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IMG_TEST_LANDSCAPE;
    use relm4::gtk::prelude::TextureExt;

    #[tokio::test]
    async fn render_to_image() {
        // fixme: find way to have a gdk display avail for dma output to work
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
