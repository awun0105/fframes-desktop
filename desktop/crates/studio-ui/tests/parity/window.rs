//! Window layout: the boxes the cached sections are given.
use crate::harness::Studio;
use serde_json::Value;

const GUTTER: f64 = 8.;
/// The probe fills a card inside its 1 px border.
const BORDERS: f64 = 2.;

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

    let header = bounds(&t, "header");
    assert!(
        close(header[2] + BORDERS, viewport_width - 2. * GUTTER),
        "header {header:?} in a {viewport_width} wide window"
    );
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
