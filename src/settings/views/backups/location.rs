use std::path::Path;

use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div,
};

use gpui_kit::component::tooltip::Tooltip;

use crate::settings::domain::SyncStatus;
use crate::shared::i18n::t;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

use super::super::chrome::card;

use super::widgets::*;

pub(super) fn location(folder: &str, name: &str, status: &SyncStatus) -> AnyElement {
    let shown = if folder.is_empty() {
        t("No location selected").to_owned()
    } else {
        shown_name(folder, name)
    };

    let place = if folder.starts_with("content://") {
        place(folder)
    } else {
        ""
    };

    let mut body = div()
        .id("backup-location")
        .flex()
        .flex_col()
        .gap(theme::unit(8.0))
        .child(text::text(t("Location"), 11.0, theme::FAINT))
        .child(
            div()
                .min_w(theme::unit(0.0))
                .flex()
                .items_baseline()
                .gap(theme::unit(4.0))
                .child(text::truncated(
                    shown,
                    12.0,
                    theme::ZINC_300,
                    theme::Weight::Medium,
                ))
                .children(
                    (!place.is_empty())
                        .then(|| text::text(format!("· {place}"), 12.0, theme::FAINT)),
                ),
        );

    if folder.is_empty() {
        return card(body).into_any_element();
    }

    let full = folder.to_owned();
    body = body.tooltip(move |window, cx| Tooltip::new(full.clone()).build(window, cx));

    if status.at > 0 {
        let mut stats = div()
            .mt(theme::unit(2.0))
            .flex()
            .flex_col()
            .gap(theme::unit(4.0))
            .child(stat(t("Last save"), ago(status.at), theme::ZINC_300))
            .child(stat(t("Size"), size(status.bytes), theme::ZINC_300))
            .child(stat(
                t("Playlists in the file"),
                status.remote.to_string(),
                theme::ZINC_300,
            ));
        if status.imported > 0 {
            stats = stats.child(stat(
                t("Imported"),
                format!("+{}", status.imported),
                theme::GREEN,
            ));
        }

        body = body.child(stats);
    }

    if status.sealed {
        body = body.child(text::text(
            t("The file carries a session this device cannot open. Set the pass it was encrypted with to bring it here."),
            11.0,
            theme::AMBER_300,
        ));
    }

    if !status.problem.is_empty() {
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(theme::unit(2.0))
                .child(text::text(
                    t("The last save failed. Check the location and its permissions."),
                    11.0,
                    theme::AMBER_300,
                ))
                .child(text::text(
                    status.problem.clone(),
                    11.0,
                    theme::alpha(theme::AMBER_300, 0.7),
                )),
        );
    }

    card(body).into_any_element()
}

pub(super) fn choose(app: &mut Jondo) {
    #[cfg(target_os = "android")]
    {
        app.ui.backup_sheet = true;
    }

    #[cfg(not(target_os = "android"))]
    {
        app.choose_folder();
    }
}

/// The three ways a phone can reach the document, named by what they leave
/// behind instead of by the picker that runs underneath.
pub(crate) fn shown_name(folder: &str, name: &str) -> String {
    if folder.starts_with("content://") && !name.is_empty() {
        return name.to_owned();
    }

    let last = folder.rsplit('/').next().unwrap_or(folder);
    if last.is_empty() {
        return folder.to_owned();
    }

    if folder.starts_with("content://") {
        decoded(last)
    } else {
        Path::new(folder)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| last.to_owned())
    }
}

pub(crate) fn place(uri: &str) -> &'static str {
    match uri.split('/').nth(2) {
        Some("com.google.android.apps.docs.storage") => t("Google Drive"),
        Some("com.android.externalstorage.documents") => t("Phone"),
        _ => "",
    }
}

pub(crate) fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && text.is_char_boundary(index + 1)
            && text.is_char_boundary(index + 3)
            && let Ok(byte) = u8::from_str_radix(&text[index + 1..index + 3], 16)
        {
            out.push(byte);
            index += 3;
            continue;
        }

        out.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}
