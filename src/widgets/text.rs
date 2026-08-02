extern crate alloc;
use alloc::{boxed::Box, string::String, string::ToString, vec, vec::Vec};
use core::hash::Hash;

use crate::TextSegment;
use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::renderable::{Caches, Renderable};
use crate::style::{HorizontalPosition, Styled};
use lemna_macros::{component, state_component_impl};

pub use crate::renderable::TextAngle;

#[derive(Debug, Default)]
struct BoundsCache {
    width: Option<f32>,
    height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    layout_angle: TextAngle,
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
    layout_angle: TextAngle,
}

impl Text {
    pub fn new(text: Vec<TextSegment>) -> Self {
        Self {
            text,
            layout_angle: Default::default(),
            class: Default::default(),
            style_overrides: Default::default(),
            state: Some(TextState::default()),
            dirty: crate::Dirty::No,
        }
    }

    pub fn font(&self) -> Option<String> {
        if self.text.len() == 1 && self.text[0].font.is_some() {
            self.text[0].font.clone()
        } else {
            self.style_val("font").map(|p| p.str().to_string())
        }
    }

    pub fn angle(mut self, layout_angle: TextAngle) -> Self {
        self.layout_angle = layout_angle;
        self
    }
}

#[state_component_impl(TextState, Internal)]
impl Component for Text {
    fn new_props(&mut self) {
        self.state = Some(TextState::default());
    }

    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.text.hash(hasher);
        self.layout_angle.hash(hasher);
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.text.hash(hasher);
        self.layout_angle.hash(hasher);
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
            && c.layout_angle == self.layout_angle
        {
            return c.output.unwrap();
        }

        let size: f32 = self.style_val("size").unwrap().f32();
        let font = self.font();
        let line_height = caches.line_height(font.as_deref(), size, scale);

        // Constraints in text-local space (swap axes for 90° rotations).
        let (local_max_w, local_max_h, wrapping_max) = if self.layout_angle.swaps_axes() {
            (
                height.or(max_height).map_or(f32::MAX, |h| h * scale),
                width.or(max_width).map_or(f32::MAX, |w| w * scale),
                max_height,
            )
        } else {
            (
                width.or(max_width).map_or(f32::MAX, |w| w * scale),
                height.or(max_height).map_or(f32::MAX, |h| h * scale),
                max_width,
            )
        };

        let (glyphs, text_height) = caches.layout_text(
            &self.text,
            font.as_deref(),
            size,
            scale,
            HorizontalPosition::Left,
            (local_max_w, local_max_h),
        );
        let output = if let Some(last_glyph) = glyphs.last() {
            // Unless there is only one row, use the max width
            let local_w = if last_glyph.y <= line_height || wrapping_max.is_none() {
                // Only one row

                // Add the advance width to the x position of the last glyph. This ensures that the last glyph will not be wrapped
                let metrics = caches.glyph_metrics(last_glyph);
                last_glyph.x.ceil() + metrics.advance_width.ceil() - metrics.bounds.xmin
            } else {
                wrapping_max.unwrap() * scale
            };

            let (mapped_w, mapped_h) = self
                .layout_angle
                .map_size(local_w / scale, text_height / scale);
            (
                Some(width.unwrap_or(mapped_w)),
                Some(height.unwrap_or(mapped_h)),
            )
        } else {
            (None, None)
        };
        self.state_mut().bounds_cache = BoundsCache {
            width,
            height,
            max_width,
            max_height,
            layout_angle: self.layout_angle,
            output: Some(output),
        };
        output
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use crate::renderable::Text;

        let h_alignment: HorizontalPosition =
            self.style_val("h_alignment").unwrap().horizontal_position();
        let font = self.font();
        let color: Color = self.style_val("color").into();
        let aabb_size = context.aabb.size();
        let size: f32 = self.style_val("size").unwrap().f32();

        let bounds = if self.layout_angle.swaps_axes() {
            Scale::new(aabb_size.height.ceil(), aabb_size.width.ceil())
        } else {
            Scale::new(aabb_size.width.ceil(), aabb_size.height.ceil())
        };

        let (glyphs, _) = context.caches.font.layout_text(
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
            Some(vec![Renderable::Text(
                Text::new(
                    glyphs,
                    Pos::default(),
                    color,
                    context.caches,
                    nth_prev_as_text!(context, 0),
                )
                .angle(self.layout_angle, bounds),
            )])
        }
    }
}
