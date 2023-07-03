use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::render::{
    renderables::{raster::Raster, raster_cache::RasterData},
    Renderable,
};
use crate::FontCache;
use lemna_macros::{component, state_component_impl};

#[derive(Debug)]
enum CanvasUpdate {
    Set((RasterData, PixelSize)),
    Update((PixelPoint, [u8; 4])),
}

#[derive(Debug, Default)]
struct CanvasState {
    // Push updates when making changes, pop when rendering
    updates: Vec<CanvasUpdate>,
    size: PixelSize,
    update_counter: usize,
}

/// Supports 8 bit rgba. E.g. `Color Into u32`
#[component(State = "CanvasState", Internal)]
#[derive(Debug)]
pub struct Canvas {
    scale: f32,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            state: Some(Default::default()),
            dirty: false,
            scale: 1.0,
        }
    }

    /// You can call this when initializing a canvas and it won't overwrite any changes (TODO: is this true?)
    pub fn set<D: Into<RasterData>>(mut self, data: D, size: PixelSize) -> Self {
        self.reset(data, size);
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn reset<D: Into<RasterData>>(&mut self, data: D, size: PixelSize) {
        self.state_mut()
            .updates
            .push(CanvasUpdate::Set((data.into(), size)));
        self.state_mut().size = size;
        self.state_mut().update_counter += 1;
    }

    pub fn update(&mut self, point: PixelPoint, color: Color) {
        self.state_mut()
            .updates
            .push(CanvasUpdate::Update((point, color.into())));
        self.state_mut().update_counter += 1;
    }

    pub fn update_bytes(&mut self, point: PixelPoint, color: [u8; 4]) {
        self.state_mut()
            .updates
            .push(CanvasUpdate::Update((point, color)));
        self.state_mut().update_counter += 1;
    }
}

#[state_component_impl(CanvasState)]
impl Component for Canvas {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.state_ref().update_counter.hash(hasher);
    }

    fn fill_bounds(
        &mut self,
        _width: Option<f32>,
        _height: Option<f32>,
        _max_width: Option<f32>,
        _max_height: Option<f32>,
        _font_cache: &FontCache,
        _scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        let size = self.state_ref().size;
        (
            Some(size.width as f32 * self.scale),
            Some(size.height as f32 * self.scale),
        )
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let mut raster = context.prev_state.and_then(|mut v| match v.pop() {
            Some(Renderable::Raster(r)) => Some(r),
            _ => None,
        });

        self.state_mut().updates.drain(..).for_each(|u| match u {
            CanvasUpdate::Set((data, size)) => {
                raster = Some(Raster::new(
                    data,
                    size,
                    &mut context.caches.image_buffer_cache.write().unwrap(),
                    &mut context.caches.raster_cache.write().unwrap(),
                    raster.as_ref().map(|r| r.buffer_id),
                    raster.as_ref().map(|r| r.raster_cache_id),
                ));
            }
            CanvasUpdate::Update((point, pixel)) => {
                // TODO
            }
        });

        raster.map(|r| vec![Renderable::Raster(r)])
    }
}
