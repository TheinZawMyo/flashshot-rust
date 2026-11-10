//! Flashshot — a lightweight macOS screenshot & annotation tool built on GPUI.
//!
//! Press ⌘⇧9 anywhere to freeze the screen, drag a region, annotate it, then
//! copy (⌘C) or save (⌘S).

mod capture;
mod export;
mod model;
mod overlay;
mod shapes;
mod text_render;
mod welcome;

use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use gpui::{
    App, Application, Bounds, Pixels, PlatformDisplay, Timer, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, prelude::*,
};

/// Shown in the welcome window and the README.
pub const HOTKEY_LABEL: &str = "⌘ ⇧ 9";

static CAPTURING: AtomicBool = AtomicBool::new(false);

fn main() {
    Application::new().run(|cx: &mut App| {
        let hotkey_error = register_hotkey().err();
        setup_tray(cx);
        welcome::open(cx, hotkey_error);
        poll_hotkey(cx);
        cx.activate(true);
    });
}

/// Create a macOS menu bar item so Flashshot runs in the background.
fn setup_tray(_cx: &mut App) {
    use tray_icon::{
        Icon, TrayIconBuilder,
        menu::{Menu, MenuItem},
    };

    let capture_item = MenuItem::with_id("capture_screen", "Capture Screen    ⌘⇧9", true, None);
    let quit_item = MenuItem::with_id("quit_flashshot", "Quit Flashshot", true, None);

    let menu = Menu::with_items(&[&capture_item, &quit_item]).expect("tray menu");

    // Draw at 2x resolution for a crisp 18pt macOS status bar icon.
    let icon = Icon::from_rgba(flashshot_tray_icon(), 36, 36).expect("tray icon");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_icon_as_template(false)
        .with_tooltip("Flashshot")
        .build()
        .expect("build tray icon");

    // Keep the tray alive for the lifetime of the app.
    std::mem::forget(tray);
}

/// Blue capture tile with a white lightning mark, rendered as RGBA pixels.
fn flashshot_tray_icon() -> Vec<u8> {
    const SIZE: usize = 36;
    const SCALE: f32 = 2.0;
    const BOLT: [(f32, f32); 7] = [
        (10.5, 2.5),
        (5.5, 10.5),
        (8.5, 10.5),
        (7.0, 15.5),
        (12.5, 8.0),
        (9.5, 8.0),
        (12.0, 2.5),
    ];

    let mut pixels = vec![0; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = (x as f32 + 0.5) / SCALE;
            let py = (y as f32 + 0.5) / SCALE;

            // Rounded 18px tile with an 8px corner radius.
            let nearest_x = px.clamp(4.0, 14.0);
            let nearest_y = py.clamp(4.0, 14.0);
            let dx = px - nearest_x;
            let dy = py - nearest_y;
            let in_tile = dx * dx + dy * dy <= 4.0 * 4.0;
            if !in_tile {
                continue;
            }

            let inside_bolt = point_in_polygon(px, py, &BOLT);
            let color = if inside_bolt {
                [255, 255, 255, 255]
            } else {
                [52, 112, 245, 255]
            };
            let offset = (y * SIZE + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&color);
        }
    }
    pixels
}

fn point_in_polygon(x: f32, y: f32, polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let (xi, yi) = polygon[current];
        let (xj, yj) = polygon[previous];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn register_hotkey() -> Result<(), String> {
    use global_hotkey::{
        GlobalHotKeyManager,
        hotkey::{Code, HotKey, Modifiers},
    };

    let manager = GlobalHotKeyManager::new().map_err(|error| error.to_string())?;
    let hotkey = HotKey::new(Some(Modifiers::META | Modifiers::SHIFT), Code::Digit9);
    manager
        .register(hotkey)
        .map_err(|error| format!("could not register {HOTKEY_LABEL}: {error}"))?;

    // The Carbon event handler lives as long as the manager, so keep it for the
    // whole process rather than dropping it at the end of this function.
    std::mem::forget(manager);
    Ok(())
}

/// GPUI has no platform hook for system-wide shortcuts, so poll the channel
/// that `global-hotkey` fills from its Carbon callback.
fn poll_hotkey(cx: &mut App) {
    use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
    use tray_icon::menu::MenuEvent;

    cx.spawn(async move |cx| loop {
        Timer::after(Duration::from_millis(60)).await;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Pressed {
                let _ = cx.update(|cx| start_capture(cx));
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                "capture_screen" => {
                    let _ = cx.update(|cx| start_capture(cx));
                }
                "quit_flashshot" => {
                    let _ = cx.update(|cx| cx.quit());
                }
                _ => {}
            }
        }
    })
    .detach();
}

/// Hide everything, capture each display at its native pixel density, then show overlays.
pub fn start_capture(cx: &mut App) {
    if CAPTURING.swap(true, Ordering::SeqCst) {
        return;
    }

    // Enumerate all displays for multi-monitor setups.
    let displays: Vec<Rc<dyn PlatformDisplay>> = cx.displays();
    if displays.is_empty() {
        CAPTURING.store(false, Ordering::SeqCst);
        welcome::open(cx, Some("No display was available to capture.".into()));
        return;
    }

    // Collect display bounds before hiding windows.
    let display_bounds: Vec<((i32, i32, i32, i32), Bounds<Pixels>)> = displays
        .iter()
        .map(|d| {
            let b = d.bounds();
            (
                (
                    Into::<f32>::into(b.origin.x) as i32,
                    Into::<f32>::into(b.origin.y) as i32,
                    Into::<f32>::into(b.size.width) as i32,
                    Into::<f32>::into(b.size.height) as i32,
                ),
                b,
            )
        })
        .collect();

    // Our own windows must not be part of the screenshot.
    for handle in cx.windows() {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
    cx.hide();

    let capture_bounds = display_bounds.clone();
    let task = cx.background_spawn(async move {
        // Give the window server a moment to actually take our windows down.
        std::thread::sleep(Duration::from_millis(140));
        capture_bounds
            .iter()
            .map(|((x, y, w, h), _)| capture::capture_rect(*x, *y, *w, *h))
            .collect::<anyhow::Result<Vec<_>>>()
    });

    cx.spawn(async move |cx| {
        let result = task.await;
        let _ = cx.update(|cx| {
            CAPTURING.store(false, Ordering::SeqCst);
            match result {
                Ok(screenshots) => {
                    for (screenshot, (_, bounds)) in screenshots.iter().zip(&display_bounds) {
                        if let Err(error) = open_overlay(cx, screenshot, *bounds) {
                            welcome::open(
                                cx,
                                Some(format!(
                                    "Could not open overlay on display: {error}"
                                )),
                            );
                        }
                    }
                }
                Err(error) => welcome::open(
                    cx,
                    Some(format!(
                        "Screen capture failed: {error}. Grant Flashshot (or your terminal) \
                         Screen Recording permission and try again."
                    )),
                ),
            }
        });
    })
    .detach();
}

fn open_overlay(
    cx: &mut App,
    screenshot: &capture::Screenshot,
    bounds: Bounds<Pixels>,
) -> anyhow::Result<()> {
    cx.activate(true);
    let captured = capture::Screenshot { image: screenshot.image.clone() };

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: None,
            }),
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        },
        move |window, cx| {
            cx.new(|cx| overlay::Overlay::new(window, cx, captured, bounds))
        },
    )?;
    Ok(())
}
