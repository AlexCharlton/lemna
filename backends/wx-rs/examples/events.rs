use lemna::{self, open_iconic::Icon, widgets, *};
use lemna_macros::{state_component, state_component_impl};
use ttf_noto_sans;
use wx_rs::{Menu, MenuBar, MenuEntry};

#[derive(Debug)]
pub struct HelloAppState {
    menu_bar: MenuBar<HelloMenu>,
    radio_selection: Vec<usize>,
    toggle_state: bool,
}

unsafe impl Send for HelloAppState {}
unsafe impl Sync for HelloAppState {}

#[state_component(HelloAppState)]
#[derive(Debug, Default)]
pub struct HelloApp {}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum HelloEvent {
    Button {
        name: String,
    },
    Selection {
        name: String,
        value: String,
    },
    TextBox {
        name: String,
        value: String,
        update_type: String,
    },
    Menu {
        selection: HelloMenu,
    },
    RadioSelect {
        selection: Vec<usize>,
    },
    Toggle(bool),
}

#[derive(Debug, Clone, Copy)]
enum HelloMenu {
    Open,
    Close,
    Copy,
    Cut,
    Paste,
}

#[state_component_impl(HelloAppState)]
impl lemna::Component for HelloApp {
    fn init(&mut self) {
        let mut menu_bar = MenuBar::new();
        menu_bar.append(
            Menu::new(Some("&File".to_string()))
                .push_entry(MenuEntry::new(HelloMenu::Open, "&Open".to_string()))
                .push_entry(MenuEntry::new(HelloMenu::Close, "&Close".to_string())),
        );
        menu_bar.append(
            Menu::new(Some("&Edit".to_string()))
                .push_entry(MenuEntry::new(HelloMenu::Copy, "&Copy".to_string()))
                .push_entry(MenuEntry::new(HelloMenu::Cut, "&Cut".to_string()))
                .push_entry(MenuEntry::new(HelloMenu::Paste, "&Paste".to_string())),
        );
        menu_bar.show();
        self.state = Some(HelloAppState {
            menu_bar,
            radio_selection: vec![],
            toggle_state: false,
        })
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use crate::render::renderables::Rect;

        Some(vec![Renderable::Rect(Rect::new(
            Pos::default(),
            context.aabb.size(),
            Color::GREEN,
        ))])
    }

    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new(),
                lay!(wrap: true, size: size_pct!(100.0), cross_alignment: Alignment::End)
            )
            .push(node!(
                EventReactor {
                    name: "SomeWidget".to_string(),
                },
                lay!(size: size!(100.0)),
                0
            ))
            .push(node!(Sorter {}, lay!(size: size!(100.0, 200.0)), 1))
            .push(node!(
                widgets::Button::new(txt!("Click me!"), widgets::ButtonStyle::default()).on_click(
                    Box::new(|| msg!(HelloEvent::Button {
                        name: "It me, a button!".to_string()
                    }))
                ),
                lay!(size: size!(100.0, 50.0)),
                2
            ))
            .push(node!(
                widgets::Button::new(
                    txt!(
                        "Click me too! ",
                        (Icon::Check, "open iconic"),
                        (" Yeah!", None, 9.0)
                    ),
                    widgets::ButtonStyle::default()
                )
                .tool_tip("Wait, don't!\nWhy not? Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string())
                .on_click(Box::new(|| msg!(HelloEvent::Button {
                    name: "jk, I'm just another button!".to_string()
                }))),
                lay!(size: size!(Auto)),
                3
            ))
            .push(node!(
                widgets::Select::<String>::new(
                    vec![
                        "Selection 1".to_string(),
                        "Sel 2".to_string(),
                        "3".to_string()
                    ],
                    1,
                    widgets::SelectStyle::default()
                )
                .on_change(Box::new(|_, s| msg!(HelloEvent::Selection {
                    name: "My selection".to_string(),
                    value: s.clone(),
                }))),
                lay!(size: size!(Auto), margin: rect!(Auto, Auto, 50.0)),
                4
            ))
            .push(node!(
                widgets::TextBox::new(Some("Hello".to_string()), widgets::TextBoxStyle::default())
                    .on_change(Box::new(|s| msg!(HelloEvent::TextBox {
                        name: "My text box".to_string(),
                        value: s.to_string(),
                        update_type: "change".to_string(),
                    })))
                    .on_commit(Box::new(|s| msg!(HelloEvent::TextBox {
                        name: "My text box".to_string(),
                        value: s.to_string(),
                        update_type: "commit".to_string(),
                    }))),
                lay!(size: size!(100.0, Auto)),
                5
            ))
            .push(node!(
                widgets::RadioButtons::new(
                    vec![txt!(Icon::Bell), txt!(Icon::Book), txt!(Icon::Bolt)],
                    self.state_ref().radio_selection.clone(),
                    widgets::ButtonStyle {
                        font: Some("open iconic".to_string()),
                        font_size: 10.0,
                        ..Default::default()
                    }
                )
                .tool_tips(vec![
                    "Bell".to_string(),
                    "Book".to_string(),
                    "Bolt".to_string(),
                ])
                .nullable(true)
                //.multi_select(true)
                .max_columns(2)
                .on_change(Box::new(|s| msg!(HelloEvent::RadioSelect { selection: s }))),
                lay!(margin: rect!(10.0)),
                6
            ))
            .push(node!(
                widgets::Toggle::new(
                    self.state_ref().toggle_state,
                    widgets::ToggleStyle::default()
                )
                .on_change(Box::new(|s| msg!(HelloEvent::Toggle(s)))),
                lay!(margin: rect!(10.0)),
                7
            )),
        )
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        println!("App was sent: {:?}", message.downcast_ref::<HelloEvent>());
        match message.downcast_ref::<HelloEvent>() {
            Some(HelloEvent::RadioSelect { selection: s }) => {
                self.state_mut().radio_selection = s.clone()
            }
            Some(HelloEvent::Toggle(s)) => self.state_mut().toggle_state = *s,
            _ => (),
        }
        vec![]
    }

    fn on_key_press(&mut self, event: &mut Event<event::KeyPress>) {
        println!(
            "The app got a key: {:?} (Modifiers: {:?})",
            event.input.0, event.modifiers_held
        );
    }

    fn on_menu_select(&mut self, event: &mut Event<event::MenuSelect>) {
        if let Some(menu) = self
            .state_ref()
            .menu_bar
            .get_entry_from_event_id(event.input.0)
        {
            event.dirty();
            event.emit(match menu {
                HelloMenu::Open => msg!(HelloEvent::Menu { selection: menu }),
                HelloMenu::Close => msg!(HelloEvent::Menu { selection: menu }),
                HelloMenu::Copy => msg!(widgets::TextBoxAction::Copy),
                HelloMenu::Cut => msg!(widgets::TextBoxAction::Cut),
                HelloMenu::Paste => msg!(widgets::TextBoxAction::Paste),
            })
        }
    }
}

#[derive(Debug)]
pub struct Sorter {}

impl Component for Sorter {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new().bg([0.8, 0.8, 0.8]),
                lay!(
                    size: size!(100.0, 200.0),
                    direction: Direction::Column,
                    padding: rect!(10.0),
                    axis_alignment: Alignment::Stretch,
                    cross_alignment: Alignment::Stretch,
                )
            )
            .push(node!(
                widgets::Div::new().bg([1.0, 0.0, 0.0]),
                lay!(margin: rect!(5.0)),
                0
            ))
            .push(node!(
                widgets::Div::new().bg([1.0, 0.5, 0.0]),
                lay!(margin: rect!(5.0)),
                1
            ))
            .push(node!(
                widgets::Div::new().bg([1.0, 1.0, 0.0]),
                lay!(margin: rect!(5.0)),
                2
            ))
            .push(node!(
                widgets::Div::new().bg([0.0, 1.0, 0.0]),
                lay!(margin: rect!(5.0)),
                3
            ))
            .push(node!(
                widgets::Div::new().bg([0.0, 0.0, 1.0]),
                lay!(margin: rect!(5.0)),
                4
            )),
        )
    }

    fn on_drag_start(&mut self, event: &mut Event<event::DragStart>) {
        println!("Drag start. Got child {:?}", event.over_subchild_n(),);
        event.stop_bubbling();
    }

    fn on_drag(&mut self, event: &mut Event<event::Drag>) {
        println!("Dragging {:?}", event.relative_logical_position());
    }

    fn on_drag_end(&mut self, event: &mut Event<event::DragEnd>) {
        println!("Drag stop at {:?}", event.relative_logical_position());
    }

    fn on_mouse_motion(&mut self, event: &mut Event<event::MouseMotion>) {
        event.stop_bubbling();
    }
}

#[derive(Debug)]
pub struct EventReactor {
    pub name: String,
}

impl Component for EventReactor {
    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        Some(vec![Renderable::Rect(
            lemna::render::renderables::Rect::new(Pos::default(), context.aabb.size(), Color::BLUE),
        )])
    }

    fn on_mouse_motion(&mut self, event: &mut Event<event::MouseMotion>) {
        println!(
            "Hovering over {} ({:?})",
            &self.name,
            event.logical_mouse_position()
        );
        event.stop_bubbling();
    }

    fn on_click(&mut self, event: &mut Event<event::Click>) {
        println!("Clicked on {} with {:?}", &self.name, event.input.0);
        match event.input.0 {
            input::MouseButton::Left => println!(
                "Got {:?} from the clipboard",
                lemna::current_window().map(|w| w.get_from_clipboard())
            ),
            input::MouseButton::Right => {
                println!("Put `Hello Events!` on the clipboard");
                lemna::current_window().map(|w| w.put_on_clipboard(&"Hello Events!".into()));
            }
            _ => (),
        };
        event.focus();
    }

    fn on_double_click(&mut self, event: &mut Event<event::DoubleClick>) {
        println!("Double clicked on {} with {:?}", &self.name, event.input.0);
    }

    fn on_mouse_enter(&mut self, _event: &mut Event<event::MouseEnter>) {
        println!("Entered {}", &self.name);
    }

    fn on_mouse_leave(&mut self, _event: &mut Event<event::MouseLeave>) {
        println!("Left {}", &self.name);
    }

    fn on_text_entry(&mut self, event: &mut Event<event::TextEntry>) {
        println!("{} got a some text: {:?})", &self.name, event.input.0);
    }
}

// App setup
fn main() {
    println!("hello");
    lemna_wx_rs::Window::<lemna::render::wgpu::WGPURenderer, HelloApp>::open_blocking(
        "Hello events!",
        800,
        600,
        vec![
            ("noto sans regular".to_string(), ttf_noto_sans::REGULAR),
            ("open iconic".to_string(), open_iconic::ICONS),
        ],
    );
    println!("bye");
}
