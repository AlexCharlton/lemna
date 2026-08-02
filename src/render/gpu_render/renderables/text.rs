use bytemuck::{Pod, Zeroable};

use super::{BufferCache, BufferCacheId};
use crate::base_types::{Color, Point, Pos, Rect, Scale};
use crate::font_cache::PositionedGlyph;
use crate::render::gpu_render::{Caches, glyph_cache::DrawCache};
use crate::renderable::TextAngle;

const INDEX_ENTRIES_PER_GLYPH: usize = 6;
const VERTEX_ENTRIES_PER_GLYPH: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub pos: Point,
    pub tex_pos: Point,
}

impl crate::render::gpu_render::VBDesc for Vertex {
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
    pub color: Color,
}

impl crate::render::gpu_render::VBDesc for Instance {
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
            ],
        }
    }
}

#[derive(Debug)]
pub struct Text {
    color: Color,
    pub(crate) glyphs: Vec<PositionedGlyph>,
    offset: Pos,
    layout_angle: TextAngle,
    // Text-local coords. Only used when `layout_angle` is not `TextAngle::None`
    bounds: Scale,
    pub(crate) buffer_id: BufferCacheId,
}

impl PartialEq for Text {
    // Should only be used for tests
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color
            && self.offset == other.offset
            && self.layout_angle == other.layout_angle
    }
}

impl Text {
    pub fn new(
        glyphs: Vec<PositionedGlyph>,
        offset: Pos,
        color: Color,
        caches: &mut Caches,
        prev: Option<&Text>,
    ) -> Self {
        let buffer_cache = &mut caches.text_buffer;
        let len = glyphs.len();
        let index_len = len * INDEX_ENTRIES_PER_GLYPH;
        let vertex_len = len * VERTEX_ENTRIES_PER_GLYPH;

        let buffer_id = if let Some(c) = prev.map(|r| r.buffer_id) {
            buffer_cache.alloc_or_reuse_chunk(c, vertex_len, index_len)
        } else {
            buffer_cache.alloc_chunk(vertex_len, index_len)
        };

        Self {
            glyphs,
            color,
            offset,
            layout_angle: TextAngle::None,
            bounds: Scale::default(),
            buffer_id,
        }
    }

    pub fn angle(mut self, layout_angle: TextAngle, bounds: Scale) -> Self {
        self.layout_angle = layout_angle;
        self.bounds = bounds;
        self
    }

    pub(crate) fn render(
        &self,
        aabb: &Rect,
        buffer_cache: &mut BufferCache<Vertex, u16>,
        glyph_cache: &DrawCache,
        instance_data: &mut Vec<Instance>,
        cache_invalid: bool,
    ) -> bool {
        let mut cache_changed = false;
        buffer_cache.register(self.buffer_id);
        let (vertex_chunk, index_chunk) = buffer_cache.get_chunks(self.buffer_id);
        let lw = self.bounds.width;
        let lh = self.bounds.height;

        if cache_invalid || !vertex_chunk.filled {
            cache_changed = true;

            let mut v = vertex_chunk.start;
            let mut i = index_chunk.start;
            let mut n_indices = 0;
            let mut v_relative = 0;
            for g in self.glyphs.iter() {
                // Skip invisible glyphs (newlines, spaces, etc.) that have 0 width or height
                if g.width == 0 || g.height == 0 {
                    continue;
                }
                if let Some(uv_rect) = glyph_cache.rect_for(g) {
                    let (x0, y0) = self.layout_angle.map_point(g.x, g.y, lw, lh);
                    let (x1, y1) = self
                        .layout_angle
                        .map_point(g.x + g.width as f32, g.y, lw, lh);
                    let (x2, y2) = self
                        .layout_angle
                        .map_point(g.x, g.y + g.height as f32, lw, lh);
                    let (x3, y3) = self.layout_angle.map_point(
                        g.x + g.width as f32,
                        g.y + g.height as f32,
                        lw,
                        lh,
                    );

                    buffer_cache.vertex_data[v] = Vertex {
                        pos: Point { x: x0, y: y0 },
                        tex_pos: Point {
                            x: uv_rect.pos.x,
                            y: uv_rect.pos.y,
                        },
                    };
                    buffer_cache.vertex_data[v + 1] = Vertex {
                        pos: Point { x: x1, y: y1 },
                        tex_pos: Point {
                            x: uv_rect.bottom_right.x,
                            y: uv_rect.pos.y,
                        },
                    };
                    buffer_cache.vertex_data[v + 2] = Vertex {
                        pos: Point { x: x2, y: y2 },
                        tex_pos: Point {
                            x: uv_rect.pos.x,
                            y: uv_rect.bottom_right.y,
                        },
                    };
                    buffer_cache.vertex_data[v + 3] = Vertex {
                        pos: Point { x: x3, y: y3 },
                        tex_pos: Point {
                            x: uv_rect.bottom_right.x,
                            y: uv_rect.bottom_right.y,
                        },
                    };

                    buffer_cache.index_data[i] = v_relative;
                    buffer_cache.index_data[i + 1] = 1 + v_relative;
                    buffer_cache.index_data[i + 2] = 2 + v_relative;
                    buffer_cache.index_data[i + 3] = 2 + v_relative;
                    buffer_cache.index_data[i + 4] = 1 + v_relative;
                    buffer_cache.index_data[i + 5] = 3 + v_relative;

                    v_relative += VERTEX_ENTRIES_PER_GLYPH as u16;
                    v += VERTEX_ENTRIES_PER_GLYPH;
                    i += INDEX_ENTRIES_PER_GLYPH;
                    n_indices += INDEX_ENTRIES_PER_GLYPH;
                }
            }
            // Reset the number of indices, because it may be less than the number of glyphs:
            buffer_cache.set_n_indices(self.buffer_id, n_indices);

            buffer_cache.fill_chunks(self.buffer_id);
        }

        instance_data.push(Instance {
            pos: Pos {
                x: (self.offset.x + aabb.pos.x),
                y: (self.offset.y + aabb.pos.y),
                z: self.offset.z + aabb.pos.z,
            },
            color: self.color,
        });

        cache_changed
    }

    pub(crate) fn z(&self) -> f32 {
        self.offset.z
    }
}
