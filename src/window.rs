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
