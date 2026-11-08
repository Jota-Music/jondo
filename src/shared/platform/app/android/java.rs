//! Calls into the Java glue the APK ships with.
//!
//! Every helper is a static method on a `com.jondo.jondo` class that takes the
//! activity first, then strings, and answers a string: empty or absent means
//! success, a message means the platform refused. Attaching the current thread
//! is the only setup, so the caller decides which thread pays for the hop.

use jni::JavaVM;
use jni::objects::{JObject, JString, JValue};

pub fn call(
    class: &str,
    method: &str,
    signature: &str,
    args: &[&str],
) -> Result<Option<String>, String> {
    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(|error| error.to_string())?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| error.to_string())?;
    let activity = unsafe { JObject::from_raw(context.context().cast()) };

    let strings: Vec<JString> = args
        .iter()
        .map(|value| env.new_string(value))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut values: Vec<JValue> = Vec::with_capacity(strings.len() + 1);
    values.push(JValue::Object(&activity));
    for value in &strings {
        values.push(JValue::Object(value));
    }

    let answer = env
        .call_static_method(class, method, signature, &values)
        .map_err(|error| format!("{method}: {error}"))?;
    let object = answer.l().map_err(|error| error.to_string())?;
    if object.is_null() {
        return Ok(None);
    }

    let text: String = env
        .get_string(&JString::from(object))
        .map_err(|error| error.to_string())?
        .into();
    Ok(Some(text))
}
