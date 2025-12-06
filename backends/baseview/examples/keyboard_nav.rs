use lemna::input::Key;
use lemna::*;

//-----------------------------------
// MARK: App
//
pub enum AppEvent {
    ItemClick(String),
}

#[derive(Debug, Default)]
pub struct AppState {
    modal_open: bool,
    items: Vec<String>,
    // items.len() means the new button is focused
    item_focused: usize,
}

#[component(State = "AppState")]
#[derive(Debug, Default)]
pub struct App {}

#[state_component_impl(AppState)]
impl lemna::Component for App {
    fn init(&mut self) {
        self.state = Some(AppState {
            modal_open: false,
            items: vec![
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
                "Item 4".to_string(),
                "Item 5".to_string(),
                "Item 6".to_string(),
                "Item 7".to_string(),
            ],
            item_focused: 0,
        })
    }

    fn view(&self) -> Option<Node> {
        let mut main = node!(
            widgets::Div::new().scroll_y(),
            lay![
                size_pct: [100.0],
                direction: Column,
                cross_alignment: Center,
            ]
        );
        for (id, item) in self.state_ref().items.iter().enumerate() {
            let cloned_item = item.clone();
            main = main.push(node!(widgets::Button::new(txt!(item.clone()))
                    .on_click(Box::new(move || msg!(AppEvent::ItemClick(cloned_item.clone())))).style("padding", 4.0), [
                    size_pct: [80.0, Auto],
                    padding: [10.0],
                ], id as u64));
        }
        main = main.push(node!(
            widgets::Button::new(txt!("New Item"))
                .style("padding", 4.0)
                .on_click(Box::new(|| msg!(ModalEvent::OpenModal))),
            [],
        ));

        // Add modal overlay if open
        let mut result = main;
        if self.state_ref().modal_open {
            result = node!(widgets::Div::new(), lay![size_pct: [100.0]])
                .push(result)
                .push(node!(
                    NewNamedModal::new(
                        "",
                        "Item",
                    ),
                    [position: [0.0], position_type: Absolute, size_pct: [100.0], z_index: 1000.0]
                ).focus());
        }

        Some(result)
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        match message.downcast_ref::<ModalEvent>() {
            Some(event) => match event {
                ModalEvent::OpenModal => {
                    self.state_mut().modal_open = true;
                }
                ModalEvent::CloseModal => {
                    self.state_mut().modal_open = false;
                }
                ModalEvent::Submit(name) => {
                    if !name.trim().is_empty() {
                        self.state_mut().modal_open = false;
                        if self.state_ref().item_focused >= self.state_ref().items.len() {
                            self.state_mut().item_focused += 1;
                        }
                        self.state_mut().items.push(name.clone());
                        return vec![];
                    }
                }
            },
            _ => return vec![message],
        }
        vec![]
    }

    fn on_focus(&mut self, event: &mut Event<event::Focus>) {
        if !self.state_ref().modal_open {
            let focused = self.state_ref().item_focused;
            event.focus_child(vec![0, focused]);
        }
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        match event.input.key {
            Key::Up => {
                let focused = self.state_ref().item_focused;
                if focused > 0 {
                    let focused = focused - 1;
                    self.state_mut().item_focused = focused;
                    event.focus_child(vec![0, focused]);
                } else {
                    event.focus_child(vec![0, focused]);
                }
            }
            Key::Down => {
                let focused = self.state_ref().item_focused;
                if focused < self.state_ref().items.len() {
                    let focused = focused + 1;
                    self.state_mut().item_focused = focused;
                    event.focus_child(vec![0, focused]);
                } else {
                    event.focus_child(vec![0, focused]);
                }
            }
            _ => {}
        }
    }
}

//------------------------------------
// Main
fn main() {
    use simplelog::*;

    println!("hello");
    let _ = WriteLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Off)
            .add_filter_ignore_str("wgpu")
            .add_filter_ignore_str("naga")
            .build(),
        std::fs::File::create("example-keyboard-nav.log").unwrap(),
    );

    lemna_baseview::Window::open_blocking::<App>(
        lemna_baseview::WindowOptions::new("Hello Keyboard Navigation", (400, 300))
            .resizable(false)
            .fonts(vec![
                ("noto sans regular".to_string(), ttf_noto_sans::REGULAR),
                ("open iconic".to_string(), open_iconic::ICONS),
            ]),
    );
    println!("bye");
}

//-----------------------------------
// Modal
#[derive(Debug)]
pub enum ModalEvent {
    OpenModal,
    CloseModal,
    Submit(String),
}

enum ModalMessage {
    UpdateName(String),
    Submit,
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum ModalFocus {
    Input,
    Submit,
    Cancel,
}

#[derive(Debug)]
struct NewNamedModalState {
    name: String,
    focused: ModalFocus,
}

#[component(State = "NewNamedModalState")]
#[derive(Debug)]
pub struct NewNamedModal {
    name: String,
    target_name: String,
}

impl NewNamedModal {
    pub fn new(name: &str, target_name: &str) -> Self {
        Self {
            name: name.to_string(),
            target_name: target_name.to_string(),
            state: None,
            dirty: lemna::Dirty::No,
        }
    }
}

#[state_component_impl(NewNamedModalState)]
impl Component for NewNamedModal {
    fn init(&mut self) {
        self.state = Some(NewNamedModalState {
            name: self.name.clone(),
            focused: ModalFocus::Input,
        });
    }

    fn view(&self) -> Option<Node> {
        // Create a semi-transparent overlay background
        let overlay = node!(
            widgets::Div::new().bg(Color::new(0.0, 0.0, 0.0, 0.5)),
            [
                size_pct: [100.0],
                direction: Column,
                cross_alignment: Center,
                axis_alignment: Center,
            ]
        );

        // Create the modal content box
        let modal_content = node!(
            widgets::RoundedRect::new(Color::WHITE, 8.0).border_width(2.0),
            [
                size: [300.0, Auto],
                direction: Column,
                cross_alignment: Center,
                padding: [10.0],
                margin: [20.0],
            ]
        )
        .push(node!(widgets::Text::new(txt!(format!(
            "New {}",
            self.target_name
        ))),))
        .push(node!(
            widgets::TextBox::new(Some(self.name.clone()))
                .on_change(Box::new(|text: &str| msg!(ModalMessage::UpdateName(text.to_string())))),
            [size_pct: [90.0, Auto], padding: [5.0]]
        ).focus_when_new().reference("input"))
        .push(
            node!(
                widgets::Div::new(),
                [size_pct: [100.0, Auto], axis_alignment: End, padding: [10.0, 0.0, 4.0]]
            )
            .push(node!(
                widgets::Button::new(txt!("Cancel"))
                    .on_click(Box::new(|| msg!(ModalEvent::CloseModal))),
                [margin: [0.0, 5.0]]
            ).reference("cancel_button"))
            .push(node!(
                widgets::Button::new(txt!("Create"))
                    .on_click(Box::new(|| msg!(ModalMessage::Submit))),
                [margin: [0.0, 5.0]]
            ).reference("submit_button")),
        );

        Some(overlay.push(modal_content))
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        match message.downcast_ref::<ModalMessage>() {
            Some(m) => match m {
                ModalMessage::UpdateName(s) => {
                    self.state_mut().name = s.clone();
                    vec![]
                }
                ModalMessage::Submit => {
                    vec![msg!(ModalEvent::Submit(self.state_ref().name.clone()))]
                }
            },
            None => vec![message],
        }
    }

    fn on_focus(&mut self, event: &mut Event<event::Focus>) {
        // If the input is focused and the event is primary target, this means the textbox or button has been blurred
        if event.is_primary_target() {
            if self.state_ref().focused == ModalFocus::Input && !self.state_ref().name.is_empty() {
                event.focus_ref("submit_button");
                self.state_mut().focused = ModalFocus::Submit;
            } else {
                event.emit(msg!(ModalEvent::CloseModal));
            }
        }
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        let focused = self.state_ref().focused;
        match event.input.key {
            input::Key::Escape => {
                event.emit(msg!(ModalEvent::CloseModal));
            }
            input::Key::Tab => {
                if focused == ModalFocus::Input {
                    if event.modifiers_held.shift {
                        event.focus_ref("cancel_button");
                        self.state_mut().focused = ModalFocus::Cancel;
                    } else {
                        event.focus_ref("submit_button");
                        self.state_mut().focused = ModalFocus::Submit;
                    }
                } else if focused == ModalFocus::Submit {
                    if event.modifiers_held.shift {
                        event.focus_ref("input");
                        self.state_mut().focused = ModalFocus::Input;
                    } else {
                        event.focus_ref("cancel_button");
                        self.state_mut().focused = ModalFocus::Cancel;
                    }
                } else if focused == ModalFocus::Cancel {
                    if event.modifiers_held.shift {
                        event.focus_ref("submit_button");
                        self.state_mut().focused = ModalFocus::Submit;
                    } else {
                        event.focus_ref("input");
                        self.state_mut().focused = ModalFocus::Input;
                    }
                }
            }
            _ => (),
        }
    }

    fn on_mouse_motion(&mut self, event: &mut Event<event::MouseMotion>) {
        event.stop_bubbling();
    }

    fn on_click(&mut self, event: &mut Event<event::Click>) {
        event.stop_bubbling();
    }

    fn on_double_click(&mut self, event: &mut Event<event::DoubleClick>) {
        event.stop_bubbling();
    }

    fn on_mouse_down(&mut self, event: &mut Event<event::MouseDown>) {
        event.stop_bubbling();
    }

    fn on_mouse_up(&mut self, event: &mut Event<event::MouseUp>) {
        event.stop_bubbling();
    }

    fn on_mouse_leave(&mut self, event: &mut Event<event::MouseLeave>) {
        event.stop_bubbling();
    }

    fn on_mouse_enter(&mut self, event: &mut Event<event::MouseEnter>) {
        event.stop_bubbling();
    }
}
