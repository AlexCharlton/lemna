use bytemuck::cast_slice;
use log::info;
use wgpu;

use super::buffer_cache::BufferCache;
use super::shared::{create_pipeline, VBDesc};
use super::texture_cache::TextureCache;
use crate::base_types::AABB;
use crate::render::next_power_of_2;
use crate::render::renderables::raster::{Instance, Raster, Vertex};
use crate::render::wgpu::context;

const DEFAULT_TEXTURE_CACHE_SIZE: u32 = 1024;

pub struct RasterPipeline {
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,

    pub(crate) texture_cache: TextureCache,
    pub(crate) buffer_cache: BufferCache<Vertex, u16>,
    instance_data: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    num_instances: usize,
}

impl RasterPipeline {
    pub(crate) fn unmark_cache(&mut self) {
        self.buffer_cache.unmark();
        self.texture_cache.unmark();
    }

    fn draw_renderables<'a: 'b, 'b>(
        &'a self,
        renderables: &[(&'a Raster, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
    ) {
        let last_texture = None;
        // We construct our instance data in the same order of our renderables,
        // so `i` can be used to index into the instance_data
        for (i, (renderable, _)) in renderables.iter().enumerate() {
            let (vertex_chunk, index_chunk) = self.buffer_cache.get_chunks(renderable.buffer_id);

            let texture_index = self.texture_cache.texture_index(renderable.raster_id);
            // We pre-sorted our renderables so that we will have to switch this a minimum number of times
            if last_texture != texture_index {
                pass.set_bind_group(
                    1,
                    &self.texture_cache.bind_group(texture_index.unwrap()),
                    &[],
                );
            }

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
                "Resizing RasterPipeline instance buffer to {}",
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
        renderables: &[(&'a Raster, &'a AABB)],
        device: &'b wgpu::Device,
        queue: &'b mut wgpu::Queue,
        cache_invalid: bool,
    ) {
        self.instance_data.clear();
        // Update CPU buffers if changed
        let mut cache_changed = false;
        for (renderable, aabb) in renderables.iter() {
            let raster_id = self
                .texture_cache
                .raster_cache
                .read()
                .unwrap()
                .get_raster(renderable.raster_id)
                .id;
            let texture_pos = self.texture_cache.texture_pos(raster_id);
            cache_changed |= renderable.render(
                aabb,
                texture_pos,
                &mut self.buffer_cache.cache.write().unwrap(),
                &mut self.texture_cache.raster_cache.write().unwrap(),
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
        renderables: &[(&'a Raster, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
    ) {
        // Draw the renderables
        pass.set_pipeline(&self.pipeline);

        self.draw_renderables(renderables, pass, instance_offset);
    }

    pub fn update_texture_cache(
        &mut self,
        renderables: &[(&Raster, &AABB)],
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
    ) -> bool {
        // Draw glyphs onto GPU texture cache
        // let mut cache_invalid = false;
        // let mut cache_success = false;
        // let mut cache_size = self.glyph_cache.size;
        // while !cache_success {
        //     // TODO Sort by height
        //     for (renderable, _) in renderables.iter() {
        //         for g in renderable.glyphs.iter().cloned() {
        //             self.glyph_cache
        //                 .glyph_cache
        //                 .queue_glyph(g.font_id.0, g.glyph);
        //         }
        //     }

        //     let cache_result = {
        //         let texture = &self.glyph_cache.texture;
        //         self.glyph_cache
        //             .glyph_cache
        //             .cache_queued(&font_cache.fonts, |region, data| {
        //                 queue.write_texture(
        //                     wgpu::ImageCopyTexture {
        //                         aspect: wgpu::TextureAspect::All,
        //                         texture,
        //                         mip_level: 0,
        //                         origin: wgpu::Origin3d {
        //                             x: 0,
        //                             y: region.min[1],
        //                             z: 0,
        //                         },
        //                     },
        //                     data,
        //                     wgpu::ImageDataLayout {
        //                         offset: 0,
        //                         bytes_per_row: NonZeroU32::new(region.width()),
        //                         rows_per_image: NonZeroU32::new(region.height()),
        //                     },
        //                     wgpu::Extent3d {
        //                         width: region.width(),
        //                         height: region.height(),
        //                         depth_or_array_layers: 1,
        //                     },
        //                 );
        //             })
        //     };
        //     match cache_result {
        //         Ok(CachedBy::Adding) => (),
        //         Ok(CachedBy::Reordering) => cache_invalid = true,
        //         Err(err) => {
        //             cache_size *= 2;
        //             eprintln!("{:?}: Resizing texture to {:?}", err, cache_size);
        //             let (texture, bind_group) = Self::create_texture(
        //                 cache_size,
        //                 cache_size,
        //                 device,
        //                 &self.bind_group_layout,
        //             );
        //             self.glyph_cache.new_texture(texture, cache_size);
        //             self.bind_group = bind_group;
        //         }
        //     };

        //     cache_success = cache_result.is_ok();
        // }
        // cache_invalid
        false
    }

    pub fn new(
        context: &context::WGPUContext,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group_layout =
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
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("text_bind_group_layout"),
                });

        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            label: Some("texture_sampler"),
            ..Default::default()
        });

        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("text_pipeline_layout"),
                bind_group_layouts: &[uniform_bind_group_layout, &bind_group_layout],
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
            .create_shader_module(wgpu::include_spirv!("shaders/image.vert.spv"));
        let fs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/image.frag.spv"));

        Self {
            buffer_cache: BufferCache::new(&context.device),
            texture_cache: TextureCache::new(),
            instance_data: vec![],
            instance_buffer,
            num_instances,

            bind_group_layout,
            sampler,
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
        }
    }
}
