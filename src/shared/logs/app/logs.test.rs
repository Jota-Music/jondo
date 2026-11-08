use super::*;

#[test]
fn other_crates_only_report_problems() {
    assert!(wanted(log::Level::Info, "jondo::shell::app"));
    assert!(wanted(log::Level::Debug, "jondo"));
    assert!(!wanted(log::Level::Debug, "gpui"));
    assert!(!wanted(log::Level::Info, "smithay_client_toolkit"));
    assert!(wanted(log::Level::Warn, "smithay_client_toolkit"));
    assert!(wanted(log::Level::Error, "wgpu_core"));
}

#[test]
fn a_missing_system_ui_font_is_not_reported() {
    let missing =
        "failed to load font Font { family: \".SystemUIFont\", weight: FontWeight(400.0) }";
    assert!(missed_system_font(&format!("{missing}: no font found")));
    assert!(!missed_system_font(
        "failed to load font Font { family: \"Inter Variable\" }: no font found"
    ));
}

#[test]
fn the_buffer_keeps_the_last_lines() {
    for index in 0..LIMIT + 5 {
        push(format!("line {index}"));
    }

    let text = text();
    assert!(text.contains(&format!("line {}", LIMIT + 4)));
    assert!(!text.contains("line 0\n"));
}
