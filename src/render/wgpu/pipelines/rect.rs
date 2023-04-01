use bytemuck::{cast_slice, Pod, Zeroable};
use log::info;
use wgpu::{self, util::DeviceExt};

use super::shared::{create_pipeline, next_power_of_2, VBDesc};
use crate::base_types::{Color, Point, Pos, Scale, AABB};
use crate::render::wgpu::context;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
}

impl VBDesc for Vertex {
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
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Instance {
    pub pos: Pos,
    pub scale: Scale,
    pub color: Color,
}

impl VBDesc for Instance {
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

#[derive(Debug)]
pub struct Rect {
    instance_data: Instance,
}

impl Rect {
    pub fn new(pos: Pos, scale: Scale, color: Color) -> Self {
        Self {
            instance_data: Instance { pos, scale, color },
        }
    }

    fn render(&self, aabb: &AABB) -> Instance {
        let mut i = self.instance_data.clone();
        i.pos += aabb.pos;
        i
    }
}

pub struct RectPipeline {
    pipeline: wgpu::RenderPipeline,
    msaa_pipeline: wgpu::RenderPipeline,
    vertex_buff: wgpu::Buffer,
    index_buff: wgpu::Buffer,
    instance_data: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    num_instances: usize,
}

impl RectPipeline {
    pub fn alloc_instance_buffer<'a: 'b, 'b>(
        &'a mut self,
        num_instances: usize,
        device: &'b wgpu::Device,
    ) {
        if num_instances > self.num_instances {
            self.num_instances = next_power_of_2(num_instances);
            info!(
                "Resizing RectPipeline instance buffer to {}",
                self.num_instances
            );
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (std::mem::size_of::<Instance>() * self.num_instances) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
    }

    pub fn fill_buffers<'a: 'b, 'b>(
        &'a mut self,
        renderables: &[(&'a Rect, &'a AABB)],
        queue: &'b mut wgpu::Queue,
    ) {
        self.instance_data.clear();
        for (renderable, aabb) in renderables {
            self.instance_data.push(renderable.render(aabb))
        }
        queue.write_buffer(&self.instance_buffer, 0, cast_slice(&self.instance_data));
    }

    pub fn render<'a: 'b, 'b>(
        &'a mut self,
        renderables: &[(&'a Rect, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
        msaa: bool,
    ) {
        pass.set_pipeline(if msaa {
            &self.msaa_pipeline
        } else {
            &self.pipeline
        });
        pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        pass.set_vertex_buffer(
            1,
            self.instance_buffer
                .slice(((instance_offset * std::mem::size_of::<Instance>()) as u64)..),
        );
        pass.set_index_buffer(self.index_buff.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6 as u32, 0, 0..(renderables.len() as u32));
    }

    pub fn new(
        context: &context::WGPUContext,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let vertex_data = vec![
            Vertex {
                pos: [0.0, 0.0].into(),
            },
            Vertex {
                pos: [1.0, 0.0].into(),
            },
            Vertex {
                pos: [0.0, 1.0].into(),
            },
            Vertex {
                pos: [1.0, 1.0].into(),
            },
        ];
        let index_data: [u16; 6] = [0, 1, 2, 2, 1, 3];
        let vertex_buff = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buff = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX,
            });
        let num_instances = 32; // Initial allocation
        let instance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<Instance>() * num_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rect_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let vs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/rect.vert.spv"));
        let fs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/vert_color.frag.spv"));

        Self {
            vertex_buff,
            index_buff,
            instance_data: vec![],
            instance_buffer,
            num_instances,
            pipeline: create_pipeline(
                context,
                layout,
                &fs_module,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[Vertex::desc(), Instance::desc()],
                },
                false,
                wgpu::ColorWrites::ALL,
            ),
            msaa_pipeline: create_pipeline(
                context,
                layout,
                &fs_module,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[Vertex::desc(), Instance::desc()],
                },
                true,
                wgpu::ColorWrites::empty(),
            ),
        }
    }
}
