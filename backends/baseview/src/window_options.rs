#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub(crate) scale_policy: baseview::WindowScalePolicy,
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
            scale_policy: baseview::WindowScalePolicy::SystemScaleFactor,
            fonts: vec![],
        }
    }

    pub fn scale_factor(mut self, scale: f32) -> Self {
        self.scale_policy = baseview::WindowScalePolicy::ScaleFactor(scale.into());
        self
    }

    pub fn system_scale_factor(mut self) -> Self {
        self.scale_policy = baseview::WindowScalePolicy::SystemScaleFactor;
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
