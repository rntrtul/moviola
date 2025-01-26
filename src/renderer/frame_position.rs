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
    pub(crate) scale: [f32; 2],
    pub(crate) orientation: Orientation,
    pub(crate) straigthen_angle: f32,
    pub(crate) original_frame_size: FrameSize,
    output_frame_size: FrameSize,
}

impl FramePosition {
    pub fn new(frame_size: FrameSize) -> Self {
        Self {
            crop_edges: [0; 4],
            translate: [0; 2],
            scale: [1.0; 2],
            orientation: Orientation::default(),
            straigthen_angle: 0.0,
            original_frame_size: frame_size,
            output_frame_size: frame_size,
        }
    }

    pub fn buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let translate = mint::Vector2::from([
            self.translate[0] - self.crop_edges[0] as i32,
            self.translate[1] - self.crop_edges[1] as i32,
        ]);

        let uniform = FramePositionUniform {
            rotation: self.straigthen_angle,
            orientation: self.orientation.absolute_angle(),
            mirrored: if self.orientation.mirrored { 1 } else { 0 },
            translate,
            scale: mint::Vector2::from(self.scale),
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

    pub fn set_output_size(&mut self, output_size: FrameSize) {
        self.scale = [
            self.original_frame_size.width as f32 / output_size.width as f32,
            self.original_frame_size.height as f32 / output_size.height as f32,
        ];
        self.output_frame_size = output_size;
    }

    pub fn positioned_frame_size(&self) -> FrameSize {
        let (mut width, mut height) = self
            .orientation
            .oriented_size(self.output_frame_size.width, self.output_frame_size.height);
        width = width - (self.crop_edges[0] + self.crop_edges[2]);
        height = height - (self.crop_edges[1] + self.crop_edges[3]);

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
