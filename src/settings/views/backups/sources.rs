use gpui::{
    AnyElement, Context, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, div,
};

use crate::shared::i18n::t;
use crate::shared::ui::views::components::text;
use crate::shared::ui::views::design::icons::{self, Icon};
use crate::shared::ui::views::design::theme;
use crate::shell::app::Jondo;

pub(crate) fn sources(app: &mut Jondo, window: &mut Window, cx: &mut Context<Jondo>) -> AnyElement {
    use crate::shared::platform::saf::Pick;
    use crate::shell::views::sheet as sheet_ui;

    let mut body = div().flex().flex_col().pb(theme::unit(12.0)).child(
        div()
            .px(theme::unit(16.0))
            .pb(theme::unit(8.0))
            .child(text::weighted(
                t("Where to save?"),
                14.0,
                theme::ZINC_200,
                theme::Weight::SemiBold,
            )),
    );

    for (icon, title, hint, pick) in [
        (
            Icon::Smartphone,
            t("Phone folder"),
            t("A local folder, such as the one Syncthing syncs."),
            Pick::Folder,
        ),
        (
            Icon::CloudDownload,
            t("Open from the cloud"),
            t("Pick the jota.backup.json you already have in Google Drive or Dropbox."),
            Pick::Document,
        ),
        (
            Icon::CloudUpload,
            t("Create in the cloud"),
            t(
                "Creates the file in the cloud. If it already existed, the provider makes a duplicate.",
            ),
            Pick::Create,
        ),
    ] {
        body = body.child(
            div()
                .id(title)
                .w_full()
                .px(theme::unit(16.0))
                .py(theme::unit(12.0))
                .flex()
                .items_start()
                .gap(theme::unit(12.0))
                .cursor_pointer()
                .role(gpui::Role::Button)
                .aria_label(title)
                .hover(|style| style.bg(theme::alpha(theme::ZINC_800, 0.8)))
                .on_click(cx.listener(move |app, _: &gpui::ClickEvent, _, cx| {
                    app.ui.backup_closing = Some(std::time::Instant::now());
                    app.pick_backup(pick);
                    cx.notify();
                }))
                .child(icons::icon(icon, 16.0, theme::FAINT))
                .child(
                    div()
                        .min_w(theme::unit(0.0))
                        .flex()
                        .flex_col()
                        .gap(theme::unit(2.0))
                        .child(text::text(title, 14.0, theme::ZINC_200))
                        .child(text::text(hint, 12.0, theme::FAINT)),
                ),
        );
    }

    sheet_ui::Sheet::new(
        "backup-sources",
        app.ui.backup_closing,
        |app, _, cx| {
            app.ui.backup_closing = Some(std::time::Instant::now());
            cx.notify();
        },
        body,
    )
    .mobile(true)
    .bar(true)
    .show(window, cx)
}
