use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use crate::component::{Component, Message};
use crate::event;
use crate::{Node, Styled, txt};
use lemna_macros::{component, state_component_impl};

#[derive(Debug, Default)]
struct FileSelectorState {
    /// Wrapped in `Mutex` so `FileSelector` remains `Sync` (`Receiver` is not `Sync`).
    pending: Option<Mutex<Receiver<Option<PathBuf>>>>,
}

#[derive(Debug)]
enum FileSelectorAction {
    Open,
}

struct DialogParams {
    title: String,
    default_path: Option<PathBuf>,
    filter: Option<(Vec<String>, String)>,
}

#[component(State = "FileSelectorState", Styled, Internal)]
pub struct FileSelector {
    pub title: String,
    pub default_path: Option<PathBuf>,
    /// Set of filters e.g. `["*.png", "*.jpg"]` plus a description e.g. "Image files"
    pub filter: Option<(Vec<String>, String)>,
    pub on_select: Option<Box<dyn Fn(Option<PathBuf>) -> Message + Send + Sync>>,
}

impl core::fmt::Debug for FileSelector {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("FileSelector")
            .field("title", &self.title)
            .field("default_path", &self.default_path)
            .finish()
    }
}

impl FileSelector {
    pub fn new(title: String) -> Self {
        Self {
            title,
            default_path: None,
            filter: None,
            on_select: None,
            state: Some(FileSelectorState::default()),
            dirty: crate::Dirty::No,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }

    pub fn on_select(mut self, f: Box<dyn Fn(Option<PathBuf>) -> Message + Send + Sync>) -> Self {
        self.on_select = Some(f);
        self
    }

    pub fn default_path(mut self, path: PathBuf) -> Self {
        self.default_path = Some(path);
        self
    }

    /// Set of filters e.g. `["*.png", "*.jpg"]` plus a description e.g. "Image files"
    pub fn filter(mut self, filters: Vec<String>, description: String) -> Self {
        self.filter = Some((filters, description));
        self
    }

    fn start_dialog(&mut self) {
        if self.state_ref().pending.is_some() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        let params = DialogParams {
            title: self.title.clone(),
            default_path: self.default_path.clone(),
            filter: self.filter.clone(),
        };

        self.state_mut().pending = Some(Mutex::new(rx));
        std::thread::spawn(move || {
            let _ = tx.send(run_dialog(params));
        });
    }
}

fn run_dialog(params: DialogParams) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(&params.title);

    if let Some(path) = &params.default_path {
        if path.is_dir() {
            dialog = dialog.set_directory(path);
        } else {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    dialog = dialog.set_directory(parent);
                }
            }
            if let Some(name) = path.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy());
            }
        }
    }

    if let Some((filters, description)) = &params.filter {
        let extensions: Vec<String> = filters
            .iter()
            .map(|f| {
                f.strip_prefix("*.")
                    .or_else(|| f.strip_prefix('.'))
                    .map(str::to_string)
                    .unwrap_or_else(|| f.clone())
            })
            .collect();
        let ext_refs: Vec<&str> = extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(description, &ext_refs);
    }

    dialog.pick_file()
}

#[state_component_impl(FileSelectorState, Internal)]
impl Component for FileSelector {
    fn update(&mut self, msg: Message) -> Vec<Message> {
        if msg.downcast_ref::<FileSelectorAction>().is_some() {
            self.start_dialog();
            vec![]
        } else {
            vec![msg]
        }
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) {
        let recv_result = {
            let Some(rx) = self.state_ref().pending.as_ref() else {
                return;
            };
            rx.lock().unwrap().try_recv()
        };

        match recv_result {
            Ok(path) => {
                self.state_mut().pending = None;
                if let Some(f) = &self.on_select {
                    event.emit(f(path));
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.state_mut().pending = None;
            }
        }
    }

    fn view(&self) -> Option<Node> {
        let mut b = super::Button::new(txt!("...")); // TODO Style override
        *b.style_overrides_mut() = self.style_overrides.clone();
        if let Some(class) = self.class {
            b = b.with_class(class);
        }
        b = b.on_click(Box::new(|| msg!(FileSelectorAction::Open)));

        Some(node!(b, lay!(size: size_pct!(100.0))))
    }
}
