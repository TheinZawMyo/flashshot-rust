# Development Plan: macOS Screen Capture Tool using GPUI & Rust

This document outlines the architecture, roadmap, and core component strategy for building a lightweight, high-performance screen capture and annotation utility (similar to Lightshot) for macOS (optimized for Apple Silicon / M2) using **GPUI** and **Rust**.

---

## 1. Project Overview & Key Features

* **Target OS:** macOS (Apple Silicon M1/M2/M3 native)
* **Framework & Language:** GPUI, Rust
* **Core Functionality:**
  * Global Hotkey activation (e.g., `Cmd + Shift + 9` or `PrintScreen`).
  * Instant full-screen screenshot freeze with transparent overlay.
  * Drag-to-select region crop tool.
  * On-screen annotation tools (Arrow, Rectangle, Freehand Brush, Text, Highlight, Redact/Blur).
  * Floating action toolbar with quick controls (Save, Copy to Clipboard, Close, Upload).
  * Native macOS menu bar / status bar app support.

---

## 2. Architecture & Technology Stack

| Layer | Technology / Library | Purpose |
| :--- | :--- | :--- |
| **UI & Rendering** | `gpui` | Hardware-accelerated desktop UI, overlays, canvas drawing, and event handling. |
| **Screen Capture** | `ScreenCaptureKit` (or `core-graphics`) | Fast, native macOS desktop image capture API. |
| **Global Hotkey** | `global-hotkey` / `rdev` | Register system-wide key shortcuts to trigger screenshot mode. |
| **System Clipboard**| `arboard` / `copypasta` | Copying image bytes directly to macOS clipboard. |
| **Image Encoding** | `image` crate | Processing PNG/JPEG encoding and image cropping. |
| **Status Bar Menu**| `tray-icon` / `tao` | System tray icon for background execution. |

---

## 3. Development Milestones & Roadmap

### Milestone 1: Environment Setup & Screen Capture Engine
- [ ] Initialize Rust workspace with `gpui` and necessary macOS bindings.
- [ ] Build a native macOS Screen Capture wrapper using `ScreenCaptureKit` or `CoreGraphics`.
- [ ] Implement a full-screen image capture trigger that grabs the main display buffer into an in-memory image container.

### Milestone 2: Overlay Window & Selection Box (GPUI)
- [ ] Set up a frameless, transparent, borderless GPUI `Window` covering the entire display bounds.
- [ ] Render the captured screenshot background with a dimmed overlay layer (`rgba(0,0,0,0.4)`).
- [ ] Add mouse event listeners (`MouseDown`, `MouseDrag`, `MouseUp`) to calculate selection coordinates ($X_1, Y_1, X_2, Y_2$).
- [ ] Highlight the selected region in real-time by clearing the dim mask over the crop box.

### Milestone 3: Annotation & Drawing Canvas
- [ ] Implement state management for active tools (Arrow, Rect, Pen, Text).
- [ ] Build interactive drawing overlays inside the selection region:
  - **Freehand Line (Pen):** Store vector path points and draw strokes.
  - **Rectangle / Arrow:** Calculate start/end vectors and render shapes.
  - **Text Input:** Inline editable text box overlay.
  - **Blur / Censor:** Apply pixelation or solid mask to selected area.
- [ ] Add Undo (`Cmd+Z`) stack for annotations.

### Milestone 4: Floating Action Toolbar & Controls
- [ ] Render a floating GPUI UI bar anchored to the bottom-right corner of the crop selection.
- [ ] Add interactive buttons:
  - **Copy to Clipboard (`Cmd+C`):** Crop image + annotations and write to `arboard`.
  - **Save to File (`Cmd+S`):** Trigger macOS File Dialog to save PNG.
  - **Cancel (`Esc`):** Close overlay window and release screen state.

### Milestone 5: Background Daemon & Polish
- [ ] Convert App into a background Menu Bar utility (`tray-icon`).
- [ ] Integrate global hotkeys for instant launch.
- [ ] Measure and optimize memory consumption (target: < 30 MB RAM) and capture-to-render latency (target: < 50ms).

---

## 4. Key Implementation Detail: Selection Logic Flow

```
[Global Hotkey Pressed]
         │
         ▼
[1. Trigger Screen Capture] ──► Capture macOS Frame Buffer
         │
         ▼
[2. Open GPUI Window]     ──► Full-Screen, Frameless Overlay
         │
         ▼
[3. User Drags Mouse]    ──► Calculate Crop Box (X, Y, W, H)
         │
         ▼
[4. Draw Annotations]     ──► Vector overlays rendered via GPUI Canvas
         │
         ▼
[5. Click Action Button]  ──► Export Cropped Image + Annotations
```

---

## 5. Potential Challenges & Solutions

1. **Multi-Monitor Setup:**
   * *Challenge:* Capturing and opening overlays across multiple Retina/non-Retina displays.
   * *Solution:* Enumerate displays using macOS CoreGraphics API and create separate borderless GPUI windows mapped to each display's specific bounds and Scale Factor.
2. **Permission Handling:**
   * *Challenge:* macOS requires explicit Screen Recording permissions.
   * *Solution:* Add permission check and request logic on first startup using macOS system settings prompt.