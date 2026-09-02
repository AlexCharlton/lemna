use std::any::Any;
use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(windows)]
fn sync_child_to_parent_client(window: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use winapi::shared::windef::RECT;
    use winapi::um::winuser::{GetClientRect, GetParent, HWND_TOP, SWP_SHOWWINDOW, SetWindowPos};

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get() as winapi::shared::windef::HWND;
    unsafe {
        let parent = GetParent(hwnd);
        if parent.is_null() {
            return;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetClientRect(parent, &mut rect) == 0 {
            return;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return;
        }
        SetWindowPos(hwnd, HWND_TOP, 0, 0, width, height, SWP_SHOWWINDOW);
    }
}

use arboard::{self, Clipboard};
use baseview::MouseCursor;
use baseview::dpi::LogicalSize;
use lemna::{Component, Data, PixelSize, UI, log_error};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

mod window_options;
pub use window_options::WindowOptions;

pub type Message = Box<dyn Any + Send>;

const POINTS_PER_SCROLL_LINE: f32 = 32.0;

#[derive(Debug)]
pub enum ParentMessage {
    Resize,
    AppMessage(Message),
}

struct BaseViewUI<A: 'static + Component + Default + Send + Sync> {
    ui: RefCell<UI<A>>,
    window: baseview::WindowContext,
    parent_channel: Option<crossbeam_channel::Receiver<ParentMessage>>,
    drop_target_valid: Arc<RwLock<bool>>,
    // For parented windows, we need to force the focus to the window when the user clicks on it
    needs_forced_focus: bool,
    focused: Cell<bool>,
}

#[derive(Debug, Clone, Copy)]
struct WindowSize {
    logical_size: (u32, u32),
    scale_factor: f32,
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize {
            logical_size: (0, 0),
            scale_factor: 1.0,
        }
    }
}

fn window_size() -> &'static RwLock<WindowSize> {
    static WINDOW_SIZE: OnceLock<RwLock<WindowSize>> = OnceLock::new();
    WINDOW_SIZE.get_or_init(|| RwLock::new(WindowSize::default()))
}

fn set_window_size(size: (u32, u32), scale_factor: f32) {
    *window_size().write().unwrap() = WindowSize {
        logical_size: size,
        scale_factor,
    };
}

fn get_window_size() -> WindowSize {
    *window_size().read().unwrap()
}

fn window_settings<P: HasWindowHandle>(
    parent: Option<&P>,
    options: &WindowOptions,
) -> baseview::WindowSettings {
    let mut settings = baseview::WindowSettings::new()
        .with_title(options.title.clone())
        .with_size(LogicalSize::new(
            options.width as f64,
            options.height as f64,
        ))
        .with_resizable(options.resizable);
    if let Some(parent) = parent {
        settings = settings.with_parent(Some(parent));
    }
    settings.with_fallback_scale_factor(options.fallback_scale_factor)
}

pub struct Window {
    handle: RawWindowHandle,
    display_handle: RawDisplayHandle,
    drop_target_valid: Arc<RwLock<bool>>,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    /// Open as a child of another window. `options.resizable` will not do anything.
    pub fn open_parented<P, A, B>(
        parent: &P,
        mut options: WindowOptions,
        build: B,
        parent_channel: Option<crossbeam_channel::Receiver<ParentMessage>>,
    ) -> baseview::Window
    where
        P: HasWindowHandle,
        A: 'static + Component + Default + Send + Sync,
        B: Fn(&mut UI<A>) + 'static + Send,
    {
        let drop_target_valid = Arc::new(RwLock::new(true));
        let drop_target_valid2 = drop_target_valid.clone();
        let settings = window_settings(Some(parent), &options);
        let window = baseview::Window::create(settings, move |window| {
            let scale_factor = window.scale_factor() as f32;
            set_window_size((options.width, options.height), scale_factor);
            let mut ui: UI<A> = UI::new(Self {
                handle: window.window_handle().expect("window handle").as_raw(),
                display_handle: window.display_handle().expect("display handle").as_raw(),
                drop_target_valid,
            });
            for (name, data) in options.fonts.drain(..) {
                if let Err(_e) = ui.add_font(name, data) {
                    log_error!("Failed to add font: {}", _e);
                }
            }
            build(&mut ui);

            Ok(BaseViewUI {
                ui: RefCell::new(ui),
                window,
                parent_channel,
                drop_target_valid: drop_target_valid2,
                needs_forced_focus: true,
                focused: Cell::new(false),
            })
        })
        .expect("failed to create window");
        window.show().expect("failed to show window");
        window
    }

    pub fn open_blocking<A>(mut options: WindowOptions)
    where
        A: 'static + Component + Default + Send + Sync,
    {
        let drop_target_valid = Arc::new(RwLock::new(true));
        let drop_target_valid2 = drop_target_valid.clone();
        let settings = window_settings::<Window>(None, &options);
        let window = baseview::Window::create(settings, move |window| {
            let scale_factor = window.scale_factor() as f32;
            set_window_size((options.width, options.height), scale_factor);
            let mut ui: UI<A> = UI::new(Self {
                handle: window.window_handle().expect("window handle").as_raw(),
                display_handle: window.display_handle().expect("display handle").as_raw(),
                drop_target_valid,
            });
            for (name, data) in options.fonts.drain(..) {
                if let Err(_e) = ui.add_font(name, data) {
                    log_error!("Failed to add font: {}", _e);
                }
            }

            Ok(BaseViewUI {
                ui: RefCell::new(ui),
                window,
                parent_channel: None,
                drop_target_valid: drop_target_valid2,
                needs_forced_focus: false,
                focused: Cell::new(false),
            })
        })
        .expect("failed to create window");
        window.run_until_closed().expect("failed to run window");
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(self.handle) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe { DisplayHandle::borrow_raw(self.display_handle) })
    }
}

thread_local!(
    static CURRENT_WINDOW: RefCell<Option<baseview::WindowContext>> = const { RefCell::new(None) };
);

/// Return the current [`WindowContext`], if called during event handling.
pub fn current_window() -> Option<baseview::WindowContext> {
    CURRENT_WINDOW.with(|r| r.borrow().clone())
}

fn clear_current_window() {
    CURRENT_WINDOW.with(|r| *r.borrow_mut() = None);
}

fn set_current_window(window: baseview::WindowContext) {
    CURRENT_WINDOW.with(|r| *r.borrow_mut() = Some(window));
}

fn physical_to_logical(x: f64, y: f64) -> (f32, f32) {
    let scale = get_window_size().scale_factor;
    ((x as f32) / scale, (y as f32) / scale)
}

use lemna::input::{Button, Drag, Input, Key, Motion, MouseButton};
impl<A: 'static + Component + Default + Send + Sync> baseview::WindowHandler for BaseViewUI<A> {
    fn on_frame(&self) -> Result<(), baseview::HandlerError> {
        if let Some(receiver) = &self.parent_channel {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    ParentMessage::AppMessage(m) => {
                        self.ui.borrow_mut().update(m);
                    }
                    ParentMessage::Resize => {
                        let size = get_window_size();
                        let _ = self.window.resize(LogicalSize::new(
                            size.logical_size.0 as f64,
                            size.logical_size.1 as f64,
                        ));
                        #[cfg(windows)]
                        sync_child_to_parent_client(&self.window);
                    }
                }
            }
        }
        let mut ui = self.ui.borrow_mut();
        ui.handle_input(&Input::Timer);
        ui.draw();
        ui.render();
        Ok(())
    }

    fn resized(&self, new_size: baseview::WindowSize) -> Result<(), baseview::HandlerError> {
        set_window_size(
            (
                new_size.logical.width as u32,
                new_size.logical.height as u32,
            ),
            new_size.scale_factor as f32,
        );
        self.ui.borrow_mut().handle_input(&Input::Resize);
        Ok(())
    }

    fn on_event(&self, event: baseview::Event) -> baseview::EventStatus {
        set_current_window(self.window.clone());
        let mut drag_event = false;
        let mut handled = true;
        match event {
            baseview::Event::Window(event) => match event {
                baseview::WindowEvent::WillClose => {
                    handled &= self.ui.borrow_mut().handle_input(&Input::Exit);
                }
                baseview::WindowEvent::Focused => {
                    handled &= self.ui.borrow_mut().handle_input(&Input::Focus(true));
                    self.focused.set(true);
                }
                baseview::WindowEvent::Unfocused => {
                    handled &= self.ui.borrow_mut().handle_input(&Input::Focus(false));
                    self.focused.set(false);
                }
                _ => {}
            },
            baseview::Event::Mouse(event) => match event {
                baseview::MouseEvent::DragEntered { position, data, .. } => {
                    drag_event = true;
                    *self.drop_target_valid.write().unwrap() = true;
                    let (x, y) = physical_to_logical(position.x, position.y);
                    let mut ui = self.ui.borrow_mut();
                    handled &= ui.handle_input(&Input::Motion(Motion::Mouse { x, y }));
                    for data in drop_data_to_lemna(data) {
                        handled &= ui.handle_input(&Input::Drag(Drag::Start(data)));
                    }
                }
                baseview::MouseEvent::DragMoved { position, .. } => {
                    drag_event = true;
                    let (x, y) = physical_to_logical(position.x, position.y);
                    let mut ui = self.ui.borrow_mut();
                    handled &= ui.handle_input(&Input::Motion(Motion::Mouse { x, y }));
                    handled &= ui.handle_input(&Input::Drag(Drag::Dragging));
                }
                baseview::MouseEvent::DragLeft => {
                    drag_event = true;
                    handled &= self.ui.borrow_mut().handle_input(&Input::Drag(Drag::End));
                }
                baseview::MouseEvent::DragDropped { position, data, .. } => {
                    drag_event = true;
                    let (x, y) = physical_to_logical(position.x, position.y);
                    let mut ui = self.ui.borrow_mut();
                    handled &= ui.handle_input(&Input::Motion(Motion::Mouse { x, y }));
                    if let Some(data) = drop_data_to_lemna(data).into_iter().next() {
                        handled &= ui.handle_input(&Input::Drag(Drag::Drop(data)));
                    }
                }
                baseview::MouseEvent::CursorMoved {
                    position,
                    modifiers: _,
                } => {
                    if self.needs_forced_focus && !self.focused.get() {
                        let _ = self.window.focus();
                    }
                    let (x, y) = physical_to_logical(position.x, position.y);
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Motion(Motion::Mouse { x, y }));
                }
                baseview::MouseEvent::ButtonPressed {
                    button,
                    modifiers: _,
                } => {
                    if let Some(button) = translate_mouse_button(&button) {
                        handled &= self.ui.borrow_mut().handle_input(&Input::Press(button));
                    }
                }
                baseview::MouseEvent::ButtonReleased {
                    button,
                    modifiers: _,
                } => {
                    if let Some(button) = translate_mouse_button(&button) {
                        handled &= self.ui.borrow_mut().handle_input(&Input::Release(button));
                    }
                }
                baseview::MouseEvent::WheelScrolled {
                    delta,
                    modifiers: _,
                } => {
                    let (mut x, y) = match delta {
                        baseview::ScrollDelta::Lines { x, y } => {
                            (x * POINTS_PER_SCROLL_LINE, -y * POINTS_PER_SCROLL_LINE)
                        }
                        baseview::ScrollDelta::Pixels { x, y } => (x, -y),
                    };
                    if cfg!(target_os = "macos") {
                        x *= -1.0;
                    }
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Motion(Motion::Scroll { x, y }));
                }
                baseview::MouseEvent::CursorEntered => {
                    handled &= self.ui.borrow_mut().handle_input(&Input::MouseEnterWindow);
                }
                baseview::MouseEvent::CursorLeft => {
                    handled &= self.ui.borrow_mut().handle_input(&Input::MouseLeaveWindow);
                }
                _ => {}
            },
            baseview::Event::Keyboard(event) => {
                let key = translate_key(event.code);
                let mut ui = self.ui.borrow_mut();
                if event.state == keyboard_types::KeyState::Down {
                    handled &= ui.handle_input(&Input::Press(key));
                    if let keyboard_types::Key::Character(s) = &event.key {
                        handled &= ui.handle_input(&Input::Text(s.to_string()));
                    }
                } else {
                    handled &= ui.handle_input(&Input::Release(key));
                }
            }
            _ => {}
        }
        clear_current_window();
        if drag_event && *self.drop_target_valid.read().unwrap() {
            baseview::EventStatus::AcceptDrop(baseview::DropEffect::Copy)
        } else if !handled {
            baseview::EventStatus::Ignored
        } else {
            baseview::EventStatus::Captured
        }
    }
}

use keyboard_types::Code;
fn translate_key(key: Code) -> Button {
    Button::Keyboard(match key {
        Code::Backspace => Key::Backspace,
        Code::Tab => Key::Tab,
        Code::Enter => Key::Return,
        Code::Escape => Key::Escape,
        Code::Space => Key::Space,

        Code::Period => Key::Exclaim,
        Code::Comma => Key::Comma,
        Code::Slash => Key::Slash,
        Code::Semicolon => Key::Semicolon,
        Code::Quote => Key::Quote,
        Code::BracketLeft => Key::LeftBracket,
        Code::BracketRight => Key::RightBracket,
        Code::Backslash => Key::Backslash,

        Code::Backquote => Key::Backquote,
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
        Code::Minus => Key::Minus,
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

        Code::ShiftLeft => Key::LShift,
        Code::AltLeft => Key::LAlt,
        Code::ControlLeft => Key::LCtrl,
        Code::ShiftRight => Key::RShift,
        Code::AltRight => Key::RAlt,
        Code::ControlRight => Key::RCtrl,

        Code::End => Key::End,
        Code::Home => Key::Home,
        Code::Insert => Key::Insert,
        Code::Delete => Key::Delete,
        Code::PageUp => Key::PageUp,
        Code::PageDown => Key::PageDown,

        Code::ArrowLeft => Key::Left,
        Code::ArrowUp => Key::Up,
        Code::ArrowRight => Key::Right,
        Code::ArrowDown => Key::Down,

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
        Code::PrintScreen => Key::PrintScreen,
        Code::ScrollLock => Key::ScrollLock,
        Code::Pause => Key::Pause,
        Code::AudioVolumeUp => Key::VolumeUp,
        Code::AudioVolumeDown => Key::VolumeDown,
        Code::AudioVolumeMute => Key::Mute,

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

        Code::NumpadEnter => Key::NumPadEnter,
        Code::NumpadMultiply => Key::NumPadMultiply,
        Code::NumpadAdd => Key::NumPadPlus,
        Code::NumpadSubtract => Key::NumPadMinus,
        Code::NumpadDecimal => Key::NumPadPeriod,
        Code::NumpadDivide => Key::NumPadDivide,

        _ => Key::Unknown,
    })
}

fn translate_mouse_button(button: &baseview::MouseButton) -> Option<Button> {
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
        let size = get_window_size();
        PixelSize {
            width: size.logical_size.0,
            height: size.logical_size.1,
        }
    }

    fn physical_size(&self) -> PixelSize {
        let size = get_window_size();
        PixelSize {
            width: ((size.logical_size.0 as f32) * size.scale_factor) as u32,
            height: ((size.logical_size.1 as f32) * size.scale_factor) as u32,
        }
    }

    fn scale_factor(&self) -> f32 {
        get_window_size().scale_factor
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

    fn start_drag(&self, _data: Data) {
        // if let Some(win) = current_window() {
        // win.start_drag(lemna_data_to_drop_data(data));
        // }
    }

    fn set_drop_target_valid(&self, valid: bool) {
        *self.drop_target_valid.write().unwrap() = valid
    }

    fn set_cursor(&self, cursor_type: &str) {
        let ct = match cursor_type {
            "Arrow" => MouseCursor::Default,
            "None" => MouseCursor::Hidden,
            "Hidden" => MouseCursor::Hidden,
            "Ibeam" | "Text" => MouseCursor::Text,
            "Hand" => MouseCursor::Hand,
            "HandGrabbing" => MouseCursor::HandGrabbing,
            "NoEntry" => MouseCursor::NotAllowed,
            "Cross" => MouseCursor::Crosshair,
            "Size" | "Move" => MouseCursor::Move,
            "SizeNWSE" => MouseCursor::NwseResize,
            "SizeNS" => MouseCursor::NsResize,
            "SizeNESW" => MouseCursor::NeswResize,
            "SizeWE" => MouseCursor::EwResize,
            _ => MouseCursor::Default,
        };
        if let Some(win) = current_window() {
            let _ = win.set_mouse_cursor(ct);
        }
    }

    fn unset_cursor(&self) {
        if let Some(win) = current_window() {
            let _ = win.set_mouse_cursor(MouseCursor::Default);
        }
    }
}

fn drop_data_to_lemna(data: baseview::DropData) -> Vec<Data> {
    match data {
        baseview::DropData::Files(paths) => paths.into_iter().map(Data::Filepath).collect(),
        _ => vec![Data::None],
    }
}
