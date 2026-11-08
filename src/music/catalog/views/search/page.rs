use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, div};

use crate::music::track::views::{list, row};
use crate::shared::i18n::t;
use crate::shared::ui::views::components::{hint, loading, text};
use crate::shared::ui::views::design::theme;
use crate::shell::app::{Jondo, Route, SearchType};
use crate::shell::views::page;

pub fn show(app: &mut Jondo, cx: &mut Context<Jondo>) -> AnyElement {
    let Route::Catalog(kind, query) = app.route.clone() else {
        return div().into_any_element();
    };

    if kind != SearchType::Track {
        return text::text(
            t("Spotify only opens a {} from a link, URI or ID. Paste one, or search by name under another type.")
                .replace("{}", kind.label()),
            14.0,
            theme::FAINT,
        )
        .into_any_element();
    }

    let stale = {
        let catalog = app.catalog.read(cx);
        catalog.query != query && !catalog.search.loading
    };
    if stale {
        app.ensure_catalog(&query, cx);
    }

    if app.catalog.read(cx).search.loading {
        return div()
            .size_full()
            .flex()
            .flex_col()
            .gap(theme::unit(8.0))
            .child(text::weighted(
                query,
                20.0,
                theme::TEXT,
                theme::Weight::SemiBold,
            ))
            .child(loading::rows())
            .into_any_element();
    }

    if let Some(error) = app.catalog.read(cx).search.error.clone() {
        return text::text(error, 13.0, theme::DANGER).into_any_element();
    }

    if app.catalog.read(cx).search.value.is_none() {
        return hint::plain(t("No results")).into_any_element();
    }

    let table = app.catalog_rows(cx);
    if table.full.is_empty() {
        return hint::plain(t("No results")).into_any_element();
    }

    let list = page::panel(list::show(
        app,
        cx,
        list::List {
            key: format!("catalog:{query}"),
            id: "catalog-tracks",
            table: table.clone(),
            playlist: None,
            removable: false,
            shape: row::Shape::Song,
            focus: true,
        },
    ));
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(theme::unit(12.0))
        .child(text::weighted(
            query,
            20.0,
            theme::TEXT,
            theme::Weight::SemiBold,
        ))
        .child(list)
        .into_any_element()
}
