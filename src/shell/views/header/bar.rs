#[cfg(not(target_os = "android"))]
use crate::shared::i18n::t;
use crate::shared::ui::views::components::paint::Scope;
use crate::shared::ui::views::design::icons;
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route};
#[cfg(not(target_os = "android"))]
use gpui::StatefulInteractiveElement;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Context, Div, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Rgba, Styled, Window, div,
};

pub(super) fn bar(
    app: &mut Jondo,
    _window: &mut Window,
    cx: &mut Context<Jondo>,
    narrow: bool,
    icon: f32,
) -> Div {
    #[cfg(not(target_os = "android"))]
    let (control, glyph) = {
        let scale = _window.scale_factor();
        (even(40.0, scale), even(14.0, scale))
    };

    let mut view = div()
        .relative()
        .w_full()
        .h(theme::unit(theme::HEADER_HEIGHT))
        .flex()
        .items_center()
        .gap(theme::unit(4.0))
        .pl(theme::unit(4.0))
        .bg(theme::SURFACE);

    let can_back = app.history.can_back();
    let can_forward = app.history.can_forward();
    view = view.child(nav_icon(
        "back",
        icons::ARROW_LEFT,
        icon,
        if can_back {
            theme::MUTED
        } else {
            theme::alpha(theme::MUTED, 0.3)
        },
        can_back,
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.back(cx);
            cx.notify();
        }),
    ));
    view = view.child(nav_icon(
        "forward",
        icons::ARROW_RIGHT,
        icon,
        if can_forward {
            theme::MUTED
        } else {
            theme::alpha(theme::MUTED, 0.3)
        },
        can_forward,
        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.forward(cx);
            cx.notify();
        }),
    ));

    if !narrow {
        view = view.child(underline(
            app.route == Route::Home,
            app.accent,
            nav_icon(
                "library",
                icons::LIST_MUSIC,
                icon,
                if app.route == Route::Home {
                    app.accent
                } else {
                    theme::MUTED
                },
                app.route != Route::Home,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.open(Route::Home, cx);
                    cx.notify();
                }),
            ),
        ));
        view = view.child(underline(
            app.route == Route::Following,
            app.accent,
            nav_icon(
                "following",
                icons::USERS,
                icon,
                if app.route == Route::Following {
                    app.accent
                } else {
                    theme::MUTED
                },
                app.route != Route::Following,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.open(Route::Following, cx);
                    cx.notify();
                }),
            ),
        ));
        let rooms_color = if app.route == Route::Rooms {
            app.accent
        } else if app.rooms.read(cx).live() {
            theme::GREEN
        } else if app.rooms.read(cx).active() {
            theme::AMBER
        } else {
            theme::MUTED
        };

        view = view.child(underline(
            app.route == Route::Rooms,
            app.accent,
            nav_icon(
                "rooms",
                icons::TURNTABLE,
                icon,
                rooms_color,
                app.route != Route::Rooms,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.open(Route::Rooms, cx);
                    cx.notify();
                }),
            ),
        ));
    }

    #[cfg(not(target_os = "android"))]
    {
        view = view.child(settings_item(app, icon, cx));
    }

    // The empty middle of the bar drags the window. It is a flex item rather
    // than an overlay, so it starts and ends exactly where the controls do and
    // never swallows their clicks. A phone has no window to move.
    let drag = div().flex_1().h_full();
    #[cfg(not(target_os = "android"))]
    let drag = drag.on_mouse_down(
        MouseButton::Left,
        cx.listener(|_, event: &gpui::MouseDownEvent, window, _| {
            if event.click_count == 2 {
                window.zoom_window();
            } else {
                window.start_window_move();
            }
        }),
    );
    view = view.child(drag);

    view = view.child(
        div()
            .absolute()
            .left(theme::unit(0.0))
            .right(theme::unit(0.0))
            .top(theme::unit(0.0))
            .h(theme::unit(theme::HEADER_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(crate::shared::ui::views::components::button::icon_flat_in(
                        Scope::Shell,
                        "search-toggle",
                        icons::SEARCH_BOLD,
                        icon,
                        theme::MUTED,
                        32.0,
                        true,
                        cx.listener(|app, _: &gpui::ClickEvent, window, cx| {
                            if app.ui.search_open {
                                app.ui.search_open = false;
                                app.ui.search_closing = Some(std::time::Instant::now());
                                app.focus.focus(window, cx);
                            } else {
                                app.ui.search_open = true;
                                app.ui.search_closing = None;
                                if let Some(input) = app.fields.get("search").cloned() {
                                    input.update(cx, |state, cx| {
                                        state.focus_handle(cx).focus(window, cx)
                                    });
                                }
                            }

                            cx.notify();
                        }),
                    )),
            ),
    );

    // Android has no window controls: the system owns close, and a pin is an
    // X11 affordance. Settings moves to the right end of the bar instead.
    let controls = div().flex().items_center();
    #[cfg(target_os = "android")]
    let controls = controls.child(settings_item(app, icon, cx));
    #[cfg(not(target_os = "android"))]
    let controls = {
        let on_top = app.store.always_on_top && crate::shared::platform::pin::pinnable();
        controls
            .child(pin_control(
                on_top,
                control,
                glyph,
                cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                    app.toggle_pin(cx);
                    cx.notify();
                }),
            ))
            .child(window_control(
                "minimize",
                icons::MINUS,
                false,
                control,
                glyph,
                cx.listener(|_, _: &gpui::ClickEvent, window, _| {
                    window.minimize_window();
                }),
            ))
            .child(window_control(
                "maximize",
                icons::COPY,
                false,
                control,
                glyph,
                cx.listener(|_, _: &gpui::ClickEvent, window, _| {
                    window.zoom_window();
                }),
            ))
            .child(window_control(
                "close",
                icons::X,
                true,
                control,
                glyph,
                cx.listener(|_, _: &gpui::ClickEvent, window, _| {
                    window.remove_window();
                }),
            ))
    };

    view.child(controls)
}

/// Window chrome sizes snap to an even number of device pixels: an odd size
/// makes the centred glyph land half a pixel off its button when the display
/// uses a fractional scale factor.
#[cfg(not(target_os = "android"))]
fn even(size: f32, scale: f32) -> f32 {
    ((size * scale / 2.0).round() * 2.0).max(2.0) / scale
}

fn settings_item(app: &Jondo, icon: f32, cx: &mut Context<Jondo>) -> Div {
    let active = app.route == Route::Settings;
    underline(
        active,
        app.accent,
        nav_icon(
            "settings",
            icons::SETTINGS,
            icon,
            if active { app.accent } else { theme::MUTED },
            !active,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.open(Route::Settings, cx);
                cx.notify();
            }),
        ),
    )
}

fn nav_icon(
    id: &'static str,
    icon: crate::shared::ui::views::design::icons::Icon,
    size: f32,
    color: Rgba,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    crate::shared::ui::views::components::button::icon_flat_in(
        Scope::Shell,
        id,
        icon,
        size,
        color,
        40.0,
        enabled,
        on_click,
    )
}

fn underline(active: bool, accent: Rgba, inner: impl IntoElement) -> Div {
    div().relative().child(inner).when(active, |this| {
        this.child(
            div()
                .absolute()
                .left(theme::unit(0.0))
                .right(theme::unit(0.0))
                .top(theme::unit(0.0))
                .h(theme::unit(2.0))
                .bg(accent),
        )
    })
}

#[cfg(not(target_os = "android"))]
fn window_control(
    id: &'static str,
    icon: icons::Icon,
    close: bool,
    width: f32,
    glyph: f32,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let base = theme::MUTED;
    let target = if close { theme::DANGER } else { theme::TEXT };
    let hover_bg = if close {
        theme::alpha(theme::DANGER, 0.1)
    } else {
        theme::white(0.05)
    };

    let label = icon.label();
    crate::shared::ui::views::components::hover::hover_in(
        Scope::Shell,
        id,
        true,
        move |fade| {
            div()
                .w(theme::unit(width))
                .h(theme::unit(theme::HEADER_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .bg(theme::mix(theme::white(0.0), hover_bg, fade))
                .child(icons::icon(icon, glyph, theme::mix(base, target, fade)))
        },
        on_click,
    )
    .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(label).build(window, cx))
}

#[cfg(not(target_os = "android"))]
fn pin_control(
    on: bool,
    width: f32,
    glyph: f32,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let base = if on { theme::TEXT } else { theme::MUTED };
    crate::shared::ui::views::components::hover::hover_in(
        Scope::Shell,
        "pin",
        true,
        move |fade| {
            let color = theme::mix(base, theme::TEXT, fade);
            let glyph = if on {
                icons::icon(icons::PIN, glyph, color).into_any_element()
            } else {
                icons::icon(icons::PIN, glyph, color)
                    .with_transformation(gpui::Transformation::rotate(gpui::Radians(
                        25f32.to_radians(),
                    )))
                    .into_any_element()
            };

            div()
                .w(theme::unit(width))
                .h(theme::unit(theme::HEADER_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .bg(theme::mix(theme::white(0.0), theme::white(0.05), fade))
                .child(if on {
                    div().mt(theme::unit(1.0)).child(glyph).into_any_element()
                } else {
                    glyph
                })
        },
        on_click,
    )
    .tooltip(move |window, cx| {
        gpui_kit::component::tooltip::Tooltip::new(if on {
            t("Unpin window")
        } else {
            t("Pin window")
        })
        .build(window, cx)
    })
}
