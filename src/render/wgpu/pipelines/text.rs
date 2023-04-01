use bytemuck::{cast_slice, Pod, Zeroable};
use log::info;
use std::num::NonZeroU32;
use wgpu;

use super::buffer_cache::{BufferCache, BufferCacheId};
use super::glyph_brush_draw_cache::{CachedBy, DrawCache};
use super::shared::{create_pipeline, next_power_of_2, VBDesc};
use crate::base_types::{Color, Point, Pos, AABB};
use crate::font_cache::{FontCache, SectionGlyph};
use crate::render::wgpu::context;

pub use glyph_brush_layout::FontId;
const DEFAULT_TEXTURE_CACHE_SIZE: u32 = 1024;
const INDEX_ENTRIES_PER_GLYPH: usize = 6;
const VERTEX_ENTRIES_PER_GLYPH: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: Point,
    pub tex_pos: Point,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub pos: Pos,
    pub color: Color,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            pos: Default::default(),
            color: Default::default(),
        }
    }
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

#[derive(Debug)]
pub struct Text {
    color: Color,
    glyphs: Vec<SectionGlyph>,
    offset: Pos,
    pub buffer_id: BufferCacheId,
}

impl Text {
    pub fn new(
        glyphs: Vec<SectionGlyph>,
        offset: Pos,
        color: Color,
        renderer: &mut TextPipeline,
        prev_buffer: Option<BufferCacheId>,
    ) -> Self {
        let len = glyphs.len();
        let index_len = len * INDEX_ENTRIES_PER_GLYPH;
        let vertex_len = len * VERTEX_ENTRIES_PER_GLYPH;

        let buffer_id = if let Some(c) = prev_buffer {
            renderer
                .buffer_cache
                .alloc_or_reuse_chunk(c, vertex_len, index_len)
        } else {
            renderer.buffer_cache.alloc_chunk(vertex_len, index_len)
        };

        Self {
            glyphs,
            color,
            offset,
            buffer_id,
        }
    }

    fn render(
        &self,
        aabb: &AABB,
        buffer_cache: &mut BufferCache<Vertex, u16>,
        glyph_cache: &GlyphCache,
        instance_data: &mut Vec<Instance>,
        cache_invalid: bool,
    ) -> bool {
        let mut cache_changed = false;
        buffer_cache.register(self.buffer_id);
        let (vertex_chunk, index_chunk) = buffer_cache.get_chunks(self.buffer_id);

        if cache_invalid || !vertex_chunk.filled {
            cache_changed = true;

            let mut v = vertex_chunk.start;
            let mut i = index_chunk.start;
            let mut n_indices = 0;
            let mut v_relative = 0;
            for g in self.glyphs.iter() {
                if let Some((uv_rect, screen_rect)) =
                    glyph_cache.glyph_cache.rect_for(g.font_id.0, &g.glyph)
                {
                    buffer_cache.vertex_data[v + 0] = Vertex {
                        pos: Point {
                            x: screen_rect.min.x as f32,
                            y: screen_rect.min.y as f32,
                        },
                        tex_pos: Point {
                            x: uv_rect.min.x,
                            y: uv_rect.min.y,
                        },
                    };
                    buffer_cache.vertex_data[v + 1] = Vertex {
                        pos: Point {
                            x: screen_rect.max.x as f32,
                            y: screen_rect.min.y as f32,
                        },
                        tex_pos: Point {
                            x: uv_rect.max.x,
                            y: uv_rect.min.y,
                        },
                    };
                    buffer_cache.vertex_data[v + 2] = Vertex {
                        pos: Point {
                            x: screen_rect.min.x as f32,
                            y: screen_rect.max.y as f32,
                        },
                        tex_pos: Point {
                            x: uv_rect.min.x,
                            y: uv_rect.max.y,
                        },
                    };
                    buffer_cache.vertex_data[v + 3] = Vertex {
                        pos: Point {
                            x: screen_rect.max.x as f32,
                            y: screen_rect.max.y as f32,
                        },
                        tex_pos: Point {
                            x: uv_rect.max.x,
                            y: uv_rect.max.y,
                        },
                    };

                    buffer_cache.index_data[i + 0] = 0 + v_relative;
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
}

struct GlyphCache {
    glyph_cache: DrawCache,
    texture: wgpu::Texture,
    size: u32,
}

impl GlyphCache {
    fn new(texture: wgpu::Texture, size: u32) -> Self {
        let glyph_cache = DrawCache::builder()
            .dimensions(size, size)
            .scale_tolerance(0.2)
            .position_tolerance(0.2)
            .multithread(false)
            .cpu_cache(true)
            .build();

        Self {
            glyph_cache,
            texture,
            size,
        }
    }

    fn new_texture(&mut self, texture: wgpu::Texture, size: u32) {
        self.glyph_cache = DrawCache::builder()
            .dimensions(size, size)
            .scale_tolerance(0.2)
            .position_tolerance(0.2)
            .multithread(false)
            .cpu_cache(true)
            .build();
        self.texture = texture;
    }
}

pub struct TextPipeline {
    pipeline: wgpu::RenderPipeline,
    msaa_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    buffer_cache: BufferCache<Vertex, u16>,
    glyph_cache: GlyphCache,
    instance_data: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    num_instances: usize,
}

impl TextPipeline {
    pub(crate) fn unmark_buffer_cache(&mut self) {
        self.buffer_cache.unmark();
    }

    fn draw_renderables<'a: 'b, 'b>(
        &'a self,
        renderables: &[(&'a Text, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
    ) {
        // We construct our instance data in the same order of our renderables,
        // so `i` can be used to index into the instance_data
        for (i, (renderable, _)) in renderables.iter().enumerate() {
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
            pass.draw_indexed(0..index_chunk.n as u32, 0, 0..1);
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
                "Resizing TextPipeline instance buffer to {}",
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
        renderables: &[(&'a Text, &'a AABB)],
        device: &'b wgpu::Device,
        queue: &'b mut wgpu::Queue,
        font_cache: &FontCache,
    ) {
        let cache_invalid = self.update_glyph_cache(renderables, device, queue, font_cache);

        self.instance_data.clear();
        // Update CPU buffers if changed
        let mut cache_changed = false;
        for (renderable, aabb) in renderables.iter() {
            cache_changed |= renderable.render(
                &aabb,
                &mut self.buffer_cache,
                &self.glyph_cache,
                &mut self.instance_data,
                cache_invalid,
            );
        }

        // Update GPU buffers
        if cache_changed {
            self.buffer_cache.sync_buffers(device, queue);
        }

        queue.write_buffer(&self.instance_buffer, 0, cast_slice(&self.instance_data));
    }

    pub fn render<'a: 'b, 'b>(
        &'a mut self,
        renderables: &[(&'a Text, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
        msaa: bool,
    ) {
        // Draw the renderables
        pass.set_pipeline(if msaa {
            &self.msaa_pipeline
        } else {
            &self.pipeline
        });

        pass.set_bind_group(1, &self.bind_group, &[]);
        self.draw_renderables(renderables, pass, instance_offset);

        // let vertex_data = vec![
        //     Vertex {
        //         pos: [0.0, 0.0].into(),
        //         tex_pos: [0.0, 0.0].into(),
        //     },
        //     Vertex {
        //         pos: [768.0, 0.0].into(),
        //         tex_pos: [1.0, 0.0].into(),
        //     },
        //     Vertex {
        //         pos: [0.0, 768.0].into(),
        //         tex_pos: [0.0, 1.0].into(),
        //     },
        //     Vertex {
        //         pos: [768.0, 768.0].into(),
        //         tex_pos: [1.0, 1.0].into(),
        //     },
        // ];

        // let index_data: [u16; 6] = [0, 1, 2, 2, 1, 3];
        // self.buffer_cache.vertex_buffer = Some(device.create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         label: None,
        //         contents: cast_slice(&vertex_data),
        //         usage: wgpu::BufferUsages::VERTEX,
        //     },
        // ));

        // self.buffer_cache.index_buffer = Some(device.create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         label: None,
        //         contents: cast_slice(&index_data),
        //         usage: wgpu::BufferUsages::INDEX,
        //     },
        // ));

        // self.instance_data.push(Instance {
        //     pos: Pos {
        //         x: 100.0,
        //         y: 70.0,
        //         z: 100.0,
        //     },
        //     color: 0.0.into(),
        // });

        // self.instance_buffer = Some(
        //     device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //         label: None,
        //         contents: cast_slice(&self.instance_data),
        //         usage: wgpu::BufferUsages::VERTEX,
        //     }),
        // );

        // pass.set_pipeline(&self.pipeline);
        // pass.set_bind_group(1, &self.bind_group, &[]);
        // pass.set_vertex_buffer(
        //     0,
        //     self.buffer_cache.vertex_buffer.as_ref().unwrap().slice(..),
        // );
        // pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));
        // pass.set_index_buffer(self.buffer_cache.index_buffer.as_ref().unwrap().slice(..));
        // pass.draw_indexed(0..6 as u32, 0, 0..1);
    }

    fn update_glyph_cache(
        &mut self,
        renderables: &[(&Text, &AABB)],
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        font_cache: &FontCache,
    ) -> bool {
        // Draw glyphs onto GPU texture cache
        let mut cache_invalid = false;
        let mut cache_success = false;
        let mut cache_size = self.glyph_cache.size;
        while !cache_success {
            for (renderable, _) in renderables.iter() {
                for g in renderable.glyphs.iter().cloned() {
                    self.glyph_cache
                        .glyph_cache
                        .queue_glyph(g.font_id.0, g.glyph);
                }
            }

            let cache_result = {
                let texture = &self.glyph_cache.texture;
                self.glyph_cache
                    .glyph_cache
                    .cache_queued(&font_cache.fonts, |region, data| {
                        queue.write_texture(
                            wgpu::ImageCopyTexture {
                                aspect: wgpu::TextureAspect::StencilOnly, // TODO YES?
                                texture,
                                mip_level: 0,
                                origin: wgpu::Origin3d {
                                    x: 0,
                                    y: region.min[1],
                                    z: 0,
                                },
                            },
                            data,
                            wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: NonZeroU32::new(region.width()),
                                rows_per_image: NonZeroU32::new(region.height()), // TODO YES?
                            },
                            wgpu::Extent3d {
                                width: region.width(),
                                height: region.height(),
                                depth_or_array_layers: 1,
                            },
                        );
                    })
            };
            match cache_result {
                Ok(CachedBy::Adding) => (),
                Ok(CachedBy::Reordering) => cache_invalid = true,
                Err(err) => {
                    cache_size *= 2;
                    eprintln!("{:?}: Resizing texture to {:?}", err, cache_size);
                    let (texture, bind_group) = Self::create_texture(
                        cache_size,
                        cache_size,
                        device,
                        &self.texture_bind_group_layout,
                    );
                    self.glyph_cache.new_texture(texture, cache_size);
                    self.bind_group = bind_group;
                }
            };

            cache_success = cache_result.is_ok();
        }
        cache_invalid
    }

    fn create_texture(
        width: u32,
        height: u32,
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, // TODO I Think this is right?
            view_formats: &[],
            label: Some("text_texture"),
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            label: Some("text_sampler"),
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("text_bind_group"),
        });

        (texture, bind_group)
    }

    pub fn new(
        context: &context::WGPUContext,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Uint,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), // TODO Am I right?
                            count: None,
                        },
                    ],
                    label: Some("text_texture_bind_group_layout"),
                });

        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("text_pipeline_layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let (texture, bind_group) = Self::create_texture(
            DEFAULT_TEXTURE_CACHE_SIZE,
            DEFAULT_TEXTURE_CACHE_SIZE,
            &context.device,
            &texture_bind_group_layout,
        );

        let num_instances = 32; // Initial allocation
        let instance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<Instance>() * num_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/text.vert.spv"));
        let fs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/text.frag.spv"));

        Self {
            buffer_cache: BufferCache::new(&context.device),
            glyph_cache: GlyphCache::new(texture, DEFAULT_TEXTURE_CACHE_SIZE),
            instance_data: vec![],
            instance_buffer,
            num_instances,

            bind_group,
            texture_bind_group_layout,
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
                wgpu::ColorWrites::empty(),
            ),
        }
    }
}
