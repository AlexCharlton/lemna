use crate::base_types::{Data, PixelSize};

#[cfg(feature = "std")]
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// The trait that backends must implement so an app may interact with the OS's windowing system.
#[cfg(feature = "std")]
pub trait Window: HasWindowHandle + HasDisplayHandle + Send + Sync {
    /// Logical size of the window. Probably only useful internally.
    fn logical_size(&self) -> PixelSize;

    /// Physical size of the window. Probably only useful internally.
    fn physical_size(&self) -> PixelSize;

    /// Scale factor of the window. Probably only useful internally.
    fn scale_factor(&self) -> f32;

    /// For internal use only.
    fn redraw(&self) {}

    /// Set the current cursor. Cursor names are backend-specific, but they should support the following:
    /// - "Arrow"
    /// - "None"
    /// - "Hidden"
    /// - "Ibeam"
    /// - "Text"
    /// - "PointingHand"
    /// - "Hand"
    /// - "HandGrabbing"
    /// - "NoEntry"
    /// - "Cross"
    /// - "Size"
    /// - "Move"
    /// - "SizeNWSE"
    /// - "SizeNS"
    /// - "SizeNESW"
    /// - "SizeWE"
    fn set_cursor(&self, _cursor_type: &str) {}

    /// Reset the cursor to the default pointer.
    fn unset_cursor(&self) {}

    /// Put the [`Data`] on the clipboard.
    fn put_on_clipboard(&self, _data: &Data) {}

    /// Get the current [`Data`] that is on the clipboard, if any.
    fn get_from_clipboard(&self) -> Option<Data> {
        None
    }

    /// Start a Drag and Drop with the given [`Data`].
    fn start_drag(&self, _data: Data) {}

    /// When responding to a Drag and Drop action, tell the window of origin whether the mouse is currently over a valid drop target.
    fn set_drop_target_valid(&self, _valid: bool) {}
}

#[cfg(not(feature = "std"))]
pub trait Window: Send + Sync {
    /// Logical size of the window. Probably only useful internally.
    fn logical_size(&self) -> PixelSize;

    /// Physical size of the window. Probably only useful internally.
    fn physical_size(&self) -> PixelSize;

    /// Scale factor of the window. Probably only useful internally.
    fn scale_factor(&self) -> f32;

    /// For internal use only.
    fn redraw(&self) {}

    /// Set the current cursor. Cursor names are backend-specific, but they should support the following:
    /// - "Arrow"
    /// - "None"
    /// - "Hidden"
    /// - "Ibeam"
    /// - "Text"
    /// - "PointingHand"
    /// - "Hand"
    /// - "HandGrabbing"
    /// - "NoEntry"
    /// - "Cross"
    /// - "Size"
    /// - "Move"
    /// - "SizeNWSE"
    /// - "SizeNS"
    /// - "SizeNESW"
    /// - "SizeWE"
    fn set_cursor(&self, _cursor_type: &str) {}

    /// Reset the cursor to the default pointer.
    fn unset_cursor(&self) {}

    /// Put the [`Data`] on the clipboard.
    fn put_on_clipboard(&self, _data: &Data) {}

    /// Get the current [`Data`] that is on the clipboard, if any.
    fn get_from_clipboard(&self) -> Option<Data> {
        None
    }

    /// Start a Drag and Drop with the given [`Data`].
    fn start_drag(&self, _data: Data) {}

    /// When responding to a Drag and Drop action, tell the window of origin whether the mouse is currently over a valid drop target.
    fn set_drop_target_valid(&self, _valid: bool) {}
}

//---------------------------------------------------
// MARK: Accessors

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::marker::PhantomData;

#[cfg(feature = "std")]
use std::cell::RefCell;

#[cfg(not(feature = "std"))]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(not(feature = "std"))]
use embassy_sync::once_lock::OnceLock;
#[cfg(not(feature = "std"))]
type RwLock<T> = embassy_sync::rwlock::RwLock<CriticalSectionRawMutex, T>;

/// Pops the window pushed by [`enter`] when dropped. Not `Send`: it must be dropped on the thread that created it.
#[must_use = "the window is popped when this guard is dropped"]
pub(crate) struct WindowScope {
    _thread: PhantomData<*const ()>,
}

/// Make `window` the active window for free functions such as [`set_cursor`] on this thread, until the guard drops.
pub(crate) fn enter(window: &Arc<dyn Window>) -> WindowScope {
    push_window(Arc::clone(window));
    WindowScope {
        _thread: PhantomData,
    }
}

impl Drop for WindowScope {
    fn drop(&mut self) {
        pop_window();
    }
}

#[cfg(feature = "std")]
thread_local! {
    static WINDOW_STACK: RefCell<Vec<Arc<dyn Window>>> = RefCell::new(Vec::new());
}

#[cfg(feature = "std")]
fn push_window(window: Arc<dyn Window>) {
    WINDOW_STACK.with(|stack| stack.borrow_mut().push(window));
}

#[cfg(feature = "std")]
fn pop_window() {
    WINDOW_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
}

#[cfg(feature = "std")]
fn with_current_window<R>(f: impl FnOnce(Option<&dyn Window>) -> R) -> R {
    WINDOW_STACK.with(|stack| {
        let stack = stack.borrow();
        f(stack.last().map(|window| window.as_ref()))
    })
}

#[cfg(not(feature = "std"))]
fn window_stack() -> &'static RwLock<Vec<Arc<dyn Window>>> {
    static STACK: OnceLock<RwLock<Vec<Arc<dyn Window>>>> = OnceLock::new();
    STACK.get_or_init(|| RwLock::new(Vec::new()))
}

#[cfg(not(feature = "std"))]
fn push_window(window: Arc<dyn Window>) {
    embassy_futures::block_on(window_stack().write()).push(window);
}

#[cfg(not(feature = "std"))]
fn pop_window() {
    embassy_futures::block_on(window_stack().write()).pop();
}

#[cfg(not(feature = "std"))]
fn with_current_window<R>(f: impl FnOnce(Option<&dyn Window>) -> R) -> R {
    let stack = embassy_futures::block_on(window_stack().read());
    f(stack.last().map(|window| window.as_ref()))
}

pub fn logical_size() -> Option<PixelSize> {
    with_current_window(|window| window.map(|window| window.logical_size()))
}

pub fn physical_size() -> Option<PixelSize> {
    with_current_window(|window| window.map(|window| window.physical_size()))
}

pub fn scale_factor() -> Option<f32> {
    with_current_window(|window| window.map(|window| window.scale_factor()))
}

pub fn set_cursor(cursor_type: &str) {
    with_current_window(|window| {
        if let Some(window) = window {
            window.set_cursor(cursor_type);
        }
    });
}

pub fn unset_cursor() {
    with_current_window(|window| {
        if let Some(window) = window {
            window.unset_cursor();
        }
    });
}

pub fn put_on_clipboard(data: &Data) {
    with_current_window(|window| {
        if let Some(window) = window {
            window.put_on_clipboard(data);
        }
    });
}

pub fn get_from_clipboard() -> Option<Data> {
    with_current_window(|window| window.and_then(|window| window.get_from_clipboard()))
}

pub fn start_drag(data: Data) {
    with_current_window(|window| match window {
        Some(window) => window.start_drag(data),
        None => drop(data),
    });
}

pub fn set_drop_target_valid(valid: bool) {
    with_current_window(|window| {
        if let Some(window) = window {
            window.set_drop_target_valid(valid);
        }
    });
}
