use lemna::*;
//-----------------------------------
// MARK: App
//

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaneId {
    Left = 0,
    Right = 1,
}

#[derive(Debug)]
pub struct AppState {
    active_pane: PaneId,
}

#[component(State = "AppState")]
#[derive(Debug, Default)]
pub struct App {}

#[state_component_impl(AppState)]
impl lemna::Component for App {
    fn init(&mut self) {
        self.state = Some(AppState {
            active_pane: PaneId::Left,
        });
    }

    fn view(&self) -> Option<Node> {
        let state = self.state_ref();

        // Main container
        let container = node!(
            widgets::Div::new().bg(Color::new(0.95, 0.95, 0.95, 1.0)),
            lay![
                size_pct: [100.0],
                direction: Column,
                padding: [20.0],
            ]
        );

        // Instructions
        let instructions = node!(
            widgets::Text::new(txt!(
                "Press Ctrl+1 for Left Pane, Ctrl+2 for Right Pane\nType in the textboxes to see focus in action"
            )),
            lay![
                size_pct: [100.0, Auto],
                margin: [0.0, 0.0, 20.0, 0.0],
            ]
        );

        // Panes container (side by side)
        let mut panes_container = node!(
            widgets::Div::new(),
            lay![
                size_pct: [100.0],
                direction: Row,
            ]
        );

        // Left pane
        let left_pane = node!(
            Pane::new("Left Pane", state.active_pane == PaneId::Left,),
            lay![
                size_pct: [50.0, 80.0],
                margin: [0.0, 10.0, 0.0, 0.0],
            ]
        )
        .focus(); // Mark as focus context

        // Right pane
        let right_pane = node!(
            Pane::new("Right Pane", state.active_pane == PaneId::Right,),
            lay![
                size_pct: [50.0, 80.0],
                margin: [0.0, 0.0, 0.0, 10.0],
            ]
        )
        .focus(); // Mark as focus context

        panes_container = panes_container.push(left_pane).push(right_pane);

        let result = container.push(instructions).push(panes_container);

        Some(result)
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        // Global shortcuts for switching panes
        if event.modifiers_held.ctrl {
            match event.input.0 {
                input::Key::D1 => {
                    self.state_mut().active_pane = PaneId::Left;
                }
                input::Key::D2 => {
                    self.state_mut().active_pane = PaneId::Right;
                }
                _ => {}
            }
        }
    }
}

//-----------------------------------
// MARK: Pane Widget
//

#[derive(Debug)]
pub struct Pane {
    title: String,
    is_active: bool,
}

impl Pane {
    pub fn new(title: &str, is_active: bool) -> Self {
        Self {
            title: title.to_string(),
            is_active,
        }
    }
}

impl Component for Pane {
    fn view(&self) -> Option<Node> {
        // Choose colors based on active state
        let (bg_color, border_color, border_width) = if self.is_active {
            (Color::WHITE, Color::new(0.2, 0.5, 0.8, 1.0), 3.0)
        } else {
            (
                Color::new(0.98, 0.98, 0.98, 1.0),
                Color::new(0.7, 0.7, 0.7, 1.0),
                1.0,
            )
        };

        let container = node!(
            widgets::RoundedRect::new(bg_color, 8.0)
                .border_color(border_color)
                .border_width(border_width),
            lay![
                size_pct: [100.0],
                direction: Column,
                padding: [15.0],
            ]
        );

        // Title
        let title = node!(
            widgets::Text::new(txt!(self.title.clone())),
            lay![
                size_pct: [100.0, Auto],
                margin: [0.0, 0.0, 10.0, 0.0],
            ]
        );

        // Status text
        let status = if self.is_active {
            "Active (has focus)"
        } else {
            "Inactive"
        };
        let status_text = node!(
            widgets::Text::new(txt!(status)),
            lay![
                size_pct: [100.0, Auto],
                margin: [0.0, 0.0, 10.0, 0.0],
            ]
        );

        // TextBox
        let textbox = node!(
            widgets::TextBox::new(None),
            lay![
                size_pct: [100.0, Auto],
                padding: [5.0],
            ]
        )
        .focus(); // Mark textbox as focus context

        let result = container.push(title).push(status_text).push(textbox);

        Some(result)
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        println!("{} got {:?}", self.title, event.input);
    }
}

//------------------------------------
// MARK: Main
//
fn main() {
    use simplelog::*;

    println!("Focus Example Starting...");
    let _ = WriteLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Off)
            .add_filter_ignore_str("wgpu")
            .add_filter_ignore_str("naga")
            .build(),
        std::fs::File::create("example-focus.log").unwrap(),
    );

    lemna_baseview::Window::open_blocking::<App>(
        lemna_baseview::WindowOptions::new("Focus Example - Two Panes", (800, 400))
            .resizable(true)
            .fonts(vec![
                ("noto sans regular".to_string(), ttf_noto_sans::REGULAR),
                ("open iconic".to_string(), open_iconic::ICONS),
            ]),
    );
    println!("Focus Example Ended");
}
