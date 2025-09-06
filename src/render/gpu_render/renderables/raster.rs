use bytemuck::{Pod, Zeroable};

use super::{BufferCache, BufferCacheId};
use super::{RasterCache, RasterCacheId};
use crate::PixelSize;
use crate::base_types::{AABB, Point, Pos};
use crate::render::RasterData;

const INDEX_ENTRIES_PER_IMAGE: usize = 6;
const VERTEX_ENTRIES_PER_IMAGE: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
    pub tex_pos: Point,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub(crate) struct Instance {
    pub pos: Pos,
}

impl crate::render::wgpu::VBDesc for Instance {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            }],
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Raster {
    pub buffer_id: BufferCacheId,
    pub raster_cache_id: RasterCacheId,
}

impl Raster {
    pub fn new(
        data: RasterData,
        size: PixelSize,
        buffer_cache: &mut BufferCache<Vertex, u16>,
        raster_cache: &mut RasterCache,
        prev_buffer: Option<BufferCacheId>,
        prev_raster: Option<RasterCacheId>,
    ) -> Self {
        let buffer_id = if let Some(c) = prev_buffer {
            buffer_cache.alloc_or_reuse_chunk(c, VERTEX_ENTRIES_PER_IMAGE, INDEX_ENTRIES_PER_IMAGE)
        } else {
            buffer_cache.alloc_chunk(VERTEX_ENTRIES_PER_IMAGE, INDEX_ENTRIES_PER_IMAGE)
        };
        let raster_cache_id = raster_cache.alloc_or_reuse_chunk(prev_raster);
        raster_cache.set_raster(raster_cache_id, data, size);

        Self {
            buffer_id,
            raster_cache_id,
        }
    }

    pub(crate) fn render(
        &self,
        aabb: &AABB,
        tex_coords: (Point, Point),
        buffer_cache: &mut BufferCache<Vertex, u16>,
        raster_cache: &mut RasterCache,
        instance_data: &mut Vec<Instance>,
        cache_invalid: bool,
    ) -> bool {
        let mut cache_changed = false;
        buffer_cache.register(self.buffer_id);
        raster_cache.register(self.raster_cache_id);
        let (vertex_chunk, index_chunk) = buffer_cache.get_chunks(self.buffer_id);

        if cache_invalid || !vertex_chunk.filled {
            cache_changed = true;
            let v = vertex_chunk.start;
            let i = index_chunk.start;
            let width = aabb.width();
            let height = aabb.height();

            buffer_cache.vertex_data[v] = Vertex {
                pos: Point { x: 0.0, y: 0.0 },
                tex_pos: Point {
                    x: tex_coords.0.x,
                    y: tex_coords.0.y,
                },
            };
            buffer_cache.vertex_data[v + 1] = Vertex {
                pos: Point { x: width, y: 0.0 },
                tex_pos: Point {
                    x: tex_coords.1.x,
                    y: tex_coords.0.y,
                },
            };
            buffer_cache.vertex_data[v + 2] = Vertex {
                pos: Point { x: 0.0, y: height },
                tex_pos: Point {
                    x: tex_coords.0.x,
                    y: tex_coords.1.y,
                },
            };
            buffer_cache.vertex_data[v + 3] = Vertex {
                pos: Point {
                    x: width,
                    y: height,
                },
                tex_pos: Point {
                    x: tex_coords.1.x,
                    y: tex_coords.1.y,
                },
            };

            buffer_cache.index_data[i] = 0;
            buffer_cache.index_data[i + 1] = 1;
            buffer_cache.index_data[i + 2] = 2;
            buffer_cache.index_data[i + 3] = 2;
            buffer_cache.index_data[i + 4] = 1;
            buffer_cache.index_data[i + 5] = 3;

            buffer_cache.fill_chunks(self.buffer_id);
        }

        instance_data.push(Instance { pos: aabb.pos });

        cache_changed
    }
}
