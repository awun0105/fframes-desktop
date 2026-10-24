use std::borrow::Cow;

use fframes::{
    Color, Svgr,
    diagnostics::{Diagnostic, report},
    usvgr::roxmltree,
};

use crate::{
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{TextAnchor, line};

/// Manim's `MarkupText`: one line with Pango-style tags, `"a <b>bold</b> <span
/// foreground='#ff5500'>word</span>"`.
///
/// - `<b>`, `<i>`, `<u>`: bold, italic, underline.
/// - `<span>` with `foreground` (or `color`, `fgcolor`): any SVG color; `size`: pixels or
///   `xx-small` to `xx-large`, `smaller`, `larger`; `weight`: `bold`, `normal` or 1 to 1000;
///   `style`: `italic` or `normal`; `underline`: `single` or `none`.
///
/// Bold and italic draw with the bold and italic faces of the family, so those font files
/// must be in the media. Unknown tags and attributes are drawn without their style and
/// `inspect` warns about them; markup that is not well formed is drawn as it is.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct MarkupText<'a> {
    pub markup: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub style: TextStyle,
    pub fill: Color,
    pub anchor: TextAnchor,
    pub opacity: f32,
}

impl Themed for MarkupText<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            markup: Cow::Borrowed(""),
            x: 0.0,
            y: 0.0,
            style: t.typography.heading,
            fill: t.colors.text,
            anchor: TextAnchor::Start,
            opacity: 1.0,
        }
    }
}

impl Default for MarkupText<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// A piece of text with the style of the tags around it.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkupRun {
    pub text: String,
    /// An SVG color, `None` for the text's fill.
    pub fill: Option<String>,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
    pub underline: bool,
}

/// The runs of `markup` in `style` and what in it was not understood.
///
/// # Errors
///
/// The parser's message when `markup` is not well formed.
pub fn parse_markup(
    markup: &str,
    style: TextStyle,
) -> Result<(Vec<MarkupRun>, Vec<String>), String> {
    let wrapped = format!("<markup>{markup}</markup>");
    let doc = roxmltree::Document::parse(&wrapped).map_err(|e| e.to_string())?;
    let base = MarkupRun {
        text: String::new(),
        fill: None,
        size: style.size as f32,
        weight: style.weight,
        italic: false,
        underline: false,
    };
    let (mut runs, mut unknown) = (Vec::new(), Vec::new());
    for child in doc.root_element().children() {
        walk(child, &base, &mut runs, &mut unknown);
    }
    Ok((runs, unknown))
}

fn walk(
    node: roxmltree::Node,
    style: &MarkupRun,
    runs: &mut Vec<MarkupRun>,
    unknown: &mut Vec<String>,
) {
    if node.is_text() {
        if let Some(text) = node.text().filter(|t| !t.is_empty()) {
            runs.push(MarkupRun {
                text: text.to_owned(),
                ..style.clone()
            });
        }
        return;
    }
    if !node.is_element() {
        return;
    }
    let mut style = style.clone();
    match node.tag_name().name() {
        "b" => style.weight = 700,
        "i" => style.italic = true,
        "u" => style.underline = true,
        "span" => {
            for a in node.attributes() {
                if !span_attribute(&mut style, a.name(), a.value()) {
                    unknown.push(format!(
                        "MarkupText: <span {}='{}'> is not drawn",
                        a.name(),
                        a.value()
                    ));
                }
            }
        }
        tag => unknown.push(format!("MarkupText: <{tag}> is not drawn")),
    }
    for child in node.children() {
        walk(child, &style, runs, unknown);
    }
}

/// Applies one `<span>` attribute, `false` when it is not understood.
fn span_attribute(style: &mut MarkupRun, name: &str, value: &str) -> bool {
    match name {
        "foreground" | "fgcolor" | "color" => style.fill = Some(value.to_owned()),
        "size" => {
            let scale = match value {
                "xx-small" => 0.579,
                "x-small" => 0.694,
                "small" | "smaller" => 0.833,
                "medium" => 1.0,
                "large" | "larger" => 1.2,
                "x-large" => 1.44,
                "xx-large" => 1.728,
                px => match px.trim_end_matches("px").parse::<f32>() {
                    Ok(px) if px > 0.0 => {
                        style.size = px;
                        return true;
                    }
                    _ => return false,
                },
            };
            style.size *= scale;
        }
        "weight" => match value {
            "bold" => style.weight = 700,
            "normal" => style.weight = 400,
            n => match n.parse::<u16>() {
                Ok(w @ 1..=1000) => style.weight = w,
                _ => return false,
            },
        },
        "style" => match value {
            "italic" | "oblique" => style.italic = true,
            "normal" => style.italic = false,
            _ => return false,
        },
        "underline" => match value {
            "single" | "double" | "low" => style.underline = true,
            "none" => style.underline = false,
            _ => return false,
        },
        _ => return false,
    }
    true
}

impl<'a> Component<'a> for MarkupText<'a> {
    fn render(self, _ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let s = self.style;
        let runs = match parse_markup(&self.markup, s) {
            Ok((runs, unknown)) => {
                for message in unknown {
                    report(Diagnostic::RendererWarning { message });
                }
                runs
            }
            Err(e) => {
                report(Diagnostic::RendererWarning {
                    message: format!("MarkupText: drawn without markup, {e}"),
                });
                return line(
                    self.x,
                    self.y,
                    s,
                    self.fill,
                    self.anchor,
                    0.0,
                    self.opacity,
                    self.markup,
                );
            }
        };
        let fill = self.fill.to_string();
        let tspans: Vec<Svgr<'a>> = runs
            .into_iter()
            .map(|r| {
                let fill = r.fill.unwrap_or_else(|| fill.clone());
                let font_style = if r.italic { "italic" } else { "normal" };
                let decoration = if r.underline { "underline" } else { "none" };
                fframes::svgr!(
                    <tspan fill={fill} font-size={r.size} font-weight={r.weight}
                           font-style={font_style} text-decoration={decoration}>{r.text}</tspan>
                )
            })
            .collect();
        fframes::svgr!(
            <text x={self.x} y={self.y} font-family={s.family} font-size={s.size} font-weight={s.weight}
                  text-anchor={self.anchor.as_str()} opacity={self.opacity}>
                {tspans}
            </text>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_become_runs() {
        let style = Theme::default().typography.body;
        let (runs, unknown) = parse_markup(
            "a <b>bold <i>both</i></b> <span foreground='#f50' size='large' underline='single'>x</span><blink>!</blink>",
            style,
        )
        .unwrap();
        let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, ["a ", "bold ", "both", " ", "x", "!"]);
        assert_eq!((runs[1].weight, runs[1].italic), (700, false));
        assert_eq!((runs[2].weight, runs[2].italic), (700, true));
        assert_eq!(runs[4].fill.as_deref(), Some("#f50"));
        assert!(runs[4].underline);
        assert!((runs[4].size - style.size as f32 * 1.2).abs() < 1e-3);
        assert_eq!(
            runs[5],
            MarkupRun {
                text: "!".into(),
                ..runs[0].clone()
            }
        );
        assert_eq!(unknown, ["MarkupText: <blink> is not drawn"]);
        assert!(parse_markup("<b>open", style).is_err());
        assert_eq!(parse_markup("1 &lt; 2", style).unwrap().0[0].text, "1 < 2");
    }
}
