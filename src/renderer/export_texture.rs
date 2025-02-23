use ash::vk;
use std::os::fd::RawFd;
use wgpu::hal;

pub(crate) struct ExportTexture {
    _raw_texture: vk::Image,
    device_memory: vk::DeviceMemory,
    pub texture: wgpu::Texture,
    pub fd: RawFd,
    device: wgpu::Device,
}

impl ExportTexture {
    pub fn new(device: &wgpu::Device, instance: &wgpu::Instance, size: wgpu::Extent3d) -> Self {
        let (raw_buffer, device_memory) = create_image(&device, size);
        let texture = upgrade_raw_image_to_wgpu(raw_buffer, size, &device);

        let mut fd = None;
        unsafe {
            device.as_hal::<hal::api::Vulkan, _, _>(|device| {
                if let Some(device) = device {
                    if let Some(instance) = instance.as_hal::<hal::api::Vulkan>() {
                        let raw_instance = instance.shared_instance().raw_instance();

                        let handle_info = vk::MemoryGetFdInfoKHR::default()
                            .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
                            .memory(device_memory);

                        let ash_device = ash::Device::load(
                            &raw_instance.fp_v1_0(),
                            device.raw_device().handle(),
                        );
                        let fd_device =
                            ash::khr::external_memory_fd::Device::new(raw_instance, &ash_device);
                        // get_memory_fd is slow. ~2ms avg
                        let raw_fd = fd_device.get_memory_fd(&handle_info).unwrap();

                        fd = Some(raw_fd as RawFd);
                    }
                }
            })
        };

        Self {
            _raw_texture: raw_buffer,
            device_memory,
            texture,
            device: device.clone(),
            fd: fd.unwrap(),
        }
    }
}

impl Drop for ExportTexture {
    fn drop(&mut self) {
        // todo: cleanup rawfd. or see if don't need to.
        self.texture.destroy();
        unsafe {
            self.device.as_hal::<hal::api::Vulkan, _, _>(|device| {
                if let Some(device) = device {
                    device.raw_device().free_memory(self.device_memory, None);
                }
            })
        };
    }
}

fn create_image(device: &wgpu::Device, size: wgpu::Extent3d) -> (vk::Image, vk::DeviceMemory) {
    let mut drm_form =
        vk::ImageDrmFormatModifierListCreateInfoEXT::default().drm_format_modifiers(&[0]);

    let mut external_image_info = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let image_create_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(
            vk::Extent3D::default()
                .width(size.width)
                .height(size.height)
                .depth(size.depth_or_array_layers),
        )
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
        .usage(vk::ImageUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .push_next(&mut external_image_info)
        .push_next(&mut drm_form);

    let mut allocation = None;
    let mut raw_image = None;

    unsafe {
        device.as_hal::<hal::api::Vulkan, _, _>(|device| {
            if let Some(device) = device {
                let raw_device = device.raw_device();
                let image = raw_device.create_image(&image_create_info, None).unwrap();
                let mem_req = raw_device.get_image_memory_requirements(image);

                let mut export_mem_alloc_info = vk::ExportMemoryAllocateInfo::default()
                    .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

                let mem_alloc_info = vk::MemoryAllocateInfo::default()
                    .push_next(&mut export_mem_alloc_info)
                    .allocation_size(mem_req.size);

                let device_memory = raw_device.allocate_memory(&mem_alloc_info, None).unwrap();

                raw_device
                    .bind_image_memory(image, device_memory, 0)
                    .expect("failed to bind image memory");

                raw_image = Some(image);
                allocation = Some(device_memory);
            }
        })
    };

    (raw_image.unwrap(), allocation.unwrap())
}

fn upgrade_raw_image_to_wgpu(
    image: vk::Image,
    size: wgpu::Extent3d,
    device: &wgpu::Device,
) -> wgpu::Texture {
    let format = wgpu::TextureFormat::Rgba8Unorm;

    let hal_texture = unsafe {
        <hal::api::Vulkan as hal::Api>::Device::texture_from_raw(
            image,
            &wgpu::hal::TextureDescriptor {
                label: Some("imported image"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                view_formats: vec![],
                usage: wgpu::hal::TextureUses::COPY_DST,
                memory_flags: wgpu::hal::MemoryFlags::empty(),
            },
            None,
        )
    };

    unsafe {
        device.create_texture_from_hal::<hal::api::Vulkan>(
            hal_texture,
            &wgpu::TextureDescriptor {
                label: Some("exported image"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                view_formats: &[],
                usage: wgpu::TextureUsages::COPY_DST,
            },
        )
    }
}
