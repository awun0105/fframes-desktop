//! Google's Noto animated emoji: the names, their download addresses and the credit line.
//! The kit ships no emoji files. Download the ones a video uses into its media folder as
//! [`file_name`] says, and play them with [`AnimatedEmoji`] (feature `lottie`).

/// The credit line for a video that shows Noto animated emoji (CC BY 4.0).
pub const ATTRIBUTION: &str = "Animated emoji: Noto Emoji Animation by Google (https://googlefonts.github.io/noto-emoji-animation/), CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/)";

/// One animated emoji: the kit's name for it, its Unicode code points as Noto writes them
/// (`_` between them), and whether it comes in skin tones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmojiName {
    pub name: &'static str,
    pub code: &'static str,
    pub skin_tones: bool,
}

const fn e(name: &'static str, code: &'static str) -> EmojiName {
    EmojiName {
        name,
        code,
        skin_tones: false,
    }
}

const fn toned(name: &'static str, code: &'static str) -> EmojiName {
    EmojiName {
        name,
        code,
        skin_tones: true,
    }
}

/// The animated emoji the kit knows by name. Each one was checked to download. Noto has more:
/// any code point of <https://googlefonts.github.io/noto-emoji-animation/> works with
/// [`url_for_code`] and a file name of your own.
pub const NAMES: &[EmojiName] = &[
    e("grinning", "1f600"),
    e("smiley", "1f603"),
    e("smile", "1f604"),
    e("grin", "1f601"),
    e("laughing", "1f606"),
    e("sweat-smile", "1f605"),
    e("joy", "1f602"),
    e("rofl", "1f923"),
    e("wink", "1f609"),
    e("blush", "1f60a"),
    e("innocent", "1f607"),
    e("heart-eyes", "1f60d"),
    e("star-struck", "1f929"),
    e("kissing-heart", "1f618"),
    e("yum", "1f60b"),
    e("stuck-out-tongue", "1f61b"),
    e("zany-face", "1f92a"),
    e("thinking-face", "1f914"),
    e("shushing-face", "1f92b"),
    e("neutral-face", "1f610"),
    e("expressionless", "1f611"),
    e("rolling-eyes", "1f644"),
    e("grimacing", "1f62c"),
    e("relieved", "1f60c"),
    e("pensive", "1f614"),
    e("sleeping", "1f634"),
    e("mask", "1f637"),
    e("hot-face", "1f975"),
    e("cold-face", "1f976"),
    e("exploding-head", "1f92f"),
    e("cowboy-face", "1f920"),
    e("partying-face", "1f973"),
    e("sunglasses", "1f60e"),
    e("nerd-face", "1f913"),
    e("confused", "1f615"),
    e("worried", "1f61f"),
    e("open-mouth", "1f62e"),
    e("astonished", "1f632"),
    e("flushed", "1f633"),
    e("pleading", "1f97a"),
    e("cry", "1f622"),
    e("sob", "1f62d"),
    e("scream", "1f631"),
    e("angry", "1f620"),
    e("rage", "1f621"),
    e("skull", "1f480"),
    e("clown", "1f921"),
    e("ghost", "1f47b"),
    e("alien", "1f47d"),
    e("robot", "1f916"),
    e("poop", "1f4a9"),
    e("red-heart", "2764_fe0f"),
    e("broken-heart", "1f494"),
    e("sparkling-heart", "1f496"),
    e("fire", "1f525"),
    e("sparkles", "2728"),
    e("hundred-points", "1f4af"),
    e("collision", "1f4a5"),
    toned("thumbs-up", "1f44d"),
    toned("thumbs-down", "1f44e"),
    toned("clap", "1f44f"),
    toned("wave", "1f44b"),
    toned("raised-hands", "1f64c"),
    toned("folded-hands", "1f64f"),
    toned("muscle", "1f4aa"),
    toned("ok-hand", "1f44c"),
    toned("victory", "270c_fe0f"),
    toned("crossed-fingers", "1f91e"),
    e("party-popper", "1f389"),
    e("rocket", "1f680"),
    e("eyes", "1f440"),
    e("confetti-ball", "1f38a"),
    e("gift", "1f381"),
    e("light-bulb", "1f4a1"),
    e("check-mark", "2705"),
    e("cross-mark", "274c"),
    e("star", "2b50"),
    e("glowing-star", "1f31f"),
    e("rainbow", "1f308"),
    e("high-voltage", "26a1"),
];

/// A skin tone of the hand emoji (Remotion's `skinTone`), the Fitzpatrick modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum SkinTone {
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

impl SkinTone {
    /// The modifier's code point.
    pub fn code(self) -> &'static str {
        match self {
            SkinTone::Light => "1f3fb",
            SkinTone::MediumLight => "1f3fc",
            SkinTone::Medium => "1f3fd",
            SkinTone::MediumDark => "1f3fe",
            SkinTone::Dark => "1f3ff",
        }
    }

    /// The tone in a file name.
    pub fn slug(self) -> &'static str {
        match self {
            SkinTone::Light => "light",
            SkinTone::MediumLight => "medium-light",
            SkinTone::Medium => "medium",
            SkinTone::MediumDark => "medium-dark",
            SkinTone::Dark => "dark",
        }
    }
}

/// The emoji called `name` in [`NAMES`].
pub fn find(name: &str) -> Option<&'static EmojiName> {
    NAMES.iter().find(|e| e.name == name)
}

/// The download address of the Lottie file of the code points `code` (`1f44d`, `2764_fe0f`)
/// with an optional skin tone. A tone replaces the variation selector `fe0f`.
pub fn url_for_code(code: &str, tone: Option<SkinTone>) -> String {
    let code = match tone {
        Some(tone) => format!("{}_{}", code.trim_end_matches("_fe0f"), tone.code()),
        None => code.to_owned(),
    };
    format!("https://fonts.gstatic.com/s/e/notoemoji/latest/{code}/lottie.json")
}

/// The download address of the emoji called `name`, `None` for a name that is not in
/// [`NAMES`]. The tone is left out for an emoji without skin tones.
pub fn url(name: &str, tone: Option<SkinTone>) -> Option<String> {
    let emoji = find(name)?;
    Some(url_for_code(emoji.code, tone.filter(|_| emoji.skin_tones)))
}

/// The file in the media folder that [`AnimatedEmoji`] plays: `<name>.json`, or
/// `<name>-<tone>.json` with a skin tone (`thumbs-up-medium-dark.json`).
pub fn file_name(name: &str, tone: Option<SkinTone>) -> String {
    match tone {
        Some(tone) => format!("{name}-{}.json", tone.slug()),
        None => format!("{name}.json"),
    }
}

#[cfg(feature = "lottie")]
pub use animated::AnimatedEmoji;

#[cfg(feature = "lottie")]
mod animated {
    use fframes::{
        Svgr,
        diagnostics::{Diagnostic, report},
    };

    use super::{SkinTone, file_name, find, url};
    use crate::{
        motion::progress,
        theme::Theme,
        ui::{Component, Themed, Ui},
    };

    /// A Noto animated emoji (Remotion's `<AnimatedEmoji>`), played from `start` seconds of
    /// its container from the file [`file_name`] gives in the media folder. `(x, y)` is the
    /// top left of a `size` pixel square. It fades in over `fade` seconds and plays at
    /// `speed`, again and again with `looping`, otherwise its last frame stays. A missing
    /// file draws nothing, and `inspect` warns with the address to download it from. The
    /// tone applies to the emoji that have skin tones.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
    pub struct AnimatedEmoji<'a> {
        pub name: &'a str,
        pub skin_tone: Option<SkinTone>,
        pub x: f32,
        pub y: f32,
        pub size: f32,
        pub start: f32,
        pub speed: f32,
        pub looping: bool,
        pub fade: f32,
    }

    impl Themed for AnimatedEmoji<'_> {
        fn themed(t: &Theme) -> Self {
            Self {
                name: "grinning",
                skin_tone: None,
                x: 880.0,
                y: 460.0,
                size: 160.0,
                start: 0.0,
                speed: 1.0,
                looping: true,
                fade: t.motion.duration.fast,
            }
        }
    }

    impl Default for AnimatedEmoji<'_> {
        fn default() -> Self {
            Self::themed(&Theme::default())
        }
    }

    impl AnimatedEmoji<'_> {
        /// The skin tone the file has: none for an emoji in [`NAMES`](super::NAMES) without
        /// tones.
        fn tone(&self) -> Option<SkinTone> {
            match find(self.name) {
                Some(emoji) if !emoji.skin_tones => None,
                _ => self.skin_tone,
            }
        }

        /// The file this emoji plays.
        pub fn file(&self) -> String {
            file_name(self.name, self.tone())
        }
    }

    impl<'a> Component<'a> for AnimatedEmoji<'a> {
        fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
            let file = self.file();
            // Not through `get_lottie`: a missing emoji is a warning with its address here.
            let Some(lottie) = ui.ctx.media_source.and_then(|m| m.resolve_lottie(&file)) else {
                let message = match url(self.name, self.tone()) {
                    Some(url) => format!(
                        "AnimatedEmoji: \"{file}\" is not in the media folder, download {url} and save it as {file}"
                    ),
                    None => format!(
                        "AnimatedEmoji: \"{file}\" is not in the media folder and \"{}\" is not in emoji::NAMES",
                        self.name
                    ),
                };
                report(Diagnostic::RendererWarning { message });
                return Svgr::empty();
            };
            let t = ui.seconds();
            let opacity = progress(t, self.start, self.start + self.fade);
            if opacity <= 0.0 {
                return Svgr::empty();
            }
            let local = (t - self.start) * self.speed;
            let length = lottie.seconds();
            let local = if self.looping && length > 0.0 {
                local.rem_euclid(length)
            } else {
                local
            };
            let side = lottie.width().max(lottie.height()).max(1) as f32;
            let place = format!(
                "translate({} {}) scale({})",
                self.x,
                self.y,
                self.size / side
            );
            fframes::svgr!(<g transform={place} opacity={opacity}>{lottie.render_at(local)}</g>)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_follow_the_noto_pattern() {
        assert_eq!(
            url("grinning", None).as_deref(),
            Some("https://fonts.gstatic.com/s/e/notoemoji/latest/1f600/lottie.json")
        );
        assert_eq!(
            url("thumbs-up", Some(SkinTone::Dark)).as_deref(),
            Some("https://fonts.gstatic.com/s/e/notoemoji/latest/1f44d_1f3ff/lottie.json")
        );
        // the tone replaces the variation selector
        assert_eq!(
            url("victory", Some(SkinTone::Light)).as_deref(),
            Some("https://fonts.gstatic.com/s/e/notoemoji/latest/270c_1f3fb/lottie.json")
        );
        // no tones on a face
        assert_eq!(url("joy", Some(SkinTone::Dark)), url("joy", None));
        assert_eq!(url("no-such-emoji", None), None);
        assert_eq!(
            file_name("clap", Some(SkinTone::MediumDark)),
            "clap-medium-dark.json"
        );
        assert_eq!(file_name("fire", None), "fire.json");
    }

    #[test]
    fn names_are_unique_and_codes_are_hex() {
        for (i, a) in NAMES.iter().enumerate() {
            assert!(
                NAMES[i + 1..]
                    .iter()
                    .all(|b| b.name != a.name && b.code != a.code),
                "{a:?}"
            );
            assert!(
                a.code
                    .split('_')
                    .all(|part| u32::from_str_radix(part, 16).is_ok()),
                "{a:?}"
            );
        }
        assert!(ATTRIBUTION.contains("CC BY 4.0"));
    }
}
