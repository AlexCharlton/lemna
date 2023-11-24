use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::style::HorizontalPosition;
use glyph_brush_layout::{
    ab_glyph::*, FontId, GlyphPositioner, HorizontalAlign, SectionGeometry, SectionText,
};

type Fonts = Vec<FontRef<'static>>;

/// Output by [`FontCache::layout_text`], and an input to [`Text::new`](crate::render::renderables::text::Text::new). Useful for text-rendering widgets to cache.
pub type SectionGlyph = glyph_brush_layout::SectionGlyph;

/// Value by which fonts are scaled. 12 px fonts render at scale 18 px for some reason.
pub const SIZE_SCALE: f32 = 1.5;

#[derive(Default)]
pub struct FontCache {
    pub(crate) fonts: Fonts,
    pub(crate) font_names: HashMap<String, usize>,
}

impl FontCache {
    fn font(&self, name: &str) -> Option<FontId> {
        self.font_names.get(name).map(|i| FontId(*i))
    }

    fn font_or_default(&self, name: Option<&str>) -> FontId {
        if let Some(name) = name {
            if let Some(i) = self.font_names.get(name) {
                return FontId(*i);
            }
        }

        self.default_font()
    }

    fn default_font(&self) -> FontId {
        if self.fonts.first().is_some() {
            FontId(0)
        } else {
            panic!("Expected at least one default font to be present")
        }
    }

    /// bytes is an OpenType font
    pub fn add_font(&mut self, name: String, bytes: &'static [u8]) {
        let i = self.fonts.len();
        self.fonts.push(FontRef::try_from_slice(bytes).unwrap());
        self.font_names.insert(name, i);
    }

    pub fn layout_text(
        &self,
        text: &[TextSegment],
        base_font: Option<&str>,
        base_size: f32,
        scale: f32,
        alignment: HorizontalPosition,
        pos: (f32, f32),
        bounds: (f32, f32),
    ) -> Vec<SectionGlyph> {
        // TODO: Should accept an AABB and a start pos within it.
        let scaled_size = base_size * scale * SIZE_SCALE;
        let base_font = self.font_or_default(base_font.as_deref());

        let section_text: Vec<_> = text
            .iter()
            .map(|TextSegment { text, size, font }| SectionText {
                text,
                scale: size.map_or(scaled_size, |s| s * scale * SIZE_SCALE).into(),
                font_id: font
                    .as_ref()
                    .and_then(|f| self.font(f))
                    .unwrap_or(base_font),
            })
            .collect();

        glyph_brush_layout::Layout::default()
            .h_align(match alignment {
                HorizontalPosition::Left => HorizontalAlign::Left,
                HorizontalPosition::Right => HorizontalAlign::Right,
                HorizontalPosition::Center => HorizontalAlign::Center,
            })
            .calculate_glyphs(
                &self.fonts,
                &SectionGeometry {
                    screen_position: pos,
                    bounds,
                },
                &section_text,
            )
    }

    pub fn glyph_widths(
        &self,
        font: Option<&str>,
        scaled_size: f32,
        glyphs: &[SectionGlyph],
    ) -> Vec<f32> {
        let font_ref = self.font_or_default(font.as_deref());
        let font = &self.fonts[font_ref.0];

        glyphs
            .iter()
            .map(|g| {
                font.as_scaled(scaled_size * SIZE_SCALE)
                    .h_advance(g.glyph.id)
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TextSegment {
    pub text: String,
    pub size: Option<f32>,
    pub font: Option<String>,
}

impl From<&str> for TextSegment {
    fn from(s: &str) -> TextSegment {
        s.to_string().into()
    }
}

impl From<String> for TextSegment {
    fn from(text: String) -> TextSegment {
        TextSegment {
            text,
            size: None,
            font: None,
        }
    }
}

#[cfg(feature = "open_iconic")]
impl From<crate::open_iconic::Icon> for TextSegment {
    fn from(icon: crate::open_iconic::Icon) -> TextSegment {
        String::from(icon).into()
    }
}

#[macro_export]
macro_rules! txt {
    // split_comma taken from: https://gist.github.com/kyleheadley/c2f64e24c14e45b1e39ee664059bd86f

    // give initial params to the function
    {@split_comma  ($($first:tt)*) <= $($item:tt)*} => {
        txt![@split_comma ($($first)*) () () <= $($item)*]

    };
    // give inital params and initial inner items in every group
    {@split_comma  ($($first:tt)*) ($($every:tt)*) <= $($item:tt)*} => {
        txt![@split_comma ($($first)*) ($($every)*) ($($every)*) <= $($item)*]

    };
    // KEYWORD line
    // on non-final seperator, stash the accumulator and restart it
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)*) <= , $($item:tt)+} => {
        txt![@split_comma ($($first)* ($($current)*)) ($($every)*) ($($every)*) <= $($item)*]

    };
    // KEYWORD line
    // ignore final seperator, run the function
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)+) <= , } => {
        txt![@txt_seg $($first)* ($($current)*)]

    };
    // on next item, add it to the accumulator
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)*) <= $next:tt $($item:tt)*} => {
        txt![@split_comma ($($first)*) ($($every)*) ($($current)* $next)  <= $($item)*]

    };
    // at end of items, run the function
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)+) <= } => {
        txt![@txt_seg $($first)* ($($current)*)]

    };
    // if there were no items and no default, run with only initial params, if any
    {@split_comma  ($($first:tt)*) () () <= } => {
        txt![@txt_seg $($first)*]

    };
    // End split_comma

    // Operation performed per comma-separated expr
    (@as_txt_seg  ($text:expr, None, $size:expr)) => { $crate::font_cache::TextSegment {
        text: $text.into(),
        size: Some($size),
        font: None,
    } };

    (@as_txt_seg  ($text:expr, $font:expr, $size:expr)) => { $crate::font_cache::TextSegment {
        text: $text.into(),
        size: Some($size),
        font: Some($font.into()),
    } };

    (@as_txt_seg  ($text:expr, $font:expr)) => { $crate::font_cache::TextSegment {
        text: $text.into(),
        size: None,
        font: Some($font.into()),
    } };

    (@as_txt_seg  $e:expr) => {
        $e.into()
    };

    // Operation called by split_comma with parenthesized groups
    (@txt_seg  $(($($item:tt)*))*) => { vec![$(txt!(@as_txt_seg $($item)*) , )*] };

    // Entry point
    ($($e:tt)*) => {
        txt![@split_comma () () () <= $($e)*]
    }
}

impl Hash for TextSegment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.size.map(|s| (s * 100.0) as u32).hash(state);
        self.font.hash(state);
        self.text.hash(state);
    }
}
