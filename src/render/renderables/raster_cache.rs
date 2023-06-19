use std::sync::atomic::{AtomicU64, Ordering};

use crate::PixelSize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RasterCacheId(usize);

static RASTER_ID_ATOMIC: AtomicU64 = AtomicU64::new(1);

fn new_raster_id() -> u64 {
    RASTER_ID_ATOMIC.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Default)]
pub struct RasterCache {
    pub rasters: Vec<Raster>,
    pub sizes: Vec<PixelSize>,
    /// One mark per raster vector
    /// Vectors are unmarked at the start of a render pass and marked as each renderable renders to them
    /// Vectors that remain unmarked at the end of the pass are free to be claimed for new renderables
    pub marks: Vec<bool>,
}

#[derive(Debug)]
pub struct Raster {
    pub(crate) id: u64,
    pub data: Vec<u8>,
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

    pub fn register(&mut self, raster_cache_id: RasterCacheId) {
        self.marks[raster_cache_id.0] = true;
    }

    pub fn get_raster(&self, raster_cache_id: RasterCacheId) -> &Raster {
        &self.rasters[raster_cache_id.0]
    }

    pub fn alloc_or_reuse_chunk(&mut self, raster_cache: Option<RasterCacheId>) -> RasterCacheId {
        if let Some(c) = raster_cache {
            c
        } else {
            RasterCacheId(if let Some(i) = self.marks.iter().position(|i| !i) {
                i
            } else {
                self.rasters.push(Raster {
                    data: vec![],
                    id: 0,
                });
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
        self.rasters[raster_cache_id.0] = Raster {
            data,
            id: new_raster_id(),
        };
        self.sizes[raster_cache_id.0] = size;
    }
}
