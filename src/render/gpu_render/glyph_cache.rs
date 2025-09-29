//! Rasterization cache for fontdue. Manages a
//! updates to a texture (e.g. one stored on a GPU) drawing new glyphs, reusing & reordering
//! as necessary.
//!

//! This is a reimplementation based on the discussion here: <https://github.com/alexheretic/glyph-brush/pull/120>
//! Without these changes, it's just too slow!
//! This has since been changed to use fontdue
extern crate alloc;

use alloc::{vec, vec::Vec};
use core::{error, fmt, hash::BuildHasherDefault, ops};

use ahash::{HashMap, HashSet};
use fontdue::Font;
use fontdue::layout::{GlyphPosition, GlyphRasterConfig};
use indexmap::IndexMap;

use crate::base_types::{Point, Pos, Rect};

type FxBuildHasher = BuildHasherDefault<ahash::AHasher>;

/// Indicates where a glyph texture is stored in the cache
/// (row position, glyph index in row)
type TextureRowGlyphIndex = (u32, u32);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ByteArray2d {
    inner_array: Vec<u8>,
    row: usize,
    col: usize,
}

impl ByteArray2d {
    #[inline]
    pub fn zeros(row: usize, col: usize) -> Self {
        ByteArray2d {
            inner_array: vec![0; row * col],
            row,
            col,
        }
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        self.inner_array.as_slice()
    }

    #[inline]
    fn get_vec_index(&self, row: usize, col: usize) -> usize {
        debug_assert!(
            row < self.row,
            "row out of range: row={}, given={}",
            self.row,
            row
        );
        debug_assert!(
            col < self.col,
            "column out of range: col={}, given={}",
            self.col,
            col
        );
        row * self.col + col
    }
}

impl ops::Index<(usize, usize)> for ByteArray2d {
    type Output = u8;

    #[inline]
    fn index(&self, (row, col): (usize, usize)) -> &u8 {
        &self.inner_array[self.get_vec_index(row, col)]
    }
}

impl ops::IndexMut<(usize, usize)> for ByteArray2d {
    #[inline]
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut u8 {
        let vec_index = self.get_vec_index(row, col);
        &mut self.inner_array[vec_index]
    }
}

/// Row of pixel data
struct Row {
    /// Row pixel height
    height: u32,
    /// Pixel width current in use by glyphs
    width: u32,
    /// Does the row have any glyphs that need to be cached?
    dirty: bool,
    glyphs: Vec<GlyphTexInfo>,
}

struct GlyphTexInfo {
    glyph_info: GlyphRasterConfig,
    tex_coords: Rectangle<u32>,
}

trait PaddingAware {
    fn unpadded(self) -> Self;
}

impl PaddingAware for Rectangle<u32> {
    /// A padded texture has 1 extra pixel on all sides
    fn unpadded(mut self) -> Self {
        self.min[0] += 1;
        self.min[1] += 1;
        self.max[0] -= 1;
        self.max[1] -= 1;
        self
    }
}

/// Builder & rebuilder for `DrawCache`.
#[derive(Debug, Clone)]
pub struct DrawCacheBuilder {
    dimensions: (u32, u32),
    scale_tolerance: f32,
    position_tolerance: f32,
    pad_glyphs: bool,
    align_4x4: bool,
}

impl Default for DrawCacheBuilder {
    fn default() -> Self {
        Self {
            dimensions: (256, 256),
            scale_tolerance: 0.2,
            position_tolerance: 0.2,
            pad_glyphs: true,
            align_4x4: false,
        }
    }
}

impl DrawCacheBuilder {
    /// `width` & `height` dimensions of the 2D texture that will hold the
    /// cache contents on the GPU.
    ///
    /// This must match the dimensions of the actual texture used, otherwise
    /// `cache_queued` will try to cache into coordinates outside the bounds of
    /// the texture.
    ///
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.dimensions = (width, height);
        self
    }

    /// Specifies the tolerances (maximum allowed difference) for judging
    /// whether an existing glyph in the cache is close enough to the
    /// requested glyph in scale to be used in its place. Due to floating
    /// point inaccuracies a min value of `0.001` is enforced.
    ///
    /// Both `scale_tolerance` and `position_tolerance` are measured in pixels.
    ///
    /// Tolerances produce even steps for scale and subpixel position. Only a
    /// single glyph texture will be used within a single step. For example,
    /// `scale_tolerance = 0.1` will have a step `9.95-10.05` so similar glyphs
    /// with scale `9.98` & `10.04` will match.
    ///
    /// A typical application will produce results with no perceptible
    /// inaccuracies with `scale_tolerance` and `position_tolerance` set to
    /// 0.1. Depending on the target DPI higher tolerance may be acceptable.
    ///
    #[allow(dead_code)]
    pub fn scale_tolerance<V: Into<f32>>(mut self, scale_tolerance: V) -> Self {
        self.scale_tolerance = scale_tolerance.into();
        self
    }
    /// Specifies the tolerances (maximum allowed difference) for judging
    /// whether an existing glyph in the cache is close enough to the requested
    /// glyph in subpixel offset to be used in its place. Due to floating
    /// point inaccuracies a min value of `0.001` is enforced.
    ///
    /// Both `scale_tolerance` and `position_tolerance` are measured in pixels.
    ///
    /// Tolerances produce even steps for scale and subpixel position. Only a
    /// single glyph texture will be used within a single step. For example,
    /// `scale_tolerance = 0.1` will have a step `9.95-10.05` so similar glyphs
    /// with scale `9.98` & `10.04` will match.
    ///
    /// Note that since `position_tolerance` is a tolerance of subpixel
    /// offsets, setting it to 1.0 or higher is effectively a "don't care"
    /// option.
    ///
    /// A typical application will produce results with no perceptible
    /// inaccuracies with `scale_tolerance` and `position_tolerance` set to
    /// 0.1. Depending on the target DPI higher tolerance may be acceptable.
    ///
    #[allow(dead_code)]
    pub fn position_tolerance<V: Into<f32>>(mut self, position_tolerance: V) -> Self {
        self.position_tolerance = position_tolerance.into();
        self
    }
    /// Pack glyphs in texture with a padding of a single zero alpha pixel to
    /// avoid bleeding from interpolated shader texture lookups near edges.
    ///
    /// If glyphs are never transformed this may be set to `false` to slightly
    /// improve the glyph packing.
    ///
    #[allow(dead_code)]
    pub fn pad_glyphs(mut self, pad_glyphs: bool) -> Self {
        self.pad_glyphs = pad_glyphs;
        self
    }
    /// Align glyphs in texture to 4x4 texel boundaries.
    ///
    /// If your backend requires texture updates to be aligned to 4x4 texel
    /// boundaries (e.g. WebGL), this should be set to `true`.
    ///
    #[allow(dead_code)]
    pub fn align_4x4(mut self, align_4x4: bool) -> Self {
        self.align_4x4 = align_4x4;
        self
    }
    fn validated(self) -> Self {
        assert!(self.scale_tolerance >= 0.0);
        assert!(self.position_tolerance >= 0.0);
        let scale_tolerance = self.scale_tolerance.max(0.001);
        let position_tolerance = self.position_tolerance.max(0.001);
        Self {
            scale_tolerance,
            position_tolerance,
            ..self
        }
    }

    /// Constructs a new cache. Note that this is just the CPU side of the
    /// cache. The GPU texture is managed by the user.
    ///
    /// # Panics
    ///
    /// `scale_tolerance` or `position_tolerance` are less than or equal to
    /// zero.
    pub fn build(self) -> DrawCache {
        let DrawCacheBuilder {
            dimensions: (width, height),
            scale_tolerance,
            position_tolerance,
            pad_glyphs,
            align_4x4,
        } = self.validated();

        DrawCache {
            scale_tolerance,
            position_tolerance,
            width,
            height,
            rows: IndexMap::default(),
            space_start_for_end: {
                let mut m = HashMap::default();
                m.insert(height, 0);
                m
            },
            space_end_for_start: {
                let mut m = HashMap::default();
                m.insert(0, height);
                m
            },
            queue: Vec::new(),
            all_glyphs: HashMap::default(),
            pad_glyphs,
            align_4x4,
            cpu_cache: ByteArray2d::zeros(width as usize, height as usize),
        }
    }

    /// Rebuilds a `DrawCache` with new attributes. All cached glyphs are cleared,
    /// however the glyph queue is retained unmodified.
    ///
    /// # Panics
    ///
    /// `scale_tolerance` or `position_tolerance` are less than or equal to
    /// zero.

    #[allow(dead_code)]
    pub fn rebuild(self, cache: &mut DrawCache) {
        let DrawCacheBuilder {
            dimensions: (width, height),
            scale_tolerance,
            position_tolerance,
            pad_glyphs,
            align_4x4,
        } = self.validated();

        cache.width = width;
        cache.height = height;
        cache.scale_tolerance = scale_tolerance;
        cache.position_tolerance = position_tolerance;
        cache.pad_glyphs = pad_glyphs;
        cache.align_4x4 = align_4x4;
        cache.cpu_cache = ByteArray2d::zeros(width as usize, height as usize);
        cache.clear();
    }
}

/// Returned from `DrawCache::cache_queued`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CacheWriteErr {
    /// At least one of the queued glyphs is too big to fit into the cache, even
    /// if all other glyphs are removed.
    GlyphTooLarge,
    /// Not all of the requested glyphs can fit into the cache, even if the
    /// cache is completely cleared before the attempt.
    NoRoomForWholeQueue,
}

impl fmt::Display for CacheWriteErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CacheWriteErr::GlyphTooLarge => "Glyph too large",
            CacheWriteErr::NoRoomForWholeQueue => "No room for whole queue",
        }
        .fmt(f)
    }
}

impl error::Error for CacheWriteErr {}

/// Successful method of caching of the queue.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CachedBy {
    /// Added any additional glyphs into the texture without affecting
    /// the position of any already cached glyphs in the latest queue.
    ///
    /// Glyphs not in the latest queue may have been removed.
    Adding,
    /// Fit the glyph queue by re-ordering all glyph texture positions.
    /// Previous texture positions are no longer valid.
    Reordering,
}

/// Dynamic rasterization draw cache.
pub struct DrawCache {
    scale_tolerance: f32,
    position_tolerance: f32,
    width: u32,
    height: u32,
    // Start y pixel position is the index
    rows: IndexMap<u32, Row, FxBuildHasher>,
    /// Mapping of row gaps bottom -> top
    space_start_for_end: HashMap<u32, u32>,
    /// Mapping of row gaps top -> bottom
    space_end_for_start: HashMap<u32, u32>,
    queue: Vec<GlyphPosition>,
    all_glyphs: HashMap<GlyphRasterConfig, TextureRowGlyphIndex>,
    pad_glyphs: bool,
    align_4x4: bool,
    cpu_cache: ByteArray2d,
}

impl DrawCache {
    /// Returns a default `DrawCacheBuilder`.
    #[inline]
    pub fn builder() -> DrawCacheBuilder {
        DrawCacheBuilder::default()
    }

    /// Returns the current scale tolerance for the cache.
    #[allow(dead_code)]
    pub fn scale_tolerance(&self) -> f32 {
        self.scale_tolerance
    }

    /// Returns the current subpixel position tolerance for the cache.
    #[allow(dead_code)]
    pub fn position_tolerance(&self) -> f32 {
        self.position_tolerance
    }

    /// Returns the cache texture dimensions assumed by the cache. For proper
    /// operation this should match the dimensions of the used GPU texture.
    #[allow(dead_code)]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Queue a glyph for caching by the next call to `cache_queued`. `font_id`
    /// is used to disambiguate glyphs from different fonts. The user should
    /// ensure that `font_id` is unique to the font the glyph is from.
    pub fn queue_glyph(&mut self, glyph: GlyphPosition) {
        self.queue.push(glyph);
    }

    /// Clears the cache. Does not affect the glyph queue.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.space_end_for_start.clear();
        self.space_end_for_start.insert(0, self.height);
        self.space_start_for_end.clear();
        self.space_start_for_end.insert(self.height, 0);
        self.all_glyphs.clear();
    }

    /// Marks all rows as not dirty
    pub fn clean_rows(&mut self) {
        for (_, row) in self.rows.iter_mut() {
            row.dirty = false;
        }
    }

    /// Clears the glyph queue.
    #[allow(dead_code)]
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Caches the queued glyphs. If this is unsuccessful, the queue is
    /// untouched. Any glyphs cached by previous calls to this function may be
    /// removed from the cache to make room for the newly queued glyphs. Thus if
    /// you want to ensure that a glyph is in the cache, the most recently
    /// cached queue must have contained that glyph.
    ///
    /// `uploader` is the user-provided function that should perform the texture
    /// uploads to the GPU. The information provided is the rectangular region
    /// to insert the pixel data into, and the pixel data itself. This data is
    /// provided in horizontal scanline format (row major), with stride equal to
    /// the rectangle width.
    ///
    /// If successful returns a `CachedBy` that can indicate the validity of
    /// previously cached glyph textures.
    pub fn cache_queued<U>(
        &mut self,
        fonts: &[Font],
        mut uploader: U,
    ) -> Result<CachedBy, CacheWriteErr>
    where
        U: FnMut(Rectangle<u32>, &[u8]),
    {
        let mut queue_success = true;
        let from_empty = self.all_glyphs.is_empty();

        {
            self.clean_rows();

            let (mut in_use_rows, uncached_glyphs) = {
                let mut in_use_rows = HashSet::with_capacity_and_hasher(
                    self.rows.len(),
                    ahash::RandomState::default(),
                );
                let mut uncached_glyphs = HashMap::with_capacity_and_hasher(
                    self.queue.len(),
                    ahash::RandomState::default(),
                );

                // divide glyphs into texture rows where a matching glyph texture
                // already exists & glyphs where new textures must be cached
                for glyph in &self.queue {
                    let glyph_info = glyph.key;
                    if let Some((row, ..)) = self.all_glyphs.get(&glyph_info) {
                        in_use_rows.insert(*row);
                    } else {
                        uncached_glyphs.insert(glyph_info, glyph);
                    }
                }

                (in_use_rows, uncached_glyphs)
            };

            for k in &in_use_rows {
                if let Some(row) = self.rows.shift_remove(k) {
                    self.rows.insert(*k, row);
                }
            }

            let mut uncached_metrics: Vec<_> = uncached_glyphs
                .into_iter()
                .filter_map(|(_, glyph)| {
                    Some((
                        glyph,
                        fonts[glyph.font_index]
                            .metrics_indexed(glyph.key.glyph_index, glyph.key.px),
                    ))
                })
                .collect();

            // tallest first gives better packing
            // can use 'sort_unstable' as order of equal elements is unimportant
            uncached_metrics.sort_unstable_by(|(_, ga), (_, gb)| {
                gb.height
                    .partial_cmp(&ga.height)
                    .unwrap_or(core::cmp::Ordering::Equal)
            });

            self.all_glyphs.reserve(uncached_metrics.len());
            let mut draw_and_upload = Vec::with_capacity(uncached_metrics.len());

            'per_glyph: for (glyph, metrics) in uncached_metrics {
                let (unaligned_width, unaligned_height) = {
                    if self.pad_glyphs {
                        (metrics.width as u32 + 2, metrics.height as u32 + 2)
                    } else {
                        (metrics.width as u32, metrics.height as u32)
                    }
                };
                let (aligned_width, aligned_height) = if self.align_4x4 {
                    // align to the next 4x4 texel boundary
                    ((unaligned_width + 3) & !3, (unaligned_height + 3) & !3)
                } else {
                    (unaligned_width, unaligned_height)
                };
                if aligned_width >= self.width || aligned_height >= self.height {
                    return Result::Err(CacheWriteErr::GlyphTooLarge);
                }
                // find row to put the glyph in, most used rows first
                let mut row_top = None;
                for (top, row) in self.rows.iter().rev() {
                    if row.height >= aligned_height && self.width - row.width >= aligned_width {
                        // found a spot on an existing row
                        row_top = Some(*top);
                        break;
                    }
                }

                if row_top.is_none() {
                    let mut gap = None;
                    // See if there is space for a new row
                    for (start, end) in &self.space_end_for_start {
                        if end - start >= aligned_height {
                            gap = Some((*start, *end));
                            break;
                        }
                    }
                    if gap.is_none() {
                        // Remove old rows until room is available
                        while !self.rows.is_empty() {
                            // check that the oldest row isn't also in use
                            if !in_use_rows.contains(self.rows.first().unwrap().0) {
                                // Remove row
                                let (top, row) = self.rows.shift_remove_index(0).unwrap();

                                for g in row.glyphs {
                                    self.all_glyphs.remove(&g.glyph_info);
                                }

                                let (mut new_start, mut new_end) = (top, top + row.height);
                                // Update the free space maps
                                // Combine with neighbouring free space if possible
                                if let Some(end) = self.space_end_for_start.remove(&new_end) {
                                    new_end = end;
                                }
                                if let Some(start) = self.space_start_for_end.remove(&new_start) {
                                    new_start = start;
                                }
                                self.space_start_for_end.insert(new_end, new_start);
                                self.space_end_for_start.insert(new_start, new_end);
                                if new_end - new_start >= aligned_height {
                                    // The newly formed gap is big enough
                                    gap = Some((new_start, new_end));
                                    break;
                                }
                            }
                            // all rows left are in use
                            // try a clean insert of all needed glyphs
                            // if that doesn't work, fail
                            else if from_empty {
                                // already trying a clean insert, don't do it again
                                return Err(CacheWriteErr::NoRoomForWholeQueue);
                            } else {
                                // signal that a retry is needed
                                queue_success = false;
                                break 'per_glyph;
                            }
                        }
                    }
                    let (gap_start, gap_end) = gap.unwrap();
                    // fill space for new row
                    let new_space_start = gap_start + aligned_height;
                    self.space_end_for_start.remove(&gap_start);
                    if new_space_start == gap_end {
                        self.space_start_for_end.remove(&gap_end);
                    } else {
                        self.space_end_for_start.insert(new_space_start, gap_end);
                        self.space_start_for_end.insert(gap_end, new_space_start);
                    }
                    // add the row
                    self.rows.insert(
                        gap_start,
                        Row {
                            width: 0,
                            height: aligned_height,
                            glyphs: Vec::new(),
                            dirty: true,
                        },
                    );
                    row_top = Some(gap_start);
                }
                let row_top = row_top.unwrap();
                // calculate the target rect
                let mut row = self.rows.shift_remove(&row_top).unwrap();

                let aligned_tex_coords = Rectangle {
                    min: [row.width, row_top],
                    max: [row.width + aligned_width, row_top + aligned_height],
                };
                let unaligned_tex_coords = Rectangle {
                    min: [row.width, row_top],
                    max: [row.width + unaligned_width, row_top + unaligned_height],
                };

                // add the glyph to the row
                row.glyphs.push(GlyphTexInfo {
                    glyph_info: glyph.key,
                    tex_coords: unaligned_tex_coords,
                });
                row.dirty = true;
                row.width += aligned_width;
                in_use_rows.insert(row_top);

                draw_and_upload.push((aligned_tex_coords, glyph));

                self.all_glyphs
                    .insert(glyph.key, (row_top, row.glyphs.len() as u32 - 1));
                self.rows.insert(row_top, row);
            }

            if queue_success {
                {
                    // single thread rasterization
                    for (tex_coords, glyph) in draw_and_upload {
                        draw_glyph_onto_buffer(
                            &mut self.cpu_cache,
                            tex_coords,
                            fonts,
                            glyph,
                            self.pad_glyphs,
                        );
                    }

                    let mut dirty_rows = self
                        .rows
                        .iter()
                        .filter(|(_, r)| r.dirty)
                        .collect::<Vec<_>>();

                    // Find contiguous slices of dirty rows
                    dirty_rows.sort_by(|(top_a, _), (top_b, _)| top_a.cmp(top_b));
                    let mut slices: Vec<(u32, u32)> = vec![];
                    for (top, row) in dirty_rows {
                        if let Some(slice) = slices.last_mut() {
                            if slice.1 == *top {
                                slice.1 = top + row.height;
                            } else {
                                slices.push((*top, top + row.height));
                            }
                        } else {
                            slices.push((*top, top + row.height));
                        }
                    }

                    // Send contiguous slices to uploader
                    for (top, bottom) in slices {
                        let tex_coords = Rectangle {
                            min: [0, top],
                            max: [self.width, bottom],
                        };
                        uploader(
                            tex_coords,
                            &self.cpu_cache.as_slice()
                                [(self.width * top) as usize..(self.width * bottom) as usize],
                        );
                    }
                }
            }
        }

        if queue_success {
            self.queue.clear();
            Ok(CachedBy::Adding)
        } else {
            // clear the cache then try again with optimal packing
            self.clear();
            self.cache_queued(fonts, uploader)
                .map(|_| CachedBy::Reordering)
        }
    }

    /// Retrieves the (floating point) texture coordinates of the quad for a
    /// glyph in the cache, as well as the pixel-space (integer) coordinates
    /// that this region should be drawn at.
    ///
    /// A successful result is `Some` if the glyph is not an empty glyph (no
    /// shape, and thus no rect to return).
    pub fn rect_for(&self, glyph: &GlyphPosition) -> Option<Rect> {
        let (row, index) = self.all_glyphs.get(&glyph.key)?;

        let (tex_width, tex_height) = (self.width as f32, self.height as f32);

        let GlyphTexInfo {
            tex_coords: mut tex_rect,
            ..
        } = self.rows[row].glyphs[*index as usize];
        if self.pad_glyphs {
            tex_rect = tex_rect.unpadded();
        }
        let uv_rect = Rect {
            pos: Pos::new(
                tex_rect.min[0] as f32 / tex_width,
                tex_rect.min[1] as f32 / tex_height,
                0.0,
            ),
            bottom_right: Point::new(
                tex_rect.max[0] as f32 / tex_width,
                tex_rect.max[1] as f32 / tex_height,
            ),
        };
        Some(uv_rect)
    }
}

#[inline]
fn draw_glyph_onto_buffer(
    buffer: &mut ByteArray2d,
    tex_coords: Rectangle<u32>,
    fonts: &[Font],
    glyph: &GlyphPosition,
    pad_glyphs: bool,
) {
    let font = &fonts[glyph.font_index];
    let (metrics, raster) = font.rasterize_config(glyph.key);
    let mut x = 0;
    let mut y = 0;

    if pad_glyphs {
        for v in raster {
            buffer[(
                (y + tex_coords.min[1]) as usize + 1,
                (x + tex_coords.min[0]) as usize + 1,
            )] = v;

            x += 1;
            if x >= metrics.width as u32 {
                x = 0;
                y += 1;
            }
        }
    } else {
        for v in raster {
            buffer[(
                (y + tex_coords.min[1]) as usize,
                (x + tex_coords.min[0]) as usize,
            )] = v;

            x += 1;
            if x >= metrics.width as u32 {
                x = 0;
                y += 1;
            }
        }
    }
}

/// A rectangle, with top-left corner at min, and bottom-right corner at max.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rectangle<N> {
    pub min: [N; 2],
    pub max: [N; 2],
}

#[allow(dead_code)]
impl<N: ops::Sub<Output = N> + Copy> Rectangle<N> {
    pub fn width(&self) -> N {
        self.max[0] - self.min[0]
    }

    pub fn height(&self) -> N {
        self.max[1] - self.min[1]
    }
}
