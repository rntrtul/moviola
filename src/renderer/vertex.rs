use crate::ui::preview::Orientation;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 2], tex_coords: [f32; 2]) -> Self {
        Self {
            position,
            tex_coords,
        }
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub const INDICES: &[u16] = &[0, 3, 2, 0, 2, 1];

pub struct FrameRect {
    position: [[f32; 2]; 4],
    tex_coords: [[f32; 2]; 4],
}

impl FrameRect {
    pub fn new() -> Self {
        Self {
            position: [[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]],
            tex_coords: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        }
    }

    pub fn reset(&mut self) {
        self.tex_coords = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    }

    pub fn vertices(&self) -> [Vertex; 4] {
        [
            Vertex::new(self.position[0], self.tex_coords[0]),
            Vertex::new(self.position[1], self.tex_coords[1]),
            Vertex::new(self.position[2], self.tex_coords[2]),
            Vertex::new(self.position[3], self.tex_coords[3]),
        ]
    }

    pub fn orient(&mut self, orientation: Orientation) {
        self.reset();
        if orientation.absolute_angle() != 0.0 {
            let rotations = (orientation.absolute_angle() / 90.0) as usize;
            self.tex_coords.rotate_right(rotations);
        }

        if orientation.mirrored {
            self.tex_coords
                .chunks_exact_mut(2)
                .for_each(|chunk| chunk.reverse());
        }
    }
}
