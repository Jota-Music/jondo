use gpui::{AnyElement, Context, Window};
use std::rc::Rc;

use crate::music::library::views::playlist;
use crate::shared::i18n::t;
use crate::shared::music::domain::Playlist;
use crate::shared::ui::views::components::hint;
use crate::shell::app::Jondo;
use crate::shell::views::page;

pub(super) fn playlists(
    app: &mut Jondo,
    window: &Window,
    cx: &mut Context<Jondo>,
    id: &str,
    items: &[Playlist],
) -> AnyElement {
    if items.is_empty() {
        return page::panel(hint::muted(t("No playlists.")));
    }

    let grid = app.ui.channel_grid;
    page::panel(playlist::results(
        app,
        window,
        cx,
        format!("channel-playlists:{id}"),
        "channel-playlists",
        Rc::new(items.to_vec()),
        grid,
    ))
}
