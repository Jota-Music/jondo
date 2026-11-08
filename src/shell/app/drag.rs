use crate::shared::ui::views::design::theme;

use crate::shared::i18n::t;
use crate::shared::music::domain::Playlist;

use crate::shell::domain::gesture::Reorder;

use super::*;

/// One frame of edge autoscroll for a lifted row.
pub(super) struct ReorderScroll {
    key: String,
    kind: Reorder,
    offset: gpui::Point<gpui::Pixels>,
    index: usize,
}

/// The scrollable list a lifted row belongs to.
pub(super) struct ReorderList {
    key: String,
    kind: Reorder,
    len: usize,
    viewport: gpui::Bounds<gpui::Pixels>,
    offset: gpui::Point<gpui::Pixels>,
    max: gpui::Point<gpui::Pixels>,
}

impl Jondo {
    pub fn reorder_shelf(&mut self, from: &str, to: &str, cx: &gpui::App) {
        let mut ids = self.shelf_ids(cx);
        let Some(from_index) = ids.iter().position(|id| id == from) else {
            return;
        };

        let Some(to_index) = ids.iter().position(|id| id == to) else {
            return;
        };

        let id = ids.remove(from_index);
        ids.insert(to_index, id);
        self.store.order = ids;
        self.store.order_at = crate::shared::kernel::app::clock::now_ms();
        self.library_changed(cx);
    }

    pub fn dragging(&self) -> bool {
        self.ui.drag.is_some()
            || self.ui.drag_from.is_some()
            || self.ui.drag_shelf_from.is_some()
            || self.rooms_ui.drag_from.is_some()
            || self.ui.drag_queue_from.is_some()
    }

    pub fn finish_drag(&mut self, cx: &mut gpui::Context<Self>) {
        let shelf = (self.ui.drag_shelf_from.take(), self.ui.drag_shelf_to.take());
        if let (Some(from), Some(to)) = shelf
            && from != to
        {
            self.reorder_shelf(&from, &to, cx);
        }

        let song = (
            self.ui.drag_from.take(),
            self.ui.drag_to.take(),
            self.ui.drag_playlist.take(),
        );
        if let (Some(from), Some(to), Some(playlist)) = song
            && from != to
        {
            self.reorder_song(&playlist, from, to, cx);
        }

        let queue = (self.ui.drag_queue_from.take(), self.ui.drag_queue_to.take());
        if let (Some(from), Some(to)) = queue
            && from != to
        {
            self.player
                .update(cx, |player, _| player.queue.move_song(from, to));
            self.persist(cx);
        }

        let rooms = (self.rooms_ui.drag_from.take(), self.rooms_ui.drag_to.take());
        if let (Some(from), Some(to)) = rooms
            && from != to
        {
            self.reorder_rooms(&from, &to, cx);
        }

        self.ui.reorder = None;
        self.ui.drag_pointer = None;
        self.ui.drag_scroll = false;
        self.ui.pressed_shelf = None;
        self.ui.pressed_song = None;
        self.ui.pressed_queue = None;
        self.rooms_ui.pressed = None;
    }

    pub fn scrolling(&self, cx: &gpui::App) -> bool {
        self.reorder_scroll(cx).is_some()
    }

    pub fn touch(
        &mut self,
        kind: &Reorder,
        event: &gpui::LongPressEvent,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        match event.phase {
            gpui::TouchPhase::Started => self.touch_start(kind, event.position, cx),
            gpui::TouchPhase::Moved => {
                if self.ui.reorder.as_ref() != Some(kind) {
                    return false;
                }

                self.ui.drag_pointer = Some(event.position);
                self.retarget(cx);
                true
            }
            gpui::TouchPhase::Ended => {
                if self.ui.reorder.as_ref() != Some(kind) {
                    return false;
                }

                self.finish_drag(cx);
                true
            }
            gpui::TouchPhase::Cancelled => {
                if self.ui.reorder.as_ref() != Some(kind) {
                    return false;
                }

                self.cancel_reorder();
                true
            }
        }
    }

    pub(super) fn touch_start(
        &mut self,
        kind: &Reorder,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        if self.ui.reorder.is_some() {
            return false;
        }

        let (key, len, live) = match kind {
            Reorder::Queue => (
                "queue".to_owned(),
                self.player.read(cx).queue.songs.len(),
                self.ui.queue_open && self.ui.selection.is_empty(),
            ),
            Reorder::Playlist(id) => {
                let here = matches!(&self.route, Route::Playlist(route) if route == id);
                let table = self.rows(id, cx);
                let live = here
                    && self.store.playlist(id).is_some()
                    && self.ui.selection.is_empty()
                    && self.ui.playlist_query.trim().is_empty()
                    && self.ui.playlist_order == Order::Added
                    && crate::shared::music::domain::reorderable(true, &table.full, &table.ids);
                (format!("playlist:{id}"), table.full.len(), live)
            }
        };

        if !live || len < 2 {
            return false;
        }

        let Some(scroll) = self.scrolls.get(&key) else {
            return false;
        };

        let (viewport, offset) = {
            let state = scroll.0.borrow();
            let base = &state.base_handle;
            (base.bounds(), base.offset())
        };

        if !viewport.contains(&position) {
            return false;
        }

        let local = position.y.as_f32() - viewport.top().as_f32() - offset.y.as_f32();
        if local < 0.0 || local >= len as f32 * crate::shared::ui::views::reorder::ROW {
            return false;
        }

        let index = crate::shared::ui::views::reorder::index(
            position.y,
            viewport,
            offset.y,
            crate::shared::ui::views::reorder::ROW,
            len,
        );
        self.ui.reorder = Some(kind.clone());
        self.ui.drag_pointer = Some(position);
        self.ui.drag_scroll = false;
        self.ui.pressed_queue = None;
        self.ui.pressed_song = None;
        match kind {
            Reorder::Queue => {
                self.ui.drag_queue_from = Some(index);
                self.ui.drag_queue_to = Some(index);
            }
            Reorder::Playlist(id) => {
                self.ui.drag_playlist = Some(id.clone());
                self.ui.drag_from = Some(index);
                self.ui.drag_to = Some(index);
            }
        }

        self.retarget(cx);
        true
    }

    pub(super) fn cancel_reorder(&mut self) {
        self.ui.drag_queue_from = None;
        self.ui.drag_queue_to = None;
        self.ui.drag_from = None;
        self.ui.drag_to = None;
        self.ui.drag_playlist = None;
        self.ui.reorder = None;
        self.ui.drag_pointer = None;
        self.ui.drag_scroll = false;
        self.ui.pressed_queue = None;
        self.ui.pressed_song = None;
    }

    pub(super) fn retarget(&mut self, cx: &mut gpui::Context<Self>) -> bool {
        let Some(list) = self.reorder_list(cx) else {
            return false;
        };

        let Some(pointer) = self.ui.drag_pointer else {
            return false;
        };

        let index = crate::shared::ui::views::reorder::index(
            pointer.y,
            list.viewport,
            list.offset.y,
            crate::shared::ui::views::reorder::ROW,
            list.len,
        );
        let changed = match &list.kind {
            Reorder::Queue => {
                let changed = self.ui.drag_queue_to != Some(index);
                self.ui.drag_queue_to = Some(index);
                changed
            }
            Reorder::Playlist(_) => {
                let changed = self.ui.drag_to != Some(index);
                self.ui.drag_to = Some(index);
                changed
            }
        };

        let scrolling = self.scrolling(cx);
        let edge = scrolling != self.ui.drag_scroll;
        self.ui.drag_scroll = scrolling;
        changed || edge
    }

    pub(super) fn reorder_list(&self, cx: &gpui::App) -> Option<ReorderList> {
        let kind = self.ui.reorder.clone()?;
        self.ui.drag_pointer?;
        let (key, len) = match &kind {
            Reorder::Queue => ("queue".to_owned(), self.player.read(cx).queue.songs.len()),
            Reorder::Playlist(id) => (format!("playlist:{id}"), self.songs(id, cx).len()),
        };

        if len < 2 {
            return None;
        }

        let scroll = self.scrolls.get(&key)?;
        let (viewport, offset, max) = {
            let state = scroll.0.borrow();
            let base = &state.base_handle;
            (base.bounds(), base.offset(), base.max_offset())
        };

        Some(ReorderList {
            key,
            kind,
            len,
            viewport,
            offset,
            max,
        })
    }

    pub(super) fn reorder_scroll(&self, cx: &gpui::App) -> Option<ReorderScroll> {
        let list = self.reorder_list(cx)?;
        let pointer = self.ui.drag_pointer?;
        let step = crate::shared::ui::views::reorder::delta(
            pointer.y,
            list.viewport,
            list.offset.y,
            list.max.y,
        )?;
        let y = (list.offset.y - theme::unit(step)).clamp(-list.max.y, theme::unit(0.0));
        let index = crate::shared::ui::views::reorder::index(
            pointer.y,
            list.viewport,
            y,
            crate::shared::ui::views::reorder::ROW,
            list.len,
        );
        Some(ReorderScroll {
            key: list.key,
            kind: list.kind,
            offset: gpui::point(list.offset.x, y),
            index,
        })
    }

    pub(super) fn reorder_tick(&mut self, cx: &gpui::Context<Self>) {
        let Some(scroll) = self.reorder_scroll(cx) else {
            self.ui.drag_scroll = false;
            return;
        };

        self.ui.drag_scroll = true;
        match scroll.kind {
            Reorder::Queue => self.ui.drag_queue_to = Some(scroll.index),
            Reorder::Playlist(_) => self.ui.drag_to = Some(scroll.index),
        }

        if let Some(handle) = self.scrolls.get(&scroll.key) {
            handle.0.borrow().base_handle.set_offset(scroll.offset);
        }
    }

    pub fn is_saved(&self, id: &str) -> bool {
        self.store.saved.iter().any(|playlist| playlist.id == id)
    }

    pub fn toggle_saved(&mut self, playlist: &Playlist, cx: &mut gpui::Context<Self>) {
        if playlist.is_local() {
            return;
        }

        if let Some(position) = self
            .store
            .saved
            .iter()
            .position(|item| item.id == playlist.id)
        {
            self.store.saved.remove(position);
            self.store.saved_at.remove(&playlist.id);
            self.store.saved_graves.insert(
                playlist.id.clone(),
                crate::shared::kernel::app::clock::now_ms(),
            );
            self.notify(t("Removed from library"));
        } else {
            let mut summary = playlist.clone();
            summary.songs.clear();
            summary.loading = false;
            self.store.saved_at.insert(
                playlist.id.clone(),
                crate::shared::kernel::app::clock::now_ms(),
            );
            self.store.saved.push(summary);
            self.notify(t("Added to library"));
        }

        self.library_changed(cx);
    }

    pub(super) fn sync_saved(&mut self, playlist: &Playlist) -> bool {
        let Some(saved) = self
            .store
            .saved
            .iter_mut()
            .find(|item| item.id == playlist.id)
        else {
            return false;
        };

        saved.name = playlist.name.clone();
        saved.subtitle = playlist.subtitle.clone();
        saved.cover = playlist.cover.clone();
        saved.covers = playlist.covers.clone();
        saved.owner = playlist.owner.clone();
        saved.track_count = playlist.track_count;
        saved.source = playlist.source;
        true
    }
}
