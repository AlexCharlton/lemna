use lemna_baseview::{self, baseview};
use nih_plug::prelude::*;
use std::{marker::PhantomData, sync::Arc};

pub extern crate nih_plug;

pub fn create_lemna_editor<R, A>(
    title: &str,
    width: u32,
    height: u32,
    fonts: Vec<(String, &'static [u8])>,
) -> Option<Box<dyn Editor>>
where
    R: lemna::render::Renderer + 'static + Send,
    <R as lemna::render::Renderer>::Renderable: std::fmt::Debug,
    A: 'static + lemna::App<R> + Send,
{
    Some(Box::new(LemnaEditor::<R, A> {
        size: (width, height),
        title: title.to_string(),
        fonts,
        phantom_app: PhantomData,
        phantom_renderer: PhantomData,
    }))
}

#[derive(Clone)]
struct LemnaEditor<R, A> {
    size: (u32, u32),
    title: String,
    fonts: Vec<(String, &'static [u8])>,
    phantom_renderer: PhantomData<R>,
    phantom_app: PhantomData<A>,
}

impl<R, A> Editor for LemnaEditor<R, A>
where
    R: lemna::render::Renderer + 'static + Send,
    <R as lemna::render::Renderer>::Renderable: std::fmt::Debug,
    A: 'static + lemna::App<R> + Send,
{
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let handle = lemna_baseview::Window::open_parented::<_, R, A>(
            &parent,
            self.title.clone(),
            self.size.0,
            self.size.1,
            baseview::WindowScalePolicy::SystemScaleFactor,
            self.fonts.clone(),
        );
        Box::new(LemnaEditorHandle { _window: handle })
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn set_scale_factor(&self, _factor: f32) -> bool {
        true
    }
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        ()
    }
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        ()
    }
    fn param_values_changed(&self) {
        ()
    }
}

struct LemnaEditorHandle {
    _window: baseview::WindowHandle,
}

unsafe impl Send for LemnaEditorHandle {}
