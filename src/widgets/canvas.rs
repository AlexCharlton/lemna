use std::hash::{Hash, Hasher};

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::render::{renderables::raster::Raster, Renderable};
use crate::FontCache;
use lemna_macros::{component, state_component_impl};

#[derive(Debug)]
enum CanvasUpdate {
    Set((Vec<u8>, PixelSize)),
    Update((PixelPoint, Color)),
}

#[derive(Debug, Default)]
struct CanvasState {
    // Push updates when making changes, pop when rendering
    updates: Vec<CanvasUpdate>,
    size: PixelSize,
    update_counter: usize,
}

#[component(State = "CanvasState", Internal)]
#[derive(Debug)]
pub struct Canvas {}

impl Canvas {
    pub fn new() -> Self {
        Self {
            state: Some(Default::default()),
            dirty: false,
        }
    }

    /// You can call this when initializing a canvas and it won't overwrite any changes (TODO: is this true?)
    /// TODO make this a [u8]
    pub fn set(mut self, data: Vec<u8>, size: PixelSize) -> Self {
        self.reset(data, size);
        self
    }

    pub fn reset(&mut self, data: Vec<u8>, size: PixelSize) {
        self.state_mut()
            .updates
            .push(CanvasUpdate::Set((data, size)));
        self.state_mut().size = size;
        self.state_mut().update_counter += 1;
    }

    pub fn update(&mut self, point: PixelPoint, color: Color) {
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
        (Some(size.width as f32), Some(size.height as f32))
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
