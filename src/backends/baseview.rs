use crate::base_types::*;
use crate::component::App;
use crate::render::Renderer;
use crate::UI;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

struct BaseViewUI<R: Renderer, A: 'static + App<R>>
where
    <R as Renderer>::Renderable: std::fmt::Debug,
{
    ui: UI<Window, R, A>,
}

pub struct Window {
    handle: RawWindowHandle,
    size: (u32, u32),
    scale_factor: f32,
}

impl Window {
    fn new(handle: RawWindowHandle) -> Self {
        Self {
            handle,
            scale_factor: 1.0, //TODO
            size: (200, 200),  //TODO
        }
    }

    pub fn open_parented<P, R, A>(
        parent: &P,
        settings: baseview::WindowOpenOptions,
    ) -> baseview::WindowHandle
    where
        P: HasRawWindowHandle,
        R: Renderer + 'static,
        <R as Renderer>::Renderable: std::fmt::Debug,
        A: 'static + App<R>,
    {
        println!("AAAAAAAAAAAAAAAAAAAAAAHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH");
        baseview::Window::open_parented(
            parent,
            settings,
            move |window: &mut baseview::Window<'_>| -> BaseViewUI<R, A> {
                println!("AAAAAAAAAAAAAAAAAAAAAAHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHBBBBBBBBBBBBBBBAAAAAAAAAAAAAA");
                BaseViewUI {
                    ui: UI::new(Self::new(window.raw_window_handle())),
                }
            },
        )
    }
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.handle
    }
}

impl<R: Renderer, A: 'static + App<R>> baseview::WindowHandler for BaseViewUI<R, A>
where
    <R as Renderer>::Renderable: std::fmt::Debug,
{
    fn on_frame(&mut self, _window: &mut baseview::Window) {
        println!("HAVE A FRAME");
        if self.ui.draw() {
            println!("DO A DRAW");
            self.ui.render()
        }
    }
    fn on_event(
        &mut self,
        _window: &mut baseview::Window,
        _event: baseview::Event,
    ) -> baseview::EventStatus {
        baseview::EventStatus::Ignored
    }
}

impl crate::window::Window for Window {
    fn client_size(&self) -> PixelSize {
        PixelSize {
            width: self.size.0,
            height: self.size.1,
        }
    }

    fn display_size(&self) -> PixelSize {
        PixelSize {
            width: self.size.0,
            height: self.size.1,
        }
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}
