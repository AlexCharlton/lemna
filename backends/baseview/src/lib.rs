#![feature(trait_upcasting)]

use std::any::Any;
use std::cell::RefMut;

use lemna::component::App;
use lemna::render::Renderer;
use lemna::{PixelSize, UI};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

pub extern crate baseview;

struct BaseViewUI<R: Renderer, A: 'static + App<R>>
where
    <R as Renderer>::Renderable: std::fmt::Debug,
{
    ui: UI<Window, R, A>,
}

pub struct Window {
    handle: RawWindowHandle,
    display_handle: RawDisplayHandle,
    size: (u32, u32),
    scale_policy: baseview::WindowScalePolicy,
    scale_factor: f32,
}

impl Window {
    pub fn open_parented<P, R, A>(
        parent: &P,
        title: String,
        width: u32,
        height: u32,
        scale_policy: baseview::WindowScalePolicy,
        mut fonts: Vec<(String, &'static [u8])>,
    ) -> baseview::WindowHandle
    where
        P: HasRawWindowHandle,
        R: Renderer + 'static,
        <R as Renderer>::Renderable: std::fmt::Debug,
        A: 'static + App<R>,
    {
        baseview::Window::open_parented(
            parent,
            baseview::WindowOpenOptions {
                title,
                size: baseview::Size::new(width.into(), height.into()),
                scale: scale_policy,
            },
            move |window: &mut baseview::Window<'_>| -> BaseViewUI<R, A> {
                let scale_factor = match scale_policy {
                    baseview::WindowScalePolicy::ScaleFactor(scale) => scale,
                    baseview::WindowScalePolicy::SystemScaleFactor => 1.0, // Assume for now until scale event
                } as f32;
                let mut ui = UI::new(Self {
                    handle: window.raw_window_handle(),
                    display_handle: window.raw_display_handle(),
                    size: (width, height),
                    scale_factor,
                    scale_policy,
                });
                for (name, data) in fonts.drain(..) {
                    ui.add_font(name, data);
                }
                // If we set the window to the wrong size, we'll get a resize event, which will let us get the scale factor
                #[cfg(windows)]
                {
                    window.resize(baseview::Size::new(1.0, 1.0));
                }
                BaseViewUI { ui }
            },
        )
    }

    pub fn open_blocking<R, A>(
        title: String,
        width: u32,
        height: u32,
        scale_policy: baseview::WindowScalePolicy,
        mut fonts: Vec<(String, &'static [u8])>,
    ) where
        R: Renderer + 'static,
        <R as Renderer>::Renderable: std::fmt::Debug,
        A: 'static + App<R>,
    {
        baseview::Window::open_blocking(
            baseview::WindowOpenOptions {
                title,
                size: baseview::Size::new(width.into(), height.into()),
                scale: scale_policy,
            },
            move |window: &mut baseview::Window<'_>| -> BaseViewUI<R, A> {
                let scale_factor = match scale_policy {
                    baseview::WindowScalePolicy::ScaleFactor(scale) => scale,
                    baseview::WindowScalePolicy::SystemScaleFactor => 1.0, // Assume for now until scale event
                } as f32;
                let mut ui = UI::new(Self {
                    handle: window.raw_window_handle(),
                    display_handle: window.raw_display_handle(),
                    size: (width, height),
                    scale_factor,
                    scale_policy,
                });
                for (name, data) in fonts.drain(..) {
                    ui.add_font(name, data);
                }
                // If we set the window to the wrong size, we'll get a resize event, which will let us get the scale factor
                #[cfg(windows)]
                {
                    window.resize(baseview::Size::new(1.0, 1.0));
                }
                BaseViewUI { ui }
            },
        )
    }
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.handle
    }
}

unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.display_handle
    }
}

use lemna::input::{Button, Input, Key, Motion, MouseButton};
impl<R: Renderer, A: 'static + App<R>> baseview::WindowHandler for BaseViewUI<R, A>
where
    <R as Renderer>::Renderable: std::fmt::Debug,
{
    fn on_frame(&mut self, _window: &mut baseview::Window) {
        if self.ui.draw() {
            println!("DO A DRAW");
            self.ui.render()
        }
    }

    fn on_event(
        &mut self,
        _window: &mut baseview::Window,
        event: baseview::Event,
    ) -> baseview::EventStatus {
        dbg!("Got event {:?}", &event);
        let mut handle_input = |x| self.ui.handle_input(x);
        match &event {
            baseview::Event::Window(event) => match event {
                baseview::WindowEvent::Resized(window_info) => {
                    if let Some(win) = lemna::current_window() {
                        RefMut::map(win, |win| {
                            if let Some(win) = (win as &mut dyn Any).downcast_mut::<Window>() {
                                win.scale_factor = match win.scale_policy {
                                    baseview::WindowScalePolicy::ScaleFactor(scale) => scale,
                                    baseview::WindowScalePolicy::SystemScaleFactor => {
                                        window_info.scale()
                                    }
                                } as f32;
                                win.size = (
                                    window_info.logical_size().width as u32,
                                    window_info.logical_size().height as u32,
                                );
                            }
                            win
                        });
                    }

                    handle_input(&Input::Resize);
                }
                baseview::WindowEvent::WillClose => (),
                baseview::WindowEvent::Focused => handle_input(&Input::Focus(true)),
                baseview::WindowEvent::Unfocused => handle_input(&Input::Focus(false)),
            },
            baseview::Event::Mouse(event) => match event {
                baseview::MouseEvent::CursorMoved {
                    position,
                    modifiers,
                } => {
                    // TODO
                }
                baseview::MouseEvent::ButtonPressed { button, modifiers } => {
                    // TODO
                }
                baseview::MouseEvent::ButtonReleased { button, modifiers } => {
                    // TODO
                }
                baseview::MouseEvent::WheelScrolled { delta, modifiers } => {
                    // TODO
                }
                baseview::MouseEvent::CursorEntered => handle_input(&Input::MouseEnterWindow),
                baseview::MouseEvent::CursorLeft => handle_input(&Input::MouseLeaveWindow),
            },
            baseview::Event::Keyboard(event) => {
                // TODO
            }
        }
        baseview::EventStatus::Captured
    }
}

impl lemna::window::Window for Window {
    fn client_size(&self) -> PixelSize {
        PixelSize {
            width: self.size.0,
            height: self.size.1,
        }
    }

    fn physical_size(&self) -> PixelSize {
        PixelSize {
            width: ((self.size.0 as f32) * self.scale_factor) as u32,
            height: ((self.size.1 as f32) * self.scale_factor) as u32,
        }
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}
