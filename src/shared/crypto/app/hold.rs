//! Keeps the process alive while a browser holds the answer.
//!
//! The Spotify redirect lands on a loopback listener inside this process. On
//! Android a backgrounded app is frozen seconds after the browser takes the
//! foreground, and a frozen listener never answers the redirect: the login then
//! dies at the timeout with nothing in the log. A dataSync foreground service
//! keeps the process out of the cached state for as long as the flow runs.

/// Holds the Android foreground service for as long as it lives. A no-op on
/// every other platform.
pub struct Hold {
    #[cfg(target_os = "android")]
    _keep: (),
}

impl Hold {
    pub fn start() -> Self {
        #[cfg(target_os = "android")]
        if let Err(error) = call("start") {
            log::warn!("login hold: cannot start: {error}");
        }

        Self {
            #[cfg(target_os = "android")]
            _keep: (),
        }
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        if let Err(error) = call("stop") {
            log::warn!("login hold: cannot stop: {error}");
        }
    }
}

#[cfg(target_os = "android")]
fn call(method: &str) -> Result<(), String> {
    use gpui_mobile::android::jni::{activity, find_app_class, with_env};

    let name = jni22::strings::JNIString::from(method);
    let signature = jni22::signature::RuntimeMethodSignature::from_str("(Landroid/app/Activity;)V")
        .map_err(|error| error.to_string())?;
    let signature = signature.method_signature();

    with_env(|env| {
        let activity = activity(env)?;
        let class = find_app_class(env, "com.jondo.jondo.LoginHold")?;
        env.call_static_method(
            &class,
            &name,
            &signature,
            &[jni22::JValue::Object(&activity)],
        )
        .map(drop)
        .map_err(|error| {
            env.exception_clear();
            error.to_string()
        })
    })
}
