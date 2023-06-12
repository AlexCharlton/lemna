use crate::renderables::raster_cache::RasterCache;
use wgpu;

pub struct TextureCache {
    raster_cache: RasterCache,
    textures: Vec<PackedTexture>,
}

pub struct PackedTexture {
    texture: wgpu::Texture, // TODO track what rasters are used where
                            // https://gamedev.stackexchange.com/questions/2829/texture-packing-algorithm
}
