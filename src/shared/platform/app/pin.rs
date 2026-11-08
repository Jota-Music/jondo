//! Always-on-top on X11. Wayland gives a client no way to sit above the others,
//! so the application runs through XWayland there while the pin is on.

#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the window lives on X11. Set before the backend is chosen, because
/// the environment no longer says it: a pinned wayland session keeps
/// `WAYLAND_DISPLAY` intact for the programs it launches.
#[cfg(target_os = "linux")]
static X11: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
pub fn on_x11(on: bool) {
    X11.store(on, Ordering::Relaxed);
}

#[cfg(target_os = "linux")]
pub fn pinnable() -> bool {
    X11.load(Ordering::Relaxed)
}

#[cfg(not(target_os = "linux"))]
pub fn pinnable() -> bool {
    false
}

#[cfg(target_os = "linux")]
pub fn set(on: bool) -> bool {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        AtomEnum, ClientMessageData, ClientMessageEvent, ConnectionExt, EventMask,
    };

    let Ok((connection, screen)) = x11rb::connect(None) else {
        return false;
    };

    let root = connection.setup().roots[screen].root;
    let Ok(cookie) = connection.query_tree(root) else {
        return false;
    };

    let Ok(tree) = cookie.reply() else {
        return false;
    };

    let above = match connection.intern_atom(false, b"_NET_WM_STATE_ABOVE") {
        Ok(cookie) => cookie.reply().map(|reply| reply.atom).unwrap_or(0),
        Err(_) => 0,
    };

    let state = match connection.intern_atom(false, b"_NET_WM_STATE") {
        Ok(cookie) => cookie.reply().map(|reply| reply.atom).unwrap_or(0),
        Err(_) => 0,
    };

    for window in tree.children {
        let Ok(cookie) =
            connection.get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
        else {
            continue;
        };

        let Ok(class) = cookie.reply() else {
            continue;
        };

        let class = String::from_utf8_lossy(&class.value).to_lowercase();
        if !class.contains("jondo") {
            continue;
        }

        let event = ClientMessageEvent {
            response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window,
            type_: state,
            data: ClientMessageData::from([if on { 1 } else { 0 }, above, 0, 1, 0]),
        };

        let _ = connection.send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        );
        let _ = connection.flush();
        return true;
    }

    false
}

#[cfg(not(target_os = "linux"))]
pub fn set(_on: bool) -> bool {
    false
}
