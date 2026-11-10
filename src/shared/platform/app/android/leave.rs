//! Leaving the app from its first page.
//!
//! GPUI consumes every key event, so the system back gesture never reaches the
//! Activity's own `onBackPressed`; the app has to background its task itself.

const CLASS: &str = "com/jondo/jondo/MainActivity";
const BACKGROUND: &str = "(Landroid/app/Activity;)Ljava/lang/String;";

/// Moves the task to the background, which is what the system back gesture does
/// at the root: the task stays in recents and playback keeps running.
pub fn background() {
    let _ = crate::shared::platform::android::java::call(CLASS, "background", BACKGROUND, &[]);
}
