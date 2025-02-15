use crate::renderer::renderer::U32_SIZE;
use crate::ui::preview::Orientation;
use encase::{ShaderType, UniformBuffer};
use wgpu::util::DeviceExt;

#[derive(ShaderType)]
pub struct FramePositionUniform {
    translate: mint::Vector2<i32>,
    scale: f32,
    rotation: f32,
    orientation: f32,
    mirrored: u32,
}

// todo: send a matrix for rotation and translate
#[derive(Copy, Clone, Debug)]
pub struct FramePosition {
    pub(crate) crop_edges: [u32; 4],
    pub(crate) translate: [i32; 2],
    pub(crate) scale: f32,
    pub(crate) orientation: Orientation,
    pub(crate) straigthen_angle: f32,
    pub(crate) original_frame_size: FrameSize,
}

impl FramePosition {
    pub fn new(frame_size: FrameSize) -> Self {
        Self {
            crop_edges: [0; 4],
            translate: [0; 2],
            scale: 1.0,
            orientation: Orientation::default(),
            straigthen_angle: 0.0,
            original_frame_size: frame_size,
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
            scale: self.scale,
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

    pub fn scale_for_output_size(&mut self, output_size: FrameSize) {
        self.scale = self.original_frame_size.width as f32 / output_size.width as f32;
    }

    pub fn output_frame_size(&self) -> FrameSize {
        let (mut width, mut height) = (
            (self.original_frame_size.width as f32 / self.scale) as u32,
            (self.original_frame_size.height as f32 / self.scale) as u32,
        );

        (width, height) = self.orientation.oriented_size(width, height);
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

    pub fn frame_buffer_size(&self, pixel_size: usize) -> usize {
        self.width as usize * pixel_size * self.height as usize
    }
}

impl From<wgpu::Extent3d> for FrameSize {
    fn from(extent: wgpu::Extent3d) -> Self {
        Self::new(extent.width, extent.height)
    }
}

impl Into<wgpu::Extent3d> for FrameSize {
    fn into(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}
