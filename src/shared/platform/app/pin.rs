//! Always-on-top.
//!
//! Linux: X11 only. Wayland gives a client no way to sit above the others, so
//! the application runs through XWayland there while the pin is on, and turning
//! it on from a Wayland session starts the app again to get there.
//! macOS and Windows: the frame hands over the native window once, and the pin
//! asks the window server directly.

use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(target_os = "linux")]
use std::sync::atomic::AtomicBool;

/// Whether the window lives on X11. Set before the backend is chosen, because
/// the environment no longer says it: a pinned wayland session keeps
/// `WAYLAND_DISPLAY` intact for the programs it launches.
#[cfg(target_os = "linux")]
static X11: AtomicBool = AtomicBool::new(false);

/// The OS window the last frame painted, in whatever form the pin takes: an
/// HWND, an NSView, or an X11 window. Attaching a Wayland surface stores zero.
static WINDOW: AtomicUsize = AtomicUsize::new(0);

/// Remembers the native window a frame painted, so the pin can reach it from
/// settings, which owns no window of its own.
pub fn attach(handle: raw_window_handle::RawWindowHandle) {
    let value = match handle {
        #[cfg(windows)]
        raw_window_handle::RawWindowHandle::Win32(handle) => handle.hwnd.get() as usize,
        #[cfg(target_os = "macos")]
        raw_window_handle::RawWindowHandle::AppKit(handle) => handle.ns_view.as_ptr() as usize,
        #[cfg(target_os = "linux")]
        raw_window_handle::RawWindowHandle::Xcb(handle) => handle.window.get() as usize,
        #[cfg(target_os = "linux")]
        raw_window_handle::RawWindowHandle::Xlib(handle) => handle.window as usize,
        _ => 0,
    };

    WINDOW.store(value, Ordering::Relaxed);
}

#[cfg(target_os = "linux")]
pub fn on_x11(on: bool) {
    X11.store(on, Ordering::Relaxed);
}

#[cfg(target_os = "linux")]
pub fn pinnable() -> bool {
    X11.load(Ordering::Relaxed)
}

/// Whether the pin can only hold after starting again: a Wayland session that
/// has an X server to fall back to. The backend is chosen once, at launch.
#[cfg(target_os = "linux")]
pub fn deferred() -> bool {
    !pinnable() && std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty())
}

#[cfg(all(not(target_os = "linux"), not(target_os = "android")))]
pub fn pinnable() -> bool {
    WINDOW.load(Ordering::Relaxed) != 0
}

#[cfg(all(not(target_os = "linux"), not(target_os = "android")))]
pub fn deferred() -> bool {
    false
}

#[cfg(target_os = "android")]
pub fn pinnable() -> bool {
    false
}

#[cfg(target_os = "android")]
pub fn deferred() -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
pub fn restart() -> bool {
    false
}

/// Starts this program again once the window is gone, so it reads the pin the
/// caller just saved. The new process comes up through XWayland, where the pin
/// is allowed to hold.
#[cfg(target_os = "linux")]
pub fn restart() -> bool {
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };

    super::relaunch::after_exit(&executable).is_ok()
}

#[cfg(target_os = "linux")]
pub fn set(on: bool) -> bool {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        ClientMessageData, ClientMessageEvent, ConnectionExt, EventMask,
    };

    let window = WINDOW.load(Ordering::Relaxed) as u32;
    if window == 0 {
        return false;
    }

    let Ok((connection, screen)) = x11rb::connect(None) else {
        return false;
    };

    let atom = |name: &[u8]| {
        connection
            .intern_atom(false, name)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.atom)
            .unwrap_or(0)
    };

    let state = atom(b"_NET_WM_STATE");
    let above = atom(b"_NET_WM_STATE_ABOVE");
    if state == 0 || above == 0 {
        return false;
    }

    let event = ClientMessageEvent {
        response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window,
        type_: state,
        data: ClientMessageData::from([u32::from(on), above, 0, 1, 0]),
    };

    let root = connection.setup().roots[screen].root;
    let sent = connection
        .send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .is_ok();
    let _ = connection.flush();
    sent
}

#[cfg(target_os = "windows")]
pub fn set(on: bool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };

    let hwnd = WINDOW.load(Ordering::Relaxed) as isize;
    if hwnd == 0 {
        return false;
    }

    let after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    unsafe { SetWindowPos(hwnd, after, 0, 0, 0, 0, flags) != 0 }
}

/// `NSWindow.setLevel:` with `NSFloatingWindowLevel` while pinned, and the
/// normal level when it is released.
#[cfg(target_os = "macos")]
pub fn set(on: bool) -> bool {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    let view = WINDOW.load(Ordering::Relaxed) as *mut AnyObject;
    if view.is_null() {
        return false;
    }

    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        if window.is_null() {
            return false;
        }

        let level: isize = if on { 3 } else { 0 };
        let _: () = msg_send![window, setLevel: level];
    }

    true
}

#[cfg(target_os = "android")]
pub fn set(_on: bool) -> bool {
    false
}
