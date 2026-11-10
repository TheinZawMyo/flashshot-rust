//! Beautiful launch window: hero section, feature cards, hotkey display, and error handling.

use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, CursorStyle, SharedString, TitlebarOptions,
    Window, WindowBounds, WindowKind, WindowOptions, div, prelude::*, px, rgb, rgba, size,
};

pub struct WelcomeView {
    error: Option<SharedString>,
}

pub fn open(cx: &mut App, error: Option<String>) {
    let bounds = Bounds::centered(None, size(px(560.0), px(540.0)), cx);
    let _ = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Flashshot".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_resizable: false,
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            ..Default::default()
        },
        move |_window, cx| cx.new(|_| WelcomeView { error: error.map(Into::into) }),
    );
    cx.activate(true);
}

impl WelcomeView {
    fn feature_card(icon: String, title: String, desc: String) -> AnyElement {
        div()
            .flex()
            .gap(px(10.0))
            .p(px(14.0))
            .rounded(px(12.0))
            .bg(rgba(0xFFFFFF07))
            .border_1()
            .border_color(rgba(0xFFFFFF12))
            .child(
                div()
                    .w(px(36.0)).h(px(36.0))
                    .flex().items_center().justify_center()
                    .rounded(px(10.0))
                    .bg(rgba(0x3B82F622))
                    .text_size(px(17.0))
                    .child(icon),
            )
            .child(
                div()
                    .flex().flex_col().gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xF3F4F6))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(rgb(0x9CA3AF))
                            .child(desc),
                    ),
            )
            .into_any_element()
    }

    fn hotkey_badge() -> AnyElement {
        div()
            .flex().items_center().gap(px(10.0))
            .child(
                div()
                    .flex().items_center().gap(px(3.0))
                    .px(px(18.0)).py(px(10.0))
                    .rounded(px(12.0))
                    .bg(rgba(0x101A30FF))
                    .border_1().border_color(rgba(0x60A5FA66))
                    .child(
                        div().flex().items_center().gap(px(3.0))
                            .child(key_badge("⌘".into()))
                            .child(key_badge("⇧".into()))
                            .child(key_badge("9".into())),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x6B7280))
                    .child("works from any app"),
            )
            .into_any_element()
    }

    fn action_button(
        &self,
        id: &'static str,
        label: &'static str,
        variant: ButtonVariant,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let (bg, text_c, border_c) = match variant {
            ButtonVariant::Primary => (rgba(0x3B82F6FF), rgb(0xFFFFFF), rgba(0x00000000)),
            ButtonVariant::Secondary => (rgba(0xFFFFFF0D), rgb(0xD1D5DB), rgba(0xFFFFFF14)),
            ButtonVariant::Ghost => (rgba(0x00000000), rgb(0x9CA3AF), rgba(0x00000000)),
        };

        div()
            .id(id)
            .flex().items_center().justify_center()
            .h(px(42.0)).px(px(17.0))
            .rounded(px(10.0))
            .text_size(px(13.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .cursor(CursorStyle::PointingHand)
            .text_color(text_c)
            .bg(bg)
            .border_1().border_color(border_c)
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                on_click(this, window, cx);
                cx.notify();
            }))
            .child(label)
            .into_any_element()
    }
}

fn key_badge(key: String) -> AnyElement {
    div()
        .px(px(6.0)).py(px(4.0)).rounded(px(5.0))
        .bg(rgba(0xFFFFFF18))
        .text_size(px(13.0))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(rgb(0xF3F4F6))
        .child(key)
        .into_any_element()
}

enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut col = div()
            .flex().flex_col()
            .size_full()
            .bg(rgba(0x0B1020FF))
            .font_family("Arial")
            .child(div().h(px(28.0)).w_full())
            .child(
                div()
                    .flex().flex_col().gap(px(18.0))
                    .px(px(32.0)).pb(px(20.0))
                    // Hero
                    .child(
                        div()
                            .flex().flex_col().items_center().gap(px(10.0))
                            .child(
                                div()
                                    .w(px(52.0)).h(px(52.0))
                                    .flex().items_center().justify_center()
                                    .rounded(px(16.0))
                                    .bg(rgba(0x3B82F626))
                                    .border_1().border_color(rgba(0x60A5FA44))
                                    .text_size(px(24.0))
                                    .child("⚡"),
                            )
                            .child(
                                div()
                                    .flex().items_center().gap(px(10.0))
                                    .child(
                                        div()
                                            .text_size(px(28.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(rgb(0xF9FAFB))
                                            .child("Flashshot"),
                                    )
                                    .child(
                                        div()
                                            .px(px(7.0)).py(px(2.0)).rounded(px(5.0))
                                            .bg(rgba(0x3B82F633))
                                            .text_size(px(10.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(rgb(0x93C5FD))
                                            .child("v0.1"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(rgb(0xA8B2C4))
                                    .child("Capture a region, add notes, and share in seconds."),
                            ),
                    )
                    // Hotkey badge
                    .child(div().flex().justify_center().child(Self::hotkey_badge()))
                    // Feature cards
                    .child(
                        div().flex().flex_col().gap(px(8.0))
                            .child(Self::feature_card("⌖".into(), "Select any area".into(), "Drag over the screen to capture exactly what you need.".into()))
                            .child(Self::feature_card("✎".into(), "Annotate your capture".into(), "Add arrows, shapes, freehand marks, text, or redactions.".into()))
                            .child(Self::feature_card("↗".into(), "Copy or save".into(), "Use ⌘C to copy or ⌘S to save your finished capture.".into())),
                    ),
            );

        // Error
        if let Some(error) = &self.error {
            col = col.child(
                div()
                    .flex().gap(px(8.0))
                    .p(px(12.0)).rounded(px(8.0))
                    .mx(px(32.0))
                    .bg(rgba(0x7F1D1D44))
                    .border_1().border_color(rgba(0xF8717180))
                    .child(div().text_size(px(14.0)).child("⚠️"))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(rgb(0xFCA5A5))
                            .child(error.clone()),
                    ),
            );
        }

        col
            .child(
                div()
                    .flex().items_center().gap(px(10.0))
                    .px(px(32.0)).mt(px(2.0))
                    .child(self.action_button("capture-now", "⚡  Capture now", ButtonVariant::Primary, cx, |_, _, cx| crate::start_capture(cx)))
                    .child(self.action_button("permissions", "Screen recording access", ButtonVariant::Secondary, cx, |_, _, _| { let _ = crate::capture::open_permission_settings(); }))
                    .child(self.action_button("quit", "Quit", ButtonVariant::Ghost, cx, |_, _, cx| cx.quit())),
            )
            .child(
                div()
                    .flex().items_center().justify_center().gap(px(6.0))
                    .mt(px(8.0)).pb(px(22.0))
                    .child(div().w(px(7.0)).h(px(7.0)).rounded_full().bg(rgb(0x34D399)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x8491A5))
                            .child("Ready in the menu bar  ·  shortcut  ⌘  ⇧  9"),
                    ),
            )
    }
}
