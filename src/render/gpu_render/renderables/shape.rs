use std::fmt;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use lyon::tessellation;

use super::{BufferCache, BufferCacheId};
use crate::base_types::{Color, Point, Pos, Rect};
use crate::render::path::Path;
use crate::renderable::Caches;

pub type ShapeGeometry = tessellation::geometry_builder::VertexBuffers<Vertex, u16>;
pub const TOLERANCE: f32 = 0.2;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
}

impl crate::render::gpu_render::VBDesc for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
            ],
        }
    }
}

struct VertexConstructor {}

impl tessellation::FillVertexConstructor<Vertex> for VertexConstructor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex<'_>) -> Vertex {
        Vertex {
            pos: Point {
                x: vertex.position().x,
                y: vertex.position().y,
            },
        }
    }
}

impl tessellation::StrokeVertexConstructor<Vertex> for VertexConstructor {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex<'_, '_>) -> Vertex {
        Vertex {
            pos: Point {
                x: vertex.position().x,
                y: vertex.position().y,
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub(crate) struct Instance {
    pub pos: Pos,
    pub color: Color,
    pub stroke_width: f32,
}

impl crate::render::gpu_render::VBDesc for Instance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                },
                // Color
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 4 * 3,
                    shader_location: 2,
                },
            ],
        }
    }
}

#[derive(PartialEq)]
pub struct Shape {
    fill_color: Color,
    stroke_color: Color,
    stroke_width: f32,
    pub(crate) fill_range: Range<u32>,
    pub(crate) stroke_range: Range<u32>,
    z: f32,
    pub(crate) buffer_id: BufferCacheId,
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
    pub(crate) fn is_stroked(&self) -> bool {
        self.stroke_width > 0.0
    }

    pub(crate) fn is_filled(&self) -> bool {
        self.fill_range.start < self.fill_range.end
    }

    fn path_to_shape_geometry(
        path: Path,
        fill: bool,
        stroke: bool,
        stroke_width: f32,
    ) -> (ShapeGeometry, u32) {
        let mut geometry = ShapeGeometry::new();
        let path = path.native_path();

        if fill {
            tessellation::FillTessellator::new()
                .tessellate_path(
                    path,
                    &tessellation::FillOptions::tolerance(TOLERANCE),
                    &mut tessellation::BuffersBuilder::new(&mut geometry, VertexConstructor {}),
                )
                .unwrap()
        }
        let fill_count = geometry.indices.len() as u32;
        if stroke {
            tessellation::StrokeTessellator::new()
                .tessellate_path(
                    path,
                    &tessellation::StrokeOptions::tolerance(TOLERANCE)
                        .with_line_width(stroke_width),
                    &mut tessellation::BuffersBuilder::new(&mut geometry, VertexConstructor {}),
                )
                .unwrap();
        }

        (geometry, fill_count)
    }

    pub fn new(
        path: Path,
        fill_color: Color,
        stroke_color: Color,
        stroke_width: f32,
        z: f32,
        caches: &mut Caches,
        prev: Option<&Shape>,
    ) -> Self {
        let buffer_cache = &mut caches.shape_buffer;
        let (geometry, fill_count) = Shape::path_to_shape_geometry(
            path,
            fill_color.is_visible(),
            stroke_color.is_visible() && stroke_width > 0.0,
            stroke_width,
        );

        let buffer_id = if let Some(c) = prev.map(|r| r.buffer_id) {
            buffer_cache.alloc_or_reuse_chunk(c, geometry.vertices.len(), geometry.indices.len())
        } else {
            assert!(
                geometry.vertices.len() + geometry.indices.len() != 0,
                "Cannot create an empty shape"
            );
            buffer_cache.alloc_chunk(geometry.vertices.len(), geometry.indices.len())
        };

        let (vertex_chunk, index_chunk) = buffer_cache.get_chunks(buffer_id);
        buffer_cache.vertex_data[vertex_chunk.start..(vertex_chunk.start + vertex_chunk.n)]
            .copy_from_slice(&geometry.vertices);
        buffer_cache.index_data[index_chunk.start..(index_chunk.start + index_chunk.n)]
            .copy_from_slice(&geometry.indices);

        Self {
            fill_color,
            stroke_color,
            stroke_width,
            fill_range: 0..fill_count,
            stroke_range: fill_count..(geometry.indices.len() as u32),
            z,
            buffer_id,
        }
    }

    pub(crate) fn render(
        &self,
        aabb: &Rect,
        buffer_cache: &mut BufferCache<Vertex, u16>,
    ) -> Vec<Instance> {
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
