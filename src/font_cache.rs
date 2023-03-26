use std::collections::HashMap;

use glyph_brush_layout::{ab_glyph::*, GlyphPositioner, SectionGeometry};
pub use glyph_brush_layout::{FontId, SectionGlyph, SectionText};
pub type Fonts = Vec<FontRef<'static>>;
pub type HorizontalAlign = glyph_brush_layout::HorizontalAlign;

pub struct FontCache {
    pub(crate) fonts: Fonts,
    pub(crate) font_names: HashMap<String, usize>,
    pub(crate) scale_factor: f32,
}

impl Default for FontCache {
    fn default() -> Self {
        Self {
            fonts: Default::default(),
            font_names: Default::default(),
            scale_factor: 1.0,
        }
    }
}

impl FontCache {
    pub fn font(&self, name: &str) -> Option<FontId> {
        if let Some(i) = self.font_names.get(name) {
            Some(FontId(*i))
        } else {
            None
        }
    }

    pub fn font_or_default(&self, name: Option<&str>) -> FontId {
        if let Some(name) = name {
            if let Some(i) = self.font_names.get(name) {
                return FontId(*i);
            }
        }

        self.default_font()
    }

    pub fn default_font(&self) -> FontId {
        if self.fonts.first().is_some() {
            FontId(0)
        } else {
            panic!("Expected at least one default font to be present")
        }
    }

    pub fn add_font(&mut self, name: &str, bytes: &'static [u8]) {
        let i = self.fonts.len();
        self.fonts.push(FontRef::try_from_slice(bytes).unwrap());
        self.font_names.insert(name.to_string(), i);
    }

    pub fn layout_text(
        &self,
        text: &[SectionText],
        alignment: HorizontalAlign,
        pos: (f32, f32),
        bounds: (f32, f32),
    ) -> Vec<SectionGlyph> {
        // TODO: Should accept an AABB and a start pos within it.
        glyph_brush_layout::Layout::default()
            .h_align(alignment)
            .calculate_glyphs(
                &self.fonts,
                &SectionGeometry {
                    screen_position: pos,
                    bounds,
                },
                text,
            )
    }
}
