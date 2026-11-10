use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};

use crate::settings::app::backup::{self, Mode};
use crate::shared::i18n::t;
use crate::shared::ui::views::components::{field, switch, text};
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;
use crate::shell::views::menu::{Command, Item, Menu};

use super::super::chrome::{card, section};

use super::{location::*, widgets::*};

const INTERVALS: [u64; 5] = [5, 10, 15, 30, 60];

pub(crate) fn backups(app: &mut Jondo, narrow: bool, cx: &mut Context<Jondo>) -> AnyElement {
    let folder = app.store.sync_folder.clone();
    let name = app.store.sync_status.name.clone();
    let auto = app.store.sync_auto;
    let import = app.store.sync_import;
    let interval = app.store.sync_interval;
    let status = app.store.sync_status.clone();
    let pending = backup::pending(&app.store);
    let interval_open = app
        .menu
        .as_ref()
        .is_some_and(|menu| menu.origin == Some("settings-interval"));
    let pass = app.fields.get("session-pass").cloned();

    #[cfg(target_os = "android")]
    let busy = app.saf.busy();
    #[cfg(not(target_os = "android"))]
    let busy = false;

    let session = session(&pass);
    let files = files(app, narrow, busy, cx);

    let mut autosave = div()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(heading(Icon::FolderSync, t("Autosave")))
        .child(text::text(
            t("Save your library to jota.backup.json every so often, in a folder or an account like Google Drive or Dropbox."),
            12.0,
            theme::FAINT,
        ))
        .child(
            div()
                .flex()
                .items_start()
                .gap(theme::unit(6.0))
                .child(icons::icon(Icon::TriangleAlert, 14.0, theme::FAINT))
                .child(text::text(
                    t("The file carries your relay address and its token in plain text, so anyone who can open it can use them."),
                    11.0,
                    theme::FAINT,
                )),
        )
        .child(card(session))
        .child(action(
            "backup-folder",
            if folder.is_empty() {
                t("Choose location")
            } else {
                t("Change location")
            },
            Icon::FolderSync,
            false,
            true,
            !busy,
            app,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                choose(app);
                cx.notify();
            }),
        ))
        .child(location(&folder, &name, &status))
        .child(action(
            "backup-save",
            t("Save now"),
            Icon::Save,
            pending,
            true,
            !folder.is_empty() && !busy,
            app,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.backup(Mode::Newer, cx);
                cx.notify();
            }),
        ))
        .child(action(
            "backup-reload",
            t("Reload backup"),
            Icon::RotateCcw,
            false,
            true,
            !folder.is_empty() && !busy,
            app,
            cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                app.backup(Mode::Restore, cx);
                cx.notify();
            }),
        ));

    if status.stale {
        autosave = autosave.child(stale(cx));
    }

    autosave = autosave.child(toggles(app, auto, import, interval, interval_open, cx));

    if !status.problem.is_empty() && !folder.is_empty() {
        autosave = autosave.child(overwrite(cx));
    }

    let body = div()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(files)
        .child(div().h(theme::unit(1.0)).w_full().bg(theme::ZINC_800))
        .child(autosave);

    section(Icon::ArchiveRestore, t("Backups"), None, body)
}

fn session(pass: &Option<gpui::Entity<gpui_kit::component::input::InputState>>) -> gpui::Div {
    let mut session = div()
        .flex()
        .flex_col()
        .gap(theme::unit(8.0))
        .child(text::weighted(
            t("Session encryption"),
            12.0,
            theme::MUTED,
            theme::Weight::SemiBold,
        ));
    if let Some(state) = pass {
        session = session.child(field::secret(state));
    }

    session = session.child(text::text(
        t("When set, the YouTube and Spotify sessions are encrypted and synced through the backup. Leave empty to keep them local only."),
        12.0,
        theme::FAINT,
    ));
    session
}

fn files(app: &Jondo, narrow: bool, busy: bool, cx: &mut Context<Jondo>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(theme::unit(16.0))
        .child(heading(Icon::Download, t("Saved")))
        .child(text::text(
            t("Back up every playlist, Spotify and YouTube included, in a JSON file or restore them. Re-importing keeps the newer copy when a playlist already exists."),
            12.0,
            theme::FAINT,
        ))
        .child(
            div()
                .flex()
                .gap(theme::unit(8.0))
                .child(
                    div().when(narrow, |this| this.flex_1()).child(action(
                        "backup-export",
                        t("Export backup"),
                        Icon::Download,
                        true,
                        narrow,
                        !busy,
                        app,
                        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                            app.export_backup();
                            cx.notify();
                        }),
                    )),
                )
                .child(
                    div().when(narrow, |this| this.flex_1()).child(action(
                        "backup-import",
                        t("Import backup"),
                        Icon::Upload,
                        false,
                        narrow,
                        !busy,
                        app,
                        cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                            app.import_backup();
                            cx.notify();
                        }),
                    )),
                ),
        )
}

fn stale(cx: &mut Context<Jondo>) -> gpui::Div {
    div()
                .flex()
                .items_center()
                .justify_between()
                .gap(theme::unit(12.0))
                .rounded(theme::unit(12.0))
                .border_1()
                .border_color(theme::alpha(theme::AMBER_500, 0.4))
                .bg(theme::alpha(theme::AMBER_500, 0.1))
                .p(theme::unit(12.0))
                .child(text::text(
                    t("The last save could not write: the service served a cached copy of the file. Choose the location again."),
                    12.0,
                    theme::AMBER_300,
                ))
                .child(
                    div()
                        .id("backup-stale-choose")
                        .flex_none()
                        .cursor_pointer()
                        .role(gpui::Role::Button)
                        .aria_label(t("Choose location"))
                        .rounded(theme::unit(8.0))
                        .border_1()
                        .border_color(theme::alpha(theme::AMBER_500, 0.4))
                        .px(theme::unit(8.0))
                        .py(theme::unit(4.0))
                        .hover(|style| style.border_color(theme::AMBER_300))
                        .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
                            choose(app);
                            cx.notify();
                        }))
                        .child(text::weighted(
                            t("Choose location"),
                            12.0,
                            theme::AMBER_200,
                            theme::Weight::SemiBold,
                        )),
                )
}

fn toggles(
    app: &mut Jondo,
    auto: bool,
    import: bool,
    interval: u64,
    interval_open: bool,
    cx: &mut Context<Jondo>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(theme::unit(8.0))
        .child(row(
            div().child(text::text(t("Autosave automatically"), 12.0, theme::FAINT)),
            switch(
                "autosave-switch",
                auto,
                app.accent,
                cx.listener(|app, value: &bool, _, cx| {
                    app.set_sync_auto(*value, cx);
                    cx.notify();
                }),
            ),
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(theme::unit(4.0))
                .rounded(theme::unit(12.0))
                .border_1()
                .border_color(theme::ZINC_800)
                .bg(theme::SURFACE)
                .p(theme::unit(12.0))
                .child(line(
                    div().child(text::text(t("Autoimport"), 12.0, theme::FAINT)),
                    switch(
                        "autoimport-switch",
                        import,
                        app.accent,
                        cx.listener(|app, value: &bool, _, cx| {
                            app.set_sync_import(*value, cx);
                            cx.notify();
                        }),
                    ),
                ))
                .child(text::text(
                    t("Bring to your library what other devices have saved in the file too."),
                    11.0,
                    theme::FAINT,
                )),
        )
        .child(row(
            div().child(text::text(t("Every"), 12.0, theme::FAINT)),
            div()
                .id("settings-interval")
                .h(theme::unit(32.0))
                .px(theme::unit(10.0))
                .flex()
                .items_center()
                .gap(theme::unit(6.0))
                .rounded(theme::unit(8.0))
                .border_1()
                .border_color(if interval_open {
                    theme::ZINC_600
                } else {
                    theme::ZINC_800
                })
                .bg(theme::SURFACE)
                .cursor_pointer()
                .role(gpui::Role::Button)
                .aria_expanded(interval_open)
                .aria_label(format!("{}: {interval} min", t("Interval")))
                .on_click(cx.listener(|app, event: &gpui::ClickEvent, _, cx| {
                    let items = INTERVALS
                        .into_iter()
                        .map(|minutes| {
                            Item::choice(
                                format!("{minutes} min"),
                                minutes == app.store.sync_interval,
                                Command::Interval(minutes),
                            )
                        })
                        .collect();
                    app.menu = Some(
                        Menu::choice(event.position(), t("Interval"), items)
                            .origin("settings-interval"),
                    );
                    app.notify_shell(cx);
                }))
                .child(text::text(format!("{interval} min"), 12.0, theme::ZINC_100))
                .child(icons::icon(icons::CHEVRON_DOWN, 14.0, theme::MUTED)),
        ))
}

fn overwrite(cx: &mut Context<Jondo>) -> impl IntoElement {
    div()
        .id("backup-overwrite")
        .self_start()
        .cursor_pointer()
        .role(gpui::Role::Button)
        .aria_label(t("Overwrite the file"))
        .on_click(cx.listener(|app, _: &gpui::ClickEvent, _, cx| {
            app.overwrite_backup(cx);
            cx.notify();
        }))
        .child(
            text::weighted(
                t("Overwrite the file"),
                12.0,
                theme::AMBER_300,
                theme::Weight::SemiBold,
            )
            .text_decoration_1(),
        )
}
