use std::path::PathBuf;

use crate::component::{Component, Message};
use crate::render::wgpu::WGPURenderer;
use crate::{node, txt, ButtonStyle, Node};

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct FileSelectorStyle {
    pub button_style: ButtonStyle,
}



pub struct FileSelector {
    pub title: String,
    pub default_path: Option<PathBuf>,
    /// Set of filters e.g. `["*.png", "*.jpg"]` plus a description e.g. "Image files"
    pub filter: Option<(Vec<String>, String)>,
    pub style: FileSelectorStyle,
    pub on_select: Option<Box<dyn Fn(Option<PathBuf>) -> Message + Send + Sync>>,
}

impl std::fmt::Debug for FileSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("FileSelector")
            .field("style", &self.style)
            .finish()
    }
}

impl FileSelector {
    pub fn new(title: String, style: FileSelectorStyle) -> Self {
        Self {
            title,
            default_path: None,
            filter: None,
            style,
            on_select: None,
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

    fn select(&self) -> Option<PathBuf> {
        let path = self
            .default_path
            .as_ref()
            .map(|p| p.to_str().expect("Expected path to be a unicode string"))
            .unwrap_or("");
        let filters: Option<Vec<&str>> = self
            .filter
            .as_ref()
            .map(|(filters, _)| filters.iter().map(|x| x.as_str()).collect());

        let f = tinyfiledialogs::open_file_dialog(
            &self.title,
            path,
            self.filter
                .as_ref()
                .map(|(_, description)| (&filters.as_ref().unwrap()[..], description.as_str())),
        );
        f.map(|s| s.into())
    }
}

impl Component<WGPURenderer> for FileSelector {
    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut b = super::Button::new(txt!("..."), self.style.button_style.clone());
        let this: &'static Self = unsafe { std::mem::transmute(self) };
        if let Some(f) = &this.on_select {
            b = b.on_click(Box::new(|| f(this.select())));
        }

        Some(node!(b))
    }
}
