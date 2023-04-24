use std::sync::{Arc, RwLock};

use arboard::{self, Clipboard};
use lemna::component::App;
use lemna::render::Renderer;
use lemna::{Data, PixelSize, UI};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

pub extern crate baseview;

#[derive(Debug)]
pub enum ParentMessage {
    Dirty,
    Resize,
}

struct BaseViewUI<R: Renderer, A: 'static + App<R>> {
    ui: UI<Window, R, A>,
    parent_channel: Option<crossbeam_channel::Receiver<ParentMessage>>,
}

pub struct Window {
    handle: RawWindowHandle,
    display_handle: RawDisplayHandle,
    size: (u32, u32),
    scale_policy: baseview::WindowScalePolicy,
    scale_factor: f32,
    baseview_window: Option<&'static baseview::Window<'static>>,
    drop_target_valid: Arc<RwLock<bool>>,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub fn open_parented<P, R, A, B>(
        parent: &P,
        title: String,
        width: u32,
        height: u32,
        scale_policy: baseview::WindowScalePolicy,
        mut fonts: Vec<(String, &'static [u8])>,
        build: B,
        parent_channel: Option<crossbeam_channel::Receiver<ParentMessage>>,
    ) -> baseview::WindowHandle
    where
        P: HasRawWindowHandle,
        R: Renderer + 'static,
        A: 'static + App<R>,
        B: Fn(&mut UI<Window, R, A>) + 'static + Send,
    {
        let drop_target_valid = Arc::new(RwLock::new(true));
        let drop_target_valid2 = drop_target_valid.clone();
        baseview::Window::open_parented(
            parent,
            baseview::WindowOpenOptions {
                title,
                size: baseview::Size::new(width.into(), height.into()),
                scale: scale_policy,
                drop_target_valid: Some(Box::new(move || -> bool {
                    *drop_target_valid2.read().unwrap()
                })),
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
                    baseview_window: None,
                    drop_target_valid,
                });
                for (name, data) in fonts.drain(..) {
                    ui.add_font(name, data);
                }
                build(&mut ui);
                // If we set the window to the wrong size, we'll get a resize event, which will let us get the scale factor
                #[cfg(windows)]
                {
                    window.resize(baseview::Size::new(1.0, 1.0));
                }

                BaseViewUI { ui, parent_channel }
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
        A: 'static + App<R>,
    {
        let drop_target_valid = Arc::new(RwLock::new(true));
        let drop_target_valid2 = drop_target_valid.clone();
        baseview::Window::open_blocking(
            baseview::WindowOpenOptions {
                title,
                size: baseview::Size::new(width.into(), height.into()),
                scale: scale_policy,
                drop_target_valid: Some(Box::new(move || -> bool {
                    *drop_target_valid2.read().unwrap()
                })),
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
                    baseview_window: None,
                    drop_target_valid,
                });
                for (name, data) in fonts.drain(..) {
                    ui.add_font(name, data);
                }
                // If we set the window to the wrong size, we'll get a resize event, which will let us get the scale factor
                #[cfg(windows)]
                {
                    window.resize(baseview::Size::new(1.0, 1.0));
                }
                BaseViewUI {
                    ui,
                    parent_channel: None,
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

unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.display_handle
    }
}

use lemna::input::{Button, Drag, Input, Key, Motion, MouseButton};
impl<R: 'static + Renderer, A: 'static + App<R>> baseview::WindowHandler for BaseViewUI<R, A> {
    fn on_frame(&mut self, window: &mut baseview::Window) {
        if let Some(receiver) = &self.parent_channel {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    ParentMessage::Dirty => {
                        self.ui.set_dirty();
                    }
                    ParentMessage::Resize => {
                        let size = self.ui.window.read().unwrap().size;
                        window.resize(baseview::Size::new(size.0.into(), size.1.into()));
                    }
                }
            }
        }
        self.ui.draw();
        self.ui.render();
    }

    fn on_event(
        &mut self,
        window: &mut baseview::Window,
        event: baseview::Event,
    ) -> baseview::EventStatus {
        unsafe {
            // We're forcing the window into a static lifetime because we release it at the end of on_event
            let baseview_window: &'static baseview::Window<'static> = std::mem::transmute::<
                &baseview::Window,
                &'static baseview::Window<'static>,
            >(window);
            self.ui.window.write().unwrap().baseview_window = Some(baseview_window);
        }
        match event {
            baseview::Event::Window(event) => match event {
                baseview::WindowEvent::Resized(window_info) => {
                    let win = &self.ui.window;
                    let scale_policy = win.read().unwrap().scale_policy;
                    win.write().unwrap().scale_factor = match scale_policy {
                        baseview::WindowScalePolicy::ScaleFactor(scale) => scale,
                        baseview::WindowScalePolicy::SystemScaleFactor => window_info.scale(),
                    } as f32;
                    win.write().unwrap().size = (
                        window_info.logical_size().width as u32,
                        window_info.logical_size().height as u32,
                    );
                    self.ui.handle_input(&Input::Resize);
                }
                baseview::WindowEvent::WillClose => self.ui.handle_input(&Input::Exit),
                baseview::WindowEvent::Focused => self.ui.handle_input(&Input::Focus(true)),
                baseview::WindowEvent::Unfocused => self.ui.handle_input(&Input::Focus(false)),
                baseview::WindowEvent::DragEnter(d) => self
                    .ui
                    .handle_input(&Input::Drag(Drag::Start(baseview_data_to_lemna(d)))),
                baseview::WindowEvent::DragLeave => self.ui.handle_input(&Input::Drag(Drag::End)),
                baseview::WindowEvent::Dragging => {
                    self.ui.handle_input(&Input::Drag(Drag::Dragging));
                }
                baseview::WindowEvent::Drop(d) => self
                    .ui
                    .handle_input(&Input::Drag(Drag::Drop(baseview_data_to_lemna(d)))),
            },
            baseview::Event::Mouse(event) => match event {
                baseview::MouseEvent::CursorMoved {
                    position,
                    modifiers: _,
                } => {
                    self.ui.handle_input(&Input::Motion(Motion::Mouse {
                        x: position.x as f32,
                        y: position.y as f32,
                    }));
                }
                baseview::MouseEvent::ButtonPressed {
                    button,
                    modifiers: _,
                } => {
                    if let Some(button) = translate_mouse_button(&button) {
                        self.ui.handle_input(&Input::Press(button));
                    }
                }
                baseview::MouseEvent::ButtonReleased {
                    button,
                    modifiers: _,
                } => {
                    if let Some(button) = translate_mouse_button(&button) {
                        self.ui.handle_input(&Input::Release(button));
                    }
                }
                baseview::MouseEvent::WheelScrolled {
                    delta,
                    modifiers: _,
                } => {
                    let (mut x, y) = match delta {
                        baseview::ScrollDelta::Lines { x, y } => {
                            let points_per_scroll_line = 10.0;
                            (x * points_per_scroll_line, -y * points_per_scroll_line)
                        }
                        baseview::ScrollDelta::Pixels { x, y } => (x, -y),
                    };
                    if cfg!(target_os = "macos") {
                        // TODO Is this necessary?
                        x *= -1.0;
                    }
                    self.ui
                        .handle_input(&Input::Motion(Motion::Scroll { x, y }));
                }
                baseview::MouseEvent::CursorEntered => {
                    self.ui.handle_input(&Input::MouseEnterWindow)
                }
                baseview::MouseEvent::CursorLeft => self.ui.handle_input(&Input::MouseLeaveWindow),
            },
            baseview::Event::Keyboard(event) => {
                let key = translate_key(event.code);
                if event.state == keyboard_types::KeyState::Down {
                    self.ui.handle_input(&Input::Press(key));
                    if let keyboard_types::Key::Character(s) = &event.key {
                        self.ui.handle_input(&Input::Text(s.to_string()));
                    }
                } else {
                    self.ui.handle_input(&Input::Release(key));
                }
            }
        }
        baseview::EventStatus::Captured
    }
}

use keyboard_types::Code;
pub fn translate_key(key: Code) -> Button {
    Button::Keyboard(match key {
        Code::Backspace => Key::Backspace,
        Code::Tab => Key::Tab,
        Code::Enter => Key::Return,
        Code::Escape => Key::Escape,
        Code::Space => Key::Space,
        Code::Period => Key::Exclaim,
        Code::Backquote => Key::Backquote,

        Code::Quote => Key::Quote,
        Code::Comma => Key::Comma,
        Code::Minus => Key::Minus,
        Code::Slash => Key::Slash,
        Code::Digit0 => Key::D0,
        Code::Digit1 => Key::D1,
        Code::Digit2 => Key::D2,
        Code::Digit3 => Key::D3,
        Code::Digit4 => Key::D4,
        Code::Digit5 => Key::D5,
        Code::Digit6 => Key::D6,
        Code::Digit7 => Key::D7,
        Code::Digit8 => Key::D8,
        Code::Digit9 => Key::D9,
        Code::Semicolon => Key::Semicolon,
        Code::Equal => Key::Equals,
        Code::KeyA => Key::A,
        Code::KeyB => Key::B,
        Code::KeyC => Key::C,
        Code::KeyD => Key::D,
        Code::KeyE => Key::E,
        Code::KeyF => Key::F,
        Code::KeyG => Key::G,
        Code::KeyH => Key::H,
        Code::KeyI => Key::I,
        Code::KeyJ => Key::J,
        Code::KeyK => Key::K,
        Code::KeyL => Key::L,
        Code::KeyM => Key::M,
        Code::KeyN => Key::N,
        Code::KeyO => Key::O,
        Code::KeyP => Key::P,
        Code::KeyQ => Key::Q,
        Code::KeyR => Key::R,
        Code::KeyS => Key::S,
        Code::KeyT => Key::T,
        Code::KeyU => Key::U,
        Code::KeyV => Key::V,
        Code::KeyW => Key::W,
        Code::KeyX => Key::X,
        Code::KeyY => Key::Y,
        Code::KeyZ => Key::Z,
        Code::BracketLeft => Key::LeftBracket,
        Code::Backslash => Key::Backslash,
        Code::BracketRight => Key::RightBracket,

        Code::ShiftLeft => Key::LShift,
        Code::AltLeft => Key::LAlt,
        Code::ControlLeft => Key::LCtrl,
        Code::ShiftRight => Key::RShift,
        Code::AltRight => Key::RAlt,
        Code::ControlRight => Key::RCtrl,

        Code::End => Key::End,
        Code::Home => Key::Home,
        Code::ArrowLeft => Key::Left,
        Code::ArrowUp => Key::Up,
        Code::ArrowRight => Key::Right,
        Code::ArrowDown => Key::Down,
        Code::Insert => Key::Insert,

        Code::Numpad0 => Key::NumPad0,
        Code::Numpad1 => Key::NumPad1,
        Code::Numpad2 => Key::NumPad2,
        Code::Numpad3 => Key::NumPad3,
        Code::Numpad4 => Key::NumPad4,
        Code::Numpad5 => Key::NumPad5,
        Code::Numpad6 => Key::NumPad6,
        Code::Numpad7 => Key::NumPad7,
        Code::Numpad8 => Key::NumPad8,
        Code::Numpad9 => Key::NumPad9,

        Code::F1 => Key::F1,
        Code::F2 => Key::F2,
        Code::F3 => Key::F3,
        Code::F4 => Key::F4,
        Code::F5 => Key::F5,
        Code::F6 => Key::F6,
        Code::F7 => Key::F7,
        Code::F8 => Key::F8,
        Code::F9 => Key::F9,
        Code::F10 => Key::F10,
        Code::F11 => Key::F11,
        Code::F12 => Key::F12,

        Code::PageUp => Key::PageUp,
        Code::PageDown => Key::PageDown,

        Code::NumpadEnter => Key::NumPadEnter,
        Code::NumpadMultiply => Key::NumPadMultiply,
        Code::NumpadAdd => Key::NumPadPlus,
        Code::NumpadSubtract => Key::NumPadMinus,
        Code::NumpadDecimal => Key::NumPadPeriod,
        Code::NumpadDivide => Key::NumPadDivide,

        _ => Key::Unknown,
    })
}

pub fn translate_mouse_button(button: &baseview::MouseButton) -> Option<Button> {
    match button {
        baseview::MouseButton::Left => Some(Button::Mouse(MouseButton::Left)),
        baseview::MouseButton::Right => Some(Button::Mouse(MouseButton::Right)),
        baseview::MouseButton::Middle => Some(Button::Mouse(MouseButton::Middle)),
        baseview::MouseButton::Forward => Some(Button::Mouse(MouseButton::Aux1)),
        baseview::MouseButton::Back => Some(Button::Mouse(MouseButton::Aux2)),
        _ => None,
    }
}

impl lemna::window::Window for Window {
    fn logical_size(&self) -> PixelSize {
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

    fn get_from_clipboard(&self) -> Option<Data> {
        let mut clipboard = Clipboard::new().expect("Could get a clipboard");
        match clipboard.get_text() {
            Ok(s) => Some(Data::String(s)),
            _ => None,
        }
    }

    fn put_on_clipboard(&self, data: &Data) {
        let mut clipboard = Clipboard::new().expect("Could get a clipboard");
        match data {
            Data::String(s) => {
                clipboard.set_text(s).unwrap();
            }
            _ => (),
        }
    }

    fn start_drag(&self, data: Data) {
        if let Some(win) = self.baseview_window {
            win.start_drag(lemna_data_to_baseview(data));
        }
    }

    fn set_drop_target_valid(&self, valid: bool) {
        *self.drop_target_valid.write().unwrap() = valid
    }
}

pub fn baseview_data_to_lemna(d: baseview::Data) -> Data {
    match d {
        baseview::Data::Filepath(p) => Data::Filepath(p),
        baseview::Data::String(s) => Data::String(s),
    }
}

pub fn lemna_data_to_baseview(d: Data) -> baseview::Data {
    match d {
        Data::Filepath(p) => baseview::Data::Filepath(p),
        Data::String(s) => baseview::Data::String(s),
        _ => unimplemented!(),
    }
}
