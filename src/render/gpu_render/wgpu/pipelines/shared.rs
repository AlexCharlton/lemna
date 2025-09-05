use super::super::context;
use wgpu;

pub trait VBDesc {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

pub fn create_pipeline(
    context: &context::WGPUContext,
    layout: &wgpu::PipelineLayout,
    frag: &wgpu::ShaderModule,
    primitive_topology: wgpu::PrimitiveTopology,
    vertex: wgpu::VertexState,
    msaa: bool,
    color_write_mask: wgpu::ColorWrites,
) -> wgpu::RenderPipeline {
    create_pipeline_depth_stencil(
        context,
        layout,
        frag,
        primitive_topology,
        vertex,
        msaa,
        color_write_mask,
        Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::GreaterEqual,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0xff,
                write_mask: 0,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
    )
}

pub fn create_pipeline_depth_stencil(
    context: &context::WGPUContext,
    layout: &wgpu::PipelineLayout,
    frag: &wgpu::ShaderModule,
    primitive_topology: wgpu::PrimitiveTopology,
    vertex: wgpu::VertexState,
    msaa: bool,
    color_write_mask: wgpu::ColorWrites,
    depth_stencil: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    let device = &context.device;

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        label: None,
        vertex,
        fragment: Some(wgpu::FragmentState {
            module: frag,
            entry_point: "main",
            targets: &[Some(wgpu::ColorTargetState {
                format: context.surface_config.format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                }),
                write_mask: color_write_mask,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: primitive_topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: if msaa { context.sample_count } else { 1 },
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}
