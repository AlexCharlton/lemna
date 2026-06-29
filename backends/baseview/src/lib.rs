use std::any::Any;
use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock, RwLock};

mod window_options;

use baseview::dpi::LogicalSize;
use baseview::{
    Event, EventStatus, MouseCursor, WindowContext, WindowHandler, WindowOpenOptions, WindowSize,
};
use keyboard_types::Code;
use lemna::input::{Button, Drag, Input, Key, Motion, MouseButton};
use lemna::{Component, Data, PixelSize, UI, log_error};
use raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};

pub use window_options::WindowOptions;

#[cfg(windows)]
fn sync_child_to_parent_client(context: &WindowContext) {
    use raw_window_handle::RawWindowHandle;
    use winapi::shared::windef::RECT;
    use winapi::um::winuser::{GetClientRect, GetParent, HWND_TOP, SWP_SHOWWINDOW, SetWindowPos};

    let Ok(handle) = context.window_handle() else {
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

pub type Message = Box<dyn Any + Send>;

const POINTS_PER_SCROLL_LINE: f32 = 32.0;

#[derive(Debug)]
pub enum ParentMessage {
    Resize,
    AppMessage(Message),
}

struct BaseViewUI<A: 'static + Component + Default + Send + Sync> {
    ui: RefCell<UI<A>>,
    window: WindowContext,
    parent_channel: Option<crossbeam_channel::Receiver<ParentMessage>>,
    drop_target_valid: Arc<RwLock<bool>>,
    // For parented windows, we need to force the focus to the window when the user clicks on it
    needs_forced_focus: bool,
    focused: Cell<bool>,
}

#[derive(Debug, Clone, Copy)]
struct WindowSizeState {
    logical_size: (u32, u32),
    scale_factor: f32,
    scale_policy: baseview::WindowScalePolicy,
}

impl Default for WindowSizeState {
    fn default() -> Self {
        WindowSizeState {
            logical_size: (0, 0),
            scale_factor: 1.0,
            scale_policy: baseview::WindowScalePolicy::SystemScaleFactor,
        }
    }
}

fn window_size() -> &'static RwLock<WindowSizeState> {
    static WINDOW_SIZE: OnceLock<RwLock<WindowSizeState>> = OnceLock::new();
    WINDOW_SIZE.get_or_init(|| RwLock::new(WindowSizeState::default()))
}

fn set_window_size(size: (u32, u32), scale_factor: f32, scale_policy: baseview::WindowScalePolicy) {
    *window_size().write().unwrap() = WindowSizeState {
        logical_size: size,
        scale_factor,
        scale_policy,
    };
}

fn get_window_size() -> WindowSizeState {
    *window_size().read().unwrap()
}

fn window_open_options(options: &WindowOptions) -> WindowOpenOptions {
    WindowOpenOptions::default()
        .with_title(options.title.clone())
        .with_size(LogicalSize::new(
            options.width as f64,
            options.height as f64,
        ))
        .with_scale_policy(options.scale_policy)
    // .resizable(options.resizable)
}

fn initial_scale_factor(options: &WindowOptions) -> f32 {
    (match options.scale_policy {
        baseview::WindowScalePolicy::ScaleFactor(scale) => scale,
        baseview::WindowScalePolicy::SystemScaleFactor => 1.0, // Assume for now until scale event
    }) as f32
}

pub struct Window {
    context: WindowContext,
    drop_target_valid: Arc<RwLock<bool>>,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    fn new(context: WindowContext, drop_target_valid: Arc<RwLock<bool>>) -> Self {
        Self {
            context,
            drop_target_valid,
        }
    }

    /// Open as a child of another window. `options.resizable` will not do anything.
    pub fn open_parented<P, A, B>(
        parent: &P,
        mut options: WindowOptions,
        build: B,
        parent_channel: Option<crossbeam_channel::Receiver<ParentMessage>>,
    ) -> baseview::WindowHandle
    where
        P: HasWindowHandle,
        A: 'static + Component + Default + Send + Sync,
        B: Fn(&mut UI<A>) + 'static + Send,
    {
        let drop_target_valid = Arc::new(RwLock::new(true));
        let drop_target_valid2 = drop_target_valid.clone();
        let open_options = window_open_options(&options);
        let scale_factor = initial_scale_factor(&options);
        let initial_size = (options.width, options.height);
        let scale_policy = options.scale_policy;

        baseview::Window::open_parented(
            parent,
            open_options,
            move |window: WindowContext| -> BaseViewUI<A> {
                set_window_size(initial_size, scale_factor, scale_policy);
                let lemna_window = Self::new(window.clone(), drop_target_valid);
                let mut ui = UI::new(lemna_window);
                for (name, data) in options.fonts.drain(..) {
                    if let Err(_e) = ui.add_font(name, data) {
                        log_error!("Failed to add font: {}", _e);
                    }
                }
                build(&mut ui);

                BaseViewUI {
                    ui: RefCell::new(ui),
                    window,
                    parent_channel,
                    drop_target_valid: drop_target_valid2,
                    needs_forced_focus: true,
                    focused: Cell::new(false),
                }
            },
        )
    }

    pub fn open_blocking<A>(mut options: WindowOptions)
    where
        A: 'static + Component + Default + Send + Sync,
    {
        let drop_target_valid = Arc::new(RwLock::new(true));
        let drop_target_valid2 = drop_target_valid.clone();
        let open_options = window_open_options(&options);
        let scale_factor = initial_scale_factor(&options);
        let initial_size = (options.width, options.height);
        let scale_policy = options.scale_policy;

        baseview::Window::open_blocking(
            open_options,
            move |window: WindowContext| -> BaseViewUI<A> {
                set_window_size(initial_size, scale_factor, scale_policy);
                let lemna_window = Self::new(window.clone(), drop_target_valid);
                let mut ui = UI::new(lemna_window);
                for (name, data) in options.fonts.drain(..) {
                    if let Err(_e) = ui.add_font(name, data) {
                        log_error!("Failed to add font: {}", _e);
                    }
                }

                BaseViewUI {
                    ui: RefCell::new(ui),
                    window,
                    parent_channel: None,
                    drop_target_valid: drop_target_valid2,
                    needs_forced_focus: false,
                    focused: Cell::new(false),
                }
            },
        );
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.context.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, HandleError> {
        self.context.display_handle()
    }
}

impl<A: 'static + Component + Default + Send + Sync> WindowHandler for BaseViewUI<A> {
    fn on_frame(&self) {
        if let Some(receiver) = &self.parent_channel {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    ParentMessage::AppMessage(m) => {
                        self.ui.borrow_mut().update(m);
                    }
                    ParentMessage::Resize => {
                        let size = get_window_size();
                        self.window.resize(LogicalSize::new(
                            size.logical_size.0 as f64,
                            size.logical_size.1 as f64,
                        ));
                        #[cfg(windows)]
                        sync_child_to_parent_client(&self.window);
                    }
                }
            }
        }
        self.ui.borrow_mut().handle_input(&Input::Timer);
        self.ui.borrow_mut().draw();
        self.ui.borrow_mut().render();
    }

    fn resized(&self, window_info: WindowSize) {
        let window_size = get_window_size();
        let scale_policy = window_size.scale_policy;
        let scale_factor = match scale_policy {
            baseview::WindowScalePolicy::ScaleFactor(scale) => scale,
            baseview::WindowScalePolicy::SystemScaleFactor => window_info.scale_factor,
        } as f32;
        set_window_size(
            (
                window_info.logical.width as u32,
                window_info.logical.height as u32,
            ),
            scale_factor,
            scale_policy,
        );
        self.ui.borrow_mut().handle_input(&Input::Resize);
    }

    fn on_event(&self, event: Event) -> EventStatus {
        let mut drag_event = false;
        let mut handled = true;
        match event {
            Event::Window(event) => match event {
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
            Event::Mouse(event) => match event {
                baseview::MouseEvent::DragEntered { position, data, .. } => {
                    drag_event = true;
                    *self.drop_target_valid.write().unwrap() = true;
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Motion(Motion::Mouse {
                            x: position.x as f32,
                            y: position.y as f32,
                        }));
                    for data in drop_data_to_lemna(data) {
                        handled &= self
                            .ui
                            .borrow_mut()
                            .handle_input(&Input::Drag(Drag::Start(data)));
                    }
                }
                baseview::MouseEvent::DragMoved { position, .. } => {
                    drag_event = true;
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Motion(Motion::Mouse {
                            x: position.x as f32,
                            y: position.y as f32,
                        }));
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Drag(Drag::Dragging));
                }
                baseview::MouseEvent::DragLeft => {
                    drag_event = true;
                    handled &= self.ui.borrow_mut().handle_input(&Input::Drag(Drag::End));
                }
                baseview::MouseEvent::DragDropped { position, data, .. } => {
                    drag_event = true;
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Motion(Motion::Mouse {
                            x: position.x as f32,
                            y: position.y as f32,
                        }));
                    if let Some(data) = drop_data_to_lemna(data).into_iter().next() {
                        handled &= self
                            .ui
                            .borrow_mut()
                            .handle_input(&Input::Drag(Drag::Drop(data)));
                    }
                }
                baseview::MouseEvent::CursorMoved {
                    position,
                    modifiers: _,
                } => {
                    if self.needs_forced_focus && !self.focused.get() {
                        self.window.focus();
                    }
                    handled &= self
                        .ui
                        .borrow_mut()
                        .handle_input(&Input::Motion(Motion::Mouse {
                            x: position.x as f32,
                            y: position.y as f32,
                        }));
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
            Event::Keyboard(event) => {
                let key = translate_key(event.code);
                if event.state == keyboard_types::KeyState::Down {
                    handled &= self.ui.borrow_mut().handle_input(&Input::Press(key));
                    if let keyboard_types::Key::Character(s) = &event.key {
                        handled &= self
                            .ui
                            .borrow_mut()
                            .handle_input(&Input::Text(s.to_string()));
                    }
                } else {
                    handled &= self.ui.borrow_mut().handle_input(&Input::Release(key));
                }
            }
            _ => {}
        }
        if drag_event && *self.drop_target_valid.read().unwrap() {
            EventStatus::AcceptDrop(baseview::DropEffect::Copy)
        } else if !handled {
            EventStatus::Ignored
        } else {
            EventStatus::Captured
        }
    }
}

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
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new().expect("Could get a clipboard");
        match clipboard.get_text() {
            Ok(s) => Some(Data::String(s)),
            _ => None,
        }
    }

    fn put_on_clipboard(&self, data: &Data) {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new().expect("Could get a clipboard");
        match data {
            Data::String(s) => {
                clipboard.set_text(s).unwrap();
            }
            _ => (),
        }
    }

    fn start_drag(&self, _data: Data) {
        // baseview drag-out not yet wired up
        // self.context.start_drag(lemna_data_to_drop_data(data));
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
        self.context.set_mouse_cursor(ct);
    }

    fn unset_cursor(&self) {
        self.context.set_mouse_cursor(MouseCursor::Default);
    }
}

fn drop_data_to_lemna(data: baseview::DropData) -> Vec<Data> {
    match data {
        baseview::DropData::None => vec![],
        baseview::DropData::Files(paths) => paths.into_iter().map(Data::Filepath).collect(),
        // baseview::DropData::Url(url) => vec![Data::String(url)],
        _ => vec![],
    }
}

#[allow(dead_code)]
fn lemna_data_to_drop_data(d: Data) -> baseview::DropData {
    match d {
        Data::Filepath(p) => baseview::DropData::Files(vec![p]),
        Data::String(_) => baseview::DropData::None,
    }
}
