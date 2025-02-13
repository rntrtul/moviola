use ash::vk;
use std::os::fd::RawFd;
use std::sync::Arc;
use vk_mem::Alloc;
use wgpu::hal;

pub(crate) struct ExportBuffer {
    raw_buffer: vk::Buffer,
    allocation: vk_mem::Allocation,
    device_memory: vk::DeviceMemory,
    allocator: Arc<vk_mem::Allocator>,
    pub buffer: wgpu::Buffer,
}

impl ExportBuffer {
    pub fn new(
        device: &wgpu::Device,
        allocator: &Arc<vk_mem::Allocator>,
        allocator_pool: &vk_mem::AllocatorPool,
        size: vk::DeviceSize,
    ) -> Self {
        let mut external_image_info = vk::ExternalMemoryBufferCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

        let buffer_create_info = vk::BufferCreateInfo::default()
            .push_next(&mut external_image_info)
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .size(size);

        let (raw_buffer, mut allocation) = unsafe {
            allocator_pool
                .create_buffer(
                    &buffer_create_info,
                    &vk_mem::AllocationCreateInfo {
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        ..Default::default()
                    },
                )
                .unwrap()
        };
        let allocation_info = allocator.get_allocation_info(&allocation);
        let device_memory = allocation_info.device_memory;
        let buffer = raw_buffer_to_wgpu(raw_buffer, size, &device);

        Self {
            raw_buffer,
            allocation,
            device_memory,
            buffer,
            allocator: allocator.clone(),
        }
    }

    pub fn fd(&self, device: &wgpu::Device, instance: &wgpu::Instance) -> RawFd {
        let mut fd = None;

        unsafe {
            device.as_hal::<hal::api::Vulkan, _, _>(|device| {
                if let Some(device) = device {
                    let handle_info = vk::MemoryGetFdInfoKHR::default()
                        .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
                        .memory(self.device_memory);

                    if let Some(instance) = instance.as_hal::<hal::api::Vulkan>() {
                        let raw_instance = instance.shared_instance().raw_instance();

                        let ash_device = ash::Device::load(
                            &raw_instance.fp_v1_0(),
                            device.raw_device().handle(),
                        );
                        let fd_device =
                            ash::khr::external_memory_fd::Device::new(raw_instance, &ash_device);
                        let raw_fd = fd_device.get_memory_fd(&handle_info).unwrap();

                        fd = Some(raw_fd as RawFd);
                    }
                }
            })
        };

        fd.unwrap()
    }
}

impl Drop for ExportBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
        unsafe {
            self.allocator.free_memory(&mut self.allocation);
        }
    }
}

fn raw_buffer_to_wgpu(
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
