use std::cell::RefCell;
use std::rc::Rc;

use gpui::App;

/// Who owns the repaint an interaction asks for.
///
/// A page surface repaints only the route page; a shell surface repaints the
/// chrome and keeps the cached page; `Both` is the safe default for controls
/// the caller does not know where to place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Page,
    Both,
    Shell,
}

type Painter = Rc<dyn Fn(Scope, &mut App)>;

thread_local! {
    static PAINTER: RefCell<Option<Painter>> = const { RefCell::new(None) };
}

/// The shell installs one painter at boot. Views never call
/// `Window::refresh` for pointer state: that ignores every cached view and
/// rebuilds the whole tree, page included.
pub fn install(painter: impl Fn(Scope, &mut App) + 'static) {
    PAINTER.with(|slot| *slot.borrow_mut() = Some(Rc::new(painter)));
}

pub fn paint(scope: Scope, cx: &mut App) {
    let painter = PAINTER.with(|slot| slot.borrow().clone());
    if let Some(painter) = painter {
        painter(scope, cx);
    }
}
