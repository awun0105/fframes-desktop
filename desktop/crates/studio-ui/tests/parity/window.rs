//! Window layout: the boxes the cached sections are given.
use crate::harness::Studio;
use serde_json::Value;

const HEADER_H: f64 = 52.;
const CAPTION_BUTTON_W: f64 = 46.;
const STATUS_BAR_H: f64 = 28.;
const GUTTER: f64 = 8.;
const SIDEBAR_W: f64 = 416.;
const TAB_BAR_H: f64 = 46.;
/// A card's probe fills it inside its 1 px border.
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
        ["header", "player", "agent-column", "agent"]
            .iter()
            .all(|section| t["sections"][section].is_array())
            && t["split"]["grip_bounds"].is_array()
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
    // The workspace runs from the gutter under the header to the gutter above the status
    // bar: the player and the sidebar side by side, the timeline across both below them.
    let viewport_height = t["viewport"][1].as_f64().unwrap();
    let row_top = HEADER_H;
    let workspace_height = viewport_height - HEADER_H - GUTTER - STATUS_BAR_H;
    assert!(
        close(available + GUTTER, workspace_height),
        "{available} available in a workspace {workspace_height} high"
    );
    let row_height = workspace_height - GUTTER - timeline;
    let column = bounds(&t, "agent-column");
    assert!(
        close(
            column[0] + column[2] + BORDERS / 2.,
            viewport_width - GUTTER
        ) && close(column[2] + BORDERS, SIDEBAR_W)
            && close(column[1] - BORDERS / 2., row_top)
            && close(column[3] + BORDERS, row_height),
        "sidebar {column:?} in a {viewport_width} wide window, row {row_height} high"
    );
    let player = bounds(&t, "player");
    assert!(
        close(player[0] - BORDERS / 2., GUTTER)
            && close(
                player[0] + player[2] + BORDERS / 2.,
                column[0] - BORDERS / 2. - GUTTER
            )
            && close(player[1] - BORDERS / 2., row_top)
            && close(player[3] + BORDERS, row_height),
        "player {player:?} left of the sidebar {column:?}, row {row_height} high"
    );
    // The timeline's grip spans the timeline card, which spans the whole workspace.
    let grip = &t["split"]["grip_bounds"];
    let at = |i: usize| grip[i].as_f64().unwrap();
    assert!(
        close(at(0), GUTTER)
            && close(at(2), viewport_width - 2. * GUTTER)
            // 11 px above the card, which starts a gutter below the row.
            && close(at(1), row_top + row_height + GUTTER - 11.),
        "timeline grip {grip} under a row {row_height} high"
    );
    // The agent panel fills the sidebar under its tab rail; it pads its own rows.
    assert_eq!(t["drawer"], "Agent");
    let agent = bounds(&t, "agent");
    assert!(
        close(agent[0], column[0])
            && close(agent[2], column[2])
            && close(agent[1], column[1] + TAB_BAR_H)
            && close(agent[1] + agent[3], column[1] + column[3]),
        "agent {agent:?} in the sidebar {column:?}"
    );
}

/// The centre of sidebar tab `index` (Agent, Project, Assets, Style): the rail spreads
/// its four 40 px boxes evenly inside 10 px of padding, 46 px high at the card's top.
fn sidebar_tab(t: &Value, index: usize) -> (f32, f32) {
    let column = bounds(t, "agent-column");
    let slot = (column[2] - 20.) / 4.;
    (
        (column[0] + 10. + slot * (index as f64 + 0.5)) as f32,
        (column[1] + TAB_BAR_H / 2.) as f32,
    )
}

/// The sidebar's tabs swap the agent panel for the Project, Assets or Style contents in
/// the same card; the player keeps its place.
#[gpui::test]
fn the_sidebar_tabs_switch_its_body(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let start = studio.wait("the sidebar measured", |t| {
        t["sections"]["agent-column"].is_array() && t["sections"]["player"].is_array()
    });
    assert_eq!(start["drawer"], "Agent");
    let player = bounds(&start, "player");
    let column = bounds(&start, "agent-column");

    studio.click(sidebar_tab(&start, 1));
    let project = studio.wait("the Project tab", |t| {
        t["drawer"] == "Project" && t["sections"]["drawer"].is_array()
    });
    let drawer = bounds(&project, "drawer");
    assert!(
        (drawer[0] - column[0]).abs() < 0.5
            && (drawer[2] - column[2]).abs() < 0.5
            && (drawer[1] - column[1] - TAB_BAR_H).abs() < 0.5,
        "drawer {drawer:?} in the sidebar {column:?}"
    );
    assert_eq!(bounds(&project, "player"), player);

    studio.click(sidebar_tab(&project, 3));
    studio.wait("the Style tab", |t| t["drawer"] == "Style");
    studio.click(sidebar_tab(&project, 0));
    studio.wait("the Agent tab again", |t| t["drawer"] == "Agent");
}

/// Focusing the prompt from another tab brings the Agent tab forward first.
#[gpui::test]
fn focusing_the_prompt_selects_the_agent_tab(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_agent(cx);
    let start = studio.wait("the sidebar measured", |t| {
        t["sections"]["agent-column"].is_array()
    });
    studio.click(sidebar_tab(&start, 2));
    studio.wait("the Assets tab", |t| t["drawer"] == "Assets");
    studio.focus_prompt();
    assert_eq!(studio.telemetry()["drawer"], "Agent");
}

fn agent_grip_centre(t: &Value) -> (f32, f32) {
    let grip = &t["agent_column"]["grip_bounds"];
    let at = |i: usize| grip[i].as_f64().unwrap() as f32;
    (at(0) + at(2) / 2., at(1) + at(3) / 2.)
}

fn agent_width(t: &Value) -> f64 {
    t["agent_column"]["width"].as_f64().unwrap()
}

/// The grip on the sidebar's left edge widens it to the left and narrows it to the right,
/// within its limits.
#[gpui::test]
fn the_agent_grip_resizes_the_column_within_its_limits(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let start = studio.wait("the agent grip painted", |t| {
        t["agent_column"]["grip_bounds"].is_array()
    });
    assert_eq!(agent_width(&start), SIDEBAR_W);

    let (x, y) = agent_grip_centre(&start);
    studio.drag((x, y), (x - 60., y));
    let wider = studio.wait("a wider column", |t| agent_width(t) != SIDEBAR_W);
    assert!(
        (agent_width(&wider) - (SIDEBAR_W + 60.)).abs() < 0.5,
        "{:#}",
        wider["agent_column"]
    );
    let column = bounds(&wider, "agent-column");
    assert!((column[2] + BORDERS - agent_width(&wider)).abs() < 0.5);

    let (x, y) = agent_grip_centre(&studio.telemetry());
    studio.drag((x, y), (x - 5000., y));
    studio.wait("the widest column", |t| agent_width(t) == 520.);

    let (x, y) = agent_grip_centre(&studio.telemetry());
    studio.drag((x, y), (x + 5000., y));
    let narrowest = studio.wait("the narrowest column", |t| agent_width(t) == 320.);
    assert_eq!(narrowest["split"]["resizing"], false);
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

/// In a 1280x800 window the agent column is short: its tabs, the pinned composer and the
/// Project tab's Undo still lie inside it.
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
        t["panel"]["buttons"]["agent-undo"].is_array()
    });
    let undo = &t["panel"]["buttons"]["agent-undo"];
    assert!(
        inside(&t, &agent, undo),
        "{undo} outside the agent tab {agent:?}"
    );
    // The review policy is read here and chosen in Settings → Agent.
    assert!(t["panel"]["buttons"]["agent-policy"].is_null());
}

/// Before the first message the setup guidance fills the transcript; with a preview shown
/// the composer adds its scope chip. In a 1280x720 window Send still lies inside the
/// sidebar.
#[gpui::test]
fn send_stays_in_a_short_sidebar_while_the_guidance_shows(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_agent(cx).display();
    studio.resize(1280., 720.);
    studio.report_button_bounds();
    let t = studio.wait("the composer measured", |t| {
        t["panel"]["buttons"]["composer-send"].is_array() && t["viewport"][1].as_f64() == Some(720.)
    });
    let agent = bounds(&t, "agent");
    let send = &t["panel"]["buttons"]["composer-send"];
    assert!(
        inside(&t, &agent, send),
        "{send} outside the agent tab {agent:?}"
    );
}

/// At its narrowest the agent column still holds the panel's tabs and composer.
#[gpui::test]
fn the_narrowest_agent_column_keeps_its_controls(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_agent(cx);
    studio.resize(1280., 800.);
    studio.report_button_bounds();
    let start = studio.wait("the agent grip painted", |t| {
        t["agent_column"]["grip_bounds"].is_array() && t["viewport"][1].as_f64() == Some(800.)
    });
    let (x, y) = agent_grip_centre(&start);
    studio.drag((x, y), (x + 5000., y));
    let t = studio.wait("the narrowest column measured", |t| {
        agent_width(t) == 320.
            && ["Chat", "History", "Project"]
                .iter()
                .all(|tab| t["panel"]["buttons"][format!("agent-tab-{tab}")].is_array())
            && t["panel"]["prompt_bounds"].is_array()
            && bounds(t, "agent")[2] < 320.
    });
    let agent = bounds(&t, "agent");
    let mut controls = vec![&t["panel"]["prompt_bounds"]];
    for (id, control) in t["panel"]["buttons"].as_object().unwrap() {
        if id.starts_with("agent-tab-") {
            controls.push(control);
        }
    }
    for control in controls {
        assert!(
            inside(&t, &agent, control),
            "{control} outside the agent column {agent:?}: {:#}",
            t["panel"]["buttons"]
        );
    }
}
