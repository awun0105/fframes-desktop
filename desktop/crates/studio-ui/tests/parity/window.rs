//! Window layout: the boxes the cached sections are given.
use crate::harness::Studio;
use serde_json::Value;

const HEADER_H: f64 = 52.;
const CAPTION_BUTTON_W: f64 = 46.;
const STATUS_BAR_H: f64 = 28.;
const GUTTER: f64 = 8.;
const RAIL_W: f64 = 56.;
const DRAWER_W: f64 = 320.;
const AGENT_W: f64 = 360.;
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
        ["header", "rail", "player", "agent-column", "agent"]
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
    // The workspace row runs from the gutter under the header to the gutter above the
    // status bar: the rail on its left, the agent column on its right, the player and the
    // timeline between them.
    let viewport_height = t["viewport"][1].as_f64().unwrap();
    let row_top = HEADER_H;
    let row_height = viewport_height - HEADER_H - GUTTER - STATUS_BAR_H;
    let rail = bounds(&t, "rail");
    assert!(
        close(rail[0], GUTTER)
            && close(rail[1], row_top)
            && close(rail[2], RAIL_W)
            && close(rail[3], row_height)
            && close(rail[3], available + GUTTER),
        "rail {rail:?}, row {row_height} high, {available} available"
    );
    let column = bounds(&t, "agent-column");
    assert!(
        close(
            column[0] + column[2] + BORDERS / 2.,
            viewport_width - GUTTER
        ) && close(column[2] + BORDERS, AGENT_W)
            && close(column[1] - BORDERS / 2., row_top)
            && close(column[3] + BORDERS, row_height),
        "agent column {column:?} in a {viewport_width} wide window, row {row_height} high"
    );
    let player = bounds(&t, "player");
    assert!(
        close(player[0] - BORDERS / 2., rail[0] + rail[2] + GUTTER)
            && close(
                player[0] + player[2] + BORDERS / 2.,
                column[0] - BORDERS / 2. - GUTTER
            )
            && close(player[1] - BORDERS / 2., row_top)
            && close(player[3] + BORDERS + GUTTER + timeline, row_height),
        "player {player:?} between the rail {rail:?} and the agent column {column:?}"
    );
    // The agent panel fills the column; it pads its own rows.
    let agent = bounds(&t, "agent");
    assert!(
        close(agent[0], column[0])
            && close(agent[2], column[2])
            && close(agent[1], column[1])
            && close(agent[3], column[3]),
        "agent {agent:?} in the column {column:?}"
    );
}

/// The centre of the first rail item: 46 x 48 px, centred in the rail, 8 px below its top.
fn first_rail_item(t: &Value) -> (f32, f32) {
    let rail = bounds(t, "rail");
    ((rail[0] + RAIL_W / 2.) as f32, (rail[1] + 8. + 24.) as f32)
}

/// The rail's Project item opens its drawer beside the rail, which pushes the player
/// right; pressing the item again closes it.
#[gpui::test]
fn the_rail_opens_and_closes_its_drawer(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let closed = studio.wait("the rail measured", |t| {
        t["sections"]["rail"].is_array() && t["sections"]["player"].is_array()
    });
    assert_eq!(closed["drawer"], Value::Null);
    let player_x = bounds(&closed, "player")[0];
    let item = first_rail_item(&closed);

    studio.click(item);
    let open = studio.wait("the Project drawer", |t| {
        t["drawer"] == "Project"
            && t["sections"]["rail"][2].as_f64() == Some(RAIL_W + GUTTER + DRAWER_W)
    });
    let pushed = bounds(&open, "player")[0];
    assert!(
        (pushed - player_x - GUTTER - DRAWER_W).abs() < 0.5,
        "player at {pushed}, was {player_x}"
    );
    // The column keeps its width: the editor gives up the drawer's.
    assert_eq!(open["agent_column"]["width"].as_f64(), Some(AGENT_W));

    studio.click(item);
    studio.wait("the drawer closed", |t| {
        t["drawer"].is_null() && t["sections"]["rail"][2].as_f64() == Some(RAIL_W)
    });
}

fn agent_grip_centre(t: &Value) -> (f32, f32) {
    let grip = &t["agent_column"]["grip_bounds"];
    let at = |i: usize| grip[i].as_f64().unwrap() as f32;
    (at(0) + at(2) / 2., at(1) + at(3) / 2.)
}

fn agent_width(t: &Value) -> f64 {
    t["agent_column"]["width"].as_f64().unwrap()
}

/// The grip on the agent column's left edge widens it to the left and narrows it to the
/// right, within its limits.
#[gpui::test]
fn the_agent_grip_resizes_the_column_within_its_limits(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::displayed(cx);
    let start = studio.wait("the agent grip painted", |t| {
        t["agent_column"]["grip_bounds"].is_array()
    });
    assert_eq!(agent_width(&start), AGENT_W);

    let (x, y) = agent_grip_centre(&start);
    studio.drag((x, y), (x - 60., y));
    let wider = studio.wait("a wider column", |t| agent_width(t) != AGENT_W);
    assert!(
        (agent_width(&wider) - (AGENT_W + 60.)).abs() < 0.5,
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
/// review policy control still lie inside it.
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
