use crate::renderer::renderer::U32_SIZE;
use crate::ui::preview::Orientation;
use encase::{ShaderType, UniformBuffer};
use wgpu::util::DeviceExt;

#[derive(ShaderType)]
pub struct FramePositionUniform {
    translate: mint::Vector2<i32>,
    scale: mint::Vector2<f32>,
    rotation: f32,
    orientation: f32,
    mirrored: u32,
}

// todo: send a matrix for rotation and translate
pub struct FramePosition {
    pub(crate) crop_edges: [u32; 4],
    pub(crate) translate: [i32; 2],
    pub(crate) orientation: Orientation,
    pub(crate) rotation_radians: f32,
    pub(crate) original_frame_size: FrameSize,
}

impl FramePosition {
    pub fn new() -> Self {
        Self {
            crop_edges: [0; 4],
            translate: [0; 2],
            orientation: Orientation::default(),
            rotation_radians: 0.0,
            original_frame_size: FrameSize::new(0, 0),
        }
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let scale = mint::Vector2::from([1.0, 1.0]);
        let translate = mint::Vector2::from([
            self.translate[0] - self.crop_edges[0] as i32,
            self.translate[1] - self.crop_edges[1] as i32,
        ]);

        let uniform = FramePositionUniform {
            rotation: self.rotation_radians,
            orientation: self.orientation.absolute_angle(),
            mirrored: if self.orientation.mirrored { 1 } else { 0 },
            translate,
            scale,
        };

        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&uniform).unwrap();
        let byte_buffer = buffer.into_inner();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Position Buffer"),
            contents: &*byte_buffer,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn positioned_frame_size(&self) -> FrameSize {
        // todo: get target output frame size and do math on that.
        let crop_width = self.original_frame_size.width - (self.crop_edges[0] + self.crop_edges[2]);
        let crop_height =
            self.original_frame_size.height - (self.crop_edges[1] + self.crop_edges[3]);
        let (width, height) = self.orientation.oriented_size(crop_width, crop_height);

        FrameSize::new(width, height)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

impl FrameSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn texture_size(&self) -> u64 {
        (self.width * self.height * U32_SIZE) as u64
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Frame Size Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}
