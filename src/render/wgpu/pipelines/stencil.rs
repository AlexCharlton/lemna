use bytemuck::{cast_slice, Pod, Zeroable};
use log::info;
use wgpu::{self, util::DeviceExt};

use super::shared::{create_pipeline_depth_stencil, next_power_of_2, VBDesc};
use crate::base_types::{Point, Pos, Scale, AABB};
use crate::render::wgpu::context;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
}

impl VBDesc for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttributeDescriptor {
                format: wgpu::VertexFormat::Float2,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub pos: Pos,
    pub scale: Scale,
}

impl From<AABB> for Instance {
    fn from(aabb: AABB) -> Self {
        Self {
            pos: aabb.pos,
            scale: aabb.size(),
        }
    }
}

impl VBDesc for Instance {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float3,
                    offset: 0,
                    shader_location: 1,
                },
                wgpu::VertexAttributeDescriptor {
                    format: wgpu::VertexFormat::Float2,
                    offset: 4 * 3,
                    shader_location: 2,
                },
            ],
        }
    }
}

pub struct StencilPipeline {
    pipeline: wgpu::RenderPipeline,
    msaa_pipeline: wgpu::RenderPipeline,
    vertex_buff: wgpu::Buffer,
    index_buff: wgpu::Buffer,
    instance_data: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    num_instances: usize,
}

impl StencilPipeline {
    pub fn alloc_instance_buffer<'a: 'b, 'b>(
        &'a mut self,
        num_instances: usize,
        device: &'b wgpu::Device,
    ) {
        if num_instances > self.num_instances {
            self.num_instances = next_power_of_2(num_instances);
            info!(
                "Resizing StencilPipeline instance buffer to {}",
                self.num_instances
            );
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (std::mem::size_of::<Instance>() * self.num_instances) as u64,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });
        }
    }

    pub fn fill_buffers<'a: 'b, 'b>(&'a mut self, aabbs: &[AABB], queue: &'b mut wgpu::Queue) {
        self.instance_data.clear();
        for aabb in aabbs {
            self.instance_data.push((*aabb).into());
        }
        queue.write_buffer(&self.instance_buffer, 0, cast_slice(&self.instance_data));
    }

    pub fn render<'a: 'b, 'b>(
        &'a mut self,
        aabbs: &[AABB],
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
        pass.set_index_buffer(self.index_buff.slice(..));
        pass.draw_indexed(0..6 as u32, 0, 0..(aabbs.len() as u32));
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
                usage: wgpu::BufferUsage::VERTEX,
            });
        let index_buff = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: cast_slice(&index_data),
                usage: wgpu::BufferUsage::INDEX,
            });
        let num_instances = 32; // Initial allocation
        let instance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<Instance>() * num_instances) as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("stencil_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let depth_stencil_state_descriptor = wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::IncrementClamp,
                },
                back: wgpu::StencilStateFaceDescriptor {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::IncrementClamp,
                },
                read_mask: 0xff,
                write_mask: 0xff,
            },
        };

        Self {
            vertex_buff,
            index_buff,
            instance_data: vec![],
            instance_buffer,
            num_instances,
            pipeline: create_pipeline_depth_stencil(
                context,
                layout,
                wgpu::include_spirv!("shaders/stencil.vert.spv"),
                wgpu::include_spirv!("shaders/stencil.frag.spv"),
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[Vertex::desc(), Instance::desc()],
                },
                false,
                wgpu::ColorWrite::ALL,
                Some(depth_stencil_state_descriptor.clone()),
            ),
            msaa_pipeline: create_pipeline_depth_stencil(
                context,
                layout,
                wgpu::include_spirv!("shaders/stencil.vert.spv"),
                wgpu::include_spirv!("shaders/stencil.frag.spv"),
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[Vertex::desc(), Instance::desc()],
                },
                true,
                wgpu::ColorWrite::ALL,
                Some(depth_stencil_state_descriptor),
            ),
        }
    }
}
