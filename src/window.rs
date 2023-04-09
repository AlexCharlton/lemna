use crate::base_types::*;
use crate::input::Data;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::any::Any;

pub trait Window: HasRawWindowHandle + HasRawDisplayHandle + Send + Sync + Any {
    fn logical_size(&self) -> PixelSize;
    fn physical_size(&self) -> PixelSize;
    fn scale_factor(&self) -> f32;
    fn redraw(&self) {}
    fn set_cursor(&self, _cursor_type: &str) {}
    fn unset_cursor(&self) {}
    fn put_on_clipboard(&self, _data: &Data) {}
    fn start_drag(&self, _data: Data) {}
    fn get_from_clipboard(&self) -> Option<Data> {
        None
    }
}
