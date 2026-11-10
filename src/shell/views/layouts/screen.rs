use gpui::{Context, IntoElement, ParentElement, Render, Styled, WeakEntity, Window, div};

use crate::shell::app::Jondo;

use super::page;

/// The route page as a view of its own, so a sheet sliding over it does not
/// rebuild it on every animation frame: GPUI reuses the frame a view that was
/// not notified already laid out and painted.
pub struct Screen {
    app: WeakEntity<Jondo>,
}

impl Screen {
    pub fn new(app: WeakEntity<Jondo>, _: &mut Context<Self>) -> Self {
        Self { app }
    }
}

impl Render for Screen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(app) = self.app.upgrade() else {
            return div().into_any_element();
        };

        app.update(cx, |app, cx| {
            // Only a frame that paints the page marks its covers as painted, so
            // eviction cannot drop the images a cached page still shows while a
            // sheet or a menu moves over it.
            app.covers.begin_frame();
            app.covers.prune();
            let page = page::route(app, window, cx);
            div().size_full().child(page).into_any_element()
        })
    }
}
