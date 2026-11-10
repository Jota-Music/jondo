//! Calls into the Java glue the APK ships with.
//!
//! Every helper is a static method on a `com.jondo.jondo` class that takes the
//! activity first, then strings, and answers a string: empty or absent means
//! success, a message means the platform refused. Attaching the current thread
//! is the only setup, so the caller decides which thread pays for the hop.
//!
//! The class is loaded through the activity's class loader, never with
//! `FindClass`: a thread attached from native code looks classes up with the
//! system loader, which cannot see the APK, and the pending
//! `ClassNotFoundException` aborts the process on the next JNI call.

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};

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

    // `ndk_context` holds the Application, not the Activity android-activity
    // runs. The glue takes an Activity, and methods such as
    // `startActivityForResult` only exist there: the Application fails with
    // IllegalMonitorStateException (or aborts the process under CheckJNI).
    let activity = gpui_mobile::android::jni::activity_as_ptr();
    if activity.is_null() {
        return Err(format!("{method}: no activity"));
    }
    let activity = unsafe { JObject::from_raw(activity.cast()) };

    let class = resolve(&mut env, &activity, class)?;

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

    let answer = match env.call_static_method(&class, method, signature, &values) {
        Ok(answer) => answer,
        Err(error) => {
            describe(&mut env);
            return Err(format!("{method}: {error}"));
        }
    };
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

/// The class the APK ships, as seen by the activity.
fn resolve(env: &mut JNIEnv, activity: &JObject, name: &str) -> Result<GlobalRef, String> {
    let dotted = name.replace('/', ".");
    let loaded = (|| {
        let loader = env
            .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
            .l()?;
        let name = env.new_string(&dotted)?;
        let class = env
            .call_method(
                &loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        env.new_global_ref(JClass::from(class))
    })();

    match loaded {
        Ok(class) => Ok(class),
        Err(error) => {
            describe(env);
            Err(format!("{dotted}: {error}"))
        }
    }
}

/// The throwable's stack trace, so the log says which frame threw and not just
/// its type.
fn trace(env: &mut JNIEnv, throwable: &JObject) -> String {
    let Ok(writer) = env.new_object("java/io/StringWriter", "()V", &[]) else {
        return String::new();
    };
    let Ok(printer) = env.new_object(
        "java/io/PrintWriter",
        "(Ljava/io/Writer;)V",
        &[JValue::Object(&writer)],
    ) else {
        return String::new();
    };

    let _ = env.call_method(
        throwable,
        "printStackTrace",
        "(Ljava/io/PrintWriter;)V",
        &[JValue::Object(&printer)],
    );
    let _ = env.call_method(&printer, "flush", "()V", &[]);
    let text = env
        .call_method(&writer, "toString", "()Ljava/lang/String;", &[])
        .and_then(|value| value.l())
        .map(|value| {
            env.get_string(&JString::from(value))
                .map(String::from)
                .unwrap_or_default()
        })
        .unwrap_or_default();
    if text.is_empty() {
        let _ = env.exception_clear();
    }

    text
}

/// Reports and clears a pending Java exception. Leaving it there aborts the
/// process on the next JNI call, so every failure path has to drop it.
fn describe(env: &mut JNIEnv) {
    let Ok(throwable) = env.exception_occurred() else {
        return;
    };
    if throwable.is_null() {
        return;
    }

    // JNI forbids every other call while an exception is pending, so it has to
    // be cleared before the throwable can be read.
    let _ = env.exception_clear();
    log::error!("java call failed: {}", trace(env, &throwable));
}
