use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, RwLock},
};

use crate::{
    render::{
        next_power_of_2,
        renderables::{
            raster_cache::{RasterCache, RasterCacheId, RasterId},
            Raster,
        },
    },
    PixelAABB, PixelPoint, PixelSize, Point,
};
use wgpu;

const DEFAULT_TEXTURE_CACHE_SIZE: u32 = 2048;

pub struct TextureCache {
    pub raster_cache: Arc<RwLock<RasterCache>>,
    pub textures: Vec<PackedTexture>,
    // Map of Raster ID (from RasterCache) to texture index
    raster_texture_map: HashMap<RasterId, usize>,
}

pub struct PackedTexture {
    // https://gamedev.stackexchange.com/questions/2829/texture-packing-algorithm
    size: PixelSize,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    // Raster ID -> (RasterCacheId, AABB within this Texture, has this been written to GPU?)
    raster_map: HashMap<RasterId, (RasterCacheId, PixelAABB, bool)>,
    /// The row of free data that is considered first.
    /// When a new row is started, any existing free space becomes a free slot
    current_row: PixelAABB,
    /// Unfilled areas of data
    free_slots: Vec<PixelAABB>,
    /// Number of pixels that have been skipped
    dead_pixels: usize,
}

impl PackedTexture {
    fn room_for_raster(&self, size: PixelSize) -> bool {
        // TODO
        true
    }

    fn insert(&mut self, id: RasterId, size: PixelSize, raster_cache_id: RasterCacheId) {
        let pos = PixelPoint { x: 0, y: 0 }; // TODO
                                             // TODO update current_row, free_slots, dead_pixels
        let aabb = PixelAABB {
            pos,
            bottom_right: PixelPoint {
                x: pos.x + size.width,
                y: pos.y + size.height,
            },
        };
        self.raster_map.insert(id, (raster_cache_id, aabb, false));
    }
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            raster_cache: Arc::new(RwLock::new(RasterCache::new())),
            raster_texture_map: HashMap::new(),
            textures: vec![],
        }
    }

    fn new_texture(
        &mut self,
        size: PixelSize,
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> usize {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
            label: Some("texture"),
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("text_bind_group"),
        });

        self.textures.push(PackedTexture {
            size,
            texture,
            bind_group,
            current_row: PixelAABB {
                pos: PixelPoint::new(0, 0),
                bottom_right: PixelPoint::new(size.width, size.height),
            },
            raster_map: Default::default(),
            free_slots: Default::default(),
            dead_pixels: 0,
        });
        self.textures.len() - 1
    }

    pub fn insert(
        &mut self,
        raster: &Raster,
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) {
        let size = self
            .raster_cache
            .read()
            .unwrap()
            .get_raster_data(raster.raster_cache_id)
            .size;
        let id = self
            .raster_cache
            .read()
            .unwrap()
            .get_raster_data(raster.raster_cache_id)
            .id;

        let tex_index = if let Some(i) = self.textures.iter().position(|t| t.room_for_raster(size))
        {
            i
        } else {
            let dim = next_power_of_2(
                size.width.max(size.height).max(DEFAULT_TEXTURE_CACHE_SIZE) as usize
            ) as u32;
            self.new_texture(
                PixelSize {
                    width: dim,
                    height: dim,
                },
                device,
                texture_bind_group_layout,
                sampler,
            )
        };

        self.textures[tex_index].insert(id, size, raster.raster_cache_id);
        self.raster_texture_map.insert(id, tex_index);
    }

    pub fn repack(&mut self) -> bool {
        // TODO when there are too many dead pixels in a texture, repack it
        false
    }

    pub fn write_to_gpu(&mut self, queue: &mut wgpu::Queue) {
        for t in self.textures.iter_mut() {
            for (_, (raster_cache_id, aabb, written)) in t.raster_map.iter_mut() {
                if !*written {
                    let size = self
                        .raster_cache
                        .read()
                        .unwrap()
                        .get_raster_data(*raster_cache_id)
                        .size;
                    queue.write_texture(
                        wgpu::ImageCopyTexture {
                            aspect: wgpu::TextureAspect::All,
                            texture: &t.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: aabb.pos.x,
                                y: aabb.pos.y,
                                z: 0,
                            },
                        },
                        (&self
                            .raster_cache
                            .read()
                            .unwrap()
                            .get_raster_data(*raster_cache_id)
                            .data)
                            .into(),
                        wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: NonZeroU32::new(size.width * 4),
                            rows_per_image: NonZeroU32::new(size.height),
                        },
                        wgpu::Extent3d {
                            width: size.width,
                            height: size.height,
                            depth_or_array_layers: 1,
                        },
                    );

                    *written = true;
                }
            }
        }
    }

    /// Top left, bottom right
    /// If this panics, it means that RasterPipeline::update_texture_cache has failed
    pub fn texture_pos(&self, raster_id: u64) -> (Point, Point) {
        let texture_cache = &self.textures[*self.raster_texture_map.get(&raster_id).unwrap()];
        let (_, coords, _) = texture_cache.raster_map.get(&raster_id).unwrap();
        let size = texture_cache.size;
        coords.normalize(size)
    }

    pub fn texture_index(&self, raster_cache_id: RasterCacheId) -> Option<usize> {
        let raster_id = self
            .raster_cache
            .read()
            .unwrap()
            .get_raster_data(raster_cache_id)
            .id;
        self.raster_texture_map.get(&raster_id).copied()
    }

    pub fn bind_group(&self, texture_index: usize) -> &wgpu::BindGroup {
        &self.textures[texture_index].bind_group
    }

    pub fn unmark(&mut self) {
        self.raster_cache.write().unwrap().unmark();
    }
}
