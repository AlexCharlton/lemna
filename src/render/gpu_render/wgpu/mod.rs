use std::fmt;

use futures::executor::block_on;
use wgpu::{self, util::DeviceExt};

mod context;

use crate::base_types::{PixelSize, Rect};
use crate::instrumenting::*;
use crate::node::{Node, ScrollFrame};
use crate::renderable::{Caches, Raster, Rectangle, Renderable, Shape, Text};
use crate::window::Window;

pub mod pipelines;
pub use pipelines::shared::VBDesc;
use pipelines::{
    RasterPipeline, RectPipeline, ShapePipeline, TextPipeline, msaa::MSAAPipeline,
    stencil::StencilPipeline,
};

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
    pub raster_pipeline: RasterPipeline,
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

#[derive(Default)]
struct FrameRenderables<'a> {
    frame: Vec<ScrollFrame>,
    opaque_rects: Vec<(&'a Rectangle, &'a Rect)>,
    translucent_rects: Vec<(&'a Rectangle, &'a Rect)>,
    opaque_shapes: Vec<(&'a Shape, &'a Rect)>,
    translucent_shapes: Vec<(&'a Shape, &'a Rect)>,
    num_opaque_shape_instances: usize,
    num_translucent_shape_instances: usize,
    /// Always drawn in the transparent pass.
    rasters: Vec<(&'a Raster, &'a Rect)>,
    /// Always drawn in the transparent pass.
    texts: Vec<(&'a Text, &'a Rect)>,
}

impl<'a> FrameRenderables<'a> {
    fn new(frame: Vec<ScrollFrame>) -> Self {
        Self {
            frame,
            ..Default::default()
        }
    }

    fn num_rects(&self) -> usize {
        self.opaque_rects.len() + self.translucent_rects.len()
    }

    fn num_shape_instances(&self) -> usize {
        self.num_opaque_shape_instances + self.num_translucent_shape_instances
    }

    fn push_shape(&mut self, shape: &'a Shape, aabb: &'a Rect) {
        let n = shape.num_instances();
        if shape.is_opaque() {
            self.opaque_shapes.push((shape, aabb));
            self.num_opaque_shape_instances += n;
        } else {
            self.translucent_shapes.push((shape, aabb));
            self.num_translucent_shape_instances += n;
        }
    }

    fn translucent_shape_instance_offset(&self, index: usize) -> usize {
        self.translucent_shapes[..index]
            .iter()
            .map(|(s, _)| s.num_instances())
            .sum()
    }
}

/// Transparent-pass drawables, identified by index into a frame's type list.
#[derive(Clone, Copy)]
enum TransparentItem {
    Rect(usize),
    Shape(usize),
    Raster(usize),
    Text(usize),
}

#[derive(Clone, Copy)]
struct GlobalTransparentItem {
    z: f32,
    frame_idx: usize,
    item: TransparentItem,
}

fn collect_global_transparent_items(frames: &[FrameRenderables<'_>]) -> Vec<GlobalTransparentItem> {
    let mut items = Vec::new();
    for (frame_idx, frame) in frames.iter().enumerate() {
        for (i, (r, aabb)) in frame.translucent_rects.iter().enumerate() {
            items.push(GlobalTransparentItem {
                z: r.z() + aabb.pos.z,
                frame_idx,
                item: TransparentItem::Rect(i),
            });
        }
        for (i, (s, aabb)) in frame.translucent_shapes.iter().enumerate() {
            items.push(GlobalTransparentItem {
                z: s.z() + aabb.pos.z,
                frame_idx,
                item: TransparentItem::Shape(i),
            });
        }
        for (i, (r, aabb)) in frame.rasters.iter().enumerate() {
            items.push(GlobalTransparentItem {
                z: r.z() + aabb.pos.z,
                frame_idx,
                item: TransparentItem::Raster(i),
            });
        }
        for (i, (t, aabb)) in frame.texts.iter().enumerate() {
            items.push(GlobalTransparentItem {
                z: t.z() + aabb.pos.z,
                frame_idx,
                item: TransparentItem::Text(i),
            });
        }
    }
    items.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
    items
}

fn frame_buffer_offsets(frames: &[FrameRenderables<'_>]) -> Vec<FrameOffsets> {
    let mut offsets = Vec::with_capacity(frames.len());
    let mut o = FrameOffsets {
        frames: 0,
        rects: 0,
        shapes: 0,
        rasters: 0,
        texts: 0,
    };
    for frame in frames {
        offsets.push(o);
        o.frames += frame.frame.len();
        o.rects += frame.num_rects();
        o.shapes += frame.num_shape_instances();
        o.rasters += frame.rasters.len();
        o.texts += frame.texts.len();
    }
    offsets
}

impl crate::render::Renderer for WGPURenderer {
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
            raster_pipeline: RasterPipeline::new(&context, &uniform_bind_group_layout),
            stencil_pipeline: StencilPipeline::new(&context, &uniform_bind_group_layout),
            context,
            uniform_bind_group,
            globals_ubo,
        }
    }

    fn render(&mut self, node: &Node, caches: &mut Caches, physical_size: PixelSize) {
        inst("WGPURenderer::render#get_current_texture");
        let was_resized = self.do_resize(physical_size);
        let output = match self.context.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(o)
            | wgpu::CurrentSurfaceTexture::Suboptimal(o) => o,
            wgpu::CurrentSurfaceTexture::Timeout => {
                evt("CurrentSurfaceTexture::Timeout");
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                evt("CurrentSurfaceTexture::Occluded");
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                evt("CurrentSurfaceTexture::Lost or Outdated");
                self.do_resize(self.context.size());
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                panic!("Failed to get current texture: validation error");
            }
        };
        inst_end();
        if was_resized {
            evt("WGPURenderer::was_resized");
            self.update_ubo(physical_size);
            self.context.queue.present(output);
            self.render(node, caches, physical_size);
            return;
        }

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.raster_pipeline.unmark_cache();
        caches.shape_buffer.unmark();
        caches.image_buffer.unmark();
        caches.text_buffer.unmark();
        caches.raster.unmark();

        inst("WGPURenderer::render#collect_frames");
        let mut frames = vec![FrameRenderables::default()];
        let mut num_rects = 0;
        let mut num_shapes = 0;
        let mut num_texts = 0;
        let mut num_rasters = 0;
        for (renderable, aabb, frame) in node.iter_renderables() {
            if frame != frames.last().unwrap().frame {
                frames.push(FrameRenderables::new(frame.clone()))
            }
            match renderable {
                Renderable::Rectangle(r) => {
                    if r.is_opaque() {
                        frames.last_mut().unwrap().opaque_rects.push((r, aabb));
                    } else {
                        frames.last_mut().unwrap().translucent_rects.push((r, aabb));
                    }
                    num_rects += 1;
                }
                Renderable::Shape(r) => {
                    frames.last_mut().unwrap().push_shape(r, aabb);
                    num_shapes += r.num_instances();
                }
                Renderable::Text(r) => {
                    frames.last_mut().unwrap().texts.push((r, aabb));
                    num_texts += 1;
                }
                Renderable::Raster(r) => {
                    frames.last_mut().unwrap().rasters.push((r, aabb));
                    num_rasters += 1;
                }
                #[cfg(test)]
                _ => panic!("Unsupported renderable: {:?}", renderable),
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
        self.raster_pipeline
            .alloc_instance_buffer(num_rasters, &self.context.device);
        self.text_pipeline
            .alloc_instance_buffer(num_texts, &self.context.device);
        inst_end();

        inst("WGPURenderer::render#fill_buffers");
        self.stencil_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.frame.clone())
                .collect::<Vec<Rect>>(),
            &mut self.context.queue,
        );
        self.rect_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.opaque_rects.iter().chain(f.translucent_rects.iter()))
                .copied()
                .collect::<Vec<(&Rectangle, &Rect)>>(),
            &mut self.context.queue,
        );
        self.shape_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.opaque_shapes.iter().chain(f.translucent_shapes.iter()))
                .copied()
                .collect::<Vec<(&Shape, &Rect)>>(),
            &self.context.device,
            &mut self.context.queue,
            &mut caches.shape_buffer,
        );
        self.text_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.texts.clone())
                .collect::<Vec<(&Text, &Rect)>>(),
            &self.context.device,
            &mut self.context.queue,
            &caches.font,
            &mut caches.text_buffer,
        );
        {
            let cache_invalid = self.raster_pipeline.update_texture_cache(
                &frames
                    .iter()
                    .flat_map(|f| f.rasters.clone())
                    .collect::<Vec<(&Raster, &Rect)>>(),
                &self.context.device,
                &mut self.context.queue,
                &mut caches.raster,
            );

            self.raster_pipeline.fill_buffers(
                &frames
                    .iter()
                    .flat_map(|f| f.rasters.clone())
                    .collect::<Vec<(&Raster, &Rect)>>(),
                &self.context.device,
                &mut self.context.queue,
                &mut caches.raster,
                &mut caches.image_buffer,
                cache_invalid,
            );
        }
        inst_end();

        inst("WGPURenderer::render#render_frames");
        let mut command_buffers: Vec<wgpu::CommandBuffer> = vec![];
        let mut load_op = wgpu::LoadOp::Clear(wgpu::Color::WHITE);
        let antialiased_shapes = cfg!(feature = "antialiased_shapes");

        num_frames = 0;
        num_rects = 0;
        num_shapes = 0;
        for frame_renderables in frames.iter() {
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("update encoder"),
                    });

            // --- Opaque pass (depth write on) ---
            {
                let base_color_view = if antialiased_shapes {
                    &self.context.framebuffer
                } else {
                    &view
                };
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: base_color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
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
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0),
                            store: wgpu::StoreOp::Store,
                        }),
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                    label: Some("opaque render pass"),
                });
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                if !frame_renderables.frame.is_empty() {
                    self.stencil_pipeline.render(
                        &frame_renderables.frame,
                        &mut pass,
                        num_frames,
                        false,
                    );
                }
                pass.set_stencil_reference(frame_renderables.frame.len() as u32);

                if !frame_renderables.opaque_rects.is_empty() {
                    self.rect_pipeline.render(
                        &frame_renderables.opaque_rects,
                        &mut pass,
                        num_rects,
                        false,
                        false,
                    );
                }
                if !frame_renderables.opaque_shapes.is_empty() {
                    self.shape_pipeline.render(
                        &frame_renderables.opaque_shapes,
                        &mut pass,
                        &mut caches.shape_buffer,
                        num_shapes,
                        false,
                        false,
                    );
                }
            }

            if antialiased_shapes {
                {
                    let mut backdrop_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &self.context.msaa_framebuffer,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                            multiview_mask: None,
                            label: Some("MSAA backdrop pass"),
                        });
                    self.msaa_pipeline.render_backdrop(&mut backdrop_pass);
                }

                let mut msaa_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.context.msaa_framebuffer,
                        depth_slice: None,
                        resolve_target: Some(&self.context.framebuffer),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
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
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0),
                            store: wgpu::StoreOp::Store,
                        }),
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                    label: Some("MSAA opaque shapes pass"),
                });

                msaa_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                if !frame_renderables.frame.is_empty() {
                    self.stencil_pipeline.render(
                        &frame_renderables.frame,
                        &mut msaa_pass,
                        num_frames,
                        true,
                    );
                }
                msaa_pass.set_stencil_reference(frame_renderables.frame.len() as u32);

                if !frame_renderables.opaque_rects.is_empty() {
                    self.rect_pipeline.render(
                        &frame_renderables.opaque_rects,
                        &mut msaa_pass,
                        num_rects,
                        true,
                        false,
                    );
                }
                if !frame_renderables.opaque_shapes.is_empty() {
                    self.shape_pipeline.render(
                        &frame_renderables.opaque_shapes,
                        &mut msaa_pass,
                        &mut caches.shape_buffer,
                        num_shapes,
                        true,
                        false,
                    );
                }
            }

            num_frames += frame_renderables.frame.len();
            num_rects += frame_renderables.num_rects();
            num_shapes += frame_renderables.num_shape_instances();

            command_buffers.push(encoder.finish());
            load_op = wgpu::LoadOp::Load;
        }

        if antialiased_shapes {
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
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                    label: Some("MSAA composite pass"),
                });

                self.msaa_pipeline.render_composite(&mut pass);
            }
            command_buffers.push(encoder.finish());
        }

        // Transparent pass is global across scroll frames so z-order matches the
        // CPU renderer (sort all translucent items, then clip per-frame).
        {
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("transparent pass encoder"),
                    });
            self.encode_transparent_pass(&mut encoder, &view, &frames, caches);
            command_buffers.push(encoder.finish());
        }
        inst_end();

        inst("WGPURenderer::render#submit_command_buffers");
        self.context.queue.submit(command_buffers);
        self.context.queue.present(output);
        inst_end();
    }
}

#[derive(Clone, Copy)]
struct FrameOffsets {
    frames: usize,
    rects: usize,
    shapes: usize,
    rasters: usize,
    texts: usize,
}

impl WGPURenderer {
    /// Draw all transparent items across every scroll frame, sorted back-to-front
    /// by z. When the scroll-frame clip stack changes, rebuild the stencil.
    fn encode_transparent_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        frames: &[FrameRenderables<'_>],
        caches: &mut Caches,
    ) {
        let items = collect_global_transparent_items(frames);
        if items.is_empty() {
            return;
        }
        let offsets = frame_buffer_offsets(frames);
        let mut active_frame: Option<usize> = None;

        for GlobalTransparentItem {
            frame_idx,
            item,
            ..
        } in items
        {
            let frame = &frames[frame_idx];
            let off = offsets[frame_idx];

            if active_frame != Some(frame_idx) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.context.depthbuffer,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(0),
                            store: wgpu::StoreOp::Store,
                        }),
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                    label: Some("transparent stencil setup"),
                });
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                if !frame.frame.is_empty() {
                    self.stencil_pipeline
                        .render(&frame.frame, &mut pass, off.frames, false);
                }
                active_frame = Some(frame_idx);
            }

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.context.depthbuffer,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
                label: Some("transparent item pass"),
            });
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_stencil_reference(frame.frame.len() as u32);

            let rect_base = off.rects + frame.opaque_rects.len();
            let shape_base = off.shapes + frame.num_opaque_shape_instances;

            match item {
                TransparentItem::Rect(i) => {
                    self.rect_pipeline.render(
                        &frame.translucent_rects[i..i + 1],
                        &mut pass,
                        rect_base + i,
                        false,
                        true,
                    );
                }
                TransparentItem::Shape(i) => {
                    let inst = shape_base + frame.translucent_shape_instance_offset(i);
                    self.shape_pipeline.render(
                        &frame.translucent_shapes[i..i + 1],
                        &mut pass,
                        &mut caches.shape_buffer,
                        inst,
                        false,
                        true,
                    );
                }
                TransparentItem::Raster(i) => {
                    self.raster_pipeline.render(
                        &frame.rasters[i..i + 1],
                        &mut pass,
                        &mut caches.raster,
                        &mut caches.image_buffer,
                        off.rasters + i,
                    );
                }
                TransparentItem::Text(i) => {
                    self.text_pipeline.render(
                        &frame.texts[i..i + 1],
                        &mut pass,
                        &self.context.device,
                        &mut caches.text_buffer,
                        off.texts + i,
                        false,
                    );
                }
            }
        }
    }

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
