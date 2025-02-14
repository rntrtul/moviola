use ash::vk;
use std::os::fd::RawFd;
use wgpu::hal;

pub(crate) struct ExportBuffer {
    _raw_buffer: vk::Buffer,
    device_memory: vk::DeviceMemory,
    pub buffer: wgpu::Buffer,
    pub fd: RawFd,
    device: wgpu::Device,
}

impl ExportBuffer {
    pub fn new(device: &wgpu::Device, instance: &wgpu::Instance, size: vk::DeviceSize) -> Self {
        let (raw_buffer, device_memory) = create_buffer(&device, size);
        let buffer = upgrade_raw_buffer_to_wgpu(raw_buffer, size, &device);

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
            _raw_buffer: raw_buffer,
            device_memory,
            buffer,
            device: device.clone(),
            fd: fd.unwrap(),
        }
    }
}

impl Drop for ExportBuffer {
    fn drop(&mut self) {
        // todo: cleanup rawfd. or see if don't need to.
        self.buffer.destroy();
        unsafe {
            self.device.as_hal::<hal::api::Vulkan, _, _>(|device| {
                if let Some(device) = device {
                    device.raw_device().free_memory(self.device_memory, None);
                }
            })
        };
    }
}

fn create_buffer(device: &wgpu::Device, size: vk::DeviceSize) -> (vk::Buffer, vk::DeviceMemory) {
    let mut external_image_info = vk::ExternalMemoryBufferCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let buffer_create_info = vk::BufferCreateInfo::default()
        .push_next(&mut external_image_info)
        .usage(vk::BufferUsageFlags::TRANSFER_DST)
        .size(size);

    let mut allocation = None;
    let mut raw_buffer = None;

    unsafe {
        device.as_hal::<hal::api::Vulkan, _, _>(|device| {
            if let Some(device) = device {
                let raw_device = device.raw_device();
                let buffer = raw_device.create_buffer(&buffer_create_info, None).unwrap();

                let mut export_mem_alloc_info = vk::ExportMemoryAllocateInfo::default()
                    .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

                let mem_alloc_info = vk::MemoryAllocateInfo::default()
                    .push_next(&mut export_mem_alloc_info)
                    .allocation_size(size);

                let device_memory = raw_device.allocate_memory(&mem_alloc_info, None).unwrap();

                raw_device
                    .bind_buffer_memory(buffer, device_memory, 0)
                    .unwrap();

                raw_buffer = Some(buffer);
                allocation = Some(device_memory);
            }
        })
    };

    (raw_buffer.unwrap(), allocation.unwrap())
}

fn upgrade_raw_buffer_to_wgpu(
    buffer: vk::Buffer,
    buffer_size: vk::DeviceSize,
    device: &wgpu::Device,
) -> wgpu::Buffer {
    let hal_buffer = unsafe { <hal::api::Vulkan as hal::Api>::Device::buffer_from_raw(buffer) };

    unsafe {
        device.create_buffer_from_hal::<hal::api::Vulkan>(
            hal_buffer,
            &wgpu::BufferDescriptor {
                label: Some("exportable texture"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            },
        )
    }
}
