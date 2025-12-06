extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};
use core::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::event;
use crate::input::MouseButton;
use crate::renderable::{Caches, RasterData, Renderable};
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
    drawing: bool,
}

/// Supports 8 bit rgba. E.g. `Color Into [u8; 4]`
#[component(State = "CanvasState", Internal, NoView)]
pub struct Canvas {
    scale: f32,
    on_draw: Option<Box<dyn Fn(PixelPoint) -> Vec<(PixelPoint, [u8; 4])> + Send + Sync>>,
}

impl core::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Canvas")
            .field("scale", &self.scale)
            .field("state", &self.state)
            .finish()
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            on_draw: None,
            state: Some(Default::default()),
            dirty: crate::Dirty::No,
        }
    }

    /// You can call this when initializing a canvas and it won't overwrite any changes because after the first instance, the state will be replaced
    /// The size of the canvas is the size in Physical pixels. This means that if you want to set
    /// the canvas to be a particular logical size, you need to multiply the width and height by the window [`scale_factor`][crate::window::scale_factor].
    pub fn set<D: Into<RasterData>>(mut self, data: D, size: PixelSize) -> Self {
        self.reset(data, size);
        self.dirty = crate::Dirty::No;
        self
    }

    pub fn init_with_color<C: Into<[u8; 4]>>(mut self, color: C, size: PixelSize) -> Self {
        self.state_mut()
            .updates
            .push(CanvasUpdate::New((color.into(), size)));
        self.state_mut().size = size;
        self.dirty = crate::Dirty::No;
        self
    }

    /// Scale the size of the rendered canvas.
    #[cfg(feature = "wgpu_renderer")]
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[allow(unused_mut)]
    #[cfg(feature = "cpu_renderer")]
    pub fn scale(mut self, _scale: f32) -> Self {
        unimplemented!("scale is not implemented for the cpu renderer");
    }

    pub fn on_draw(
        mut self,
        f: Box<dyn Fn(PixelPoint) -> Vec<(PixelPoint, [u8; 4])> + Send + Sync>,
    ) -> Self {
        self.on_draw = Some(f);
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

#[state_component_impl(CanvasState, Internal)]
impl Component for Canvas {
    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        if self.state_ref().drawing {
            // TODO should interpolate from last position
            if let Some(f) = &self.on_draw {
                for update in f(event.relative_physical_position().into()).drain(..) {
                    self.state_mut().updates.push(CanvasUpdate::Update(update));
                    self.state_mut().update_counter += 1;
                }
            }
        }
        event.stop_bubbling();
    }

    fn on_mouse_down(&mut self, event: &mut event::Event<event::MouseDown>) {
        if event.input.button == MouseButton::Left {
            self.state_mut().drawing = true;
        }
    }

    fn on_mouse_up(&mut self, event: &mut event::Event<event::MouseUp>) {
        if event.input.button == MouseButton::Left {
            self.state_mut().drawing = false;
        }
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        self.state_mut().drawing = false;
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.state_ref().update_counter.hash(hasher);
    }

    fn fill_bounds(
        &mut self,
        _width: Option<f32>,
        _height: Option<f32>,
        _max_width: Option<f32>,
        _max_height: Option<f32>,
        _caches: &Caches,
        scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        let size = self.state_ref().size;
        (
            Some(size.width as f32 * self.scale / scale_factor),
            Some(size.height as f32 * self.scale / scale_factor),
        )
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use crate::renderable::Raster;

        let mut raster = context
            .prev_state
            .and_then(|mut v| v.pop())
            .and_then(|r| r.into_raster());
        let size = self.state_ref().size;

        self.state_mut().updates.drain(..).for_each(|u| match u {
            CanvasUpdate::Set((data, size)) => {
                raster = Some(Raster::new(
                    data,
                    size,
                    context.caches,
                    raster.as_ref(),
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
                    context.caches,
                    raster.as_ref(),
                ));
            }
            CanvasUpdate::Update((point, pixel)) => {
                if let Some(r) = raster.as_ref() {
                    match r.get_mut_raster_data(context.caches) {
                        RasterData::Vec(v) => {
                            let i = ((point.x + (point.y * size.width)) * 4) as usize;
                            if i < v.len() {
                                v[i] = pixel[0];
                                v[i+1] = pixel[1];
                                v[i+2] = pixel[2];
                                v[i+3] = pixel[3];
                            }
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
