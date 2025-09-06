use std::fmt;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use lyon;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::geometry_builder::VertexBuffers;
use lyon::tessellation::math as lyon_math;

use super::{BufferCache, BufferCacheId};
use crate::base_types::{AABB, Color, Point, Pos};

pub type ShapeGeometry = VertexBuffers<Vertex, u16>;
pub const TOLERANCE: f32 = 0.2;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
    pub norm: Point,
}

impl crate::render::wgpu::VBDesc for Vertex {
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
pub(crate) struct Instance {
    pub pos: Pos,
    pub color: Color,
    pub stroke_width: f32,
}

impl crate::render::wgpu::VBDesc for Instance {
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

#[derive(PartialEq)]
pub struct Shape {
    fill_color: Color,
    stroke_color: Color,
    stroke_width: f32,
    pub fill_range: Range<u32>,
    pub stroke_range: Range<u32>,
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
        buffer_cache: &mut BufferCache<Vertex, u16>,
        prev_buffer: Option<BufferCacheId>,
    ) -> Self {
        let buffer_id = if let Some(c) = prev_buffer {
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
        buffer_cache: &mut BufferCache<Vertex, u16>,
        prev_buffer: Option<BufferCacheId>,
    ) -> Self {
        let buffer_id = if let Some(c) = prev_buffer {
            buffer_cache.alloc_or_reuse_chunk(c, geometry.vertices.len(), geometry.indices.len())
        } else {
            buffer_cache.alloc_chunk(geometry.vertices.len(), geometry.indices.len())
        };

        let (vertex_chunk, index_chunk) = buffer_cache.get_chunks(buffer_id);
        buffer_cache.vertex_data[vertex_chunk.start..(vertex_chunk.start + vertex_chunk.n)]
            .copy_from_slice(&geometry.vertices);
        buffer_cache.index_data[index_chunk.start..(index_chunk.start + index_chunk.n)]
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

    pub(crate) fn render(
        &self,
        aabb: &AABB,
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
