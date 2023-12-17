//! The [`FontCache`] is where fonts are stored, and where text layout happens.
//!
//! Adding fonts to the `FontCache` is done via [`UI#add_font`][crate::UI#add_font], and window backends may include hooks to add fonts on application load.
//!
//! The `FontCache` is exposed to users so that you can lay out text (i.e. when you're not using a Component that lays out text for you, like [`widgets::Text`][crate::widgets::Text]) via the [`Caches`][crate::Caches] referenced by the [`RenderContext`][crate::RenderContext] which gets passed to [`Component#render`][crate::Component#render].
//!
//! The text-layout interface uses a slice of [`TextSegment`]s as a Component-agnostic way of representing text. A `TextSegment` stores a text string, and optionally a font size and font name (defaults will be used otherwise). In this way, we can lay out text in a variety of types and sizes. [`txt`][crate::txt] is provided as a convenient way of creating `TextSegment`s.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::style::HorizontalPosition;
use glyph_brush_layout::{
    ab_glyph::*, FontId, GlyphPositioner, HorizontalAlign, SectionGeometry, SectionText,
};

type Fonts = Vec<FontRef<'static>>;

/// Output by [`FontCache::layout_text`], and an input to [`Text::new`](crate::render::renderables::text::Text::new). Useful for text-rendering widgets to cache in their state, so that they don't need to be recomputed unless necessary.
pub type SectionGlyph = glyph_brush_layout::SectionGlyph;

/// Value by which fonts are scaled. 12 px fonts render at scale 18 px for some reason. Useful if you need to compute the line height: it will be `<font_size> * SIZE_SCALE` in logical size, and `<font_size> * SIZE_SCALE * <scale_factor>` in physical pixels.
pub const SIZE_SCALE: f32 = 1.5;

/// Stores fonts, and provides text layout functionality to Components who render.
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
    pub(crate) fn add_font(&mut self, name: String, bytes: &'static [u8]) {
        let i = self.fonts.len();
        self.fonts.push(FontRef::try_from_slice(bytes).unwrap());
        self.font_names.insert(name, i);
    }

    /// Given a set of [`TextSegment`]s, create [`SectionGlyph`]s, which are then used by the [`Text`][crate::renderables::Text] renderable.
    ///
    /// `base_font` and `base_size` are provided as fallbacks for when a `TextSegment` does not specify a font or size. `scale_factor` is the display scale factor. `alignment` dictates how the text is aligned, and `bounds` sets the maximum width and height.
    pub fn layout_text(
        &self,
        text: &[TextSegment],
        base_font: Option<&str>,
        base_size: f32,
        scale_factor: f32,
        alignment: HorizontalPosition,
        bounds: (f32, f32),
    ) -> Vec<SectionGlyph> {
        // TODO: Should accept an AABB and a start pos within it.
        let scaled_size = base_size * scale_factor * SIZE_SCALE;
        let base_font = self.font_or_default(base_font);

        let section_text: Vec<_> = text
            .iter()
            .map(|TextSegment { text, size, font }| SectionText {
                text,
                scale: size
                    .map_or(scaled_size, |s| s * scale_factor * SIZE_SCALE)
                    .into(),
                font_id: font
                    .as_ref()
                    .and_then(|f| self.font(f))
                    .unwrap_or(base_font),
            })
            .collect();

        let screen_position = (
            match alignment {
                HorizontalPosition::Left => 0.0,
                HorizontalPosition::Center => bounds.0 / 2.0,
                HorizontalPosition::Right => bounds.0,
            },
            0.0,
        );

        glyph_brush_layout::Layout::default()
            .h_align(match alignment {
                HorizontalPosition::Left => HorizontalAlign::Left,
                HorizontalPosition::Right => HorizontalAlign::Right,
                HorizontalPosition::Center => HorizontalAlign::Center,
            })
            .calculate_glyphs(
                &self.fonts,
                &SectionGeometry {
                    screen_position,
                    bounds,
                },
                &section_text,
            )
    }

    /// Given a slice of [`SectionGlyph`]s (which would have been returned by [`#layout_text`][FontCache#layout_text]), and a known **fixed** `font` and `font_size`, return the width of each glyph. This is useful if you need to e.g. render a cursor between characters as in [`TextBox`][crate::widgets::TextBox].
    pub fn glyph_widths(
        &self,
        font: Option<&str>,
        font_size: f32,
        scale_factor: f32,
        glyphs: &[SectionGlyph],
    ) -> Vec<f32> {
        let font_ref = self.font_or_default(font);
        let font = &self.fonts[font_ref.0];

        glyphs
            .iter()
            .map(|g| {
                font.as_scaled(font_size * scale_factor * SIZE_SCALE)
                    .h_advance(g.glyph.id)
            })
            .collect()
    }
}

/// Used by [`FontCache#layout_text`][FontCache#layout_text] as an input. Accordingly, it is also commonly used as the input to Components that display text, e.g. [`widgets::Text`][crate::widgets::Text] and [`widgets::Button`][crate::widgets::Button].
///
/// [`txt`][crate::txt] is provided as a convenient constructor, but you can also use `into` from a `&str` or `String`, e.g. `"some text".into()`.
#[derive(Debug, Clone)]
pub struct TextSegment {
    /// The text to be laid out.
    pub text: String,
    /// An optional size. A default will be selected if `None`.
    pub size: Option<f32>,
    /// An optional font name. A default will be selected if `None`.
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

/// Convenience constructor for a `Vec` of [`TextSegment`]s.
///
/// `txt` accepts a variable number of arguments. Each argument can come in one of four forms:
/// - `"text"`: A value that is `Into<String>`.
/// - `("text", "font_name")`: A text string, and a font name, both must be `Into<String>`.
/// - `("text", "font_name", 12.0)`: A text string, a font name, and an `f32` font size.
/// - `("text", None, 12.0)`: A text string and a font size.
///
/// If no font name or size is given, defaults are assumed.
///
/// This lets you mix different text styles, e.g.: `txt!("Hello", ("world", "Helvetica Bold", 22.0), "!")`
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
