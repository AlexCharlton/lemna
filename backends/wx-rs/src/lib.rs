use std::any::Any;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::c_void;

use lemna::input::{Button, Input, Key, Motion, MouseButton};
use lemna::{render::Renderer, App, PixelSize, UI};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use wx_rs::{CursorType, EventType, WheelAxis};

pub struct Window<R, A> {
    wx_rs_window: wx_rs::Window,
    phantom_renderer: PhantomData<R>,
    phantom_app: PhantomData<A>,
}

thread_local!(
    pub static UI: UnsafeCell<Box<dyn Any>> = UnsafeCell::new(Box::new(1))
);

pub fn ui() -> &'static mut Box<dyn Any> {
    UI.with(|r| unsafe { r.get().as_mut().unwrap() })
}

impl<R, A> Window<R, A>
where
    R: Renderer + 'static,
    <R as Renderer>::Renderable: std::fmt::Debug,
    A: 'static + App<R>,
{
    pub fn open_blocking(
        title: &str,
        width: u32,
        height: u32,
        mut fonts: Vec<(String, &'static [u8])>,
    ) {
        wx_rs::init_app(title, width, height);
        let mut ui: UI<Window<R, A>, R, A> = UI::new(Window::<R, A> {
            wx_rs_window: wx_rs::Window::new(),
            phantom_app: PhantomData,
            phantom_renderer: PhantomData,
        });
        for (name, data) in fonts.drain(..) {
            ui.add_font(name, data);
        }

        UI.with(|r| unsafe {
            let r = r.get().as_mut().unwrap();
            *r = Box::new(ui);
        });

        wx_rs::set_render(Self::render);
        wx_rs::bind_canvas_events(Self::handle_event);
        wx_rs::run_app();
    }

    extern "C" fn render() {
        let ui = ui().downcast_mut::<UI<Window<R, A>, R, A>>().unwrap();
        if ui.draw() {
            ui.render();
        }
    }

    extern "C" fn handle_event(event: *const c_void) {
        let ui = ui().downcast_mut::<UI<Window<R, A>, R, A>>().unwrap();
        for input in event_to_input(event).iter() {
            ui.handle_input(input);
        }
    }
}

impl<R, A> lemna::window::Window for Window<R, A>
where
    R: 'static,
    A: 'static,
{
    fn client_size(&self) -> PixelSize {
        unsafe { mem::transmute(wx_rs::get_client_size()) }
    }

    fn physical_size(&self) -> PixelSize {
        unsafe { mem::transmute(wx_rs::get_display_size()) }
    }

    fn scale_factor(&self) -> f32 {
        wx_rs::get_scale_factor()
    }

    fn put_on_clipboard(&self, data: &lemna::window::Data) {
        unsafe { wx_rs::put_on_clipboard(mem::transmute(data)) }
    }

    fn get_from_clipboard(&self) -> Option<lemna::window::Data> {
        unsafe { mem::transmute(wx_rs::get_from_clipboard()) }
    }

    fn set_cursor(&self, cursor_type: &str) {
        let ct = match cursor_type {
            "Arrow" => CursorType::Arrow,
            "None" => CursorType::None,
            "Ibeam" => CursorType::Ibeam,
            "Hand" => CursorType::Hand,
            "Pencil" => CursorType::Pencil,
            "NoEntry" => CursorType::NoEntry,
            "Cross" => CursorType::Cross,
            "Size" => CursorType::Size,
            "SizeNWSE" => CursorType::SizeNWSE,
            "SizeNS" => CursorType::SizeNS,
            "SizeNESW" => CursorType::SizeNESW,
            "SizeWE" => CursorType::SizeWE,
            _ => CursorType::Arrow,
        };
        wx_rs::set_cursor(ct);
    }

    fn unset_cursor(&self) {
        wx_rs::set_cursor(CursorType::Arrow);
    }
}

unsafe impl<R, A> HasRawWindowHandle for Window<R, A> {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.wx_rs_window.raw_window_handle()
    }
}

unsafe impl<R, A> HasRawDisplayHandle for Window<R, A> {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.wx_rs_window.raw_display_handle()
    }
}

pub fn event_to_input(event: *const c_void) -> Vec<Input> {
    match wx_rs::get_event_type(event) {
        EventType::MouseLeftDown => vec![Input::Press(Button::Mouse(MouseButton::Left))],
        EventType::MouseLeftUp => vec![Input::Release(Button::Mouse(MouseButton::Left))],
        EventType::MouseLeftDclick => vec![
            Input::Press(Button::Mouse(MouseButton::Left)),
            Input::Release(Button::Mouse(MouseButton::Left)),
        ],
        EventType::MouseRightDown => vec![Input::Press(Button::Mouse(MouseButton::Right))],
        EventType::MouseRightUp => vec![Input::Release(Button::Mouse(MouseButton::Right))],
        EventType::MouseRightDclick => vec![
            Input::Press(Button::Mouse(MouseButton::Right)),
            Input::Release(Button::Mouse(MouseButton::Right)),
        ],
        EventType::MouseMiddleDown => vec![Input::Press(Button::Mouse(MouseButton::Middle))],
        EventType::MouseMiddleUp => vec![Input::Release(Button::Mouse(MouseButton::Middle))],
        EventType::MouseMiddleDclick => vec![
            Input::Press(Button::Mouse(MouseButton::Middle)),
            Input::Release(Button::Mouse(MouseButton::Middle)),
        ],
        EventType::MouseAux1Down => vec![Input::Press(Button::Mouse(MouseButton::Aux1))],
        EventType::MouseAux1Up => vec![Input::Release(Button::Mouse(MouseButton::Aux1))],
        EventType::MouseAux1Dclick => vec![
            Input::Press(Button::Mouse(MouseButton::Aux1)),
            Input::Release(Button::Mouse(MouseButton::Aux1)),
        ],
        EventType::MouseAux2Down => vec![Input::Press(Button::Mouse(MouseButton::Aux2))],
        EventType::MouseAux2Up => vec![Input::Release(Button::Mouse(MouseButton::Aux2))],
        EventType::MouseAux2Dclick => vec![
            Input::Press(Button::Mouse(MouseButton::Aux2)),
            Input::Release(Button::Mouse(MouseButton::Aux2)),
        ],
        EventType::MouseMotion => {
            let position = wx_rs::get_mouse_position(event);
            vec![Input::Motion(Motion::Mouse {
                x: position.x as f32,
                y: position.y as f32,
            })]
        }
        EventType::MouseWheel => {
            const ARBITRARY_POINTS_PER_LINE_FACTOR: f32 = 10.0;
            let (x, y) = match wx_rs::get_mouse_wheel_axis(event) {
                WheelAxis::Vertical => (
                    0.0,
                    -(wx_rs::get_mouse_wheel_rotation(event) / wx_rs::get_mouse_wheel_delta(event))
                        as f32
                        * ARBITRARY_POINTS_PER_LINE_FACTOR,
                ),
                WheelAxis::Horizontal => (
                    (wx_rs::get_mouse_wheel_rotation(event) / wx_rs::get_mouse_wheel_delta(event))
                        as f32
                        * ARBITRARY_POINTS_PER_LINE_FACTOR,
                    0.0,
                ),
            };
            let motion = Motion::Scroll { x, y };
            vec![Input::Motion(motion)]
        }
        EventType::MouseLeaveWindow => vec![Input::MouseLeaveWindow],
        EventType::MouseEnterWindow => vec![Input::MouseEnterWindow],
        EventType::Resize | EventType::WindowMove => {
            // Also send resize signal on (MSW only) move event to prevent some tearing
            vec![Input::Resize]
        }
        EventType::KeyDown => {
            let key = Input::Press(Button::Keyboard(event_to_key(event)));
            if let Some(s) = wx_rs::get_event_string(event) {
                vec![key, Input::Text(s)]
            } else {
                vec![key]
            }
        }
        EventType::KeyUp => vec![Input::Release(Button::Keyboard(event_to_key(event)))],
        EventType::Focus => vec![Input::Focus(wx_rs::get_event_focused(event))],
        EventType::Timer => vec![Input::Timer],
        EventType::Exit => vec![Input::Exit],
        EventType::Menu => vec![Input::Menu(wx_rs::get_event_id(event))],
        e => {
            println!("Got a {:?} but didn't handle it", e);
            vec![]
        }
    }
}

pub fn event_to_key(event: *const c_void) -> Key {
    match wx_rs::get_event_key(event) {
        8 => Key::Backspace,
        9 => Key::Tab,
        13 => Key::Return,
        27 => Key::Escape,
        32 => Key::Space,
        33 => Key::Exclaim,
        34 => Key::Quotedbl,
        35 => Key::Hash,
        36 => Key::Dollar,
        37 => Key::Percent,
        38 => Key::Ampersand,
        39 => Key::Quote,
        40 => Key::LeftParen,
        41 => Key::RightParen,
        42 => Key::Asterisk,
        43 => Key::Plus,
        44 => Key::Comma,
        45 => Key::Minus,
        46 => Key::Period,
        47 => Key::Slash,
        48 => Key::D0,
        49 => Key::D1,
        50 => Key::D2,
        51 => Key::D3,
        52 => Key::D4,
        53 => Key::D5,
        54 => Key::D6,
        55 => Key::D7,
        56 => Key::D8,
        57 => Key::D9,
        58 => Key::Colon,
        59 => Key::Semicolon,
        60 => Key::Less,
        61 => Key::Equals,
        62 => Key::Greater,
        63 => Key::Question,
        64 => Key::At,
        65 => Key::A,
        66 => Key::B,
        67 => Key::C,
        68 => Key::D,
        69 => Key::E,
        70 => Key::F,
        71 => Key::G,
        72 => Key::H,
        73 => Key::I,
        74 => Key::J,
        75 => Key::K,
        76 => Key::L,
        77 => Key::M,
        78 => Key::N,
        79 => Key::O,
        80 => Key::P,
        81 => Key::Q,
        82 => Key::R,
        83 => Key::S,
        84 => Key::T,
        85 => Key::U,
        86 => Key::V,
        87 => Key::W,
        88 => Key::X,
        89 => Key::Y,
        90 => Key::Z,
        91 => Key::LeftBracket,
        92 => Key::Backslash,
        93 => Key::RightBracket,
        94 => Key::Caret,
        95 => Key::Underscore,
        96 => Key::Backquote,
        97 => Key::A,
        98 => Key::B,
        99 => Key::C,
        100 => Key::D,
        101 => Key::E,
        102 => Key::F,
        103 => Key::G,
        104 => Key::H,
        105 => Key::I,
        106 => Key::J,
        107 => Key::K,
        108 => Key::L,
        109 => Key::M,
        110 => Key::N,
        111 => Key::O,
        112 => Key::P,
        113 => Key::Q,
        114 => Key::R,
        115 => Key::S,
        116 => Key::T,
        117 => Key::U,
        118 => Key::V,
        119 => Key::W,
        120 => Key::X,
        121 => Key::Y,
        122 => Key::Z,
        123 => Key::LeftBracket,
        124 => Key::Backslash,
        125 => Key::RightBracket,
        126 => Key::Backquote,
        127 => Key::Delete,

        306 => Key::LShift,
        307 => Key::LAlt,
        308 => Key::LCtrl,

        312 => Key::End,
        313 => Key::Home,
        314 => Key::Left,
        315 => Key::Up,
        316 => Key::Right,
        317 => Key::Down,
        322 => Key::Insert,

        324 => Key::NumPad0,
        325 => Key::NumPad1,
        326 => Key::NumPad2,
        327 => Key::NumPad3,
        328 => Key::NumPad4,
        329 => Key::NumPad5,
        330 => Key::NumPad6,
        331 => Key::NumPad7,
        332 => Key::NumPad8,
        333 => Key::NumPad9,

        340 => Key::F1,
        341 => Key::F2,
        342 => Key::F3,
        343 => Key::F4,
        344 => Key::F5,
        345 => Key::F6,
        346 => Key::F7,
        347 => Key::F8,
        348 => Key::F9,
        349 => Key::F10,
        350 => Key::F11,
        351 => Key::F12,

        366 => Key::PageUp,
        367 => Key::PageDown,

        370 => Key::NumPadEnter,
        387 => Key::NumPadMultiply,
        388 => Key::NumPadPlus,
        390 => Key::NumPadMinus,
        391 => Key::NumPadPeriod,
        392 => Key::NumPadDivide,

        _ => Key::Unknown,
    }
}
