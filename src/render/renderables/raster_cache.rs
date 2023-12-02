use std::sync::atomic::{AtomicU64, Ordering};

use crate::PixelSize;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RasterCacheId(usize);

impl RasterCacheId {
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

pub enum RasterData {
    Vec(Vec<u8>),
    Slice(&'static [u8]),
}

impl std::fmt::Debug for RasterData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let (t, len) = match self {
            RasterData::Slice(d) => ("Slice", d.len()),
            RasterData::Vec(d) => ("Vec", d.len()),
        };
        write!(f, "RasterData::{}<len: {}>", t, len)?;
        Ok(())
    }
}

impl From<&'static [u8]> for RasterData {
    fn from(d: &'static [u8]) -> Self {
        RasterData::Slice(d)
    }
}

impl From<Vec<u8>> for RasterData {
    fn from(d: Vec<u8>) -> Self {
        RasterData::Vec(d)
    }
}

impl<'a> From<&'a RasterData> for &'a [u8] {
    fn from(d: &'a RasterData) -> &'a [u8] {
        match d {
            RasterData::Vec(v) => &v[..],
            RasterData::Slice(s) => s,
        }
    }
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
