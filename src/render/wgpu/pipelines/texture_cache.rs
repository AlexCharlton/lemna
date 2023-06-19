use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    render::renderables::raster_cache::{RasterCache, RasterCacheId},
    PixelAABB, PixelSize, Point,
};
use wgpu;

pub struct TextureCache {
    pub raster_cache: Arc<RwLock<RasterCache>>,
    textures: Vec<PackedTexture>,
    // Map of Raster ID (from RasterCache) to texture index
    raster_texture_map: HashMap<u64, usize>,
}

pub struct PackedTexture {
    // https://gamedev.stackexchange.com/questions/2829/texture-packing-algorithm
    size: PixelSize,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    raster_map: HashMap<u64, (RasterCacheId, PixelAABB)>,
    /// The row of free data that is considered first.
    /// When a new row is started, any existing free space becomes a free slot
    current_row: PixelAABB,
    /// Unfilled areas of data
    free_slots: Vec<PixelAABB>,
    /// Number of pixels that have been skipped
    dead_pixels: usize,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            raster_cache: Arc::new(RwLock::new(RasterCache::new())),
            raster_texture_map: HashMap::new(),
            textures: vec![],
        }
    }

    pub fn new_texture(
        size: PixelSize,
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        // TODO
        //     let texture = device.create_texture(&wgpu::TextureDescriptor {
        //         size: wgpu::Extent3d {
        //             width,
        //             height,
        //             depth_or_array_layers: 1,
        //         },
        //         mip_level_count: 1,
        //         sample_count: 1,
        //         dimension: wgpu::TextureDimension::D2,
        //         format: wgpu::TextureFormat::R8Unorm,
        //         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //         view_formats: &[],
        //         label: Some("text_texture"),
        //     });

        //     let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        //     let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        //         mag_filter: wgpu::FilterMode::Linear,
        //         min_filter: wgpu::FilterMode::Linear,
        //         mipmap_filter: wgpu::FilterMode::Linear,
        //         lod_min_clamp: 0.0,
        //         lod_max_clamp: 100.0,
        //         label: Some("text_sampler"),
        //         ..Default::default()
        //     });

        //     let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //         layout: texture_bind_group_layout,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: wgpu::BindingResource::TextureView(&texture_view),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: wgpu::BindingResource::Sampler(&sampler),
        //             },
        //         ],
        //         label: Some("text_bind_group"),
        //     });
    }

    /// Top left, bottom right
    /// If this panics, it means that RasterPipeline::update_texture_cache has failed
    pub fn texture_pos(&self, raster_id: u64) -> (Point, Point) {
        // TODO
        (Point { x: 0.0, y: 0.0 }, Point { x: 0.0, y: 0.0 })
    }

    pub fn unmark(&mut self) {
        self.raster_cache.write().unwrap().unmark();
    }
}
