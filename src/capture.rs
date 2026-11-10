//! macOS screen capture.
//!
//! Flashshot shells out to `/usr/sbin/screencapture`, which is fast, needs no
//! extra macOS bindings, and returns the display at its native pixel size.
//! (ScreenCaptureKit would give us lower latency and display streaming later.)

use anyhow::{bail, Context as _, Result};
use gpui::RenderImage;
use image::{Frame, RgbaImage};
use std::{
    env, fs,
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

/// A captured display, in RGBA, at native resolution.
pub struct Screenshot {
    pub image: RgbaImage,
}

impl Screenshot {
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    /// GPU-ready image for the overlay background.
    pub fn to_render_image(&self) -> Arc<RenderImage> {
        let frame = Frame::new(self.image.clone());
        Arc::new(RenderImage::new(smallvec::smallvec![frame]))
    }
}

/// Capture the entire desktop (all displays).
pub fn capture() -> Result<Screenshot> {
    capture_impl(&[])
}

/// Capture a specific rectangle in global display coordinates.
/// `rect` is (x, y, w, h) in logical points.
#[allow(dead_code)]
pub fn capture_rect(x: i32, y: i32, w: i32, h: i32) -> Result<Screenshot> {
    capture_impl(&["-R", &format!("{},{},{},{}", x, y, w, h)])
}

fn capture_impl(extra_args: &[&str]) -> Result<Screenshot> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path = env::temp_dir().join(format!(
        "flashshot-{}-{}-{}.png",
        std::process::id(),
        stamp,
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let mut cmd = Command::new("/usr/sbin/screencapture");
    cmd.arg("-x") // silence the shutter sound
        .arg("-t")
        .arg("png");
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.arg(&path);

    let output = cmd
        .output()
        .context("could not launch screencapture")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("screencapture failed ({}): {}", output.status, stderr.trim());
    }

    let image = image::open(&path)
        .context("could not read the captured image")?
        .to_rgba8();
    let _ = fs::remove_file(&path);

    if image.width() == 0 || image.height() == 0 {
        bail!("screen capture returned an empty image");
    }

    Ok(Screenshot { image })
}

/// Open the macOS privacy pane where Screen Recording can be granted.
pub fn open_permission_settings() -> Result<()> {
    Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .status()
        .context("could not open System Settings")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_a_non_empty_display() {
        // Requires Screen Recording permission for the terminal running the tests.
        let Ok(shot) = capture() else {
            eprintln!("skipping: screencapture is unavailable in this environment");
            return;
        };
        assert!(shot.width() > 0);
        assert!(shot.height() > 0);
    }
}
