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
    New(([u8; 4], PixelSize)),
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

    pub fn init_with_color<C: Into<[u8; 4]>>(mut self, color: C, size: PixelSize) -> Self {
        self.state_mut()
            .updates
            .push(CanvasUpdate::New((color.into(), size)));
        self.state_mut().size = size;
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

    pub fn update<C: Into<[u8; 4]>>(&mut self, point: PixelPoint, color: C) {
        self.state_mut()
            .updates
            .push(CanvasUpdate::Update((point, color.into())));
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
            CanvasUpdate::New((color, size)) => {
                let len = (size.width * size.height * 4) as usize;
                let mut data = vec![0; len];
                for i in (0..(len)).step_by(4) {
                    data[i] = color[0];
                    data[i + 1] = color[1];
                    data[i + 2] = color[2];
                    data[i + 3] = color[3];
                }
                raster = Some(Raster::new(
                    data.into(),
                    size,
                    &mut context.caches.image_buffer_cache.write().unwrap(),
                    &mut context.caches.raster_cache.write().unwrap(),
                    raster.as_ref().map(|r| r.buffer_id),
                    raster.as_ref().map(|r| r.raster_cache_id),
                ));
            }
            CanvasUpdate::Update((point, pixel)) => {
                if let Some(r) = raster.as_mut() {
                    match &mut context.caches.raster_cache.write().unwrap().get_mut_raster_data(r.raster_cache_id).data {
                        RasterData::Vec(ref mut v) => {
                            let i = (point.x * point.y * 4) as usize;
                            v[i] = pixel[0];
                            v[i+1] = pixel[1];
                            v[i+2] = pixel[2];
                            v[i+3] = pixel[3];
                        }
                        _ => panic!("Cannot update a canvas that was not created with `init_with_color` or a Vec")
                    }
                } else {
                    panic!("Cannot update a canvas that has not been initialized");
                }
            }
        });

        raster.map(|r| vec![Renderable::Raster(r)])
    }
}
