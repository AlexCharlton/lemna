use crate::PixelSize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RasterCacheId(usize);

#[derive(Debug, Default)]
pub struct RasterCache {
    pub rasters: Vec<Vec<u8>>,
    pub sizes: Vec<PixelSize>,
    /// One mark per raster vector
    /// Vectors are unmarked at the start of a render pass and marked as each renderable renders to them
    /// Vectors that remain unmarked at the end of the pass are free to be claimed for new renderables
    pub marks: Vec<bool>,
}

impl RasterCache {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn unmark(&mut self) {
        for m in self.marks.iter_mut() {
            *m = false;
        }
    }

    pub fn register(&mut self, vector: RasterCacheId) {
        self.marks[vector.0] = true;
    }

    pub fn alloc_or_reuse_chunk(&mut self, raster_cache: Option<RasterCacheId>) -> RasterCacheId {
        if let Some(c) = raster_cache {
            c
        } else {
            RasterCacheId(if let Some(i) = self.marks.iter().position(|i| !i) {
                i
            } else {
                self.rasters.push(vec![]);
                self.marks.push(false);
                self.sizes.push(PixelSize {
                    width: 0,
                    height: 0,
                });
                self.rasters.len() - 1
            })
        }
    }

    pub fn set_raster(&mut self, raster_cache_id: RasterCacheId, data: Vec<u8>, size: PixelSize) {
        self.rasters[raster_cache_id.0] = data;
        self.sizes[raster_cache_id.0] = size;
    }
}
