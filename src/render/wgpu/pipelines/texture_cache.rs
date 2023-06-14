use std::sync::{Arc, RwLock};

use crate::render::renderables::raster_cache::RasterCache;
use wgpu;

pub struct TextureCache {
    pub raster_cache: Arc<RwLock<RasterCache>>,
    textures: Vec<PackedTexture>,
}

pub struct PackedTexture {
    texture: wgpu::Texture, // TODO track what rasters are used where
                            // https://gamedev.stackexchange.com/questions/2829/texture-packing-algorithm
}

impl TextureCache {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            raster_cache: Arc::new(RwLock::new(RasterCache::new())),
            textures: vec![],
        }
    }

    pub fn unmark(&mut self) {
        self.raster_cache.write().unwrap().unmark();
    }
}
