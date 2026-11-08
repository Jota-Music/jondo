use gpui::{App, AppContext as _, WindowOptions};

use crate::shell::app::Jondo;

/// Called by `android-activity` on a dedicated native thread once the library
/// is loaded. Blocks for the lifetime of the activity: the platform drives the
/// event loop, and the closure runs when the native surface exists.
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("jondo"),
    );
    gpui_mobile::android::jni::install_panic_hook();
    crate::shared::storage::use_home_from_env();

    let _platform = gpui_mobile::android::jni::init_platform(&app);
    let Some(shared) = gpui_mobile::android::jni::shared_platform() else {
        log::error!("jondo: platform unavailable");
        return;
    };

    gpui::Application::with_platform(shared.into_rc())
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            crate::shared::ui::views::design::theme::install(cx);
            super::media::install();

            let options = WindowOptions {
                titlebar: None,
                window_decorations: None,
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                let app = cx.new(|cx| Jondo::new(window, cx));
                cx.new(|cx| gpui_kit::component::Root::new(app, window, cx))
            })
            .expect("window");
        });
}
