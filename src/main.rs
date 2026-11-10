#[cfg(not(target_os = "android"))]
use std::sync::Arc;

extern crate gpui_kit as gpui;

#[cfg(not(target_os = "android"))]
use gpui::{
    App, AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowDecorations, WindowOptions,
    px, size,
};

#[cfg(target_os = "linux")]
use gpui::{GraphicalEnvironment, WindowingRequest};
#[cfg(not(target_os = "android"))]
use jondo::shell::app::Jondo;

#[cfg(target_os = "android")]
fn main() {}

#[cfg(not(target_os = "android"))]
fn main() {
    tune();
    jondo::shared::storage::use_home_from_env();
    jondo::shared::logs::install();

    let mut application = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    // The store is read once: the windowing request needs one flag from it, and
    // parsing the state twice only delays the first frame.
    let store = jondo::shared::storage::Store::load();

    #[cfg(target_os = "linux")]
    if let Some(request) = windowing(&store) {
        application = application.with_windowing(request);
    }

    application.run(|cx: &mut App| {
        gpui_kit::init(cx);
        jondo::shared::ui::views::design::theme::install(cx);

        let bounds = Bounds::centered(None, size(px(1120.0), px(920.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(520.0), px(520.0))),
            titlebar: Some(TitlebarOptions {
                title: Some("Jondo".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            app_id: Some("jondo".into()),
            window_decorations: Some(WindowDecorations::Client),
            icon: icon(),
            ..Default::default()
        };

        let (_, view) = gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Jondo::boot(store, window, cx))
        })
        .expect("window");

        cx.on_window_closed(move |cx, _| {
            view.update(cx, |app, cx| app.shutdown(cx));
        })
        .detach();
    });
}

/// Only a pin needs X11: a client cannot sit above the others on Wayland. A
/// Wayland session therefore keeps its native backend, where the compositor
/// drives a resize edge, and asks for X11 only while the pin is on.
///
/// The request names the backend instead of editing `WAYLAND_DISPLAY`, because
/// every program this process launches inherits that environment: a browser
/// opened with the variable gone cannot reach the compositor, and `xdg-open`
/// then starts nothing at all.
#[cfg(target_os = "linux")]
fn windowing(store: &jondo::shared::storage::Store) -> Option<WindowingRequest> {
    let wayland = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET"]
        .into_iter()
        .any(|key| std::env::var_os(key).is_some_and(|value| !value.is_empty()));
    let x11 = std::env::var_os("DISPLAY").filter(|value| !value.is_empty());
    let pinned = wayland && x11.is_some() && store.always_on_top;

    jondo::shared::platform::pin::on_x11(!wayland || pinned);

    pinned.then(|| {
        WindowingRequest::Windowed(GraphicalEnvironment {
            wayland_display: None,
            x11_display: x11,
            xdg_runtime_dir: std::env::var_os("XDG_RUNTIME_DIR"),
            activation_token: None,
        })
    })
}

#[cfg(not(target_os = "android"))]
fn icon() -> Option<Arc<image::RgbaImage>> {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?;
    Some(Arc::new(image.to_rgba8()))
}

/// glibc grows one malloc arena per thread and every arena keeps its own heap.
/// On a UI process with dozens of short-lived threads that cost tens of MB of
/// resident pages, all of them almost empty. Two arenas are plenty.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn tune() {
    // SAFETY: mallopt only reads its arguments and touches the allocator's own
    // tunables; it is safe to call before any other thread exists.
    unsafe {
        libc::mallopt(libc::M_ARENA_MAX, 2);
    }
}

#[cfg(all(
    not(target_os = "android"),
    not(all(target_os = "linux", target_env = "gnu"))
))]
fn tune() {}
