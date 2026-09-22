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
    /// Original traversal order, used as the stable tie-breaker for equal z.
    layer_order: Vec<LayerItem>,
    rects: Vec<(&'a Rectangle, &'a Rect)>,
    shapes: Vec<(&'a Shape, &'a Rect)>,
    num_shape_instances: usize,
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

    fn push_shape(&mut self, shape: &'a Shape, aabb: &'a Rect) {
        self.layer_order.push(LayerItem::Shape(self.shapes.len()));
        self.shapes.push((shape, aabb));
        self.num_shape_instances += shape.num_instances();
    }

    fn shape_instance_offset(&self, index: usize) -> usize {
        self.shapes[..index]
            .iter()
            .map(|(s, _)| s.num_instances())
            .sum()
    }
}

/// Drawables identified by index into a frame's type list.
#[derive(Clone, Copy)]
enum LayerItem {
    Rect(usize),
    Shape(usize),
    Raster(usize),
    Text(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayerBatchKind {
    OpaqueRect,
    TranslucentRect,
    OpaqueShape,
    TranslucentShape,
    Raster,
    Text,
}

#[derive(Clone, Copy)]
struct GlobalLayerItem {
    z: f32,
    frame_idx: usize,
    item: LayerItem,
}

fn collect_global_layer_items(frames: &[FrameRenderables<'_>]) -> Vec<GlobalLayerItem> {
    let mut items = Vec::new();
    for (frame_idx, frame) in frames.iter().enumerate() {
        for &item in &frame.layer_order {
            let z = match item {
                LayerItem::Rect(i) => {
                    let (r, aabb) = frame.rects[i];
                    r.z() + aabb.pos.z
                }
                LayerItem::Shape(i) => {
                    let (s, aabb) = frame.shapes[i];
                    s.z() + aabb.pos.z
                }
                LayerItem::Raster(i) => {
                    let (r, aabb) = frame.rasters[i];
                    r.z() + aabb.pos.z
                }
                LayerItem::Text(i) => {
                    let (t, aabb) = frame.texts[i];
                    t.z() + aabb.pos.z
                }
            };
            items.push(GlobalLayerItem { z, frame_idx, item });
        }
    }
    items.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
    items
}

fn layer_batch_kind(item: GlobalLayerItem, frames: &[FrameRenderables<'_>]) -> LayerBatchKind {
    let frame = &frames[item.frame_idx];
    match item.item {
        LayerItem::Rect(i) if frame.rects[i].0.is_opaque() => LayerBatchKind::OpaqueRect,
        LayerItem::Rect(_) => LayerBatchKind::TranslucentRect,
        LayerItem::Shape(i) if frame.shapes[i].0.is_opaque() => LayerBatchKind::OpaqueShape,
        LayerItem::Shape(_) => LayerBatchKind::TranslucentShape,
        LayerItem::Raster(_) => LayerBatchKind::Raster,
        LayerItem::Text(_) => LayerBatchKind::Text,
    }
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
        o.rects += frame.rects.len();
        o.shapes += frame.num_shape_instances;
        o.rasters += frame.rasters.len();
        o.texts += frame.texts.len();
    }
    offsets
}

#[derive(Clone, Copy)]
struct FrameOffsets {
    frames: usize,
    rects: usize,
    shapes: usize,
    rasters: usize,
    texts: usize,
}

enum LayerTarget<'a> {
    Surface(&'a wgpu::TextureView),
    Framebuffer,
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
                    let frame = frames.last_mut().unwrap();
                    frame.layer_order.push(LayerItem::Rect(frame.rects.len()));
                    frame.rects.push((r, aabb));
                    num_rects += 1;
                }
                Renderable::Shape(r) => {
                    frames.last_mut().unwrap().push_shape(r, aabb);
                    num_shapes += r.num_instances();
                }
                Renderable::Text(r) => {
                    let frame = frames.last_mut().unwrap();
                    frame.layer_order.push(LayerItem::Text(frame.texts.len()));
                    frame.texts.push((r, aabb));
                    num_texts += 1;
                }
                Renderable::Raster(r) => {
                    let frame = frames.last_mut().unwrap();
                    frame
                        .layer_order
                        .push(LayerItem::Raster(frame.rasters.len()));
                    frame.rasters.push((r, aabb));
                    num_rasters += 1;
                }
                #[cfg(test)]
                _ => panic!("Unsupported renderable: {:?}", renderable),
            }
        }
        let num_frames = frames.len();
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
                .flat_map(|f| f.rects.clone())
                .collect::<Vec<(&Rectangle, &Rect)>>(),
            &mut self.context.queue,
        );
        self.shape_pipeline.fill_buffers(
            &frames
                .iter()
                .flat_map(|f| f.shapes.clone())
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
        let antialiased_shapes = cfg!(feature = "antialiased_shapes");

        if antialiased_shapes {
            {
                let mut encoder =
                    self.context
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("aa layer encoder"),
                        });
                self.clear_layer_target(&mut encoder, LayerTarget::Framebuffer);
                self.encode_aa_layered_pass(&mut encoder, &frames, caches);
                command_buffers.push(encoder.finish());
            }
            {
                let mut encoder =
                    self.context
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("msaa composite encoder"),
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
        } else {
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("layer encoder"),
                    });
            self.clear_layer_target(&mut encoder, LayerTarget::Surface(&view));
            self.encode_layered_pass(&mut encoder, &view, &frames, caches);
            command_buffers.push(encoder.finish());
        }
        inst_end();

        inst("WGPURenderer::render#submit_command_buffers");
        self.context.queue.submit(command_buffers);
        self.context.queue.present(output);
        inst_end();
    }
}

impl WGPURenderer {
    /// Initialize the painter's-algorithm target and its non-MSAA depth/stencil.
    fn clear_layer_target(&mut self, encoder: &mut wgpu::CommandEncoder, target: LayerTarget<'_>) {
        let color_view = match target {
            LayerTarget::Surface(v) => v,
            LayerTarget::Framebuffer => &self.context.framebuffer,
        };
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.context.depthbuffer,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
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
            label: Some("layer target clear"),
        });
    }

    /// Draw every renderable in global z order into the offscreen framebuffer.
    /// Consecutive opaque shapes share one MSAA backdrop blit/resolve.
    fn encode_aa_layered_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frames: &[FrameRenderables<'_>],
        caches: &mut Caches,
    ) {
        let items = collect_global_layer_items(frames);
        if items.is_empty() {
            return;
        }
        let offsets = frame_buffer_offsets(frames);

        let mut msaa_depth_load = wgpu::LoadOp::Clear(0.0);

        let mut i = 0;
        while i < items.len() {
            match layer_batch_kind(items[i], frames) {
                LayerBatchKind::OpaqueShape => {
                    let start = i;
                    i += 1;
                    while i < items.len()
                        && layer_batch_kind(items[i], frames) == LayerBatchKind::OpaqueShape
                    {
                        i += 1;
                    }
                    self.encode_msaa_opaque_shape_run(
                        encoder,
                        frames,
                        &offsets,
                        caches,
                        &items[start..i],
                        msaa_depth_load,
                    );
                    msaa_depth_load = wgpu::LoadOp::Load;
                }
                _ => {
                    let start = i;
                    i += 1;
                    while i < items.len()
                        && layer_batch_kind(items[i], frames) != LayerBatchKind::OpaqueShape
                    {
                        i += 1;
                    }
                    self.encode_layer_run(
                        encoder,
                        LayerTarget::Framebuffer,
                        frames,
                        &offsets,
                        caches,
                        &items[start..i],
                    );
                }
            }
        }
    }

    /// One backdrop blit for a run of consecutive opaque shapes, then draw them
    /// (rebuilding stencil when the scroll frame changes). Resolve once at the end.
    fn encode_msaa_opaque_shape_run(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frames: &[FrameRenderables<'_>],
        offsets: &[FrameOffsets],
        caches: &mut Caches,
        run: &[GlobalLayerItem],
        msaa_depth_load: wgpu::LoadOp<f32>,
    ) {
        let shapes: Vec<(usize, usize)> = run
            .iter()
            .map(|item| match item {
                GlobalLayerItem {
                    frame_idx,
                    item: LayerItem::Shape(shape_idx),
                    ..
                } if frames[*frame_idx].shapes[*shape_idx].0.is_opaque() => {
                    (*frame_idx, *shape_idx)
                }
                _ => unreachable!(),
            })
            .collect();
        if shapes.is_empty() {
            return;
        }

        {
            let mut backdrop_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                label: Some("MSAA backdrop blit"),
            });
            self.msaa_pipeline.render_backdrop(&mut backdrop_pass);
        }

        // Split the run into same-frame groups so stencil can be rebuilt between
        // scroll frames without re-blitting the backdrop.
        let mut group_starts: Vec<usize> = vec![0];
        for i in 1..shapes.len() {
            if shapes[i].0 != shapes[i - 1].0 {
                group_starts.push(i);
            }
        }

        for (group_i, &start) in group_starts.iter().enumerate() {
            let end = group_starts
                .get(group_i + 1)
                .copied()
                .unwrap_or(shapes.len());
            let is_last = group_i + 1 == group_starts.len();
            let (frame_idx, _) = shapes[start];
            let frame = &frames[frame_idx];
            let off = offsets[frame_idx];

            let mut msaa_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.context.msaa_framebuffer,
                    depth_slice: None,
                    resolve_target: is_last.then_some(&self.context.framebuffer),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.context.msaa_depthbuffer,
                    depth_ops: Some(wgpu::Operations {
                        load: if group_i == 0 {
                            msaa_depth_load
                        } else {
                            wgpu::LoadOp::Load
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
                label: Some("MSAA opaque shape run"),
            });
            msaa_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            if !frame.frame.is_empty() {
                self.stencil_pipeline
                    .render(&frame.frame, &mut msaa_pass, off.frames, true);
            }
            msaa_pass.set_stencil_reference(frame.frame.len() as u32);

            let indices: Vec<usize> = shapes[start..end].iter().map(|&(_, si)| si).collect();
            let shape_base = off.shapes;
            self.shape_pipeline.render_selected(
                &frame.shapes,
                &indices,
                &mut msaa_pass,
                &mut caches.shape_buffer,
                |shape_idx| shape_base + frame.shape_instance_offset(shape_idx),
                true,
                false,
            );
        }
    }

    /// Draw a z-ordered run of non-MSAA items. Shares stencil setup per scroll
    /// frame and batches consecutive same-type draws into one pass.
    fn encode_layer_run(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: LayerTarget<'_>,
        frames: &[FrameRenderables<'_>],
        offsets: &[FrameOffsets],
        caches: &mut Caches,
        run: &[GlobalLayerItem],
    ) {
        if run.is_empty() {
            return;
        }

        let color_view = match target {
            LayerTarget::Surface(v) => v,
            LayerTarget::Framebuffer => &self.context.framebuffer,
        };

        let mut frame_starts: Vec<usize> = vec![0];
        for i in 1..run.len() {
            if run[i].frame_idx != run[i - 1].frame_idx {
                frame_starts.push(i);
            }
        }

        for (group_i, &start) in frame_starts.iter().enumerate() {
            let end = frame_starts.get(group_i + 1).copied().unwrap_or(run.len());
            let frame_idx = run[start].frame_idx;
            let frame = &frames[frame_idx];
            let off = offsets[frame_idx];

            {
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
            }

            let mut j = start;
            while j < end {
                let kind = layer_batch_kind(run[j], frames);
                let type_start = j;
                j += 1;
                while j < end && layer_batch_kind(run[j], frames) == kind {
                    j += 1;
                }
                let indices: Vec<usize> = run[type_start..j]
                    .iter()
                    .map(|item| match item.item {
                        LayerItem::Rect(i)
                        | LayerItem::Shape(i)
                        | LayerItem::Raster(i)
                        | LayerItem::Text(i) => i,
                    })
                    .collect();

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
                    label: Some("transparent type batch"),
                });
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_stencil_reference(frame.frame.len() as u32);

                match kind {
                    LayerBatchKind::OpaqueRect => {
                        self.rect_pipeline.render_selected(
                            &indices,
                            &mut pass,
                            |i| off.rects + i,
                            false,
                            false,
                        );
                    }
                    LayerBatchKind::TranslucentRect => {
                        self.rect_pipeline.render_selected(
                            &indices,
                            &mut pass,
                            |i| off.rects + i,
                            false,
                            true,
                        );
                    }
                    LayerBatchKind::OpaqueShape => {
                        self.shape_pipeline.render_selected(
                            &frame.shapes,
                            &indices,
                            &mut pass,
                            &mut caches.shape_buffer,
                            |i| off.shapes + frame.shape_instance_offset(i),
                            false,
                            false,
                        );
                    }
                    LayerBatchKind::TranslucentShape => {
                        self.shape_pipeline.render_selected(
                            &frame.shapes,
                            &indices,
                            &mut pass,
                            &mut caches.shape_buffer,
                            |i| off.shapes + frame.shape_instance_offset(i),
                            false,
                            true,
                        );
                    }
                    LayerBatchKind::Raster => {
                        self.raster_pipeline.render_selected(
                            &frame.rasters,
                            &indices,
                            &mut pass,
                            &caches.raster,
                            &caches.image_buffer,
                            |i| off.rasters + i,
                        );
                    }
                    LayerBatchKind::Text => {
                        self.text_pipeline.render_selected(
                            &frame.texts,
                            &indices,
                            &mut pass,
                            &mut caches.text_buffer,
                            |i| off.texts + i,
                        );
                    }
                }
            }
        }
    }

    /// Draw all items across every scroll frame in global back-to-front z order.
    fn encode_layered_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        frames: &[FrameRenderables<'_>],
        caches: &mut Caches,
    ) {
        let items = collect_global_layer_items(frames);
        if items.is_empty() {
            return;
        }
        let offsets = frame_buffer_offsets(frames);
        self.encode_layer_run(
            encoder,
            LayerTarget::Surface(color_view),
            frames,
            &offsets,
            caches,
            &items,
        );
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
