use crate::shared::ui::views::design::theme;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, ScrollStrategy,
    StatefulInteractiveElement, Styled, div, uniform_list,
};

use crate::music::track::views::jump;
use crate::music::track::views::row::{self, Shape};
use crate::shared::ui::views::components::scroll;
use crate::shared::ui::views::reorder;
use crate::shell::app::{Jondo, Order, Reorder, Table};

/// A song list page: the uniform list over a shared table, the scroll handle
/// and the scrollbar. The caller only wraps it in a panel.
pub struct List {
    pub key: String,
    pub id: &'static str,
    pub table: Table,
    pub playlist: Option<String>,
    pub removable: bool,
    pub shape: Shape,
    /// Whether the list opens on the playing row and offers the pill that
    /// returns to it once it scrolls out of view.
    pub focus: bool,
}

pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>, list: List) -> AnyElement {
    let entity = cx.entity();
    let key = list.key.clone();
    let scroll = app.scroll(key.clone());
    let count = list.table.rows.len();
    let reorder = list.playlist.is_some()
        && list.removable
        && app.ui.playlist_query.trim().is_empty()
        && app.ui.playlist_order == Order::Added
        && crate::shared::music::domain::reorderable(true, &list.table.full, &list.table.ids);
    let now = app.player.read(cx).current().cloned();
    let position = now
        .as_ref()
        .and_then(|song| list.table.ids.iter().position(|id| id == &song.id));
    let first = list.focus && count > 0 && app.ui.opened.insert(key.clone());
    if let Some(index) = first.then_some(position).flatten() {
        scroll.scroll_to_item(index, ScrollStrategy::Center);
    }

    // A scroll already queued by the handle lands on this layout, so the pill
    // is hidden for that frame instead of pointing at the old position.
    let jumping = scroll.0.borrow().deferred_scroll_to_item.is_some();
    let jump = if !jumping {
        position.and_then(|index| {
            jump::offscreen(&scroll, index, count, 0.0).map(|above| (index, above))
        })
    } else {
        None
    };

    let pill = jump.and_then(|(index, above)| {
        let song = now.as_ref()?;
        let key = key.clone();
        Some(
            jump::pill(
                app,
                gpui::ElementId::Name(format!("{}-jump", list.id).into()),
                song,
                above,
            )
            .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                app.scroll(key.clone())
                    .scroll_to_item(index, ScrollStrategy::Center);
                cx.notify();
            }))
            .into_any_element(),
        )
    });
    let gap = if pill.is_some() && jump::ending(&scroll) {
        jump::GAP
    } else {
        0.0
    };

    let rows = uniform_list(
        list.id,
        count,
        row::items(
            entity,
            list.table.full.clone(),
            list.table.ids.clone(),
            Some(list.table.rows.clone()),
            list.playlist.clone(),
            list.removable,
            reorder,
            list.shape,
        ),
    )
    .track_scroll(&scroll);
    let touch = list.playlist.as_ref().filter(|_| reorder).map(|id| {
        let id = id.clone();
        let handler = cx.processor(move |app: &mut Jondo, event: gpui::LongPressEvent, _, cx| {
            let claimed = app.touch(&Reorder::Playlist(id.clone()), &event, cx);
            if claimed {
                cx.notify();
            }

            claimed
        });
        reorder::Touch::new(handler)
    });
    div()
        .id(gpui::ElementId::Name(format!("{}-list", list.id).into()))
        .relative()
        .size_full()
        .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            if !app.ui.selection.is_empty() {
                app.clear_selection();
                cx.notify();
            }
        }))
        .when_some(touch, |this, touch| this.child(touch))
        .child(rows.pr(theme::unit(15.0)).pb(theme::unit(gap)).h_full())
        .child(scroll::bar(&scroll))
        .children(pill)
        .into_any_element()
}
