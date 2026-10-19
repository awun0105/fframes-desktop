//! Scenes 6–9, 42–74 s, chapter 02: one scene per college, each on its own footage with
//! the college's official lockup (from the end cards of the colleges' films), its colour and mark, its
//! line from its site, and two of its site photos on cards.

use fframes::{Duration, FFramesContext, Frame, Scene, Svgr};

use super::Sfx;
use super::art::{Align, Card, body, kicker_in, lockup, wash_left};
use crate::{Grade, Shot, reel, scene_fade};

super::scene_length!(EngineeringScene, 5);
super::scene_length!(HealthScene, 6);
super::scene_length!(BusinessScene, 7);
super::scene_length!(ArtsScene, 8);

/// Each college's colour (`--primary` of its site), mark (the faceted triangle its site's
/// pages carry) and lockup (its film's end card), fetched as `mark-*.png` and `lockup-*-wide.png`.
pub(crate) const CECS: (&str, &str, &str) = ("#C72127", "mark-cecs.png", "lockup-cecs-wide.png");
pub(crate) const CHS: (&str, &str, &str) = ("#018369", "mark-chs.png", "lockup-chs-wide.png");
pub(crate) const CBM: (&str, &str, &str) = ("#FFA200", "mark-cbm.png", "lockup-cbm-wide.png");
pub(crate) const CAS: (&str, &str, &str) = ("#75A7DC", "mark-cas.png", "lockup-cas-wide.png");

struct College {
    kicker: &'static str,
    /// Colour, mark and lockup.
    brand: (&'static str, &'static str, &'static str),
    line: &'static str,
    shots: &'static [Shot],
    /// Two photos from the college's site, captioned with the heading they sit under
    /// there.
    photos: [(&'static str, &'static str); 2],
}

const ENGINEERING: College = College {
    kicker: "COLLEGE 01 / 04",
    brand: CECS,
    line: "Shape the future of innovation",
    shots: &[
        Shot::Clip("machine.mp4", 0.0),
        Shot::Clip("printer.mp4", 1.5),
        Shot::Clip("computers.mp4", 4.42),
    ],
    photos: [
        ("engineering-team.jpg", "INTERDISCIPLINARY PROJECTS"),
        ("engineering-robotics.jpg", "INNOVATION & RESEARCH"),
    ],
};
const HEALTH: College = College {
    kicker: "COLLEGE 02 / 04",
    brand: CHS,
    line: "Health leaders to advance the future of care",
    shots: &[
        Shot::Clip("surgery.mp4", 0.0),
        Shot::Clip("sim.mp4", 3.2),
        Shot::Clip("chshall.mp4", 6.0),
    ],
    photos: [
        ("health-nursing.jpg", "BACHELOR OF NURSING"),
        ("health-3d.jpg", "3D TECHNOLOGY IN MEDICINE"),
    ],
};
const BUSINESS: College = College {
    kicker: "COLLEGE 03 / 04",
    brand: CBM,
    line: "Developing future entrepreneurs and leaders",
    shots: &[
        Shot::Clip("discuss.mp4", 0.0),
        Shot::Clip("meeting.mp4", 2.1),
        Shot::Clip("hackteam.mp4", 6.0),
    ],
    photos: [
        ("business-study.jpg", "PROGRAMS VALIDATED BY CORNELL"),
        ("business-team.jpg", "TEAM PROJECTS"),
    ],
};
const ARTS: College = College {
    kicker: "COLLEGE 04 / 04",
    brand: CAS,
    line: "Redefining personalized learning",
    shots: &[
        Shot::Clip("lab.mp4", 0.0),
        Shot::Clip("labclass.mp4", 3.1),
        Shot::Clip("casclass.mp4", 6.3),
    ],
    photos: [
        ("arts-campus.jpg", "SOCIAL SCIENCES & HUMANITIES"),
        ("arts-study.jpg", "ECONOMICS · PSYCHOLOGY"),
    ],
};

/// The shots of the college scene at `index` of [`super::LENGTH`]; none for other scenes.
pub(crate) fn shots(index: usize) -> &'static [Shot] {
    match index {
        EngineeringScene::INDEX => ENGINEERING.shots,
        HealthScene::INDEX => HEALTH.shots,
        BusinessScene::INDEX => BUSINESS.shots,
        ArtsScene::INDEX => ARTS.shots,
        _ => &[],
    }
}

const NAME_AT: f32 = 0.4;
const LINE_AT: f32 = 1.4;
/// The photo cards land on the beats after the line.
const CARD_AT: [f32; 2] = [1.5, 2.0];

/// The same cues for every college: the name lands, then its line and the two cards.
pub(crate) fn sfx() -> Vec<Sfx> {
    vec![
        ("pop.wav", NAME_AT, -13.0, -0.2),
        ("blip.wav", LINE_AT, -15.0, 0.2),
        ("swoosh.wav", CARD_AT[0], -18.0, 0.4),
        ("swoosh.wav", CARD_AT[1], -18.0, 0.5),
    ]
}

fn render<'a>(
    index: usize,
    college: &College,
    length: f32,
    frame: &Frame,
    ctx: &FFramesContext<'a, '_>,
) -> Svgr<'a> {
    let t = frame.seconds();
    let shots = reel(frame, ctx, index, college.shots, Grade::default());
    let card = |i: usize, center, size, tilt, id| Card {
        file: college.photos[i].0,
        id,
        caption: college.photos[i].1,
        center,
        size,
        tilt,
        at: CARD_AT[i],
        until: length,
        accent: college.brand.0,
        logo: false,
        mark: None,
    };
    let cards = [
        card(0, (1450.0, 330.0), (520.0, 320.0), 3.0, "college-card-a"),
        card(1, (1520.0, 710.0), (440.0, 280.0), -4.0, "college-card-b"),
    ];
    fframes::svgr!(<g>
        {shots}
        {wash_left(0.9)}
        <g opacity={scene_fade(t, length)}>
            {kicker_in(college.kicker, (110.0, 680.0), Align::Start, t, 0.2, college.brand.0)}
            {lockup(ctx, college.brand, (104.0, 702.0), t, NAME_AT)}
            {body(college.line, (110.0, 990.0), 34.0, Align::Start, t, LINE_AT)}
            {cards[0].draw(ctx, t)}
            {cards[1].draw(ctx, t)}
        </g>
    </g>)
}

macro_rules! college_scene {
    ($scene:ident, $college:ident, $key:literal) => {
        impl Scene for $scene {
            fn duration(&self) -> Duration<'_> {
                Duration::Seconds(Self::LENGTH)
            }

            fn editor_instance_key(&self) -> Option<&str> {
                Some($key)
            }

            fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
                render(Self::INDEX, &$college, Self::LENGTH, &frame, ctx)
            }
        }
    };
}

college_scene!(EngineeringScene, ENGINEERING, "engineering");
college_scene!(HealthScene, HEALTH, "health");
college_scene!(BusinessScene, BUSINESS, "business");
college_scene!(ArtsScene, ARTS, "arts");
