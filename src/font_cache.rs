//! The [`FontCache`] is where fonts are stored, and where text layout happens.
//!
//! Adding fonts to the `FontCache` is done via [`UI#add_font`][crate::UI#method.add_font], and window backends may include hooks to add fonts on application load.
//!
//! The `FontCache` is exposed to users so that you can lay out text (i.e. when you're not using a Component that lays out text for you, like [`widgets::Text`][crate::widgets::Text]) via the [`Caches`][crate::renderable::Caches] referenced by the [`RenderContext`][crate::RenderContext] which gets passed to [`Component#render`][crate::Component#method.render].
//!
//! The text-layout interface uses a slice of [`TextSegment`]s as a Component-agnostic way of representing text. A `TextSegment` stores a text string, and optionally a font size and font name (defaults will be used otherwise). In this way, we can lay out text in a variety of types and sizes. [`txt`][crate::txt] is provided as a convenient way of creating `TextSegment`s.

extern crate alloc;

use alloc::{
    borrow::Cow,
    string::{String, ToString},
    vec::Vec,
};
use core::hash::{Hash, Hasher};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use fontdue::{
    Font, FontResult, FontSettings, Metrics,
    layout::{HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign},
};
use hashbrown::HashMap;

use crate::style::HorizontalPosition;

type RwLock<T> = embassy_sync::rwlock::RwLock<CriticalSectionRawMutex, T>;

/// Output by [`Caches::layout_text`][crate::renderable::Caches::layout_text], and an input to [`Text::new`][crate::renderable::Text::new]. Useful for text-rendering widgets to cache in their state, so that they don't need to be recomputed unless necessary.
pub type PositionedGlyph = fontdue::layout::GlyphPosition;

// The index of the font in the `fonts` vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FontId(usize);

const DEFAULT_LINE_HEIGHT_SCALE: f32 = 1.5;

/// Stores fonts, and provides text layout functionality to Components who render.
pub(crate) struct FontCache {
    pub fonts: Vec<Font>,
    pub font_names: HashMap<String, usize>,
    layout: RwLock<Layout>,
}

impl Default for FontCache {
    fn default() -> Self {
        Self {
            fonts: Vec::new(),
            font_names: HashMap::default(),
            layout: RwLock::new(Layout::new(
                fontdue::layout::CoordinateSystem::PositiveYDown,
            )),
        }
    }
}

impl FontCache {
    fn font(&self, name: &str) -> Option<FontId> {
        self.font_names.get(name).map(|i| FontId(*i))
    }

    fn font_or_default(&self, name: Option<&str>) -> FontId {
        if let Some(name) = name
            && let Some(i) = self.font_names.get(name)
        {
            return FontId(*i);
        }

        self.default_font()
    }

    fn default_font(&self) -> FontId {
        if !self.fonts.is_empty() {
            FontId(0)
        } else {
            panic!("Expected at least one default font to be present")
        }
    }

    /// bytes is an OpenType font
    pub fn add_font(&mut self, name: String, bytes: &'static [u8]) -> FontResult<()> {
        let i = self.fonts.len();
        let font = Font::from_bytes(bytes, FontSettings::default())?;
        self.fonts.push(font);
        self.font_names.insert(name, i);
        Ok(())
    }

    /// Given a set of [`TextSegment`]s, create [`PositionedGlyph`]s, which are then used by the [`Text`][crate::renderable::Text] renderable.
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
    ) -> Vec<PositionedGlyph> {
        let mut layout = embassy_futures::block_on(self.layout.write());

        let settings = LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width: Some(bounds.0),
            max_height: Some(bounds.1),
            horizontal_align: match alignment {
                HorizontalPosition::Left => HorizontalAlign::Left,
                HorizontalPosition::Right => HorizontalAlign::Right,
                HorizontalPosition::Center => HorizontalAlign::Center,
            },
            vertical_align: VerticalAlign::Top,
            line_height: 1.0,
            wrap_style: fontdue::layout::WrapStyle::Word,
            wrap_hard_breaks: true,
        };
        layout.reset(&settings);
        // TODO: Should accept an AABB and a start pos within it.
        let scaled_size = base_size * scale_factor;
        let base_font = self.font_or_default(base_font);

        for TextSegment { text, size, font } in text {
            layout.append(
                &self.fonts,
                &TextStyle {
                    text,
                    px: size.map_or(scaled_size, |s| s * scale_factor),
                    font_index: font
                        .as_ref()
                        .and_then(|f| self.font(f))
                        .unwrap_or(base_font)
                        .0,
                    user_data: (),
                },
            )
        }

        let mut glyphs = layout.glyphs().to_vec();
        if let Some(last_glyph) = glyphs.last_mut() {
            // Hack to ensure that the last glyph has a width, otherwise the text will be too narrow to be laid out correctly
            if last_glyph.width == 0 {
                let metrics = self.glyph_metrics(last_glyph);
                last_glyph.width = metrics.advance_width.ceil() as usize;
            }
        }
        glyphs
    }

    pub fn glyph_metrics(&self, glyph: &PositionedGlyph) -> Metrics {
        let font = &self.fonts[glyph.font_index];
        font.metrics_indexed(glyph.key.glyph_index, glyph.key.px)
    }

    pub fn line_height(&self, font: Option<&str>, size: f32, scale_factor: f32) -> f32 {
        let font = &self.fonts[self.font_or_default(font).0];
        if let Some(metrics) = font.horizontal_line_metrics(size * scale_factor) {
            metrics.new_line_size
        } else {
            size * scale_factor * DEFAULT_LINE_HEIGHT_SCALE
        }
    }

    /// Given a slice of [`SectionGlyph`]s (which would have been returned by [`#layout_text`][FontCache#method.layout_text]), and a known **fixed** `font` and `font_size`, return the width of each glyph. This is useful if you need to e.g. render a cursor between characters as in [`TextBox`][crate::widgets::TextBox].
    pub fn glyph_widths(&self, glyphs: &[PositionedGlyph]) -> Vec<f32> {
        glyphs.iter().map(|g| g.width as f32).collect()
    }
}

/// Used by [`Caches::layout_text`][crate::renderable::Caches::layout_text] as an input. Accordingly, it is also commonly used as the input to Components that display text, e.g. [`widgets::Text`][crate::widgets::Text] and [`widgets::Button`][crate::widgets::Button].
///
/// [`txt`][crate::txt] is provided as a convenient constructor, but you can also use `into` from a `&str` or `String`, e.g. `"some text".into()`.
#[derive(Debug, Clone)]
pub struct TextSegment {
    /// The text to be laid out.
    pub text: Cow<'static, str>,
    /// An optional size. A default will be selected if `None`.
    pub size: Option<f32>,
    /// An optional font name. A default will be selected if `None`.
    pub font: Option<String>,
}

impl TextSegment {
    /// Create a `TextSegment` from a static string without allocation.
    pub fn from_static(s: &'static str) -> Self {
        TextSegment {
            text: Cow::Borrowed(s),
            size: None,
            font: None,
        }
    }
}

impl From<&str> for TextSegment {
    fn from(s: &str) -> TextSegment {
        TextSegment {
            text: Cow::Owned(s.to_string()),
            size: None,
            font: None,
        }
    }
}

impl From<String> for TextSegment {
    fn from(text: String) -> TextSegment {
        TextSegment {
            text: Cow::Owned(text),
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
/// This lets you mix different text styles, e.g.:
/// ```
/// # use lemna::*;
/// let text = txt!("Hello", ("world", "Helvetica Bold", 22.0), "!");
/// ```
#[macro_export]
macro_rules! txt {
    // split_comma taken from: https://gist.github.com/kyleheadley/c2f64e24c14e45b1e39ee664059bd86f

    // give initial params to the function
    {@split_comma  ($($first:tt)*) <= $($item:tt)*} => {
        txt![@split_comma ($($first)*) () () <= $($item)*]

    };
    // give initial params and initial inner items in every group
    {@split_comma  ($($first:tt)*) ($($every:tt)*) <= $($item:tt)*} => {
        txt![@split_comma ($($first)*) ($($every)*) ($($every)*) <= $($item)*]

    };
    // KEYWORD line
    // on non-final separator, stash the accumulator and restart it
    {@split_comma  ($($first:tt)*) ($($every:tt)*) ($($current:tt)*) <= , $($item:tt)+} => {
        txt![@split_comma ($($first)* ($($current)*)) ($($every)*) ($($every)*) <= $($item)*]

    };
    // KEYWORD line
    // ignore final separator, run the function
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
    // Special case for string literals to avoid allocation
    (@as_txt_seg  ($text:literal, None, $size:expr)) => { $crate::TextSegment {
        text: $crate::TextSegment::from_static($text).text,
        size: Some($size),
        font: None,
    } };

    (@as_txt_seg  ($text:literal, $font:expr, $size:expr)) => { $crate::TextSegment {
        text: $crate::TextSegment::from_static($text).text,
        size: Some($size),
        font: Some($font.into()),
    } };

    (@as_txt_seg  ($text:literal, $font:expr)) => { $crate::TextSegment {
        text: $crate::TextSegment::from_static($text).text,
        size: None,
        font: Some($font.into()),
    } };

    (@as_txt_seg  ($text:expr, None, $size:expr)) => { {
        let mut ts: $crate::TextSegment = $text.into();
        ts.size = Some($size);
        ts
    } };

    (@as_txt_seg  ($text:expr, $font:expr, $size:expr)) => { {
        let mut ts: $crate::TextSegment = $text.into();
        ts.size = Some($size);
        ts.font = Some($font.into());
        ts
    } };

    (@as_txt_seg  ($text:expr, $font:expr)) => { {
        let mut ts: $crate::TextSegment = $text.into();
        ts.font = Some($font.into());
        ts
    } };

    // Special case for string literals to avoid allocation
    (@as_txt_seg  $text:literal) => {
        $crate::TextSegment::from_static($text)
    };

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
