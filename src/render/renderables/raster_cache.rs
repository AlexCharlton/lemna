use std::sync::atomic::{AtomicU64, Ordering};

use crate::PixelSize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RasterCacheId(usize);

pub type RasterId = u64;

static RASTER_ID_ATOMIC: AtomicU64 = AtomicU64::new(1);

fn new_raster_id() -> RasterId {
    RASTER_ID_ATOMIC.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Default)]
pub struct RasterCache {
    rasters: Vec<RasterData>,
}

#[derive(Debug)]
pub struct RasterData {
    pub(crate) id: RasterId,
    // TODO data should be an enum type that's either a static slice or a Vec
    pub data: Vec<u8>,
    pub size: PixelSize,
    /// Rasters are unmarked at the start of a render pass and marked as each renderable renders to them
    /// Rasters that remain unmarked at the end of the pass are free to be claimed for new renderables
    marked: bool,
}

impl RasterCache {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn unmark(&mut self) {
        for r in self.rasters.iter_mut() {
            r.marked = false;
        }
    }

    pub fn register(&mut self, raster_cache_id: RasterCacheId) {
        self.rasters[raster_cache_id.0].marked = true;
    }

    pub fn get_raster_data(&self, raster_cache_id: RasterCacheId) -> &RasterData {
        &self.rasters[raster_cache_id.0]
    }

    pub fn alloc_or_reuse_chunk(&mut self, raster_cache: Option<RasterCacheId>) -> RasterCacheId {
        if let Some(c) = raster_cache {
            c
        } else {
            RasterCacheId(
                if let Some(i) = self.rasters.iter().position(|r| !r.marked) {
                    i
                } else {
                    self.rasters.push(RasterData {
                        data: vec![],
                        id: 0,
                        marked: false,
                        size: PixelSize {
                            width: 0,
                            height: 0,
                        },
                    });
                    self.rasters.len() - 1
                },
            )
        }
    }

    pub fn set_raster(&mut self, raster_cache_id: RasterCacheId, data: Vec<u8>, size: PixelSize) {
        self.rasters[raster_cache_id.0] = RasterData {
            data,
            id: new_raster_id(),
            marked: false,
            size,
        };
    }
}
