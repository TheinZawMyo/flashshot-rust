//! Full-screen capture overlay: region selection, annotation tools, action bar.

use std::{sync::Arc, time::Duration};

use gpui::{
    Bounds, ClickEvent, Context, CursorStyle, FocusHandle, Hsla, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels,
    Point, Render, SharedString, Size, Timer, Window, canvas, div, point,
    prelude::*, px, rgb, rgba, transparent_black,
};

use crate::{
    capture::Screenshot,
    export,
    model::{Annotation, PALETTE, TEXT_SIZE, Tool},
    shapes::{self, RectF, V2},
};

const TOOLBAR_W: f32 = 510.0;
const TOOLBAR_H: f32 = 44.0;
const MARGIN: f32 = 10.0;

#[derive(Copy, Clone)]
enum Drag {
    None,
    Select(V2),
    Annotate,
}

struct TextDraft {
    origin: V2,
    text: String,
}

pub struct Overlay {
    screenshot: Arc<Screenshot>,
    /// Capture pixels per overlay point (2.0 on a Retina display).
    scale: f32,
    viewport: Size<Pixels>,
    focus: FocusHandle,
    tool: Tool,
    color: [u8; 4],
    stroke_width: f32,
    selection: Option<RectF>,
    annotations: Vec<Annotation>,
    /// Undo history — each entry is a snapshot of annotations before an action.
    undo_stack: Vec<Vec<Annotation>>,
    draft: Option<Annotation>,
    drag: Drag,
    text_draft: Option<TextDraft>,
    status: Option<SharedString>,
}

impl Overlay {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        screenshot: Screenshot,
        display_bounds: Bounds<Pixels>,
    ) -> Self {
        let logical_width: f32 = display_bounds.size.width.into();
        let scale = if logical_width > 0.0 {
            screenshot.width() as f32 / logical_width
        } else {
            1.0
        };

        let focus = cx.focus_handle();
        focus.focus(window);

        Self {
            screenshot: Arc::new(screenshot),
            scale,
            viewport: window.viewport_size(),
            focus,
            tool: Tool::Select,
            color: PALETTE[0],
            stroke_width: 4.0,
            selection: None,
            annotations: Vec::new(),
            undo_stack: Vec::new(),
            draft: None,
            drag: Drag::None,
            text_draft: None,
            status: None,
        }
    }

    fn viewport_rect(&self) -> RectF {
        RectF::new(
            0.0,
            0.0,
            self.viewport.width.into(),
            self.viewport.height.into(),
        )
    }

    fn effective_selection(&self) -> RectF {
        self.selection.unwrap_or_else(|| self.viewport_rect())
    }

    fn toolbar_rect(&self) -> RectF {
        let Some(selection) = self.selection else {
            return RectF::empty();
        };
        let viewport = self.viewport_rect();
        let x = (selection.right() - TOOLBAR_W)
            .clamp(MARGIN, (viewport.w - TOOLBAR_W - MARGIN).max(MARGIN));
        let below = selection.bottom() + MARGIN;
        let y = if below + TOOLBAR_H <= viewport.h - MARGIN {
            below
        } else {
            (selection.y - TOOLBAR_H - MARGIN).max(MARGIN)
        };
        RectF::new(x, y, TOOLBAR_W, TOOLBAR_H)
    }

    fn point_of(&self, position: Point<Pixels>) -> V2 {
        V2::new(position.x.into(), position.y.into())
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let position = self.point_of(event.position);

        if self.toolbar_rect().contains(position) {
            return;
        }
        if self.text_draft.is_some() {
            self.commit_text(cx);
            return;
        }

        if self.tool == Tool::Text {
            let Some(selection) = self.selection else {
                return;
            };
            if selection.contains(position) {
                self.text_draft = Some(TextDraft {
                    origin: selection.clamp_point(position),
                    text: String::new(),
                });
                cx.notify();
            }
            return;
        }

        if self.tool == Tool::Select {
            self.selection = Some(RectF::new(position.x, position.y, 0.0, 0.0));
            self.annotations.clear();
            self.draft = None;
            self.drag = Drag::Select(position);
            cx.notify();
            return;
        }

        if let Some(selection) = self.selection {
            if selection.contains(position) {
                let start = selection.clamp_point(position);
                self.draft = Annotation::new(self.tool, start, self.color, self.stroke_width);
                self.drag = Drag::Annotate;
                cx.notify();
            }
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let position = self.viewport_rect().clamp_point(self.point_of(event.position));
        match self.drag {
            Drag::Select(start) => {
                self.selection = Some(RectF::from_points(start, position));
            }
            Drag::Annotate => {
                if let (Some(selection), Some(draft)) = (self.selection, self.draft.as_mut()) {
                    draft.drag_to(selection.clamp_point(position));
                }
            }
            Drag::None => {}
        }
        cx.notify();
    }

    fn on_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match self.drag {
            Drag::Annotate => {
                if let Some(draft) = self.draft.take() {
                    if draft.is_meaningful() {
                        self.undo_stack.push(self.annotations.clone());
                        self.annotations.push(draft);
                    }
                }
            }
            Drag::Select(_) => {
                if let Some(selection) = self.selection {
                    if selection.w < 6.0 || selection.h < 6.0 {
                        self.selection = None;
                    } else {
                        self.selection = Some(selection.clamp_within(self.viewport_rect()));
                    }
                }
            }
            Drag::None => {}
        }
        self.drag = Drag::None;
        cx.notify();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.is_held {
            return;
        }
        let keystroke = &event.keystroke;
        let key = keystroke.key.as_str();

        if keystroke.modifiers.platform {
            match key {
                "z" => self.undo(cx),
                "c" => self.copy(window, cx),
                "s" => self.save(window, cx),
                _ => {}
            }
            return;
        }

        if let Some(draft) = self.text_draft.as_mut() {
            match key {
                "escape" => {
                    self.text_draft = None;
                    cx.notify();
                }
                "enter" => self.commit_text(cx),
                "backspace" => {
                    draft.text.pop();
                    cx.notify();
                }
                _ => {
                    if let Some(typed) = &keystroke.key_char {
                        if !typed.is_empty() && !typed.chars().any(char::is_control) {
                            draft.text.push_str(typed);
                            cx.notify();
                        }
                    }
                }
            }
            return;
        }

        match key {
            "escape" => self.commit_text(cx),
            "enter" => self.copy(window, cx),
            _ => {}
        }

        if key == "escape" {
            window.remove_window();
        }
    }

    fn commit_text(&mut self, cx: &mut Context<Self>) {
        if let Some(draft) = self.text_draft.take() {
            if !draft.text.trim().is_empty() {
                self.undo_stack.push(self.annotations.clone());
                self.annotations.push(Annotation::Text {
                    origin: draft.origin,
                    text: draft.text,
                    color: self.color,
                    size: TEXT_SIZE,
                });
            }
            cx.notify();
        }
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        if self.text_draft.take().is_some() {
            cx.notify();
            return;
        }
        if let Some(prev) = self.undo_stack.pop() {
            self.annotations = prev;
            cx.notify();
        }
    }

    fn copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_text(cx);
        let selection = self.effective_selection();
        let image = export::render(&self.screenshot, selection, &self.annotations, self.scale);
        match export::copy_to_clipboard(&image) {
            Ok(()) => {
                self.status = Some("Copied to clipboard".into());
                cx.notify();
                self.close_after(window, cx, 450);
            }
            Err(error) => {
                self.status = Some(format!("Copy failed: {error}").into());
                cx.notify();
            }
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_text(cx);
        let selection = self.effective_selection();
        let image = export::render(&self.screenshot, selection, &self.annotations, self.scale);
        match export::prompt_and_save(&image) {
            Ok(Some(path)) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.status = Some(format!("Saved {name}").into());
                cx.notify();
                self.close_after(window, cx, 650);
            }
            Ok(None) => {
                self.status = None;
                cx.notify();
            }
            Err(error) => {
                self.status = Some(format!("Save failed: {error}").into());
                cx.notify();
            }
        }
    }

    fn close_after(&self, window: &mut Window, cx: &mut Context<Self>, delay_ms: u64) {
        cx.spawn_in(window, async move |_this, cx| {
            Timer::after(Duration::from_millis(delay_ms)).await;
            let _ = cx.update(|window, _| window.remove_window());
        })
        .detach();
    }

    fn set_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        if self.tool == Tool::Text && tool != Tool::Text {
            self.commit_text(cx);
        }
        self.tool = tool;
        cx.notify();
    }

    fn toolbar_button(
        &self,
        id: &'static str,
        label: &'static str,
        active: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> gpui::AnyElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(32.0))
            .h(px(32.0))
            .rounded(px(7.0))
            .text_size(px(16.0))
            .cursor(CursorStyle::PointingHand)
            .text_color(if active {
                rgb(0xFFFFFF)
            } else {
                rgb(0xCBD5E1)
            })
            .bg(if active {
                rgba(0x3B82F6FF)
            } else {
                rgba(0x00000000)
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                on_click(this, window, cx);
                cx.notify();
            }))
            .child(label)
            .into_any_element()
    }

    fn swatch_button(
        &self,
        index: usize,
        color: [u8; 4],
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let selected = self.color == color;
        div()
            .id(("swatch", index))
            .w(px(18.0))
            .h(px(18.0))
            .rounded(px(5.0))
            .cursor(CursorStyle::PointingHand)
            .bg(rgba(pack(color)))
            .border_1()
            .border_color(if selected {
                rgb(0xFFFFFFFF)
            } else {
                rgba(0xFFFFFF33)
            })
            .when(selected, |el| {
                el.shadow_md() // visual pop for selected
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.color = color;
                cx.notify();
            }))
            .into_any_element()
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let rect = self.toolbar_rect();

        let mut tools = div().flex().items_center().gap(px(2.0));
        for tool in Tool::ALL {
            let active = self.tool == tool;
            tools = tools.child(self.toolbar_button(
                tool.name(),
                tool.glyph(),
                active,
                cx,
                move |this, _, cx| this.set_tool(tool, cx),
            ));
        }

        let mut swatches = div().flex().items_center().gap(px(3.0));
        for (index, color) in PALETTE.iter().enumerate() {
            swatches = swatches.child(self.swatch_button(index, *color, cx));
        }

        div()
            .absolute()
            .left(px(rect.x))
            .top(px(rect.y))
            .w(px(TOOLBAR_W))
            .h(px(TOOLBAR_H))
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .bg(rgba(0x111827F2))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgba(0xFFFFFF26))
            .child(tools)
            .child(div().w(px(1.0)).h(px(22.0)).bg(rgba(0xFFFFFF26)))
            .child(swatches)
            .child(div().w(px(1.0)).h(px(22.0)).bg(rgba(0xFFFFFF26)))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(self.toolbar_button("undo", "↶", false, cx, |this, _, cx| {
                        this.undo(cx)
                    }))
                    .child(self.toolbar_button("copy-action", "⧉", false, cx, |this, window, cx| {
                        this.copy(window, cx)
                    }))
                    .child(self.toolbar_button("save-action", "⤓", false, cx, |this, window, cx| {
                        this.save(window, cx)
                    }))
                    .child(self.toolbar_button("cancel-action", "✕", false, cx, |_, window, _| {
                        window.remove_window()
                    })),
            )
            .into_any_element()
    }
}

fn pack(color: [u8; 4]) -> u32 {
    ((color[0] as u32) << 24)
        | ((color[1] as u32) << 16)
        | ((color[2] as u32) << 8)
        | color[3] as u32
}

fn to_hsla(color: [u8; 4]) -> Hsla {
    rgba(pack(color)).into()
}

impl Render for Overlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.viewport = window.viewport_size();

        let selection = self.selection;
        let annotations = self.annotations.clone();
        let draft = self.draft.clone();
        let viewport = self.viewport_rect();
        let border: Hsla = rgba(0xFFFFFFFF).into();

        let mut text_elements: Vec<gpui::AnyElement> = Vec::new();
        for annotation in annotations.iter().chain(draft.iter()) {
            if let Some((origin, text, color, size)) = annotation.text_parts() {
                text_elements.push(
                    div()
                        .absolute()
                        .left(px(origin.x))
                        .top(px(origin.y))
                        .font_family("Arial")
                        .text_size(px(size))
                        .text_color(to_hsla(color))
                        .whitespace_nowrap()
                        .child(text.to_string())
                        .into_any_element(),
                );
            }
        }
        if let Some(text_draft) = &self.text_draft {
            text_elements.push(
                div()
                    .absolute()
                    .left(px(text_draft.origin.x))
                    .top(px(text_draft.origin.y))
                    .flex()
                    .items_start()
                    .child(
                        div()
                            .font_family("Arial")
                            .text_size(px(TEXT_SIZE))
                            .text_color(to_hsla(self.color))
                            .whitespace_nowrap()
                            .child(text_draft.text.clone()),
                    )
                    .child(
                        div()
                            .w(px(2.0))
                            .h(px(TEXT_SIZE * 1.15))
                            .bg(rgb(0xFFFFFFFF)),
                    )
                    .into_any_element(),
            );
        }

        let mut overlay_elements: Vec<gpui::AnyElement> = Vec::new();
        if let Some(selection) = selection {
            let label = format!(
                "{} × {}",
                selection.w.round() as i32,
                selection.h.round() as i32
            );
            let label_x = selection.x.clamp(0.0, (viewport.w - 90.0).max(0.0));
            let label_y = if selection.y > 30.0 {
                selection.y - 26.0
            } else {
                selection.y + 8.0
            };
            overlay_elements.push(
                div()
                    .absolute()
                    .left(px(label_x))
                    .top(px(label_y))
                    .px(px(8.0))
                    .py(px(3.0))
                    .rounded(px(6.0))
                    .bg(rgba(0x000000CC))
                    .border_1()
                    .border_color(rgba(0xFFFFFF1A))
                    .text_size(px(11.5))
                    .text_color(rgb(0xF3F4F6))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(label)
                    .into_any_element(),
            );
        } else {
            overlay_elements.push(
                div()
                    .absolute()
                    .left(px(viewport.w / 2.0 - 210.0))
                    .top(px(24.0))
                    .w(px(420.0))
                    .flex()
                    .justify_center()
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(10.0))
                    .bg(rgba(0x000000CC))
                    .border_1()
                    .border_color(rgba(0xFFFFFF14))
                    .text_size(px(12.5))
                    .text_color(rgb(0xD1D5DB))
                    .child("Drag to select a region  ·  Esc to cancel")
                    .into_any_element(),
            );
        }
        if let Some(status) = &self.status {
            overlay_elements.push(
                div()
                    .absolute()
                    .left(px(viewport.w / 2.0 - 120.0))
                    .top(px(viewport.h - 60.0))
                    .w(px(240.0))
                    .flex()
                    .justify_center()
                    .px(px(14.0))
                    .py(px(10.0))
                    .rounded(px(10.0))
                    .bg(rgba(0x22C55EF2))
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(0xFFFFFF))
                    .child(status.clone())
                    .into_any_element(),
            );
        }

        let toolbar = if selection.is_some() {
            self.render_toolbar(cx)
        } else {
            div().into_any_element()
        };

        div()
            .relative()
            .size_full()
            .bg(transparent_black())
            .track_focus(&self.focus)
            .cursor(CursorStyle::Crosshair)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(Self::on_mouse_down),
            )
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(
                canvas(
                    move |_, _, _| (),
                    move |_, _, window, _| {
                        if let Some(selection) = selection {
                            for polygon in shapes::rect_outline(selection, 2.0) {
                                paint_polygon(window, &polygon, border);
                            }
                        }

                        for annotation in annotations.iter().chain(draft.iter()) {
                            for (polygon, color) in annotation.polygons() {
                                paint_polygon(window, &polygon, to_hsla(color));
                            }
                        }
                    },
                )
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .size_full(),
            )
            .children(text_elements)
            .children(overlay_elements)
            .child(toolbar)
    }
}

fn paint_polygon(window: &mut Window, polygon: &[V2], color: Hsla) {
    if polygon.len() < 3 {
        return;
    }
    let mut builder = PathBuilder::fill();
    let points: Vec<Point<Pixels>> = polygon
        .iter()
        .map(|p| point(px(p.x), px(p.y)))
        .collect();
    builder.add_polygon(&points, true);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
