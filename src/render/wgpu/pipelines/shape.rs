use std::fmt;
use std::ops::Range;

use bytemuck::{cast_slice, Pod, Zeroable};
use log::info;
use lyon;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::VertexBuffers;
use lyon::tessellation::math as lyon_math;
use wgpu;

use super::buffer_cache::{BufferCache, BufferCacheId};
use super::shared::{create_pipeline, next_power_of_2, VBDesc};
use crate::base_types::{Color, Point, Pos, AABB};
use crate::render::wgpu::context;

pub type ShapeGeometry = VertexBuffers<Vertex, u16>;
pub const TOLERANCE: f32 = 0.2;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
    pub norm: Point,
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

impl Vertex {
    pub fn basic_vertex_constructor(position: lyon_math::Point) -> Vertex {
        Vertex {
            pos: Point {
                x: position.x,
                y: position.y,
            },
            norm: Point { x: 0.0, y: 0.0 },
        }
    }

    pub fn fill_vertex_constructor(
        position: lyon_math::Point,
        _attributes: tessellation::FillAttributes,
    ) -> Vertex {
        Vertex {
            pos: Point {
                x: position.x,
                y: position.y,
            },
            norm: Point { x: 0.0, y: 0.0 },
        }
    }

    pub fn stroke_vertex_constructor(
        position: lyon_math::Point,
        attributes: tessellation::StrokeAttributes,
    ) -> Vertex {
        Vertex {
            pos: Point {
                x: position.x,
                y: position.y,
            },
            norm: Point {
                x: attributes.normal().x,
                y: attributes.normal().y,
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Instance {
    pub pos: Pos,
    pub color: Color,
    pub stroke_width: f32,
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
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 4 * 3,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 4 * 7,
                    shader_location: 4,
                },
            ],
        }
    }
}

pub struct Shape {
    fill_color: Color,
    stroke_color: Color,
    stroke_width: f32,
    fill_range: Range<u32>,
    stroke_range: Range<u32>,
    z: f32,
    pub buffer_id: BufferCacheId,
}

impl fmt::Debug for Shape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<Shape with fill {:?} and stroke {} {:?}>",
            self.fill_color, self.stroke_width, self.stroke_color
        )?;
        Ok(())
    }
}

impl Shape {
    pub fn is_stroked(&self) -> bool {
        self.stroke_width > 0.0
    }

    pub fn is_filled(&self) -> bool {
        self.fill_range.start < self.fill_range.end
    }

    pub fn fill_options() -> tessellation::FillOptions {
        tessellation::FillOptions::tolerance(TOLERANCE)
    }

    pub fn stroke_options() -> tessellation::StrokeOptions {
        tessellation::StrokeOptions::tolerance(TOLERANCE).dont_apply_line_width()
    }

    pub fn path_to_shape_geometry(path: Path, fill: bool, stroke: bool) -> (ShapeGeometry, u32) {
        let mut geometry = ShapeGeometry::new();

        let fill_count = if fill {
            tessellation::FillTessellator::new()
                .tessellate_path(
                    &path,
                    &Shape::fill_options(),
                    &mut tessellation::BuffersBuilder::new(
                        &mut geometry,
                        Vertex::fill_vertex_constructor,
                    ),
                )
                .unwrap()
                .indices
        } else {
            0
        };
        if stroke {
            tessellation::StrokeTessellator::new()
                .tessellate_path(
                    &path,
                    &Shape::stroke_options(),
                    &mut tessellation::BuffersBuilder::new(
                        &mut geometry,
                        Vertex::stroke_vertex_constructor,
                    ),
                )
                .unwrap();
        }

        (geometry, fill_count)
    }

    pub fn new(
        geometry: ShapeGeometry,
        fill_index_count: u32,
        fill_color: Color,
        stroke_color: Color,
        stroke_width: f32,
        z: f32,
        renderer: &mut ShapePipeline,
        prev_buffer: Option<BufferCacheId>,
    ) -> Self {
        let buffer_id = if let Some(c) = prev_buffer {
            renderer.buffer_cache.alloc_or_reuse_chunk(
                c,
                geometry.vertices.len(),
                geometry.indices.len(),
            )
        } else {
            assert!(
                geometry.vertices.len() + geometry.indices.len() != 0,
                "Cannot create an empty shape"
            );
            renderer
                .buffer_cache
                .alloc_chunk(geometry.vertices.len(), geometry.indices.len())
        };

        let (vertex_chunk, index_chunk) = renderer.buffer_cache.get_chunks(buffer_id);
        renderer.buffer_cache.vertex_data
            [vertex_chunk.start..(vertex_chunk.start + vertex_chunk.n)]
            .copy_from_slice(&geometry.vertices);
        renderer.buffer_cache.index_data[index_chunk.start..(index_chunk.start + index_chunk.n)]
            .copy_from_slice(&geometry.indices);

        Self {
            fill_color,
            stroke_color,
            stroke_width,
            fill_range: 0..fill_index_count,
            stroke_range: fill_index_count..(geometry.indices.len() as u32),
            z,
            buffer_id,
        }
    }

    pub fn stroke(
        geometry: ShapeGeometry,
        color: Color,
        stroke_width: f32,
        z: f32,
        renderer: &mut ShapePipeline,
        prev_buffer: Option<BufferCacheId>,
    ) -> Self {
        let buffer_id = if let Some(c) = prev_buffer {
            renderer.buffer_cache.alloc_or_reuse_chunk(
                c,
                geometry.vertices.len(),
                geometry.indices.len(),
            )
        } else {
            renderer
                .buffer_cache
                .alloc_chunk(geometry.vertices.len(), geometry.indices.len())
        };

        let (vertex_chunk, index_chunk) = renderer.buffer_cache.get_chunks(buffer_id);
        renderer.buffer_cache.vertex_data
            [vertex_chunk.start..(vertex_chunk.start + vertex_chunk.n)]
            .copy_from_slice(&geometry.vertices);
        renderer.buffer_cache.index_data[index_chunk.start..(index_chunk.start + index_chunk.n)]
            .copy_from_slice(&geometry.indices);

        Self {
            fill_color: color,
            stroke_color: color,
            stroke_width,
            fill_range: 0..0,
            stroke_range: 0..(geometry.indices.len() as u32),
            z,
            buffer_id,
        }
    }

    fn render(&self, aabb: &AABB, buffer_cache: &mut BufferCache<Vertex, u16>) -> Vec<Instance> {
        buffer_cache.register(self.buffer_id);
        let mut ret = vec![];
        let mut pos = aabb.pos;
        pos.z += self.z;
        if self.is_filled() {
            ret.push(Instance {
                pos,
                color: self.fill_color,
                stroke_width: 0.0,
            });
        }
        if self.is_stroked() {
            ret.push(Instance {
                pos,
                color: self.stroke_color,
                stroke_width: self.stroke_width,
            });
        }
        ret
    }
}

pub struct ShapePipeline {
    pipeline: wgpu::RenderPipeline,
    msaa_pipeline: wgpu::RenderPipeline,
    buffer_cache: BufferCache<Vertex, u16>,
    instance_data: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    num_instances: usize,
}

impl ShapePipeline {
    pub(crate) fn unmark_buffer_cache(&mut self) {
        self.buffer_cache.unmark();
    }

    fn draw_renderables<'a: 'b, 'b>(
        &'a self,
        renderables: &[(&'a Shape, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        msaa: bool,
        instance_offset: usize,
    ) {
        let mut i = 0;
        for (renderable, _) in renderables.iter() {
            let (vertex_chunk, index_chunk) = self.buffer_cache.get_chunks(renderable.buffer_id);

            pass.set_vertex_buffer(
                0,
                self.buffer_cache
                    .vertex_buffer
                    .slice(((vertex_chunk.start * std::mem::size_of::<Vertex>()) as u64)..),
            );
            pass.set_vertex_buffer(
                1,
                self.instance_buffer
                    .slice((((i + instance_offset) * std::mem::size_of::<Instance>()) as u64)..),
            );
            pass.set_index_buffer(
                self.buffer_cache
                    .index_buffer
                    .slice(((index_chunk.start * std::mem::size_of::<u16>()) as u64)..),
                wgpu::IndexFormat::Uint16,
            );
            if renderable.is_filled() {
                pass.draw_indexed(renderable.fill_range.clone(), 0, 0..1);
                i += 1;
            }
            if renderable.is_stroked() {
                // Don't draw stroked lines unless doing the MSAA pass
                if msaa || !cfg!(feature = "msaa_shapes") {
                    let instances = if renderable.is_filled() { 1..2 } else { 0..1 };
                    pass.draw_indexed(renderable.stroke_range.clone(), 0, instances);
                }
                i += 1;
            }
        }
    }

    pub fn alloc_instance_buffer<'a: 'b, 'b>(
        &'a mut self,
        num_instances: usize,
        device: &'b wgpu::Device,
    ) {
        if num_instances > self.num_instances {
            self.num_instances = next_power_of_2(num_instances);
            info!(
                "Resizing ShapePipeline instance buffer to {}",
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
        renderables: &[(&'a Shape, &'a AABB)],
        device: &'b wgpu::Device,
        queue: &'b mut wgpu::Queue,
    ) {
        self.instance_data.clear();

        let mut cache_changed = false;
        for (renderable, aabb) in renderables {
            self.instance_data
                .extend(renderable.render(aabb, &mut self.buffer_cache));
            let (vertex_chunk, _) = self.buffer_cache.get_chunks(renderable.buffer_id);
            cache_changed |= !vertex_chunk.filled;
            // Maybe TODO: Only write chunks that have changed (combining continuous changes?)
        }

        if cache_changed {
            self.buffer_cache.sync_buffers(device, queue);
        }

        queue.write_buffer(&self.instance_buffer, 0, cast_slice(&self.instance_data));
    }

    pub fn render<'a: 'b, 'b>(
        &'a mut self,
        renderables: &[(&'a Shape, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
        msaa: bool,
    ) {
        pass.set_pipeline(if msaa {
            &self.msaa_pipeline
        } else {
            &self.pipeline
        });
        self.draw_renderables(renderables, pass, msaa, instance_offset);
    }

    pub fn new(
        context: &context::WGPUContext,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shape_pipeline_layout"),
                bind_group_layouts: &[uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let num_instances = 32; // Initial allocation
        let instance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<Instance>() * num_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/shape.vert.spv"));
        let fs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/vert_color.frag.spv"));

        Self {
            buffer_cache: BufferCache::new(&context.device),
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
                wgpu::ColorWrites::ALL,
            ),
        }
    }
}
