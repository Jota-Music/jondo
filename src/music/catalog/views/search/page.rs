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
        return div()
            .size_full()
            .flex()
            .flex_col()
            .child(page::panel(
                div().size_full().p(theme::unit(16.0)).child(text::text(
                    t("Spotify only opens a {} from a link, URI or ID. Paste one, or search by name under another type.")
                        .replace("{}", kind.label()),
                    14.0,
                    theme::FAINT,
                )),
            ))
            .into_any_element();
    }

    let header = div()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .px(theme::unit(4.0))
        .pt(theme::unit(4.0))
        .pb(theme::unit(8.0))
        .child(text::truncated(
            query.clone(),
            16.0,
            theme::TEXT,
            theme::Weight::SemiBold,
        ));

    let body: AnyElement = if app.catalog.read(cx).search.loading {
        div()
            .py(theme::unit(24.0))
            .child(loading::spinner())
            .into_any_element()
    } else if let Some(error) = app.catalog.read(cx).search.error.clone() {
        div()
            .child(text::text(error, 13.0, theme::DANGER))
            .into_any_element()
    } else if app.catalog.read(cx).search.value.is_none() {
        hint::plain(t("No results")).into_any_element()
    } else {
        let table = app.catalog_rows(cx);
        if table.full.is_empty() {
            hint::plain(t("No results")).into_any_element()
        } else {
            list::show(
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
            )
            .into_any_element()
        }
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(page::panel(
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(header)
                .child(body),
        ))
        .into_any_element()
}
