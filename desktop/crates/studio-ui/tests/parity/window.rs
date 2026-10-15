//! Window layout: the boxes the cached sections are given.
use crate::harness::Studio;
use serde_json::Value;

const HEADER_H: f64 = 52.;
const CAPTION_BUTTON_W: f64 = 46.;
/// The sidebar probe fills a card inside its 1 px border.
const BORDERS: f64 = 2.;
const SIDEBAR_W: f64 = 416.;
/// The sidebar card's padding around the open tab.
const SIDEBAR_PADDING: f64 = 16.;

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
        ["header", "sidebar", "player", "agent"]
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
    // The sidebar shares the row above the timeline with the player, on its left.
    let sidebar = bounds(&t, "sidebar");
    assert!(
        close(sidebar[3] + BORDERS, available - timeline) && close(sidebar[2] + BORDERS, SIDEBAR_W),
        "sidebar {sidebar:?}, {available} available, timeline {timeline}"
    );
    let player = bounds(&t, "player");
    assert!(
        sidebar[0] + sidebar[2] <= player[0] && close(sidebar[1], player[1]),
        "sidebar {sidebar:?} is not left of the player {player:?}"
    );
    // The agent panel is the sidebar's open tab: its full inner width, down to its padding.
    let agent = bounds(&t, "agent");
    assert!(
        close(agent[0], sidebar[0] + SIDEBAR_PADDING)
            && close(agent[2], sidebar[2] - 2. * SIDEBAR_PADDING)
            && close(
                agent[1] + agent[3],
                sidebar[1] + sidebar[3] - SIDEBAR_PADDING
            )
            && agent[3] > sidebar[3] / 2.,
        "agent {agent:?} in the sidebar {sidebar:?}"
    );
}

fn inside(t: &Value, area: &[f64; 4], control: &Value) -> bool {
    let Some(b) = control.as_array() else {
        panic!("not measured: {control} in {:#}", t["panel"]);
    };
    let b: Vec<f64> = b.iter().map(|v| v.as_f64().unwrap()).collect();
    b[0] >= area[0]
        && b[1] >= area[1]
        && b[0] + b[2] <= area[0] + area[2] + 0.5
        && b[1] + b[3] <= area[1] + area[3] + 0.5
}

/// In a 1280x800 window the sidebar's Agent tab is short: its tabs, the pinned composer
/// and the review policy control still lie inside it.
#[gpui::test]
fn the_agent_tab_keeps_its_controls_in_a_short_window(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_agent(cx);
    studio.resize(1280., 800.);
    studio.report_button_bounds();
    let t = studio.wait("the agent tab measured", |t| {
        t["panel"]["buttons"]["agent-tab-Project"].is_array()
            && t["panel"]["prompt_bounds"].is_array()
            && t["viewport"][1].as_f64() == Some(800.)
    });
    let agent = bounds(&t, "agent");
    for control in [
        &t["panel"]["buttons"]["agent-tab-Chat"],
        &t["panel"]["buttons"]["agent-tab-Project"],
        &t["panel"]["prompt_bounds"],
    ] {
        assert!(
            inside(&t, &agent, control),
            "{control} outside the agent tab {agent:?}"
        );
    }
    let tab = &t["panel"]["buttons"]["agent-tab-Project"];
    let at = |i: usize| tab[i].as_f64().unwrap() as f32;
    studio.click((at(0) + at(2) / 2., at(1) + at(3) / 2.));
    let t = studio.wait("the Project tab", |t| {
        t["panel"]["buttons"]["agent-policy"].is_array()
    });
    let policy = &t["panel"]["buttons"]["agent-policy"];
    assert!(
        inside(&t, &agent, policy),
        "{policy} outside the agent tab {agent:?}"
    );
}
