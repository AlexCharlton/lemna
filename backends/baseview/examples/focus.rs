use lemna::*;
//-----------------------------------
// MARK: App
//

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn view(&self) -> Option<Node> {
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
            widgets::Text::new(txt!("Press Ctrl+1 for Left Pane, Ctrl+2 for Right Pane")),
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
            Pane::new("Left Pane"),
            lay![
                size_pct: [50.0, 80.0],
                margin: [0.0, 10.0, 0.0, 0.0],
            ]
        )
        .reference("left_pane")
        .focus() // Mark as focus context
        .focus_priority(1); // Higher priority than the right pane

        // Right pane
        let right_pane = node!(
            Pane::new("Right Pane"),
            lay![
                size_pct: [50.0, 80.0],
                margin: [0.0, 0.0, 0.0, 10.0],
            ]
        )
        .reference("right_pane")
        .focus(); // Mark as focus context

        panes_container = panes_container.push(left_pane).push(right_pane);

        let result = container.push(instructions).push(panes_container);

        Some(result)
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        println!("App on_key_down: {:?}", event.input.0);

        // Global shortcuts for switching panes
        if event.modifiers_held.ctrl {
            match event.input.0 {
                input::Key::D1 => event.focus_ref("left_pane"),
                input::Key::D2 => event.focus_ref("right_pane"),
                _ => {}
            }
        }
    }
}

//-----------------------------------
// MARK: Pane Widget
//

#[derive(Debug)]
struct PaneState {
    focused: bool,
}

#[component(State = "PaneState")]
#[derive(Debug)]
pub struct Pane {
    title: String,
}

impl Pane {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            state: None,
            dirty: lemna::Dirty::No,
        }
    }
}

#[state_component_impl(PaneState)]
impl Component for Pane {
    fn init(&mut self) {
        self.state = Some(PaneState { focused: false });
    }

    fn view(&self) -> Option<Node> {
        // Choose colors based on active state
        let (bg_color, border_color, border_width) = if self.state_ref().focused {
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
        let status = if self.state_ref().focused {
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
        .reference(format!("{}::Textbox", self.title));

        let result = container.push(title).push(status_text).push(textbox);

        Some(result)
    }

    fn on_key_down(&mut self, event: &mut Event<event::KeyDown>) {
        println!("{} got {:?}", self.title, event.input);
    }

    fn on_focus(&mut self, event: &mut event::Event<event::Focus>) {
        // When (directly) focusing a new pane, this will fire three times:
        // 1. When the pane is focused directly
        // 2. When the TextBox is focused and the focus bubbles
        // 3. When the TextBoxText is focused and the focus bubbles
        println!("{} got focus", self.title);
        event.focus_ref(format!("{}::Textbox", self.title));
        self.state_mut().focused = true;
    }

    fn on_blur(&mut self, _event: &mut event::Event<event::Blur>) {
        self.state_mut().focused = false;
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
            .fonts(vec![(
                "noto sans regular".to_string(),
                ttf_noto_sans::REGULAR,
            )]),
    );
    println!("Focus Example Ended");
}
