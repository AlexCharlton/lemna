use crossbeam_channel::{Receiver, Sender, unbounded};
use lemna::UI;
use lemna_baseview::{self, Message, ParentMessage};
use nice_plug_core::{
    context::gui::GuiContext,
    editor::{Editor, ParentWindowHandle, dpi},
};
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

pub use lemna_baseview::WindowOptions;

#[derive(Clone)]
struct LemnaEditor<A: lemna::Component + Default + Send + Sync> {
    window_options: WindowOptions,
    phantom_app: PhantomData<A>,
    scale_factor: Arc<RwLock<Option<f64>>>,
    // Called when initializing the app
    build: Arc<dyn Fn(Arc<dyn GuiContext>, &mut UI<A>) + 'static + Send + Sync>,
    on_param_change: Arc<dyn Fn() -> Vec<Message> + 'static + Send + Sync>,
    // Used to communicate with the baseview WindowHandler
    sender: Sender<ParentMessage>,
    receiver: Receiver<ParentMessage>,
}

pub fn create_lemna_editor<A, B, P>(
    options: WindowOptions,
    build: B,
    on_param_change: P,
) -> Option<Box<dyn Editor>>
where
    A: 'static + lemna::Component + Default + Send + Sync,
    B: Fn(Arc<dyn GuiContext>, &mut UI<A>) + 'static + Send + Sync,
    P: Fn() -> Vec<Message> + 'static + Send + Sync,
{
    let (sender, receiver) = unbounded::<ParentMessage>();

    Some(Box::new(LemnaEditor::<A> {
        window_options: options,
        scale_factor: Arc::new(RwLock::new(None)),
        phantom_app: PhantomData,
        build: Arc::new(build),
        on_param_change: Arc::new(on_param_change),
        sender,
        receiver,
    }))
}

impl<A> Editor for LemnaEditor<A>
where
    A: 'static + lemna::Component + Default + Send + Sync,
{
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any> {
        let build = self.build.clone();
        // Trigger a resize on the first frame
        self.sender.send(ParentMessage::Resize).unwrap();
        // And trigger a param change too
        for m in (self.on_param_change)().drain(..) {
            self.sender.send(ParentMessage::AppMessage(m)).unwrap();
        }

        let mut options = self.window_options.clone();
        options = options.fallback_scale_factor(*self.scale_factor.read().unwrap());

        let handle = lemna_baseview::Window::open_parented::<_, A, _>(
            &parent,
            options,
            move |ui| (build)(context.clone(), ui),
            Some(self.receiver.clone()),
        );
        Box::new(LemnaEditorHandle { _window: handle })
    }

    fn size(&self) -> dpi::Size {
        dpi::LogicalSize::new(
            self.window_options.width as f64,
            self.window_options.height as f64,
        )
        .into()
    }
    fn set_scale_factor(&self, factor: f64) -> bool {
        *self.scale_factor.write().unwrap() = Some(factor);
        true
    }
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        for m in (self.on_param_change)().drain(..) {
            self.sender.send(ParentMessage::AppMessage(m)).unwrap();
        }
    }
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        for m in (self.on_param_change)().drain(..) {
            self.sender.send(ParentMessage::AppMessage(m)).unwrap();
        }
    }
    fn param_values_changed(&self) {
        for m in (self.on_param_change)().drain(..) {
            self.sender.send(ParentMessage::AppMessage(m)).unwrap();
        }
    }
}

struct LemnaEditorHandle {
    _window: baseview::Window,
}

unsafe impl Send for LemnaEditorHandle {}
