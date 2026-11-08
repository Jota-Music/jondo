use crate::shared::ui::views::design::theme;

use super::*;
use gpui::{bounds, point, size};

fn viewport() -> Bounds<Pixels> {
    bounds(
        point(theme::unit(0.0), theme::unit(100.0)),
        size(theme::unit(300.0), theme::unit(400.0)),
    )
}

#[test]
fn the_middle_rests_and_the_edges_scroll() {
    let view = viewport();
    assert_eq!(
        delta(
            theme::unit(300.0),
            view,
            theme::unit(0.0),
            theme::unit(800.0)
        ),
        None
    );
    assert_eq!(
        delta(
            theme::unit(100.0),
            view,
            theme::unit(-100.0),
            theme::unit(800.0)
        ),
        Some(-16.0)
    );
    assert_eq!(
        delta(
            theme::unit(500.0),
            view,
            theme::unit(-100.0),
            theme::unit(800.0)
        ),
        Some(16.0)
    );
    assert_eq!(
        delta(
            theme::unit(10.0),
            view,
            theme::unit(0.0),
            theme::unit(800.0)
        ),
        None
    );
    assert_eq!(
        delta(
            theme::unit(500.0),
            view,
            theme::unit(-800.0),
            theme::unit(800.0)
        ),
        None
    );
}

#[test]
fn a_scrolled_list_moves_the_row_under_the_pointer() {
    let view = viewport();
    assert_eq!(
        index(theme::unit(100.0), view, theme::unit(0.0), ROW, 10),
        0
    );
    assert_eq!(
        index(theme::unit(355.0), view, theme::unit(0.0), ROW, 10),
        3
    );
    assert_eq!(
        index(theme::unit(355.0), view, theme::unit(-64.0), ROW, 10),
        4
    );
    assert_eq!(index(theme::unit(999.0), view, theme::unit(0.0), ROW, 2), 1);
}
