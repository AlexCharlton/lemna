use ahash::HashMap;
use fontdue::Font;
use fontdue::layout::{GlyphPosition, GlyphRasterConfig};

use tiny_skia::{IntSize, Mask};

#[derive(Default)]
pub struct GlyphCache {
    // Masks are just a convenient way to store a rectangle of u8 alpha values.
    glyphs: HashMap<GlyphRasterConfig, Mask>,
}

impl GlyphCache {
    // Maybe TODO: This never frees any glyphs, so it can grow without bounds.
    pub fn glyph_mask(&mut self, fonts: &[Font], glyph: &GlyphPosition) -> Option<&Mask> {
        if !self.glyphs.contains_key(&glyph.key) {
            let (metrics, raster) = fonts[glyph.font_index].rasterize_config(glyph.key);
            if let Some(size) = IntSize::from_wh(metrics.width as u32, metrics.height as u32) {
                if let Some(mask) = Mask::from_vec(raster, size) {
                    self.glyphs.insert(glyph.key, mask);
                }
            }
        }
        self.glyphs.get(&glyph.key)
    }
}
