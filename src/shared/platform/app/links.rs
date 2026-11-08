/// Opens a link where the platform can show it: the system browser on desktop,
/// an `ACTION_VIEW` intent on Android.
pub fn open(url: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        gpui_mobile::android::jni::open_url(url)
    }

    #[cfg(not(target_os = "android"))]
    {
        open::that(url).map_err(|error| error.to_string())
    }
}
