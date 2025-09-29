use bytemuck::{Pod, Zeroable};

use crate::base_types::{Color, Point, Pos, Rect, Scale};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
}

impl crate::render::gpu_render::VBDesc for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable, PartialEq)]
pub(crate) struct Instance {
    pub pos: Pos,
    pub scale: Scale,
    pub color: Color,
}

impl crate::render::gpu_render::VBDesc for Instance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 3,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 4 * 5,
                    shader_location: 3,
                },
            ],
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Rectangle {
    instance_data: Instance,
}

impl Rectangle {
    pub fn new(pos: Pos, scale: Scale, color: Color) -> Self {
        Self {
            instance_data: Instance { pos, scale, color },
        }
    }

    pub(crate) fn render(&self, rect: &Rect) -> Instance {
        let mut i = self.instance_data;
        i.pos += rect.pos;
        i
    }

    pub(crate) fn z(&self) -> f32 {
        self.instance_data.pos.z
    }
}
