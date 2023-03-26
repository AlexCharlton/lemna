use crate::base_types::*;
use raw_window_handle::HasRawWindowHandle;

impl crate::window::Window for winit::window::Window {
    // TODO: This isn't good

    fn client_size(&self) -> PixelSize {
        let size = self.inner_size();
        PixelSize {
            width: size.width as u32,
            height: size.width as u32,
        }
    }

    fn display_size(&self) -> PixelSize {
        let size = self.inner_size();
        return self.client_size(); // This should transform to device size
    }

    fn scale_factor(&self) -> f32 {
        winit::window::Window::scale_factor(self) as f32
    }

    fn redraw(&self) {
        self.request_redraw();
    }
}
