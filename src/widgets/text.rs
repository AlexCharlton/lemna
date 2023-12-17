use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::font_cache::{FontCache, TextSegment};
use crate::render::{renderables::text, Renderable};
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
            dirty: false,
        }
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
        (self.style_val("h_alignment").unwrap().horizontal_position()).hash(hasher);
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
        let font = self.style_val("font").map(|p| p.str().to_string());
        let scaled_size = size * scale * crate::font_cache::SIZE_SCALE;

        let glyphs = font_cache.layout_text(
            &self.text,
            font.as_deref(),
            size,
            scale,
            HorizontalPosition::Left,
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
            // Force h to the next multiple of size, in order to account for some lines not otherwise having the same height as others
            let h = if p.y % scaled_size > 0.001 {
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
        let h_alignment: HorizontalPosition =
            self.style_val("h_alignment").unwrap().horizontal_position();
        let font = self.style_val("font").map(|p| p.str().to_string());
        let color: Color = self.style_val("color").into();
        let bounds = context.aabb.size();
        let size: f32 = self.style_val("size").unwrap().f32();

        let glyphs = context.caches.font.read().unwrap().layout_text(
            &self.text,
            font.as_deref(),
            size,
            context.scale_factor,
            h_alignment,
            (bounds.width, bounds.height),
        );

        if glyphs.is_empty() {
            Some(vec![])
        } else {
            Some(vec![Renderable::Text(text::Text::new(
                glyphs,
                Pos::default(),
                color,
                &mut context.caches.text_buffer.write().unwrap(),
                context.prev_state.and_then(|v| match v.get(0) {
                    Some(Renderable::Text(r)) => Some(r.buffer_id),
                    _ => None,
                }),
            ))])
        }
    }
}
