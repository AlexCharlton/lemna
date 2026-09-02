#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub(crate) fallback_scale_factor: Option<f64>,
    pub(crate) fonts: Vec<(String, &'static [u8])>,
}

impl WindowOptions {
    /// Construct window options. `resizable` defaults to true, and the scale factor of the window defaults to the value inferred from the system.
    pub fn new<T: Into<String>>(title: T, dims: (u32, u32)) -> Self {
        Self {
            title: title.into(),
            width: dims.0,
            height: dims.1,
            resizable: true,
            fallback_scale_factor: None,
            fonts: vec![],
        }
    }

    /// Sets a fallback scale factor, used when baseview cannot determine one from the platform.
    pub fn fallback_scale_factor(mut self, scale_factor: impl Into<Option<f64>>) -> Self {
        self.fallback_scale_factor = scale_factor.into();
        self
    }

    pub fn fonts(mut self, mut fonts: Vec<(String, &'static [u8])>) -> Self {
        self.fonts.append(&mut fonts);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}
