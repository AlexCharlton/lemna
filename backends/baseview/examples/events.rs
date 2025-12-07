use lemna::renderable::{Rectangle, Renderable};
use lemna::*;

#[derive(Debug)]
pub struct AppState {
    radio_selection: Vec<usize>,
    toggle_state: bool,
}

#[component(State = "AppState")]
#[derive(Debug, Default)]
pub struct App {}

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
    RadioSelect {
        selection: Vec<usize>,
    },
    Toggle(bool),
    #[cfg(feature = "file_dialogs")]
    FileSelect {
        selection: Option<std::path::PathBuf>,
    },
}

#[state_component_impl(AppState)]
impl lemna::Component for App {
    fn init(&mut self) {
        self.state = Some(AppState {
            radio_selection: vec![],
            toggle_state: false,
        })
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        Some(vec![Renderable::Rectangle(Rectangle::new(
            Pos::default(),
            context.aabb.size(),
            [0.5, 0.7, 0.7].into(),
        ))])
    }

    fn view(&self) -> Option<Node> {
        #[allow(unused_mut)] // Feature-conditional
        let mut node =
            node!(
                widgets::Div::new(),
                [wrap: true, size_pct: [100], cross_alignment: End]
            )
            .push(node!(
                EventReactor {
                    name: "SomeWidget".to_string(),
                },
                [size: [100]]
            ).focus())
            .push(node!(Sorter {}, [size: [100, 200]]))
            .push(node!(
                widgets::Button::new(txt!("Click me!")).on_click(
                    Box::new(|| msg!(HelloEvent::Button {
                        name: "It me, a button!".to_string()
                    }))
                ),
                [size: [100, 50]]
            ))
            .push(node!(
                widgets::Button::new(
                    txt!(
                        "Click me too! ",
                        (Icon::Check, "open iconic", 10.0),
                        (" Yeah!", None, 8.0)
                    ),
                )
                .tool_tip("Wait, don't!\nWhy not? Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string())
                .on_click(Box::new(|| msg!(HelloEvent::Button {
                    name: "jk, I'm just another button!".to_string()
                }))),
                [size: [Auto]]
            ))
            .push(node!(
                widgets::Select::<String>::new(
                    vec![
                        "Selection 1".to_string(),
                        "Sel 2".to_string(),
                        "3".to_string()
                    ],
                    1,
                )
                .on_change(Box::new(|_, s| msg!(HelloEvent::Selection {
                    name: "My selection".to_string(),
                    value: s.clone(),
                })))
            ))
            .push(node!(
                widgets::TextBox::new(Some("Hello".to_string()))
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
                [size: [100, Auto]]
            ))
            .push(node!(
                widgets::RadioButtons::new(
                    vec![txt!(Icon::Bell), txt!(Icon::Book), txt!(Icon::Bolt)],
                    self.state_ref().radio_selection.clone(),
                )
                    .style("font_size", 20.0)
                    .style("font", "open iconic")
                .tool_tips(vec![
                    "Bell".to_string(),
                    "Book".to_string(),
                    "Bolt".to_string(),
                ])
                .nullable(true)
                //.multi_select(true)
                .max_columns(2)
                .on_change(Box::new(|s| msg!(HelloEvent::RadioSelect { selection: s }))),
                [margin: [10]]
            ))
            .push(node!(
                widgets::Toggle::new(
                    self.state_ref().toggle_state,
                )
                .on_change(Box::new(|s| msg!(HelloEvent::Toggle(s)))),
                [margin: [10]]
            ));
        #[cfg(feature = "file_dialogs")]
        {
            node = node.push(node!(
            widgets::FileSelector::new("Choose a file".to_string())
                    .on_select(Box::new(|f| msg!(HelloEvent::FileSelect { selection: f.clone() }))),
                [size: [Auto], margin: [Auto, Auto, 50]]
            ));
        }
        Some(node)
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
            "The app got a key press: {:?} (Modifiers: {:?})",
            event.input.key, event.modifiers_held
        );
    }
}

#[derive(Debug)]
pub struct Sorter {}

impl Component for Sorter {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new().bg([0.8, 0.8, 0.8]),
                [
                    size: [100, 200],
                    direction: Column,
                    padding: [10],
                    axis_alignment: Stretch,
                    cross_alignment: Stretch,
                ]
            )
            .push(node!(
                widgets::Div::new().bg([1.0, 0.0, 0.0]),
                [margin: [5]]
            ))
            .push(node!(
                widgets::Div::new().bg([1.0, 0.5, 0.0]),
                [margin: [5]]
            ))
            .push(node!(
                widgets::Div::new().bg([1.0, 1.0, 0.0]),
                [margin: [5]]
            ))
            .push(node!(
                widgets::Div::new().bg([0.0, 1.0, 0.0]),
                [margin: [5]]
            ))
            .push(node!(
                widgets::Div::new().bg([0.0, 0.0, 1.0]),
                [margin: [5]]
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
    fn view(&self) -> Option<Node> {
        Some(node!(widgets::Text::new(txt!(
            "Try pressing some keys..."
        )).style("h_alignment", style::HorizontalPosition::Center)
          .style("color", Color::WHITE),
         [
             size_pct: [100],
             margin: [10],
             padding: [5],
             cross_alignment: Center,
             axis_alignment: Center,
         ]))
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        Some(vec![Renderable::Rectangle(Rectangle::new(
            Pos::default(),
            context.aabb.size(),
            Color::BLUE,
        ))])
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
        println!("Clicked on {} with {:?}", &self.name, event.input.button);
        match event.input.button {
            input::MouseButton::Left => {
                println!("Got {:?} from the clipboard", window::get_from_clipboard())
            }
            input::MouseButton::Right => {
                println!("Put `Hello Events!` on the clipboard");
                window::put_on_clipboard(&"Hello Events!".into());
            }
            _ => (),
        };
        event.focus();
    }

    fn on_double_click(&mut self, event: &mut Event<event::DoubleClick>) {
        println!(
            "Double clicked on {} with {:?}",
            &self.name, event.input.button
        );
    }

    fn on_mouse_enter(&mut self, _event: &mut Event<event::MouseEnter>) {
        println!("Entered {}", &self.name);
    }

    fn on_mouse_leave(&mut self, _event: &mut Event<event::MouseLeave>) {
        println!("Left {}", &self.name);
    }

    fn on_text_entry(&mut self, event: &mut Event<event::TextEntry>) {
        println!("{} got a some text: {:?})", &self.name, event.input.text);
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        println!("The EventReactor got a key down: {:?}", event.input.key);
    }
}

// App setup
fn main() {
    use simplelog::*;

    println!("hello");
    let _ = WriteLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new().build(),
        std::fs::File::create("example-events.log").unwrap(),
    );

    lemna_baseview::Window::open_blocking::<App>(
        lemna_baseview::WindowOptions::new("Hello Events", (800, 600))
            .resizable(false)
            .fonts(vec![
                ("noto sans regular".to_string(), ttf_noto_sans::REGULAR),
                ("open iconic".to_string(), open_iconic::ICONS),
            ]),
    );
    println!("bye");
}
