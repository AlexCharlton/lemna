use crate::base_types::*;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::any::Any;

#[derive(Debug)]
pub enum Data {
    String(String),
    Custom(Vec<u8>),
}

impl From<&str> for Data {
    fn from(s: &str) -> Data {
        Data::String(s.to_string())
    }
}

pub trait Window: HasRawWindowHandle + HasRawDisplayHandle + Send + Sync + Any {
    fn logical_size(&self) -> PixelSize;
    fn physical_size(&self) -> PixelSize;
    fn scale_factor(&self) -> f32;
    fn redraw(&self) {}
    fn set_cursor(&self, _cursor_type: &str) {}
    fn unset_cursor(&self) {}
    fn put_on_clipboard(&self, _data: &Data) {}
    fn get_from_clipboard(&self) -> Option<Data> {
        None
    }
}
