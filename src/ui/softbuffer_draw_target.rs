use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

use crate::base_types::PixelSize;
use crate::{log_debug, log_error};

/// Owned window/display handles for softbuffer 0.4, which stores `D`/`W` inside
/// [`Context`](softbuffer::Context) / [`Surface`](softbuffer::Surface).
#[derive(Clone, Copy)]
struct SoftBufferHandles {
    window: RawWindowHandle,
    display: RawDisplayHandle,
}

// Safety: same contract as backend `Window` types — handles remain valid for the
// lifetime of the UI / OS window that owns them.
unsafe impl Send for SoftBufferHandles {}
unsafe impl Sync for SoftBufferHandles {}

impl HasWindowHandle for SoftBufferHandles {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe { WindowHandle::borrow_raw(self.window) })
    }
}

impl HasDisplayHandle for SoftBufferHandles {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe { DisplayHandle::borrow_raw(self.display) })
    }
}

pub(crate) struct SoftBufferDrawTarget {
    size: PixelSize,
    _context: softbuffer::Context<SoftBufferHandles>,
    surface: softbuffer::Surface<SoftBufferHandles, SoftBufferHandles>,
}

impl SoftBufferDrawTarget {
    pub(crate) fn new<W: HasDisplayHandle + HasWindowHandle + ?Sized>(
        window: &W,
        size: PixelSize,
    ) -> Self {
        let handles = SoftBufferHandles {
            window: window
                .window_handle()
                .expect("Failed to get window handle")
                .as_raw(),
            display: window
                .display_handle()
                .expect("Failed to get display handle")
                .as_raw(),
        };
        let context = softbuffer::Context::new(handles).unwrap();
        let surface = softbuffer::Surface::new(&context, handles).unwrap();
        let mut target = Self {
            // Start with a zero size so that we can resize it
            size: PixelSize {
                width: 0,
                height: 0,
            },
            _context: context,
            surface,
        };
        target.resize(size);
        target
    }

    pub(crate) fn resize(&mut self, size: PixelSize) {
        if self.size != size && size.width > 0 && size.height > 0 {
            self.size = size;
            if let Err(_e) = self.surface.resize(
                core::num::NonZero::new(size.width).unwrap(),
                core::num::NonZero::new(size.height).unwrap(),
            ) {
                log_error!("Failed to resize softbuffer surface: {}", _e);
            }
            log_debug!("Resized softbuffer surface to {:?}", self.size);
        }
    }

    // TODO: Use present_with_damage
    pub(crate) fn present(&mut self) {
        let buffer = self.surface.buffer_mut().unwrap();
        if let Err(_e) = buffer.present() {
            log_error!("Failed to present softbuffer surface: {}", _e);
        }
    }
}

impl embedded_graphics::draw_target::DrawTarget for SoftBufferDrawTarget {
    type Color = embedded_graphics::pixelcolor::Rgb888;
    type Error = softbuffer::SoftBufferError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::prelude::Pixel<Self::Color>>,
    {
        use embedded_graphics::prelude::{IntoStorage, Pixel};

        let mut buffer = self.surface.buffer_mut()?;
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.y >= 0
                && (coord.x as u32) < self.size.width
                && (coord.y as u32) < self.size.height
            {
                let index = coord.y as usize * self.size.width as usize + coord.x as usize;
                buffer[index] = color.into_storage();
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        use embedded_graphics::prelude::{IntoStorage, Point, Size};
        use embedded_graphics::primitives::Rectangle;

        let mut buffer = self.surface.buffer_mut()?;

        let self_area = Rectangle::new(
            Point::zero(),
            Size::new(self.size.width as u32, self.size.height as u32),
        );
        let target_area = self_area.intersection(area);
        if let Some(bottom_right) = target_area.bottom_right() {
            let width = self.size.width as usize;
            let mut x = target_area.top_left.x;
            let mut y = target_area.top_left.y;
            for color in colors {
                if x > bottom_right.x {
                    x = target_area.top_left.x;
                    y += 1;
                } else if y > bottom_right.y {
                    break;
                }
                let index = y as usize * width + x as usize;
                buffer[index] = color.into_storage();
                x += 1;
            }
        }
        Ok(())
    }
}

impl embedded_graphics::geometry::Dimensions for SoftBufferDrawTarget {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(0, 0),
            embedded_graphics::geometry::Size::new(self.size.width, self.size.height),
        )
    }
}
