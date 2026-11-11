# Flashshot

Flashshot is a lightweight macOS screenshot and annotation app built with Rust and GPUI. It runs from the menu bar and lets you select a screen area, mark it up, then copy or save it.

## Features

- Capture from the menu bar or with **⌘⇧9**.
- Drag to select an area while the original screen remains visible.
- Annotate with a pen, arrows, rectangles, highlights, redaction blocks, and text.
- Undo annotations with **⌘Z**.
- Copy the selection with **⌘C** or save it as a PNG with **⌘S**.
- Use the menu bar icon to start a capture or quit Flashshot.
- Supports multiple displays.

## Use Flashshot

1. Launch Flashshot. Its blue lightning icon appears in the macOS menu bar.
2. Start a capture by pressing **⌘⇧9**, or open the menu bar icon and choose **Capture Screen**.
3. Drag over the screen to select an area. The selection border and action toolbar appear after you select.
4. Choose an annotation tool from the toolbar and draw inside the selected area.
5. Press **⌘C** to copy the result, or **⌘S** to save it as a PNG. Press **Esc** to cancel.

The menu bar menu also includes **Quit Flashshot**. The shortcut is shown beside **Capture Screen** in that menu.

## Permission

macOS requires Screen Recording access to capture the display. If capture fails, choose **Screen recording access** in the welcome window, then enable Flashshot (or the terminal running it) under **System Settings → Privacy & Security → Screen Recording**. Restart Flashshot after changing permission if macOS asks you to.

## Run from source

You need macOS and a Rust toolchain installed.

```sh
cargo run
```

To build an optimized binary:

```sh
cargo build --release
```

The release binary is written to `target/release/flashshot`.

## Build an Apple Silicon app

On an M1, M2, or later Apple Silicon Mac, create a local `.app` bundle with:

```sh
./scripts/package-macos.sh
```

This creates `dist/Flashshot.app`. Drag it into **Applications** and open it from Finder. The bundle is ad-hoc signed for local use; distributing it to other Macs requires Developer ID signing and notarization.
