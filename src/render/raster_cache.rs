extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::{PixelSize, renderable::RasterData};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RasterCacheId(usize);

impl RasterCacheId {
    #[allow(unused)]
    // TODO: Is this needed?
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

pub(crate) type RasterId = u64;

static RASTER_ID_ATOMIC: AtomicU64 = AtomicU64::new(1);

fn new_raster_id() -> RasterId {
    RASTER_ID_ATOMIC.fetch_add(1, Ordering::SeqCst)
}

#[derive(Default, Debug)]
pub struct RasterCache {
    rasters: Vec<RasterCacheData>,
}

#[derive(Debug)]
pub struct RasterCacheData {
    pub(crate) id: RasterId,
    pub data: RasterData,
    pub size: PixelSize,
    /// Has this raster been altered?
    pub dirty: bool,
    /// Rasters are unmarked at the start of a render pass and marked as each renderable renders to them
    /// Rasters that remain unmarked at the end of the pass are free to be claimed for new renderables
    marked: bool,
}

impl RasterCacheData {
    pub fn dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clean(&mut self) {
        self.dirty = false;
    }
}

impl RasterCache {
    pub fn unmark(&mut self) {
        for r in self.rasters.iter_mut() {
            r.marked = false;
        }
    }

    pub fn register(&mut self, raster_cache_id: RasterCacheId) {
        self.rasters[raster_cache_id.0].marked = true;
    }

    pub fn get_raster_data(&self, raster_cache_id: RasterCacheId) -> &RasterCacheData {
        &self.rasters[raster_cache_id.0]
    }

    pub fn get_mut_raster_data(&mut self, raster_cache_id: RasterCacheId) -> &mut RasterCacheData {
        &mut self.rasters[raster_cache_id.0]
    }

    pub fn alloc_or_reuse_chunk(&mut self, raster_cache: Option<RasterCacheId>) -> RasterCacheId {
        if let Some(c) = raster_cache {
            c
        } else {
            RasterCacheId(
                if let Some(i) = self.rasters.iter().position(|r| !r.marked) {
                    i
                } else {
                    self.rasters.push(RasterCacheData {
                        data: RasterData::Slice(&[]),
                        id: 0,
                        marked: true,
                        dirty: true,
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

    pub fn set_raster<D: Into<RasterData>>(
        &mut self,
        raster_cache_id: RasterCacheId,
        data: D,
        size: PixelSize,
    ) {
        self.rasters[raster_cache_id.0] = RasterCacheData {
            data: data.into(),
            id: new_raster_id(),
            marked: true,
            dirty: true,
            size,
        };
    }
}
