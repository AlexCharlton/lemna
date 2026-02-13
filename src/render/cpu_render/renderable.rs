extern crate alloc;

use alloc::vec::Vec;

use tiny_skia::{BlendMode, Mask, Paint, Pixmap, Shader, Stroke, Transform};

use crate::PositionedGlyph;
use crate::base_types::{Color, PixelSize, Pos, Rect, Scale};
use crate::render::path::Path;
use crate::render::raster_cache::RasterCacheId;
use crate::renderable::{Caches, RasterData};

//--------------------------------
// MARK: Rectangle

#[derive(Debug, PartialEq)]
pub struct Rectangle {
    pub pos: Pos,
    pub scale: Scale,
    pub color: Color,
}

impl Rectangle {
    pub fn new(pos: Pos, scale: Scale, color: Color) -> Self {
        Self { pos, scale, color }
    }

    pub(crate) fn render(&self, aabb: &Rect, mask: Option<&Mask>, pixmap: &mut Pixmap) {
        let paint = Paint {
            shader: Shader::SolidColor(self.color.into()),
            blend_mode: BlendMode::SourceOver,
            ..Default::default()
        };

        pixmap.fill_rect(
            rect_from_pos_scale(&(aabb.pos + self.pos), &self.scale),
            &paint,
            Transform::identity(),
            mask,
        )
    }

    pub(crate) fn z(&self) -> f32 {
        self.pos.z
    }
}

fn rect_from_pos_scale(pos: &Pos, scale: &Scale) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(pos.x, pos.y, scale.width.max(1.0), scale.height.max(1.0)).unwrap()
}

//--------------------------------
// MARK: Shape

#[derive(Debug, PartialEq)]
pub struct Shape {
    path: Path,
    fill_color: Color,
    stroke_color: Color,
    stroke_width: f32,
    z: f32,
}

impl Shape {
    pub fn new(
        path: Path,
        fill_color: Color,
        stroke_color: Color,
        stroke_width: f32,
        z: f32,
        _caches: &mut Caches,
        _prev: Option<&Shape>,
    ) -> Self {
        Self {
            path,
            fill_color,
            stroke_color,
            stroke_width,
            z,
        }
    }

    pub(crate) fn render(&self, aabb: &Rect, mask: Option<&Mask>, pixmap: &mut Pixmap) {
        let transform = Transform::from_translate(aabb.pos.x, aabb.pos.y);
        let path = self.path.native_path();
        if self.fill_color.is_visible() {
            let paint = Paint {
                shader: Shader::SolidColor(self.fill_color.into()),
                anti_alias: cfg!(feature = "antialiased_shapes"),
                blend_mode: BlendMode::SourceOver,
                ..Default::default()
            };

            pixmap.fill_path(
                path,
                &paint,
                tiny_skia::FillRule::default(),
                transform,
                mask,
            );
        }
        if self.stroke_color.is_visible() && self.stroke_width > 0.0 {
            let paint = Paint {
                shader: Shader::SolidColor(self.stroke_color.into()),
                anti_alias: cfg!(feature = "antialiased_shapes"),
                blend_mode: BlendMode::SourceOver,
                ..Default::default()
            };
            let stroke = Stroke {
                width: self.stroke_width,
                ..Default::default()
            };
            pixmap.stroke_path(path, &paint, &stroke, transform, mask);
        }
    }

    pub(crate) fn z(&self) -> f32 {
        self.z
    }
}

//--------------------------------
// MARK: Text

#[derive(Debug)]
pub struct Text {
    glyphs: Vec<PositionedGlyph>,
    offset: Pos,
    color: Color,
}

impl PartialEq for Text {
    // Should only be used for tests
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color && self.offset == other.offset
    }
}

impl Text {
    pub fn new(
        glyphs: Vec<PositionedGlyph>,
        offset: Pos,
        color: Color,
        _caches: &mut Caches,
        _prev: Option<&Text>,
    ) -> Self {
        Self {
            glyphs,
            offset,
            color,
        }
    }

    pub(crate) fn render(
        &self,
        aabb: &Rect,
        mask: Option<&Mask>,
        pixmap: &mut Pixmap,
        caches: &mut Caches,
    ) {
        let mut text_mask = Mask::new(pixmap.width(), pixmap.height()).unwrap();
        let mask_data = text_mask.data_mut();

        // Draw each glyph into the mask
        // The text_mask will be black where the render mask is
        for glyph in self.glyphs.iter() {
            // Skip invisible glyphs (newlines, spaces, etc.) that have 0 width or height
            if glyph.width == 0 || glyph.height == 0 {
                continue;
            }
            let mask_x_initial = aabb.pos.x as usize + self.offset.x as usize + glyph.x as usize;
            let mut mask_y = aabb.pos.y as usize + self.offset.y as usize + glyph.y as usize;
            if mask_y >= pixmap.height() as usize {
                continue;
            }
            let mut mask_i = mask_x_initial + (mask_y * pixmap.width() as usize);
            let mut glyph_x = 0;
            if let Some(glyph_mask) = caches.glyph.glyph_mask(&caches.font.fonts, glyph) {
                for v in glyph_mask.data() {
                    if mask_i >= mask_data.len() {
                        break;
                    }
                    if let Some(mask) = mask {
                        // If our current render mask position is not white, skip the pixel
                        if mask.data()[mask_i] == 255 {
                            mask_data[mask_i] = *v;
                        }
                    } else {
                        // If we don't have a mask, just draw the glyph
                        mask_data[mask_i] = *v;
                    }

                    glyph_x += 1;
                    mask_i += 1;
                    if glyph_x >= glyph_mask.width() as usize {
                        glyph_x = 0;
                        mask_y += 1;
                        mask_i = mask_x_initial + glyph_x + (mask_y * pixmap.width() as usize);
                    }
                }
            }
        }
        // Draw the mask with the text color
        let paint = Paint {
            shader: Shader::SolidColor(self.color.into()),
            anti_alias: false,
            blend_mode: BlendMode::SourceOver,
            ..Default::default()
        };
        pixmap.fill_rect(aabb.into(), &paint, Transform::identity(), Some(&text_mask));
    }

    pub(crate) fn z(&self) -> f32 {
        self.offset.z
    }
}

//--------------------------------
// MARK: Raster

#[derive(Debug, PartialEq)]
pub struct Raster {
    raster_cache_id: RasterCacheId,
}

impl Raster {
    pub fn new(
        data: RasterData,
        size: PixelSize,
        caches: &mut Caches,
        prev: Option<&Raster>,
    ) -> Self {
        let raster_cache = &mut caches.raster;
        let raster_cache_id = raster_cache.alloc_or_reuse_chunk(prev.map(|r| r.raster_cache_id));
        raster_cache.set_raster(raster_cache_id, data, size);

        Self { raster_cache_id }
    }

    pub fn get_mut_raster_data<'a>(&self, caches: &'a mut Caches) -> &'a mut RasterData {
        let raster_cache = &mut caches.raster;
        raster_cache
            .get_mut_raster_data(self.raster_cache_id)
            .dirty();
        &mut raster_cache.get_mut_raster_data(self.raster_cache_id).data
    }

    pub fn get_raster_data<'a>(&self, caches: &'a Caches) -> &'a RasterData {
        &caches.raster.get_raster_data(self.raster_cache_id).data
    }

    pub(crate) fn render(
        &self,
        aabb: &Rect,
        mask: Option<&Mask>,
        pixmap: &mut Pixmap,
        caches: &Caches,
    ) {
        let screen_width = pixmap.width();
        let screen_height = pixmap.height();
        let raster_size = caches.raster.get_raster_size(self.raster_cache_id);
        let mut pixmap_i = None;
        let pixmap_data = pixmap.data_mut();
        let data: &[u8] = self.get_raster_data(caches).into();
        let mut raster_x = 0;
        let initial_pixmap_x = aabb.pos.x as i32;
        let mut pixmap_x = initial_pixmap_x;
        let mut pixmap_y = aabb.pos.y as i32;
        for i in (0..data.len()).step_by(4) {
            if raster_x >= raster_size.width {
                raster_x = 0;
                pixmap_x = initial_pixmap_x;
                pixmap_y += 1;
                pixmap_i = None; // Force a new pixmap_i
            }

            // Make sure we are within the pixmap
            if pixmap_x < 0
                || pixmap_x >= screen_width as i32
                || pixmap_y < 0
                || pixmap_y >= screen_height as i32
            {
                raster_x += 1;
                pixmap_x += 1;
                pixmap_i = None;
                continue;
            }

            // all values are within the pixmap now
            if pixmap_i.is_none() {
                pixmap_i = Some((pixmap_x + (pixmap_y * screen_width as i32)) as usize * 4);
            }

            // We always have a pixmap_i now
            let pi = pixmap_i.unwrap();

            if pi >= pixmap_data.len() {
                break;
            }

            // Obey the mask
            if let Some(mask) = mask {
                // If the mask is not white, skip the pixel
                if mask.data()[pi] != 255 {
                    raster_x += 1;
                    pixmap_x += 1;
                    *pixmap_i.as_mut().unwrap() += 4;
                    continue;
                }
            }

            pixmap_data[pi] = data[i];
            pixmap_data[pi + 1] = data[i + 1];
            pixmap_data[pi + 2] = data[i + 2];
            pixmap_data[pi + 3] = data[i + 3];

            raster_x += 1;
            pixmap_x += 1;
            *pixmap_i.as_mut().unwrap() += 4;
        }
    }

    pub(crate) fn z(&self) -> f32 {
        // TODO: Should rasters have a z-index?
        0.0
    }
}
