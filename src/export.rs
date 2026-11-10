//! Cropping, annotation compositing and export (clipboard / PNG file).

use crate::{
    capture::Screenshot,
    model::Annotation,
    shapes::{RectF, V2},
    text_render,
};
use anyhow::{Context as _, Result};
use image::{imageops, RgbaImage};
use std::{borrow::Cow, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

/// Crop a raw RGBA image (from full-desktop capture) to a single display's
/// region. `x`, `y` are the display origin in full-desktop pixel coordinates,
/// `w` × `h` is the display size in pixels.
pub fn crop_raw(
    source: &RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Screenshot {
    let x = x.max(0).min(source.width() as i32) as u32;
    let y = y.max(0).min(source.height() as i32) as u32;
    let w = (w.max(1) as u32).min(source.width().saturating_sub(x)).max(1);
    let h = (h.max(1) as u32).min(source.height().saturating_sub(y)).max(1);

    let cropped = imageops::crop_imm(source, x, y, w, h).to_image();
    Screenshot { image: cropped }
}

/// Crop the selection out of the screenshot and burn annotations into it.
/// `selection` is in overlay points, `scale` converts points to capture pixels.
pub fn render(
    shot: &Screenshot,
    selection: RectF,
    annotations: &[Annotation],
    scale: f32,
) -> RgbaImage {
    let source = &shot.image;
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    let x = (selection.x * scale).round().clamp(0.0, source.width() as f32) as u32;
    let y = (selection.y * scale)
        .round()
        .clamp(0.0, source.height() as f32) as u32;
    let w = (selection.w * scale).round() as u32;
    let h = (selection.h * scale).round() as u32;
    let w = w.min(source.width().saturating_sub(x)).max(1);
    let h = h.min(source.height().saturating_sub(y)).max(1);

    let mut output = imageops::crop_imm(source, x, y, w, h).to_image();
    let to_local = |p: V2| V2::new((p.x - selection.x) * scale, (p.y - selection.y) * scale);

    for annotation in annotations {
        for (polygon, color) in annotation.polygons() {
            let points: Vec<V2> = polygon.into_iter().map(to_local).collect();
            fill_polygon(&mut output, &points, color);
        }
        if let Some((origin, text, color, size)) = annotation.text_parts() {
            let origin = to_local(origin);
            text_render::draw_text(
                &mut output,
                text,
                (origin.x, origin.y),
                size * scale,
                color,
            );
        }
    }

    output
}

/// Scanline fill with 4x vertical supersampling and horizontal anti-aliasing.
pub fn fill_polygon(img: &mut RgbaImage, polygon: &[V2], color: [u8; 4]) {
    if polygon.len() < 3 || color[3] == 0 {
        return;
    }
    let width = img.width() as i32;
    let height = img.height() as i32;
    if width == 0 || height == 0 {
        return;
    }

    let min_y = polygon
        .iter()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as i32;
    let max_y = polygon
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(height as f32) as i32;
    if max_y <= min_y {
        return;
    }

    let mut crossings: Vec<f32> = Vec::with_capacity(polygon.len());
    let mut coverage: Vec<f32> = vec![0.0; width as usize];

    for row in min_y..max_y {
        coverage.fill(0.0);
        for sub in 0..4 {
            let sample_y = row as f32 + (sub as f32 + 0.5) * 0.25;
            crossings.clear();

            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                if (sample_y >= a.y && sample_y < b.y) || (sample_y >= b.y && sample_y < a.y) {
                    let t = (sample_y - a.y) / (b.y - a.y);
                    crossings.push(a.x + t * (b.x - a.x));
                }
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            for span in crossings.chunks_exact(2) {
                let (start, end) = (span[0], span[1]);
                let start = start.max(0.0);
                let end = end.min(width as f32);
                if end <= start {
                    continue;
                }
                let first = start.floor() as i32;
                let last = end.ceil() as i32;
                for px in first..last {
                    if px < 0 || px >= width {
                        continue;
                    }
                    let overlap = ((px + 1) as f32).min(end) - (px as f32).max(start);
                    if overlap > 0.0 {
                        coverage[px as usize] += overlap * 0.25;
                    }
                }
            }
        }

        for px in 0..width {
            let amount = coverage[px as usize];
            if amount > 0.002 {
                blend(img, px as u32, row as u32, color, amount);
            }
        }
    }
}

fn blend(img: &mut RgbaImage, x: u32, y: u32, color: [u8; 4], coverage: f32) {
    let alpha = coverage.clamp(0.0, 1.0) * (color[3] as f32 / 255.0);
    if alpha <= 0.0 {
        return;
    }
    let pixel = img.get_pixel_mut(x, y);
    for channel in 0..3 {
        pixel[channel] = (color[channel] as f32 * alpha + pixel[channel] as f32 * (1.0 - alpha))
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    pixel[3] = 255;
}

pub fn copy_to_clipboard(img: &RgbaImage) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("could not open the clipboard")?;
    clipboard
        .set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: Cow::Owned(img.clone().into_raw()),
        })
        .context("could not write an image to the clipboard")?;
    Ok(())
}

/// Ask the user where to save, then write a PNG. `Ok(None)` means cancelled.
pub fn prompt_and_save(img: &RgbaImage) -> Result<Option<PathBuf>> {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(default_file_name())
        .add_filter("PNG image", &["png"])
        .save_file()
    else {
        return Ok(None);
    };
    let path = if path.extension().is_some() {
        path
    } else {
        path.with_extension("png")
    };
    img.save(&path)
        .with_context(|| format!("could not save to {}", path.display()))?;
    Ok(Some(path))
}

pub fn default_file_name() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format!("Flashshot {}.png", timestamp(seconds))
}

/// `YYYY-MM-DD at HH.MM.SS` in UTC, mirroring macOS screenshot naming.
fn timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} at {:02}.{:02}.{:02}",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil conversion.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_polygon_paints_its_interior_only() {
        let mut img = RgbaImage::new(10, 10);
        fill_polygon(
            &mut img,
            &[
                V2::new(2.0, 2.0),
                V2::new(8.0, 2.0),
                V2::new(8.0, 8.0),
                V2::new(2.0, 8.0),
            ],
            [255, 0, 0, 255],
        );
        assert_eq!(img.get_pixel(5, 5)[0], 255);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn fill_polygon_respects_alpha() {
        let mut img = RgbaImage::new(4, 4);
        fill_polygon(
            &mut img,
            &[
                V2::new(0.0, 0.0),
                V2::new(4.0, 0.0),
                V2::new(4.0, 4.0),
                V2::new(0.0, 4.0),
            ],
            [0, 0, 0, 102],
        );
        assert!(img.get_pixel(2, 2)[0] < 90);
    }

    #[test]
    fn civil_dates_are_correct() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(timestamp(0), "1970-01-01 at 00.00.00");
        assert_eq!(timestamp(1_700_000_000), "2023-11-14 at 22.13.20");
    }
}
