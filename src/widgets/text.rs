extern crate alloc;

use alloc::{boxed::Box, string::ToString, vec, vec::Vec};
use core::hash::Hash;

use crate::TextSegment;
use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::renderable::{Caches, Renderable};
use crate::style::{HorizontalPosition, Styled};
use lemna_macros::{component, state_component_impl};

#[derive(Debug, Default)]
struct BoundsCache {
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

// Text can't be `NoView`, because updates to text can trigger a re-layout, which requires a view.
#[component(State = "TextState", Styled, Internal)]
#[derive(Debug)]
pub struct Text {
    pub text: Vec<TextSegment>,
}

impl Text {
    pub fn new(text: Vec<TextSegment>) -> Self {
        Self {
            text,
            class: Default::default(),
            style_overrides: Default::default(),
            state: Some(TextState::default()),
            dirty: crate::Dirty::No,
        }
    }
}

#[state_component_impl(TextState, Internal)]
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
        (self.style_val("h_alignment").unwrap().horizontal_position()).hash(hasher);
    }

    fn fill_bounds(
        &mut self,
        width: Option<f32>,
        height: Option<f32>,
        max_width: Option<f32>,
        max_height: Option<f32>,
        caches: &Caches,
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
        let font = self.style_val("font").map(|p| p.str().to_string());
        let line_height = caches.line_height(font.as_deref(), size, scale);

        let glyphs = caches.layout_text(
            &self.text,
            font.as_deref(),
            size,
            scale,
            HorizontalPosition::Left,
            (
                width.or(max_width).map_or(f32::MAX, |w| w * scale),
                height.or(max_height).map_or(f32::MAX, |h| h * scale),
            ),
        );
        let output = if let Some(last_glyph) = glyphs.last() {
            // Unless there is only one row, use the max width
            let w = if last_glyph.y <= line_height || max_width.is_none() {
                // Only one row

                // Add the advance width to the x position of the last glyph. This ensures that the last glyph will not be wrapped
                let metrics = caches.glyph_metrics(last_glyph);
                last_glyph.x.ceil() + metrics.advance_width.ceil() - metrics.bounds.xmin
            } else {
                max_width.unwrap() * scale
            };
            // Force h to the next multiple of size, in order to account for some lines not otherwise having the same height as others, unless we have final glyphs exceeding the expected line height (which will happen if using text segments with different sizes)
            let h = if last_glyph.y % line_height > 0.001 {
                // Iterate backwards to find the maximum glyph.y + glyph.height
                let mut max_height = last_glyph.y + last_glyph.height as f32;
                for glyph in glyphs.iter().rev().skip(1) {
                    let glyph_bottom = glyph.y + glyph.height as f32;
                    if max_height - glyph_bottom > line_height {
                        break;
                    }
                    if glyph_bottom > max_height {
                        max_height = glyph_bottom;
                    }
                }

                (last_glyph.y + (line_height - last_glyph.y % line_height)).max(max_height)
            } else {
                line_height
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
        use crate::renderable::Text;

        let h_alignment: HorizontalPosition =
            self.style_val("h_alignment").unwrap().horizontal_position();
        let font = self.style_val("font").map(|p| p.str().to_string());
        let color: Color = self.style_val("color").into();
        let bounds = context.aabb.size();
        let size: f32 = self.style_val("size").unwrap().f32();

        let glyphs = context.caches.font.layout_text(
            &self.text,
            font.as_deref(),
            size,
            context.scale_factor,
            h_alignment,
            (bounds.width.ceil(), bounds.height.ceil()),
        );

        if glyphs.is_empty() {
            Some(vec![])
        } else {
            Some(vec![Renderable::Text(Text::new(
                glyphs,
                Pos::default(),
                color,
                context.caches,
                context
                    .prev_state
                    .as_ref()
                    .and_then(|r| r.first())
                    .and_then(|r| r.as_text()),
            ))])
        }
    }
}
