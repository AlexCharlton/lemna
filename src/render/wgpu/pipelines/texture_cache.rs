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
const MIN_SLOT_DIM: u32 = 4; // px

pub struct TextureCache {
    pub raster_cache: Arc<RwLock<RasterCache>>,
    /// We separate textures from texture_info so we can test the latter
    pub textures: Vec<(wgpu::Texture, wgpu::BindGroup)>,
    pub texture_info: Vec<PackedTextureInfo>,
    // Map of Raster ID (from RasterCache) to texture index
    raster_texture_map: HashMap<RasterId, usize>,
}

#[derive(Debug)]
pub struct PackedTextureInfo {
    size: PixelSize,
    // Raster ID -> (RasterCacheId, AABB within this Texture, has this been written to GPU?)
    raster_map: HashMap<RasterId, (RasterCacheId, PixelAABB, bool)>,
    /// Unfilled areas of data
    free_slots: Vec<PixelAABB>,
    /// Number of pixels taken out of contention
    dead_pixels: u32,
}

impl PackedTextureInfo {
    fn room_for_raster(&self, size: PixelSize) -> bool {
        self.free_slots
            .iter()
            .any(|s| Self::fits_into_slot(size, s.size()))
    }

    fn fits_into_slot(size: PixelSize, slot_size: PixelSize) -> bool {
        size.width <= slot_size.width && size.height <= slot_size.height
    }

    fn dead_slot(aabb: PixelAABB) -> bool {
        aabb.width() <= MIN_SLOT_DIM || aabb.height() <= MIN_SLOT_DIM
    }

    /// When inserting, we iterate through free slots. For the first one that can hold the data,
    /// we select it and split it into two free slots: one for the row made by the inserted data,
    /// and the other for everything else
    /// Not really the same as described [here](https://gamedev.stackexchange.com/questions/2829/texture-packing-algorithm),
    /// but this has some interesting discussion
    fn insert(&mut self, id: RasterId, size: PixelSize, raster_cache_id: RasterCacheId) {
        let mut extra_slot: Option<PixelAABB> = None;
        let mut remove_slot = false;
        let mut pos = PixelPoint { x: 0, y: 0 };
        let mut i = 0;
        for (j, slot) in self.free_slots.iter_mut().enumerate() {
            i = j;
            if Self::fits_into_slot(size, slot.size()) {
                pos = slot.pos;

                let mut remainder1 = slot.clone();
                remainder1.pos.x += size.width;
                remainder1.bottom_right.y = remainder1.pos.y + size.height;
                let mut remainder2 = slot.clone();
                remainder2.pos.y += size.height;

                if !Self::dead_slot(remainder1) {
                    *slot = remainder1;
                    if !Self::dead_slot(remainder2) {
                        extra_slot = Some(remainder2);
                    } else {
                        self.dead_pixels += remainder2.area();
                    }
                } else if !Self::dead_slot(remainder2) {
                    *slot = remainder2;
                    self.dead_pixels += remainder1.area();
                } else {
                    self.dead_pixels += remainder1.area() + remainder2.area();
                    remove_slot = true;
                }
                break;
            }
        }
        if remove_slot {
            self.free_slots.remove(i);
        }
        if let Some(aabb) = extra_slot {
            self.free_slots.push(aabb);
        }

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
            texture_info: vec![],
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

        self.textures.push((texture, bind_group));
        self.texture_info.push(PackedTextureInfo {
            size,
            raster_map: Default::default(),
            free_slots: vec![PixelAABB {
                pos: PixelPoint::new(0, 0),
                bottom_right: PixelPoint::new(size.width, size.height),
            }],
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

        let tex_index = if let Some(i) = self
            .texture_info
            .iter()
            .position(|t| t.room_for_raster(size))
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

        self.texture_info[tex_index].insert(id, size, raster.raster_cache_id);
        self.raster_texture_map.insert(id, tex_index);
    }

    pub fn repack(&mut self) -> bool {
        // TODO when there are too many dead pixels in a texture, repack it
        false
    }

    pub fn write_to_gpu(&mut self, queue: &mut wgpu::Queue) {
        for (i, t) in self.texture_info.iter_mut().enumerate() {
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
                            texture: &self.textures[i].0,
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
        let texture_cache = &self.texture_info[*self.raster_texture_map.get(&raster_id).unwrap()];
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
        &self.textures[texture_index].1
    }

    pub fn unmark(&mut self) {
        self.raster_cache.write().unwrap().unmark();
    }
}

#[cfg(test)]
mod tests {
    use super::PackedTextureInfo;
    use crate::{base_types::*, render::renderables::raster_cache::RasterCacheId};

    #[test]
    fn test_insert() {
        let mut t1 = PackedTextureInfo {
            size: PixelSize {
                width: 200,
                height: 200,
            },
            raster_map: Default::default(),
            free_slots: vec![
                PixelAABB {
                    pos: PixelPoint::new(0, 0),
                    bottom_right: PixelPoint::new(20, 100),
                },
                PixelAABB {
                    pos: PixelPoint::new(0, 100),
                    bottom_right: PixelPoint::new(200, 200),
                },
            ],
            dead_pixels: 0,
        };

        t1.insert(
            0,
            PixelSize {
                width: 50,
                height: 50,
            },
            RasterCacheId::new(10),
        );
        /* The free slots now look like:
        |--------------------------------------|
        |   |                                  |
        | f |                                  |
        | r |                                  |
        | e |          no free space           |
        | e |                                  |
        |   |                                  |
        |--------------------------------------| (200, 100)
        |        |                             |
        |inserted|       new free 1            |
        |        |                             |
        |--------------------------------------| (200, 150)
        |     (50, 150)                        |
        |               new free 2             |
        |                                      |
        |--------------------------------------| (200, 200)

         */
        assert_eq!(t1.raster_map.len(), 1);
        assert_eq!(
            t1.raster_map[&0],
            (
                RasterCacheId::new(10),
                PixelAABB {
                    pos: PixelPoint::new(0, 100),
                    bottom_right: PixelPoint::new(50, 150),
                },
                false
            )
        );
        assert_eq!(t1.free_slots.len(), 3);
        assert_eq!(
            t1.free_slots[0],
            PixelAABB {
                pos: PixelPoint::new(0, 0),
                bottom_right: PixelPoint::new(20, 100),
            }
        );
        assert_eq!(
            t1.free_slots[1],
            PixelAABB {
                pos: PixelPoint::new(50, 100),
                bottom_right: PixelPoint::new(200, 150),
            }
        );
        assert_eq!(
            t1.free_slots[2],
            PixelAABB {
                pos: PixelPoint::new(0, 150),
                bottom_right: PixelPoint::new(200, 200),
            }
        );

        // Now perfectly fill in the last free slot
        t1.insert(
            1,
            PixelSize {
                width: 200,
                height: 50,
            },
            RasterCacheId::new(11),
        );
        /* The free slots now look like:
        |--------------------------------------|
        |   |                                  |
        | f |                                  |
        | r |                                  |
        | e |          no free space           |
        | e |                                  |
        |   |                                  |
        |--------------------------------------| (200, 100)
        |        |                             |
        | used   |       free                  |
        |        |                             |
        |--------------------------------------| (200, 150)
        |                                      |
        |               used                   |
        |                                      |
        |--------------------------------------| (200, 200)

         */
        assert_eq!(t1.raster_map.len(), 2);
        assert_eq!(t1.free_slots.len(), 2);
        assert_eq!(
            t1.free_slots[0],
            PixelAABB {
                pos: PixelPoint::new(0, 0),
                bottom_right: PixelPoint::new(20, 100),
            }
        );
        assert_eq!(
            t1.free_slots[1],
            PixelAABB {
                pos: PixelPoint::new(50, 100),
                bottom_right: PixelPoint::new(200, 150),
            }
        );
    }

    #[test]
    fn test_room_for_raster() {
        let t1 = PackedTextureInfo {
            size: PixelSize {
                width: 200,
                height: 200,
            },
            raster_map: Default::default(),
            free_slots: vec![
                PixelAABB {
                    pos: PixelPoint::new(0, 0),
                    bottom_right: PixelPoint::new(20, 100),
                },
                PixelAABB {
                    pos: PixelPoint::new(0, 100),
                    bottom_right: PixelPoint::new(200, 200),
                },
            ],
            dead_pixels: 0,
        };
        assert!(t1.room_for_raster(PixelSize {
            width: 50,
            height: 50
        }));
        assert!(!t1.room_for_raster(PixelSize {
            width: 250,
            height: 50
        }));
    }
}
