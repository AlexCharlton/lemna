use std::hash::{Hash, Hasher};

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::font_cache::{FontCache, HorizontalAlign, SectionText};
use crate::render::{renderables::text, Renderable};
use crate::style::Styled;
use lemna_macros::{component, state_component_impl};

#[derive(Debug, Default)]
pub struct BoundsCache {
    width: Option<f32>,
    height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    output: Option<(Option<f32>, Option<f32>)>,
}

#[derive(Debug, Default)]
pub struct TextState {
    bounds_cache: BoundsCache,
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
    (@as_txt_seg  ($text:expr, None, $size:expr)) => { $crate::widgets::TextSegment {
        text: $text.into(),
        size: Some($size),
        font: None,
    } };

    (@as_txt_seg  ($text:expr, $font:expr, $size:expr)) => { $crate::widgets::TextSegment {
        text: $text.into(),
        size: Some($size),
        font: Some($font.into()),
    } };

    (@as_txt_seg  ($text:expr, $font:expr)) => { $crate::widgets::TextSegment {
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

#[component(State = "TextState", Styled, Internal)]
#[derive(Debug)]
pub struct Text {
    pub text: Vec<TextSegment>,
}

impl Text {
    pub const SIZE_SCALE: f32 = 1.5; // 12 px fonts render at scale 18 px for some reason

    pub fn new(text: Vec<TextSegment>) -> Self {
        Self {
            text,
            class: Default::default(),
            style_overrides: Default::default(),
            state: Some(TextState::default()),
            dirty: false,
        }
    }

    fn to_section_text(&self, font_cache: &FontCache, scale: f32) -> Vec<SectionText> {
        let font = self.style_val("font").map(|p| p.str().to_string());
        let size: f32 = self.style_val("size").unwrap().f32();
        let scaled_size = size * scale * Text::SIZE_SCALE;
        let base_font = font_cache.font_or_default(font.as_deref());

        self.text
            .iter()
            .map(|TextSegment { text, size, font }| SectionText {
                text,
                scale: size
                    .map_or(scaled_size, |s| s * scale * Text::SIZE_SCALE)
                    .into(),
                font_id: font
                    .as_ref()
                    .and_then(|f| font_cache.font(f))
                    .unwrap_or(base_font),
            })
            .collect()
    }
}

#[state_component_impl(TextState)]
impl Component for Text {
    fn new_props(&mut self) {
        self.state = Some(TextState::default());
    }

    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.text.hash(hasher);
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.text.hash(hasher);
        (self.style_val("size").unwrap().f32() as u32).hash(hasher);
        (self.style_val("color").unwrap().color()).hash(hasher);
        (self.style_val("font").map(|p| p.str().to_string())).hash(hasher);
        (self.style_val("h_alignment").unwrap().horizontal_align()).hash(hasher);
    }

    fn fill_bounds(
        &mut self,
        width: Option<f32>,
        height: Option<f32>,
        max_width: Option<f32>,
        max_height: Option<f32>,
        font_cache: &FontCache,
        scale: f32,
    ) -> (Option<f32>, Option<f32>) {
        let c = &self.state_ref().bounds_cache;
        if c.output.is_some()
            && c.width == width
            && c.height == height
            && c.max_width == max_width
            && c.max_height == max_height
        {
            return c.output.unwrap();
        }

        let size: f32 = self.style_val("size").unwrap().f32();
        let scaled_size = size * scale * Text::SIZE_SCALE;

        let glyphs = font_cache.layout_text(
            &self.to_section_text(font_cache, scale),
            HorizontalAlign::Left,
            (0.0, 0.0),
            (
                width.or(max_width).unwrap_or(std::f32::MAX) * scale,
                height.or(max_height).unwrap_or(std::f32::MAX) * scale,
            ),
        );
        let output = if let Some(last_glyph) = glyphs.last() {
            let p = last_glyph.glyph.position;
            // Unless there is only one row, use the max width
            let w = if p.y <= scaled_size || max_width.is_none() {
                p.x + last_glyph.glyph.scale.x
            } else {
                max_width.unwrap() * scale
            };
            // Force h to the next multiple of size, which can result in some inconsistent results
            let h = if p.y % scaled_size > 0.0 {
                p.y + (scaled_size - p.y % scaled_size)
            } else {
                p.y
            };
            (
                Some(width.unwrap_or(w / scale)),
                Some(height.unwrap_or(h / scale)),
            )
        } else {
            (None, None)
        };
        self.state_mut().bounds_cache = BoundsCache {
            width,
            height,
            max_width,
            max_height,
            output: Some(output),
        };
        output
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let h_alignment: HorizontalAlign =
            self.style_val("h_alignment").unwrap().horizontal_align();
        let color: Color = self.style_val("color").into();
        let bounds = context.aabb.size();
        let glyphs = context.font_cache.read().unwrap().layout_text(
            &self.to_section_text(&context.font_cache.read().unwrap(), context.scale_factor),
            h_alignment,
            (
                match h_alignment {
                    HorizontalAlign::Left => 0.0,
                    HorizontalAlign::Center => bounds.width / 2.0,
                    HorizontalAlign::Right => bounds.width,
                },
                0.0,
            ),
            (bounds.width, bounds.height),
        );

        if glyphs.is_empty() {
            Some(vec![])
        } else {
            Some(vec![Renderable::Text(text::Text::new(
                glyphs,
                Pos::default(),
                color,
                &mut context.buffer_caches.text_cache.write().unwrap(),
                context.prev_state.and_then(|v| match v.get(0) {
                    Some(Renderable::Text(r)) => Some(r.buffer_id),
                    _ => None,
                }),
            ))])
        }
    }
}
