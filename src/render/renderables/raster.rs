use bytemuck::{Pod, Zeroable};

use super::buffer_cache::{BufferCache, BufferCacheId};
use super::raster_cache::{RasterCache, RasterCacheId};
use crate::base_types::{Point, Pos, AABB};

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
    offset: Pos,
    pub buffer_id: BufferCacheId,
    pub raster_id: RasterCacheId,
}

impl Raster {
    pub fn new(
        offset: Pos,
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
        let raster_id = raster_cache.alloc_or_reuse_chunk(prev_raster);

        Self {
            offset,
            buffer_id,
            raster_id,
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
        raster_cache.register(self.raster_id);
        // let (vertex_chunk, index_chunk) = buffer_cache.get_chunks(self.buffer_id);

        // if cache_invalid || !vertex_chunk.filled {
        //     cache_changed = true;

        //     let mut v = vertex_chunk.start;
        //     let mut i = index_chunk.start;
        //     let mut n_indices = 0;
        //     let mut v_relative = 0;
        //     for g in self.glyphs.iter() {
        //         if let Some((uv_rect, screen_rect)) = glyph_cache.rect_for(g.font_id.0, &g.glyph) {
        //             buffer_cache.vertex_data[v] = Vertex {
        //                 pos: Point {
        //                     x: screen_rect.min.x,
        //                     y: screen_rect.min.y,
        //                 },
        //                 tex_pos: Point {
        //                     x: uv_rect.min.x,
        //                     y: uv_rect.min.y,
        //                 },
        //             };
        //             buffer_cache.vertex_data[v + 1] = Vertex {
        //                 pos: Point {
        //                     x: screen_rect.max.x,
        //                     y: screen_rect.min.y,
        //                 },
        //                 tex_pos: Point {
        //                     x: uv_rect.max.x,
        //                     y: uv_rect.min.y,
        //                 },
        //             };
        //             buffer_cache.vertex_data[v + 2] = Vertex {
        //                 pos: Point {
        //                     x: screen_rect.min.x,
        //                     y: screen_rect.max.y,
        //                 },
        //                 tex_pos: Point {
        //                     x: uv_rect.min.x,
        //                     y: uv_rect.max.y,
        //                 },
        //             };
        //             buffer_cache.vertex_data[v + 3] = Vertex {
        //                 pos: Point {
        //                     x: screen_rect.max.x,
        //                     y: screen_rect.max.y,
        //                 },
        //                 tex_pos: Point {
        //                     x: uv_rect.max.x,
        //                     y: uv_rect.max.y,
        //                 },
        //             };

        //             buffer_cache.index_data[i] = v_relative;
        //             buffer_cache.index_data[i + 1] = 1 + v_relative;
        //             buffer_cache.index_data[i + 2] = 2 + v_relative;
        //             buffer_cache.index_data[i + 3] = 2 + v_relative;
        //             buffer_cache.index_data[i + 4] = 1 + v_relative;
        //             buffer_cache.index_data[i + 5] = 3 + v_relative;

        //             v_relative += VERTEX_ENTRIES_PER_IMAGE as u16;
        //             v += VERTEX_ENTRIES_PER_IMAGE;
        //             i += INDEX_ENTRIES_PER_IMAGE;
        //             n_indices += INDEX_ENTRIES_PER_IMAGE;
        //         }
        //     }
        //     // Reset the number of indices, because it may be less than the number of glyphs:
        //     buffer_cache.set_n_indices(self.buffer_id, n_indices);

        //     buffer_cache.fill_chunks(self.buffer_id);
        // }

        // instance_data.push(Instance {
        //     pos: Pos {
        //         x: (self.offset.x + aabb.pos.x),
        //         y: (self.offset.y + aabb.pos.y),
        //         z: self.offset.z + aabb.pos.z,
        //     },
        //     color: self.color,
        // });

        cache_changed
    }
}
