use gpui::{ClipboardItem, Context, Window};

use crate::shared::i18n::t;
use crate::shell::app::Jondo;

use super::Command;

pub(super) fn execute(
    command: Command,
    app: &mut Jondo,
    window: &mut Window,
    cx: &mut Context<Jondo>,
) {
    match command {
        Command::Play(songs, index) => app.play_list(songs, index, cx),
        Command::Toggle => app.toggle(cx),
        Command::Enqueue(songs) => app.enqueue(songs, cx),
        Command::Pick(songs) => app.ui.picker = Some(songs),
        Command::Select(songs) => app.select_all(&songs),
        Command::Clear => app.clear_selection(),
        Command::SelectOne(id, index) => app.select_single(&id, index),
        Command::ToggleOne(id, index) => app.select_toggle(&id, index),
        Command::Remove(playlist, ids) => app.ui.removing = Some((playlist, ids)),
        Command::Edit(song) => crate::music::track::views::youtube::edit(app, song, cx),
        Command::Copy(link) => {
            app.remember_clipboard(&link);
            cx.write_to_clipboard(ClipboardItem::new_string(link));
            app.notify(t("Link copied."));
        }
        Command::QueuePlay(index) => {
            app.player.update(cx, |player, _| player.queue.at = index);
            app.play_current(cx);
        }
        Command::QueueDown(index) => {
            app.player
                .update(cx, |player, _| player.queue.move_song(index, index + 1));
            app.persist(cx);
        }
        Command::QueueSnap(index) => {
            app.player
                .update(cx, |player, _| player.queue.move_after_current(index));
            app.persist(cx);
        }
        Command::QueueRemove(index) => {
            app.player.update(cx, |player, _| {
                player.queue.remove(index);
                if player.queue.songs.is_empty() {
                    player.stop();
                }
            });

            app.persist(cx);
        }
        Command::PlaylistPlay(id) => app.play_playlist(&id, cx),
        Command::AlbumPlay(id) => app.play_album(&id, cx),
        Command::PlaylistSave(playlist) => app.toggle_saved(&playlist, cx),
        Command::Follow(artist) => app.toggle_follow(&artist, cx),
        Command::PlaylistRename(id, name) => {
            app.ui.rename = Some((id, name.clone()));
            app.set_field("rename", &name, window, cx);
            app.ui.closing = None;
        }
        Command::PlaylistDelete(id) => app.ui.confirm = Some(id),
        Command::RoomConnect(room) => app.join_room(&room, cx),
        Command::RoomLeave => app.stop_room(cx),
        Command::RoomSave => app.save_current_room(cx),
        Command::RoomRemove(id) => app.forget_room(&id, cx),
        Command::Open(route) => app.open(route, cx),
        Command::SearchType(kind) => {
            app.ui.search_type = kind;
            app.ui.search_focus = true;
            app.refresh_search_placeholder(window, cx);
        }
        Command::PlaylistOrder(order) => app.ui.playlist_order = order,
        Command::ChannelOrder(order) => app.ui.channel_order = order,
        Command::Locale(locale) => app.set_locale(locale, cx),
        Command::Browser(browser) => app.ui.browser_pick = browser,
        Command::Interval(minutes) => app.set_sync_interval(minutes, cx),
    }
}
