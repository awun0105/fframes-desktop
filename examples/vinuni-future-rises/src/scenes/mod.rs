//! The nineteen scenes: the intro, then six chapters. Each one runs on its own clock
//! (`frame.seconds()` starts at 0); cuts sit on 2 s bars of the 120 BPM grid and accents
//! on its beats. The chapter wipe covers the cut into every chapter and every other cut
//! dissolves the footage across it ([`carry_in`]), so the film plays as one continuous
//! introduction. Each scene lists its own sound effects
//! next to the moments they accent.

use fframes::Scene;

use crate::{CHAPTERS, Shot};

pub mod art;
pub mod campus;
pub mod clubs;
pub mod college;
pub mod colleges;
pub mod community;
pub mod dawn;
pub mod globe;
pub mod join;
pub mod learn;
pub mod mission;
pub mod outcomes;
pub mod rise;
pub mod stars;
pub mod study;
pub mod values;
pub mod who;

/// Scene lengths in seconds: 156 s, 9360 frames at 60 fps.
pub const LENGTH: [f32; 19] = [
    10.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 10.0, 8.0, 10.0, 8.0, 6.0, 8.0, 8.0, 6.0, 8.0,
    10.0,
];

/// Start of each scene on the video timeline.
pub const START: [f32; 19] = {
    let mut start = [0.0; 19];
    let mut i = 1;
    while i < 19 {
        start[i] = start[i - 1] + LENGTH[i - 1];
        i += 1;
    }
    start
};

/// A sound effect: file, second (scene clock until [`sound_effects`] places it), gain in
/// dB, pan.
pub type Sfx = (&'static str, f32, f32, f32);

pub fn all() -> Vec<&'static dyn Scene> {
    vec![
        &dawn::DawnScene,
        &who::WhoScene,
        &mission::MissionScene,
        &stars::StarsScene,
        &colleges::CollegesScene,
        &college::EngineeringScene,
        &college::HealthScene,
        &college::BusinessScene,
        &college::ArtsScene,
        &learn::LearnScene,
        &community::CommunityScene,
        &globe::GlobeScene,
        &campus::CampusScene,
        &clubs::ClubsScene,
        &values::ValuesScene,
        &outcomes::OutcomesScene,
        &study::StudyScene,
        &join::JoinScene,
        &rise::RiseScene,
    ]
}

/// The full-screen shots of each scene, for the dissolve into the next one; empty for the
/// scenes that draw their own backdrop.
fn shots(index: usize) -> &'static [Shot] {
    match index {
        who::WhoScene::INDEX => &who::SHOTS,
        mission::MissionScene::INDEX => &mission::SHOTS,
        stars::StarsScene::INDEX => &stars::SHOTS,
        colleges::CollegesScene::INDEX => &colleges::SHOTS,
        learn::LearnScene::INDEX => &learn::SHOTS,
        community::CommunityScene::INDEX => &community::SHOTS,
        campus::CampusScene::INDEX => &campus::SHOTS,
        clubs::ClubsScene::INDEX => &clubs::SHOTS,
        values::ValuesScene::INDEX => &values::SHOTS,
        outcomes::OutcomesScene::INDEX => &outcomes::SHOTS,
        study::StudyScene::INDEX => &study::SHOTS,
        join::JoinScene::INDEX => &join::SHOTS,
        _ => college::shots(index),
    }
}

/// How long the footage of one scene dissolves into the next.
pub(crate) const DISSOLVE: f32 = 0.5;

/// The previous scene's last shot on scene `index`'s clock, to dissolve from, unless a
/// chapter starts at the cut (the chapter wipe covers it) or the previous scene draws its
/// own backdrop.
pub(crate) fn carry_in(index: usize) -> Option<Shot> {
    if index == 0 || CHAPTERS.iter().any(|&(start, _)| start == START[index]) {
        return None;
    }
    let previous = LENGTH[index - 1];
    shots(index - 1).last().map(|&shot| match shot {
        Shot::Clip(clip, at) => Shot::Clip(clip, at - previous),
        Shot::Photo(photo, at) => Shot::Photo(photo, at - previous),
    })
}

/// Every scene's effects on the video timeline, plus a swoosh into every cut.
pub fn sound_effects() -> Vec<Sfx> {
    let per_scene: [fn() -> Vec<Sfx>; 19] = [
        dawn::sfx,
        who::sfx,
        mission::sfx,
        stars::sfx,
        colleges::sfx,
        college::sfx,
        college::sfx,
        college::sfx,
        college::sfx,
        learn::sfx,
        community::sfx,
        globe::sfx,
        campus::sfx,
        clubs::sfx,
        values::sfx,
        outcomes::sfx,
        study::sfx,
        join::sfx,
        rise::sfx,
    ];
    let mut out = Vec::new();
    for (i, cues) in per_scene.iter().enumerate() {
        out.extend(
            cues()
                .into_iter()
                .map(|(file, second, gain, pan)| (file, START[i] + second, gain, pan)),
        );
        if i > 0 {
            let pan = if i % 2 == 0 { 0.35 } else { -0.35 };
            out.push(("swoosh.wav", START[i] - 0.6, -10.0, pan));
        }
    }
    out
}

/// Declares a scene struct, its place in [`LENGTH`] and its length.
macro_rules! scene_length {
    ($name:ident, $index:expr) => {
        #[derive(Debug)]
        pub struct $name;

        impl $name {
            #[allow(dead_code)]
            pub(crate) const INDEX: usize = $index;
            pub(crate) const LENGTH: f32 = $crate::scenes::LENGTH[$index];
        }
    };
}
pub(crate) use scene_length;

#[cfg(test)]
mod tests {
    use fframes::Duration;

    use super::{LENGTH, START, all};

    #[test]
    fn two_minutes_thirty_six_on_two_second_bars() {
        assert_eq!(LENGTH.iter().sum::<f32>(), 156.0);
        for start in START {
            assert_eq!(start % 2.0, 0.0, "cut at {start} s is off the bar grid");
        }
    }

    #[test]
    fn every_scene_has_its_length() {
        let scenes = all();
        assert_eq!(scenes.len(), LENGTH.len());
        for (i, scene) in scenes.iter().enumerate() {
            match scene.duration() {
                Duration::Seconds(seconds) => assert_eq!(seconds, LENGTH[i], "scene {i}"),
                other => panic!("scene {i} has {other:?}"),
            }
        }
    }
}
