use crate::base_types::{Data, PixelSize};

#[cfg(feature = "std")]
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

/// The trait that backends must implement. An instance is returned by [`current_window`][crate::current_window] so that an app may interact with the OS's windowing system.
#[cfg(feature = "std")]
pub trait Window: HasRawWindowHandle + HasRawDisplayHandle + Send + Sync {
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
use alloc::boxed::Box;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::once_lock::OnceLock;

type RwLock<T> = embassy_sync::rwlock::RwLock<CriticalSectionRawMutex, T>;

fn _current_window() -> &'static RwLock<Option<Box<dyn Window>>> {
    static CURRENT_WINDOW: OnceLock<RwLock<Option<Box<dyn Window>>>> = OnceLock::new();
    CURRENT_WINDOW.get_or_init(|| RwLock::new(None))
}

#[doc(hidden)]
/// Return a reference to the current [`Window`].
/// Only for use in windowing backends and internal use.
pub fn current_window<'a>()
-> embassy_sync::rwlock::RwLockReadGuard<'a, CriticalSectionRawMutex, Option<Box<dyn Window>>> {
    embassy_futures::block_on(_current_window().read())
}

pub(crate) fn clear_current_window() {
    *embassy_futures::block_on(_current_window().write()) = None;
}

pub(crate) fn set_current_window(window: Box<dyn Window>) {
    *embassy_futures::block_on(_current_window().write()) = Some(window);
}

pub fn logical_size() -> Option<PixelSize> {
    current_window().as_ref().map(|w| w.logical_size())
}

pub fn physical_size() -> Option<PixelSize> {
    current_window().as_ref().map(|w| w.physical_size())
}

pub fn scale_factor() -> Option<f32> {
    current_window().as_ref().map(|w| w.scale_factor())
}

pub fn set_cursor(cursor_type: &str) {
    if let Some(w) = current_window().as_ref() {
        w.set_cursor(cursor_type)
    }
}

pub fn unset_cursor() {
    if let Some(w) = current_window().as_ref() {
        w.unset_cursor()
    }
}

pub fn put_on_clipboard(data: &Data) {
    if let Some(w) = current_window().as_ref() {
        w.put_on_clipboard(data)
    }
}

pub fn get_from_clipboard() -> Option<Data> {
    current_window()
        .as_ref()
        .and_then(|w| w.get_from_clipboard())
}

pub fn start_drag(data: Data) {
    if let Some(w) = current_window().as_ref() {
        w.start_drag(data)
    }
}

pub fn set_drop_target_valid(valid: bool) {
    if let Some(w) = current_window().as_ref() {
        w.set_drop_target_valid(valid)
    }
}
