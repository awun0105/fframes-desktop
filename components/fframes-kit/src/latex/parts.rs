//! `{{ }}` groups in a formula's source (Manim's rule): each marks one part.

/// A `{{ }}` group of a formula's source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartSource {
    /// Its number, counted from 0 in source order.
    pub index: usize,
    /// The TeX between the double braces.
    pub tex: String,
}

/// Splits `source` into its `{{ }}` groups. A group opens at `{{` that is not followed by
/// another `{` (so `\frac{{{a}}}{b}` has the group `a`), and closes at the `}}` that
/// balances it. `wrap(index, tex)` gives what replaces each group in the returned source;
/// it must keep `tex` a braced group if spacing matters. Groups do not nest.
pub fn split_parts(
    source: &str,
    wrap: impl Fn(usize, &str) -> String,
) -> (String, Vec<PartSource>) {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut parts = Vec::new();
    let mut i = 0;
    let mut plain = 0;
    while i < bytes.len() {
        let opens = bytes[i] == b'{'
            && bytes.get(i + 1) == Some(&b'{')
            && bytes.get(i + 2) != Some(&b'{')
            && (i == 0 || bytes[i - 1] != b'\\');
        if !opens {
            i += 1;
            continue;
        }
        // find the `}}` that closes it, counting single braces inside
        let start = i + 2;
        let mut depth = 0usize;
        let mut j = start;
        let mut end = None;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' => j += 1,
                b'{' => depth += 1,
                b'}' if depth > 0 => depth -= 1,
                b'}' if bytes.get(j + 1) == Some(&b'}') => {
                    end = Some(j);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let Some(end) = end else { break };
        let tex = &source[start..end];
        out.push_str(&source[plain..i]);
        out.push_str(&wrap(parts.len(), tex));
        parts.push(PartSource {
            index: parts.len(),
            tex: tex.to_owned(),
        });
        i = end + 2;
        plain = i;
    }
    out.push_str(&source[plain..]);
    (out, parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marked(source: &str) -> (String, Vec<String>) {
        let (out, parts) = split_parts(source, |i, tex| format!("<{i}:{tex}>"));
        (out, parts.into_iter().map(|p| p.tex).collect())
    }

    #[test]
    fn groups_are_found_in_the_tricky_places() {
        assert_eq!(marked("{{a}}^2"), ("<0:a>^2".into(), vec!["a".into()]));
        assert_eq!(
            marked("\\frac{{{a}}}{b}"),
            ("\\frac{<0:a>}{b}".into(), vec!["a".into()])
        );
        assert_eq!(
            marked("\\left({{x}}\\right)"),
            ("\\left(<0:x>\\right)".into(), vec!["x".into()])
        );
        assert_eq!(
            marked("{{a^{2}}} + {{\\frac{1}{2}}}"),
            (
                "<0:a^{2}> + <1:\\frac{1}{2}>".into(),
                vec!["a^{2}".into(), "\\frac{1}{2}".into()]
            )
        );
    }

    #[test]
    fn plain_braces_and_escapes_are_not_groups() {
        assert_eq!(marked("x^{2}"), ("x^{2}".into(), vec![]));
        assert_eq!(marked("\\{{a}\\}"), ("\\{{a}\\}".into(), vec![]));
        // an unclosed group is left as it is
        assert_eq!(marked("{{a"), ("{{a".into(), vec![]));
    }
}
