use crossbeam_channel::{unbounded, Receiver, Sender};
use lemna::UI;
use lemna_baseview::{self, baseview, ParentMessage, Window};
use nih_plug::prelude::*;
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

pub extern crate nih_plug;

#[derive(Clone)]
struct LemnaEditor<R: lemna::render::Renderer, A: lemna::App<R>> {
    size: (u32, u32),
    title: String,
    fonts: Vec<(String, &'static [u8])>,
    phantom_renderer: PhantomData<R>,
    phantom_app: PhantomData<A>,
    scale_factor: Arc<RwLock<Option<f32>>>,
    // Called when initializing the app
    build: Arc<dyn Fn(Arc<dyn GuiContext>, &mut UI<Window, R, A>) + 'static + Send + Sync>,
    // Used to communicate with the baseview WindowHandler
    sender: Sender<ParentMessage>,
    receiver: Receiver<ParentMessage>,
}

pub fn create_lemna_editor<R, A, B>(
    title: &str,
    width: u32,
    height: u32,
    fonts: Vec<(String, &'static [u8])>,
    build: B,
) -> Option<Box<dyn Editor>>
where
    R: lemna::render::Renderer + 'static + Send,
    A: 'static + lemna::App<R> + Send,
    B: Fn(Arc<dyn GuiContext>, &mut UI<Window, R, A>) + 'static + Send + Sync,
{
    let (sender, receiver) = unbounded::<ParentMessage>();

    // Trigger a resize on the first frame
    // This is only needed by nih_plug's standalone wrapper
    sender.send(ParentMessage::Resize).unwrap();

    Some(Box::new(LemnaEditor::<R, A> {
        size: (width, height),
        title: title.to_string(),
        fonts,
        scale_factor: Arc::new(RwLock::new(None)),
        phantom_app: PhantomData,
        phantom_renderer: PhantomData,
        build: Arc::new(build),
        sender,
        receiver,
    }))
}

impl<R, A> Editor for LemnaEditor<R, A>
where
    R: lemna::render::Renderer + 'static + Send,
    A: 'static + lemna::App<R> + Send,
{
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let build = self.build.clone();
        let handle = lemna_baseview::Window::open_parented::<_, R, A, _>(
            &parent,
            self.title.clone(),
            self.size.0,
            self.size.1,
            self.scale_factor
                .read()
                .unwrap()
                .map(|factor| baseview::WindowScalePolicy::ScaleFactor(factor as f64))
                .unwrap_or(baseview::WindowScalePolicy::SystemScaleFactor),
            self.fonts.clone(),
            move |ui| (build)(context.clone(), ui),
            Some(self.receiver.clone()),
        );
        Box::new(LemnaEditorHandle { _window: handle })
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn set_scale_factor(&self, factor: f32) -> bool {
        *self.scale_factor.write().unwrap() = Some(factor);
        true
    }
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        self.sender.send(ParentMessage::Dirty).unwrap();
        ()
    }
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        self.sender.send(ParentMessage::Dirty).unwrap();
        ()
    }
    fn param_values_changed(&self) {
        self.sender.send(ParentMessage::Dirty).unwrap();
        ()
    }
}

struct LemnaEditorHandle {
    _window: baseview::WindowHandle,
}

unsafe impl Send for LemnaEditorHandle {}
