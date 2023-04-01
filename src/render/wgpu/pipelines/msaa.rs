use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{self, util::DeviceExt};

use super::shared::{create_pipeline_depth_stencil, VBDesc};
use crate::base_types::Point;
use crate::render::wgpu::context;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
    pub tex_pos: Point,
}

impl VBDesc for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 2,
                    shader_location: 1,
                },
            ],
        }
    }
}

pub struct MSAAPipeline {
    pipeline: wgpu::RenderPipeline,
    vertex_buff: wgpu::Buffer,
    index_buff: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl MSAAPipeline {
    pub fn render<'a: 'b, 'b>(&'a mut self, pass: &'b mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
        pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        pass.set_index_buffer(self.index_buff.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..1);
    }

    pub fn resize<'a: 'b, 'b>(
        &'a mut self,
        device: &'b wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            label: Some("msaa_sampler"),
            ..Default::default()
        });

        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("msaa_bind_group"),
        }));
    }

    pub fn new(context: &context::WGPUContext) -> Self {
        let vertex_data = vec![
            Vertex {
                pos: [-1.0, -1.0].into(),
                tex_pos: [0.0, 1.0].into(),
            },
            Vertex {
                pos: [1.0, -1.0].into(),
                tex_pos: [1.0, 1.0].into(),
            },
            Vertex {
                pos: [-1.0, 1.0].into(),
                tex_pos: [0.0, 0.0].into(),
            },
            Vertex {
                pos: [1.0, 1.0].into(),
                tex_pos: [1.0, 0.0].into(),
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

        let texture_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("msaa_bind_group_layout"),
                });

        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("msaa_pipeline_layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let vs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/msaa.vert.spv"));
        let fs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/msaa.frag.spv"));

        let mut r = Self {
            vertex_buff,
            index_buff,
            texture_bind_group_layout,
            bind_group: None,
            pipeline: create_pipeline_depth_stencil(
                context,
                layout,
                &fs_module,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[Vertex::desc()],
                },
                false,
                wgpu::ColorWrites::ALL,
                None,
            ),
        };
        r.resize(&context.device, &context.framebuffer);
        r
    }
}
