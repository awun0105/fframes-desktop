//! Window layout: the boxes the cached sections are given.
use crate::harness::Studio;
use serde_json::Value;

const GUTTER: f64 = 8.;
const HEADER_H: f64 = 52.;
const CAPTION_BUTTON_W: f64 = 46.;
/// The sidebar and agent probes fill a card inside its 1 px border.
const BORDERS: f64 = 2.;

fn button_width(t: &Value, id: &str) -> Option<f64> {
    t["buttons"][id][2].as_f64()
}

/// The header actions drop their labels when the window gets too narrow for them and get
/// them back when it widens again; Close project has no icon and keeps its label.
#[gpui::test]
fn narrow_windows_show_the_header_actions_as_icons(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::open(cx);
    studio.report_button_bounds();
    let wide = studio.wait("header actions measured", |t| {
        button_width(t, "create").is_some() && button_width(t, "close").is_some()
    });
    let labelled = button_width(&wide, "create").unwrap();
    let close = button_width(&wide, "close").unwrap();
    let height = wide["viewport"][1].as_f64().unwrap() as f32;

    studio.resize(800., height);
    let narrow = studio.wait("icon-only actions", |t| {
        button_width(t, "create").is_some_and(|width| width < labelled - 20.)
    });
    assert_eq!(
        button_width(&narrow, "close"),
        Some(close),
        "{:#}",
        narrow["buttons"]
    );

    studio.resize(1536., height);
    studio.wait("labelled actions again", |t| {
        button_width(t, "create") == Some(labelled)
    });
}

fn bounds(t: &Value, section: &str) -> [f64; 4] {
    let b = &t["sections"][section];
    std::array::from_fn(|i| b[i].as_f64().unwrap())
}

#[gpui::test]
fn the_sections_fill_their_boxes(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let t = studio.wait("every section measured", |t| {
        ["header", "sidebar", "agent"]
            .iter()
            .all(|section| t["sections"][section].is_array())
    });
    let viewport_width = t["viewport"][0].as_f64().unwrap();
    let available = t["split"]["available"].as_f64().unwrap();
    let timeline = t["split"]["timeline_height"].as_f64().unwrap();
    let close = |a: f64, b: f64| (a - b).abs() < 0.5;

    // The header is flush with the window's top and sides; on Windows the caption buttons
    // take its right end.
    let caption_width = if cfg!(target_os = "windows") {
        3. * CAPTION_BUTTON_W
    } else {
        0.
    };
    let header = bounds(&t, "header");
    assert!(
        close(header[0], 0.)
            && close(header[1], 0.)
            && close(header[2], viewport_width - caption_width)
            && close(header[3], HEADER_H),
        "header {header:?} in a {viewport_width} wide window"
    );
    if cfg!(target_os = "windows") {
        let caption = bounds(&t, "caption");
        assert!(
            close(caption[0] + caption[2], viewport_width)
                && close(caption[1], 0.)
                && close(caption[2], caption_width)
                && close(caption[3], HEADER_H),
            "caption {caption:?} in a {viewport_width} wide window"
        );
    } else {
        assert!(t["sections"]["caption"].is_null());
    }
    // The sidebar shares the row above the timeline with the player.
    let sidebar = bounds(&t, "sidebar");
    assert!(
        close(sidebar[3] + BORDERS, available - timeline),
        "sidebar {sidebar:?}, {available} available, timeline {timeline}"
    );
    // The agent column runs the editor column's full height.
    let agent = bounds(&t, "agent");
    assert!(
        close(agent[3] + BORDERS, available + GUTTER),
        "agent {agent:?}, editor column {}",
        available + GUTTER
    );
}
