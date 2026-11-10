//! Rasterise annotation text into the exported image.
//!
//! GPUI draws the live preview with a system font; the exporter needs to burn
//! the same text into pixels, so it loads the system Arial face directly.

use ab_glyph::{Font, FontVec, Glyph, PxScale, ScaleFont, point as glyph_point};
use image::RgbaImage;
use std::sync::OnceLock;

const FONT_CANDIDATES: [&str; 3] = [
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/System/Library/Fonts/Geneva.ttf",
    "/System/Library/Fonts/SFNS.ttf",
];

fn font() -> Option<&'static FontVec> {
    static FONT: OnceLock<Option<FontVec>> = OnceLock::new();
    FONT.get_or_init(|| {
        FONT_CANDIDATES.iter().find_map(|path| {
            let bytes = std::fs::read(path).ok()?;
            FontVec::try_from_vec(bytes).ok()
        })
    })
    .as_ref()
}

/// Draw `text` with its top-left corner at `origin`, in device pixels.
pub fn draw_text(img: &mut RgbaImage, text: &str, origin: (f32, f32), size_px: f32, color: [u8; 4]) {
    let Some(font) = font() else {
        return;
    };
    if size_px <= 0.5 {
        return;
    }

    let scale = PxScale::from(size_px);
    let scaled = font.as_scaled(scale);
    let baseline = origin.1 + scaled.ascent();
    let mut caret = origin.0;

    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(outline) = font.outline_glyph(Glyph {
            id,
            scale,
            position: glyph_point(caret, baseline),
        }) {
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let px = bounds.min.x as i32 + x as i32;
                let py = bounds.min.y as i32 + y as i32;
                blend(img, px, py, color, coverage);
            });
        }
        caret += scaled.h_advance(id);
    }
}

fn blend(img: &mut RgbaImage, x: i32, y: i32, color: [u8; 4], coverage: f32) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as u32, y as u32);
    if x >= img.width() || y >= img.height() {
        return;
    }
    let alpha = (coverage.clamp(0.0, 1.0) * (color[3] as f32 / 255.0)).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let dst = img.get_pixel_mut(x, y);
    for channel in 0..3 {
        dst[channel] = (color[channel] as f32 * alpha + dst[channel] as f32 * (1.0 - alpha)).round()
            as u8;
    }
    dst[3] = 255;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_something_visible() {
        let mut img = RgbaImage::new(200, 60);
        draw_text(&mut img, "Hi", (4.0, 4.0), 28.0, [255, 0, 0, 255]);
        assert!(img.pixels().any(|p| p[0] > 100 && p[1] < 100));
    }
}
