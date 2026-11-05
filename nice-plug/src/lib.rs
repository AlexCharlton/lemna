use crossbeam_channel::{Receiver, Sender, unbounded};
use lemna::UI;
use lemna_baseview::{self, Message, ParentMessage};
use nice_plug_core::{
    context::gui::GuiContext,
    editor::{
        Editor, EditorHandle, HostCallbacks, HostMainThreadCaller, HostMethods, ParentWindowHandle,
        SpawnedEditor, dpi,
    },
    plugin::TrackInfo,
};
use std::{
    error::Error,
    marker::PhantomData,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

pub use lemna_baseview::WindowOptions;

/// Host callbacks are only invoked from the GUI thread, but `EditorHandle` must
/// be `Send`. nice-plug's `HostCallbacks` is not `Send`, so we assert that here.
struct SharedHostCallbacks {
    inner: Mutex<Box<dyn HostCallbacks>>,
}

impl SharedHostCallbacks {
    fn new(callbacks: Box<dyn HostCallbacks>) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(callbacks),
        })
    }
}

// SAFETY: Editor open/close/resize and host callbacks all run on the GUI thread.
unsafe impl Send for SharedHostCallbacks {}
unsafe impl Sync for SharedHostCallbacks {}

/// Per-editor logical size + scale. Must not be process-global: hosts often create
/// several editors (main UI, midi out, …) and query `Editor::size()` independently.
#[derive(Debug)]
struct EditorSizeState {
    /// Designed logical size (from WindowOptions). Fixed-size editors keep this.
    logical_size: (u32, u32),
    scale_factor: f64,
}

impl EditorSizeState {
    fn new(logical_size: (u32, u32)) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            logical_size,
            scale_factor: 1.0,
        }))
    }

    fn native_size(state: &Arc<RwLock<Self>>) -> dpi::NativeSize<u32> {
        let state = state.read().unwrap();
        dpi::NativeSize::from_size(
            dpi::Size::Logical(dpi::LogicalSize::new(
                state.logical_size.0 as f64,
                state.logical_size.1 as f64,
            )),
            state.scale_factor,
        )
    }

    fn set_scale(state: &Arc<RwLock<Self>>, scale_factor: f64) {
        state.write().unwrap().scale_factor = scale_factor;
    }

    fn snapshot(state: &Arc<RwLock<Self>>) -> Self {
        let state = state.read().unwrap();
        Self {
            logical_size: state.logical_size,
            scale_factor: state.scale_factor,
        }
    }
}

#[derive(Clone)]
pub struct LemnaEditor<A: lemna::Component + Default + Send + Sync> {
    window_options: WindowOptions,
    size_state: Arc<RwLock<EditorSizeState>>,
    phantom_app: PhantomData<A>,
    build: Arc<dyn Fn(GuiContext, &mut UI<A>) + 'static + Send + Sync>,
    on_param_change: Arc<dyn Fn() -> Vec<Message> + 'static + Send + Sync>,
    on_track_info: Arc<dyn Fn(TrackInfo) + 'static + Send + Sync>,
    sender: Sender<ParentMessage>,
    receiver: Receiver<ParentMessage>,
    /// Wakes the window so `poll` drains pending parent messages.
    waker: Arc<RwLock<Option<baseview::WindowWaker>>>,
}

pub fn create_lemna_editor<A, B, P>(
    options: WindowOptions,
    build: B,
    on_param_change: P,
) -> Option<LemnaEditor<A>>
where
    A: 'static + lemna::Component + Default + Send + Sync,
    B: Fn(GuiContext, &mut UI<A>) + 'static + Send + Sync,
    P: Fn() -> Vec<Message> + 'static + Send + Sync,
{
    let (sender, receiver) = unbounded::<ParentMessage>();
    let size_state = EditorSizeState::new((options.width, options.height));

    Some(LemnaEditor::<A> {
        window_options: options,
        size_state,
        phantom_app: PhantomData,
        build: Arc::new(build),
        on_param_change: Arc::new(on_param_change),
        on_track_info: Arc::new(|_| {}),
        sender,
        receiver,
        waker: Arc::new(RwLock::new(None)),
    })
}

impl<A> LemnaEditor<A>
where
    A: 'static + lemna::Component + Default + Send + Sync,
{
    /// Called when the host updates track information for this plugin instance.
    pub fn on_track_info<F>(mut self, f: F) -> Self
    where
        F: Fn(TrackInfo) + 'static + Send + Sync,
    {
        self.on_track_info = Arc::new(f);
        self
    }
}

impl<A> Editor for LemnaEditor<A>
where
    A: 'static + lemna::Component + Default + Send + Sync,
{
    type Handle = LemnaEditorHandle;

    fn spawn(
        &self,
        parent: Option<ParentWindowHandle>,
        wait_for_parent: bool,
        fallback_scale_factor: Option<f64>,
        gui_context: GuiContext,
        host: Option<HostMethods>,
    ) -> Result<SpawnedEditor<Self::Handle>, Box<dyn Error>> {
        tracing::debug!(
            target: "lemna_nice_plug",
            has_parent = parent.is_some(),
            wait_for_parent,
            fallback_scale_factor,
            logical = ?(self.window_options.width, self.window_options.height),
            "Editor::spawn"
        );

        let build = self.build.clone();
        for m in (self.on_param_change)() {
            self.sender.send(ParentMessage::AppMessage(m))?;
        }

        if let Some(factor) = fallback_scale_factor {
            EditorSizeState::set_scale(&self.size_state, factor);
        }

        let mut options = self.window_options.clone();
        options = options.fallback_scale_factor(fallback_scale_factor);

        let (host, host_callbacks) = match host {
            Some(methods) => {
                let (bv_host, shared) = into_baseview_host(methods);
                (Some(bv_host), Some(shared))
            }
            None => (None, None),
        };
        let window = lemna_baseview::Window::create::<_, A, _>(
            parent.as_ref(),
            wait_for_parent,
            host,
            options,
            move |ui| (build)(gui_context.clone(), ui),
            Some(self.receiver.clone()),
        )?;
        let created = window.size();
        // Adopt a real DPI from the window when available; never replace a known
        // host scale (>1) with a placeholder 1.0 (Reaper wait_for_parent path).
        if created.scale_factor > 1.0 + f64::EPSILON {
            EditorSizeState::set_scale(&self.size_state, created.scale_factor);
        }

        let waker = window.waker();
        *self.waker.write().unwrap() = Some(waker.clone());
        let _ = waker.request_poll();

        Ok(SpawnedEditor {
            handle: LemnaEditorHandle {
                sender: self.sender.clone(),
                on_param_change: self.on_param_change.clone(),
                size_state: self.size_state.clone(),
                host_callbacks,
                waker: self.waker.clone(),
                in_host_resize: Arc::new(AtomicBool::new(false)),
            },
            window,
        })
    }

    fn size(&self) -> dpi::NativeSize<u32> {
        EditorSizeState::native_size(&self.size_state)
    }

    fn track_info_updated(&self, info: TrackInfo) {
        (self.on_track_info)(info);
    }
}

pub struct LemnaEditorHandle {
    sender: Sender<ParentMessage>,
    on_param_change: Arc<dyn Fn() -> Vec<Message> + 'static + Send + Sync>,
    size_state: Arc<RwLock<EditorSizeState>>,
    host_callbacks: Option<Arc<SharedHostCallbacks>>,
    waker: Arc<RwLock<Option<baseview::WindowWaker>>>,
    /// Prevents `request_host_resize` → host `set_size` → `request_host_resize` loops.
    in_host_resize: Arc<AtomicBool>,
}

impl LemnaEditorHandle {
    fn send_parent_messages(&self, messages: impl IntoIterator<Item = ParentMessage>) {
        for message in messages {
            let _ = self.sender.send(message);
        }
        if let Some(waker) = self.waker.read().unwrap().as_ref() {
            waker.request_poll();
        }
    }

    fn request_host_resize(&self) {
        if self
            .in_host_resize
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let Some(host) = &self.host_callbacks else {
            self.in_host_resize.store(false, Ordering::SeqCst);
            return;
        };
        let state = EditorSizeState::snapshot(&self.size_state);
        let size = dpi::Size::Logical(dpi::LogicalSize::new(
            state.logical_size.0 as f64,
            state.logical_size.1 as f64,
        ));
        let native = EditorSizeState::native_size(&self.size_state);
        tracing::debug!(
            target: "lemna_nice_plug",
            logical = ?state.logical_size,
            scale = state.scale_factor,
            native = ?(native.width, native.height),
            "request_host_resize"
        );
        let result = host
            .inner
            .lock()
            .unwrap()
            .request_resize(size, state.scale_factor);
        self.in_host_resize.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            tracing::warn!(
                target: "lemna_nice_plug",
                error = %e,
                "request_host_resize failed"
            );
        }
    }

    /// If the window reports a real DPI (>1), adopt it. Never replace a known
    /// host scale with a placeholder 1.0.
    fn adopt_window_scale(&self, window: &baseview::Window) {
        let scale = window.size().scale_factor;
        if scale > 1.0 + f64::EPSILON {
            EditorSizeState::set_scale(&self.size_state, scale);
        }
    }

    /// Keep this editor window's tracked logical size/scale in sync with editor state.
    fn sync_lemna_layout(&self) {
        let state = EditorSizeState::snapshot(&self.size_state);
        self.send_parent_messages([ParentMessage::SetSize {
            logical_size: state.logical_size,
            scale_factor: state.scale_factor as f32,
        }]);
    }

    /// Resize the HWND to `design_logical * scale` if needed. Does not notify the
    /// host (that re-enters `set_size` on Reaper and can hang).
    fn ensure_native_size(&self, window: &baseview::Window) {
        let expected = EditorSizeState::native_size(&self.size_state);
        let actual = window.size();
        if actual.physical.width != expected.width || actual.physical.height != expected.height {
            let size = baseview::dpi::NativeSize::new(expected.width, expected.height);
            let _ = window.resize(baseview::dpi::Size::from(size));
        }
    }

    fn after_attach_or_show(&self, window: &baseview::Window) {
        self.adopt_window_scale(window);
        let scale = self.size_state.read().unwrap().scale_factor;
        if scale > 1.0 + f64::EPSILON {
            self.ensure_native_size(window);
            self.sync_lemna_layout();
        }
    }
}

impl Drop for LemnaEditorHandle {
    fn drop(&mut self) {
        *self.waker.write().unwrap() = None;
    }
}

impl EditorHandle for LemnaEditorHandle {
    type Window = baseview::Window;
    type Error = baseview::Error;

    fn run_until_closed(window: Self::Window) -> Result<(), Self::Error> {
        window.run_until_closed()
    }

    fn set_parent(
        &self,
        parent: ParentWindowHandle,
        window: &Self::Window,
    ) -> Result<(), Self::Error> {
        let result = window.set_parent(&parent);
        self.after_attach_or_show(window);
        result
    }

    fn show(&self, window: &Self::Window) -> Result<(), Self::Error> {
        let result = window.show();
        self.after_attach_or_show(window);
        result
    }

    fn hide(&self, window: &Self::Window) -> Result<(), Self::Error> {
        window.hide()
    }

    fn set_size(
        &self,
        new_size: dpi::NativeSize<u32>,
        window: &Self::Window,
    ) -> Result<(), Self::Error> {
        let expected = EditorSizeState::native_size(&self.size_state);
        tracing::debug!(
            target: "lemna_nice_plug",
            requested_native = ?(new_size.width, new_size.height),
            expected_native = ?(expected.width, expected.height),
            "EditorHandle::set_size"
        );

        if new_size.width != expected.width || new_size.height != expected.height {
            // Ableton without auto-scale: re-asserts pre-DPI getSize(). Keep design
            // size and ask the host to match (reentrancy-guarded).
            tracing::warn!(
                target: "lemna_nice_plug",
                requested_native = ?(new_size.width, new_size.height),
                expected_native = ?(expected.width, expected.height),
                "Ignoring host set_size that doesn't match scaled design size"
            );
            self.ensure_native_size(window);
            self.sync_lemna_layout();
            self.request_host_resize();
        } else {
            // Host agrees on native size. Only sync lemna layout — do not HWND
            // resize here (resize ↔ set_size feedback hangs Reaper).
            self.sync_lemna_layout();
        }
        Ok(())
    }

    fn host_main_thread_callback(&self, window: &Self::Window) {
        window.host_main_thread_callback();
    }

    fn set_fallback_scale_factor(
        &self,
        scale_factor: f64,
        window: &Self::Window,
    ) -> Result<(), Self::Error> {
        let before = window.size();
        EditorSizeState::set_scale(&self.size_state, scale_factor);
        let expected = EditorSizeState::native_size(&self.size_state);
        let physical_already_ok =
            before.physical.width == expected.width && before.physical.height == expected.height;

        // VST Reaper often pre-sizes the HWND to design*scale while still reporting
        // scale 1.0. suggest_fallback would treat that physical size as unscaled and
        // grow it again (1536→2304).
        let result = if physical_already_ok {
            tracing::debug!(
                target: "lemna_nice_plug",
                scale_factor,
                suggested = false,
                "EditorHandle::set_fallback_scale_factor"
            );
            Ok(())
        } else {
            tracing::debug!(
                target: "lemna_nice_plug",
                scale_factor,
                suggested = true,
                "EditorHandle::set_fallback_scale_factor"
            );
            let result = window.suggest_fallback_scale_factor(scale_factor);
            let after_suggest = window.size();
            if after_suggest.scale_factor > 1.0 + f64::EPSILON {
                EditorSizeState::set_scale(&self.size_state, after_suggest.scale_factor);
            }
            self.ensure_native_size(window);
            result
        };

        self.sync_lemna_layout();
        // HWND can already be design*scale while the host frame is still unscaled
        // (Ableton first open). Always ask the host to match.
        self.request_host_resize();
        result
    }

    fn state_changed(&self) {
        self.send_parent_messages(
            (self.on_param_change)()
                .into_iter()
                .map(ParentMessage::AppMessage),
        );
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        self.send_parent_messages(
            (self.on_param_change)()
                .into_iter()
                .map(ParentMessage::AppMessage),
        );
    }

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        self.send_parent_messages(
            (self.on_param_change)()
                .into_iter()
                .map(ParentMessage::AppMessage),
        );
    }
}

struct BaseviewHostCallbacks {
    inner: Arc<SharedHostCallbacks>,
}

impl baseview::host::HostCallbacks for BaseviewHostCallbacks {
    fn request_resize(
        &mut self,
        new_size: baseview::WindowSize,
    ) -> Result<(), baseview::HandlerError> {
        let size = dpi::Size::Logical(dpi::LogicalSize::new(
            new_size.logical.width,
            new_size.logical.height,
        ));
        self.inner
            .inner
            .lock()
            .unwrap()
            .request_resize(size, new_size.scale_factor)
            .map_err(baseview::HandlerError::from_boxed)
            .inspect_err(|e| {
                tracing::warn!(
                    target: "lemna_nice_plug",
                    error = %e,
                    "HostCallbacks::request_resize failed"
                );
            })
    }

    fn destroyed(&mut self) {
        self.inner.inner.lock().unwrap().destroyed();
    }
}

struct BaseviewHostMainThreadCaller {
    inner: Box<dyn HostMainThreadCaller>,
}

impl baseview::host::HostMainThreadCaller for BaseviewHostMainThreadCaller {
    fn call_main_thread(&mut self) {
        self.inner.call_main_thread();
    }
}

fn into_baseview_host(host: HostMethods) -> (baseview::host::Host, Arc<SharedHostCallbacks>) {
    let shared = SharedHostCallbacks::new(host.callbacks);
    let bv_host = baseview::host::Host::new()
        .with_callbacks(BaseviewHostCallbacks {
            inner: shared.clone(),
        })
        .with_main_thread(BaseviewHostMainThreadCaller {
            inner: host.main_thread_caller,
        });
    (bv_host, shared)
}
