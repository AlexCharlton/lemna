use lemna::input::{Button, Input, Motion, MouseButton};
use lemna::{Component, PixelSize, UI, log_error};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window as WinitWindow, WindowId};

pub struct Window {
    winit_window: WinitWindow,
}

impl Window {
    pub fn open_blocking<A>(
        title: &str,
        width: u32,
        height: u32,
        fonts: Vec<(String, &'static [u8])>,
    ) where
        A: 'static + Component + Default + Send + Sync,
    {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = App::<A> {
            title: title.to_string(),
            width,
            height,
            fonts,
            ui: None,
        };
        event_loop
            .run_app(&mut app)
            .expect("Event loop terminated with an error");
    }
}

struct App<A: 'static + Component + Default + Send + Sync> {
    title: String,
    width: u32,
    height: u32,
    fonts: Vec<(String, &'static [u8])>,
    ui: Option<UI<A>>,
}

impl<A: 'static + Component + Default + Send + Sync> ApplicationHandler for App<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.ui.is_some() {
            return;
        }

        let attrs = WinitWindow::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64));
        let window = event_loop
            .create_window(attrs)
            .expect("Failed to create window");

        let mut ui: UI<A> = UI::new(Window {
            winit_window: window,
        });
        for (name, data) in self.fonts.drain(..) {
            if let Err(_e) = ui.add_font(name, data) {
                log_error!("Failed to add font: {}", _e);
            }
        }
        self.ui = Some(ui);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ui) = self.ui.as_mut() {
            ui.draw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(ui) = self.ui.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => ui.render(),
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = lemna::window::scale_factor().unwrap_or(1.0);
                ui.handle_input(&Input::Motion(Motion::Mouse {
                    x: position.x as f32 / scale_factor,
                    y: position.y as f32 / scale_factor,
                }));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                ui.handle_input(&Input::Press(Button::Mouse(MouseButton::Left)));
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            } => {
                ui.handle_input(&Input::Release(Button::Mouse(MouseButton::Left)));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Motion::Scroll {
                        x: x * -10.0,
                        y: y * -10.0,
                    },
                    MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { x, y }) => {
                        Motion::Scroll {
                            x: -x as f32,
                            y: -y as f32,
                        }
                    }
                };
                ui.handle_input(&Input::Motion(scroll));
            }
            _ => (),
        }
    }
}

impl lemna::window::Window for Window {
    fn logical_size(&self) -> PixelSize {
        let size = self.winit_window.inner_size();
        let scale_factor = self.scale_factor();
        PixelSize {
            width: (size.width as f32 / scale_factor) as u32,
            height: (size.height as f32 / scale_factor) as u32,
        }
    }

    fn physical_size(&self) -> PixelSize {
        let size = self.winit_window.inner_size();
        PixelSize {
            width: size.width,
            height: size.height,
        }
    }

    fn scale_factor(&self) -> f32 {
        self.winit_window.scale_factor() as f32
    }

    fn redraw(&self) {
        self.winit_window.request_redraw();
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.winit_window.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.winit_window.display_handle()
    }
}
