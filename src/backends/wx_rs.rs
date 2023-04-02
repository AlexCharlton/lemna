use std::mem;
use std::os::raw::c_void;

use crate::input::{Button, Input, Key, Motion, MouseButton};
use crate::PixelSize;
use wx_rs::*;

impl crate::window::Window for Window {
    fn client_size(&self) -> PixelSize {
        unsafe { mem::transmute(get_client_size()) }
    }

    fn physical_size(&self) -> PixelSize {
        unsafe { mem::transmute(get_display_size()) }
    }

    fn scale_factor(&self) -> f32 {
        get_scale_factor()
    }

    fn put_on_clipboard(&self, data: &crate::window::Data) {
        unsafe { put_on_clipboard(mem::transmute(data)) }
    }

    fn get_from_clipboard(&self) -> Option<crate::window::Data> {
        unsafe { mem::transmute(get_from_clipboard()) }
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
        set_cursor(ct);
    }

    fn unset_cursor(&self) {
        set_cursor(CursorType::Arrow);
    }
}

pub fn event_to_input(event: *const c_void) -> Vec<Input> {
    match get_event_type(event) {
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
            let position = get_mouse_position(event);
            vec![Input::Motion(Motion::Mouse {
                x: position.x as f32,
                y: position.y as f32,
            })]
        }
        EventType::MouseWheel => {
            const ARBITRARY_POINTS_PER_LINE_FACTOR: f32 = 10.0;
            let (x, y) = match get_mouse_wheel_axis(event) {
                WheelAxis::Vertical => (
                    0.0,
                    -(get_mouse_wheel_rotation(event) / get_mouse_wheel_delta(event)) as f32
                        * ARBITRARY_POINTS_PER_LINE_FACTOR,
                ),
                WheelAxis::Horizontal => (
                    (get_mouse_wheel_rotation(event) / get_mouse_wheel_delta(event)) as f32
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
            if let Some(s) = get_event_string(event) {
                vec![key, Input::Text(s)]
            } else {
                vec![key]
            }
        }
        EventType::KeyUp => vec![Input::Release(Button::Keyboard(event_to_key(event)))],
        EventType::Focus => vec![Input::Focus(get_event_focused(event))],
        EventType::Timer => vec![Input::Timer],
        EventType::Exit => vec![Input::Exit],
        EventType::Menu => vec![Input::Menu(get_event_id(event))],
        e => {
            println!("Got a {:?} but didn't handle it", e);
            vec![]
        }
    }
}

pub fn event_to_key(event: *const c_void) -> Key {
    match get_event_key(event) {
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
