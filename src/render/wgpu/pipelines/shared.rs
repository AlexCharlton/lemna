use super::super::context;
use wgpu;

pub trait VBDesc {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

pub fn create_pipeline(
    context: &context::WGPUContext,
    layout: &wgpu::PipelineLayout,
    vert: wgpu::ShaderModuleSource,
    frag: wgpu::ShaderModuleSource,
    primitive_topology: wgpu::PrimitiveTopology,
    vertex_state: wgpu::VertexStateDescriptor,
    msaa: bool,
    color_write_mask: wgpu::ColorWrite,
) -> wgpu::RenderPipeline {
    create_pipeline_depth_stencil(
        context,
        layout,
        vert,
        frag,
        primitive_topology,
        vertex_state,
        msaa,
        color_write_mask,
        Some(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::GreaterEqual,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilStateFaceDescriptor {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0xff,
                write_mask: 0,
            },
        }),
    )
}

pub fn create_pipeline_depth_stencil(
    context: &context::WGPUContext,
    layout: &wgpu::PipelineLayout,
    vert: wgpu::ShaderModuleSource,
    frag: wgpu::ShaderModuleSource,
    primitive_topology: wgpu::PrimitiveTopology,
    vertex_state: wgpu::VertexStateDescriptor,
    msaa: bool,
    color_write_mask: wgpu::ColorWrite,
    depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>,
) -> wgpu::RenderPipeline {
    let device = &context.device;

    let vs_module = device.create_shader_module(vert);
    let fs_module = device.create_shader_module(frag);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        label: None,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            ..Default::default()
        }),
        primitive_topology,
        color_states: &[wgpu::ColorStateDescriptor {
            format: context.swap_chain_desc.format,
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: color_write_mask,
        }],
        depth_stencil_state,
        vertex_state,
        sample_count: if msaa { context.sample_count } else { 1 },
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}

pub fn next_power_of_2(n: usize) -> usize {
    let mut n = n - 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n + 1
}
