use std::fmt;

use cgmath;
use futures::executor::block_on;
use wgpu::{self, util::DeviceExt};

mod context;

use crate::base_types::{PixelSize, AABB};
use crate::font_cache::FontCache;
use crate::instrumenting::*;
use crate::node::{Node, ScrollFrame};
use crate::window::Window;

pub mod pipelines;
use pipelines::{
    msaa::MSAAPipeline, stencil::StencilPipeline, RectPipeline, ShapePipeline, TextPipeline,
};
pub use pipelines::{Rect, Shape, Text};

#[repr(C)]
#[derive(Clone, Copy)]
struct Globals {
    pub viewport: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Globals {}
unsafe impl bytemuck::Zeroable for Globals {}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub const MAX_DEPTH: f32 = 10000.0;

pub struct WGPURenderer {
    pub rect_pipeline: RectPipeline,
    pub msaa_pipeline: MSAAPipeline,
    pub shape_pipeline: ShapePipeline,
    pub text_pipeline: TextPipeline,
    stencil_pipeline: StencilPipeline,
    context: context::WGPUContext,
    uniform_bind_group: wgpu::BindGroup,
    globals_ubo: wgpu::Buffer,
}

impl fmt::Debug for WGPURenderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WGPURenderer")?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum WGPURenderable {
    Rect(Rect),
    Shape(Shape),
    Text(Text),
}

#[derive(Default)]
struct FrameRenderables<'a> {
    frame: Vec<ScrollFrame>,
    rects: Vec<(&'a Rect, &'a AABB)>,
    shapes: Vec<(&'a Shape, &'a AABB)>,
    num_shape_instances: usize,
    texts: Vec<(&'a Text, &'a AABB)>,
}

impl<'a> FrameRenderables<'a> {
    fn new(frame: Vec<ScrollFrame>) -> Self {
        Self {
            frame,
            ..Default::default()
        }
    }
}

impl super::Renderer for WGPURenderer {
    type Renderable = WGPURenderable;

    fn new<W: Window>(window: &W) -> Self {
        let size = window.physical_size();
        let context = block_on(context::get_wgpu_context(
            window,
            // This ensures that the first render will always resize, which resolves issues on some backends
            size.width - 1,
            size.height - 1,
        ));
        let device = &context.device;

        let uniform_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("globals_bind_group_layout"),
                });

        let globals_ubo = device.create_buffer(&wgpu::BufferDescriptor {
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("globals_globals_ubo"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_ubo.as_entire_binding(),
            }],
            label: Some("globals_uniform_bind_group"),
        });

        Self {
            rect_pipeline: RectPipeline::new(&context, &uniform_bind_group_layout),
            msaa_pipeline: MSAAPipeline::new(&context),
            shape_pipeline: ShapePipeline::new(&context, &uniform_bind_group_layout),
            text_pipeline: TextPipeline::new(&context, &uniform_bind_group_layout),
            stencil_pipeline: StencilPipeline::new(&context, &uniform_bind_group_layout),
            context,
            uniform_bind_group,
            globals_ubo,
        }
    }

    fn render(&mut self, node: &Node<Self>, physical_size: PixelSize, font_cache: &FontCache) {
        inst("WGPURenderer::render#get_current_texture");
        let was_resized = self.do_resize(physical_size);
        let output = match self.context.surface.get_current_texture() {
            Ok(o) => o,
            Err(wgpu::SurfaceError::Timeout) => {
                evt("SurfaceError::Timeout");
                return;
            }
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                evt("SurfaceError::Lost or Outdated");
                self.do_resize(self.context.size());
                return;
            }
            Err(e) => panic!("Failed to get current texture: {}", e),
        };
        inst_end();
        if was_resized {
            evt("WGPURenderer::was_resized");
            self.update_ubo(physical_size);
            output.present();
            self.render(node, physical_size, font_cache);
            return;
        }

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.text_pipeline.unmark_buffer_cache();
        self.shape_pipeline.unmark_buffer_cache();

        inst("WGPURenderer::render#collect_frames");
        let mut frames = vec![FrameRenderables::default()];
        let mut num_rects = 0;
        let mut num_shapes = 0;
        let mut num_texts = 0;
        for (renderable, aabb, frame) in node.iter_renderables() {
            if frame != frames.last().unwrap().frame {
                frames.push(FrameRenderables::new(frame.clone()))
            }
            match renderable {
                WGPURenderable::Rect(r) => {
                    frames.last_mut().unwrap().rects.push((r, aabb));
                    num_rects += 1;
                }
                WGPURenderable::Shape(r) => {
                    frames.last_mut().unwrap().shapes.push((r, aabb));
                    if r.is_filled() {
                        frames.last_mut().unwrap().num_shape_instances += 1;
                        num_shapes += 1;
                    }
                    if r.is_stroked() {
                        frames.last_mut().unwrap().num_shape_instances += 1;
                        num_shapes += 1;
                    }
                }
                WGPURenderable::Text(r) => {
                    frames.last_mut().unwrap().texts.push((r, aabb));
                    num_texts += 1;
                }
            }
        }
        let mut num_frames = frames.len();
        inst_end();

        inst("WGPURenderer::render#alloc_buffers");
        self.stencil_pipeline
            .alloc_instance_buffer(num_frames, &self.context.device);
        self.rect_pipeline
            .alloc_instance_buffer(num_rects, &self.context.device);
        self.shape_pipeline
            .alloc_instance_buffer(num_shapes, &self.context.device);
        self.text_pipeline
            .alloc_instance_buffer(num_texts, &self.context.device);
        inst_end();

        inst("WGPURenderer::render#fill_buffers");
        self.stencil_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.frame.clone())
                .collect::<Vec<AABB>>(),
            &mut self.context.queue,
        );
        self.rect_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.rects.clone())
                .collect::<Vec<(&Rect, &AABB)>>(),
            &mut self.context.queue,
        );
        self.shape_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.shapes.clone())
                .collect::<Vec<(&Shape, &AABB)>>(),
            &self.context.device,
            &mut self.context.queue,
        );
        self.text_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.texts.clone())
                .collect::<Vec<(&Text, &AABB)>>(),
            &self.context.device,
            &mut self.context.queue,
            font_cache,
        );
        inst_end();

        inst("WGPURenderer::render#render_frames");
        let mut command_buffers: Vec<wgpu::CommandBuffer> = vec![];
        let mut load_op = wgpu::LoadOp::Clear(wgpu::Color::WHITE);
        num_frames = 0;
        num_rects = 0;
        num_shapes = 0;
        num_texts = 0;
        for frame_renderables in frames.iter() {
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("update encoder"),
                    });
            {
                // Non-MSAA pass
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.context.depthbuffer,
                        depth_ops: Some(wgpu::Operations {
                            load: if load_op == wgpu::LoadOp::Load {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(0.0)
                            },
                            store: true,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0),
                            store: true,
                        }),
                    }),
                    label: Some("non-MSAA render pass"),
                });
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                // Each frame increments the stencil buffer.
                if !frame_renderables.frame.is_empty() {
                    self.stencil_pipeline.render(
                        &frame_renderables.frame,
                        &mut pass,
                        num_frames,
                        false,
                    );
                }
                // We only want the top frame in a given pass:
                pass.set_stencil_reference(frame_renderables.frame.len() as u32);

                if !frame_renderables.rects.is_empty() {
                    self.rect_pipeline.render(
                        &frame_renderables.rects,
                        &mut pass,
                        num_rects,
                        false,
                    );
                }
                if !frame_renderables.shapes.is_empty() {
                    self.shape_pipeline.render(
                        &frame_renderables.shapes,
                        &mut pass,
                        num_shapes,
                        false,
                    );
                }
                // Text comes last because of transparency
                if !frame_renderables.texts.is_empty() {
                    self.text_pipeline.render(
                        &frame_renderables.texts,
                        &mut pass,
                        num_texts,
                        false,
                    );
                }
            }

            if cfg!(feature = "msaa_shapes") {
                let mut msaa_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.context.msaa_framebuffer,
                        resolve_target: Some(&self.context.framebuffer),
                        ops: wgpu::Operations {
                            load: if load_op == wgpu::LoadOp::Load {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                            },
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.context.msaa_depthbuffer,
                        depth_ops: Some(wgpu::Operations {
                            load: if load_op == wgpu::LoadOp::Load {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(0.0)
                            },
                            store: true,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0),
                            store: true,
                        }),
                    }),
                    label: Some("MSAA shapes render pass"),
                });

                msaa_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                // Each frame increments the stencil buffer.
                if !frame_renderables.frame.is_empty() {
                    self.stencil_pipeline.render(
                        &frame_renderables.frame,
                        &mut msaa_pass,
                        num_frames,
                        true,
                    );
                }
                // // We only want the top frame in a given pass:
                msaa_pass.set_stencil_reference(frame_renderables.frame.len() as u32);

                if !frame_renderables.rects.is_empty() {
                    self.rect_pipeline.render(
                        &frame_renderables.rects,
                        &mut msaa_pass,
                        num_rects,
                        true,
                    );
                }
                if !frame_renderables.texts.is_empty() {
                    self.text_pipeline.render(
                        &frame_renderables.texts,
                        &mut msaa_pass,
                        num_texts,
                        true,
                    );
                }
                // Shape comes last because we don't want to render fragments that
                // are covered by others
                if !frame_renderables.shapes.is_empty() {
                    self.shape_pipeline.render(
                        &frame_renderables.shapes,
                        &mut msaa_pass,
                        num_shapes,
                        true,
                    );
                }
            }

            num_frames += frame_renderables.frame.len();
            num_rects += frame_renderables.rects.len();
            num_shapes += frame_renderables.num_shape_instances;
            num_texts += frame_renderables.texts.len();

            command_buffers.push(encoder.finish());
            // All depth & color loads after the first should not clear
            load_op = wgpu::LoadOp::Load;
        }

        // Draw the results of the MSAA'd framebuffer
        if cfg!(feature = "msaa_shapes") {
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("update encoder"),
                    });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                    label: Some("MSAA render pass"),
                });

                self.msaa_pipeline.render(&mut pass);
            }
            command_buffers.push(encoder.finish());
        }
        inst_end();

        inst("WGPURenderer::render#submit_command_buffers");
        self.context.queue.submit(command_buffers.into_iter());
        output.present();
        inst_end();
    }
}

impl WGPURenderer {
    fn do_resize(&mut self, size: PixelSize) -> bool {
        if size.width != self.context.surface_config.width
            || size.height != self.context.surface_config.height
        {
            inst("WGPURenderer::resize_context");
            self.context.resize(size.width, size.height);
            self.msaa_pipeline
                .resize(&self.context.device, &self.context.framebuffer);
            inst_end();
            true
        } else {
            false
        }
    }

    fn update_ubo(&mut self, physical_size: PixelSize) {
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("update encoder"),
                });
        let globals_staging_buffer =
            self.context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&[Globals {
                        viewport: OPENGL_TO_WGPU_MATRIX
                // Viewport is (0,0) at top left and (width, height) at bottom right
                // Depth goes from 0 (far) to MAX_DEPTH (near)
                    * cgmath::ortho(
                        0.0,
                        physical_size.width as f32,
                        physical_size.height as f32,
                        0.0,
                        0.0,
                        -MAX_DEPTH,
                    ),
                    }]),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });
        encoder.copy_buffer_to_buffer(
            &globals_staging_buffer,
            0,
            &self.globals_ubo,
            0,
            std::mem::size_of::<Globals>() as wgpu::BufferAddress,
        );
        self.context.queue.submit(Some(encoder.finish()));
    }
}
