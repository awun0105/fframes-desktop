//! Slash command completion: which token is being typed, which commands match
//! it, and what accepting one inserts.
//!
//! Commands are display and insertion only. The owner supplies the list the
//! agent advertises; accepting a row types `/name ` into the draft and nothing
//! else happens until the user sends it.

use std::ops::Range;

use gpui::SharedString;
use unicode_segmentation::UnicodeSegmentation;

/// A command the agent advertises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommand {
    /// The name without the leading slash.
    pub name: SharedString,
    pub description: SharedString,
}

impl SlashCommand {
    pub fn new(name: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// The `/word` under the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashToken {
    /// The whole command word, including text after the caret.
    pub range: Range<usize>,
    /// What was typed after the slash, up to the caret.
    pub query: String,
}

/// The slash token the caret is in, if any.
///
/// A token starts at the beginning of the draft or after whitespace or one of
/// `([{>`, and the caret must sit inside the word. Paths such as `/usr/bin` and
/// text after the command word (its arguments) are not tokens.
///
/// Text in a Markdown code span is not recognised as code here, so a slash
/// typed inside backticks still opens the menu.
pub fn slash_token(text: &str, cursor: usize) -> Option<SlashToken> {
    if cursor > text.len() || !text.is_char_boundary(cursor) {
        return None;
    }
    let start = text[..cursor]
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace() || matches!(c, '(' | '[' | '{' | '>'))
        .map_or(0, |(i, c)| i + c.len_utf8());
    if !text[start..cursor].starts_with('/') {
        return None;
    }
    let query = &text[start + 1..cursor];
    let name_grapheme = |grapheme: &str| {
        grapheme.chars().next().is_some_and(char::is_alphanumeric)
            || matches!(grapheme, "-" | "_" | ":" | ".")
    };
    if !query.graphemes(true).all(name_grapheme) {
        return None;
    }
    // Scan from the token's start so combining marks after a caret placed
    // between code points of one grapheme stay in the word.
    let end = text[start + 1..]
        .grapheme_indices(true)
        .find_map(|(offset, grapheme)| (!name_grapheme(grapheme)).then_some(start + 1 + offset))
        .unwrap_or(text.len());
    if text.get(end..end + 1) == Some("/") {
        return None;
    }
    Some(SlashToken {
        range: start..end,
        query: query.to_string(),
    })
}

/// Indices of the commands matching `query`, best first.
///
/// Order: names starting with the query, then names containing it, then names
/// holding its letters in order. Case is ignored and ties keep list order. An
/// empty query matches everything.
pub fn filter_indices(commands: &[SlashCommand], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return (0..commands.len()).collect();
    }
    let mut ranked: Vec<(u8, usize)> = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let name = command.name.to_lowercase();
            let rank = if name.starts_with(&query) {
                0
            } else if name.contains(&query) {
                1
            } else if is_subsequence(&query, &name) {
                2
            } else {
                return None;
            };
            Some((rank, index))
        })
        .collect();
    ranked.sort_by_key(|(rank, index)| (*rank, *index));
    ranked.into_iter().map(|(_, index)| index).collect()
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut hay = haystack.chars();
    needle.chars().all(|n| hay.any(|h| h == n))
}

/// Moves the highlighted row by `delta`, wrapping at both ends.
pub fn menu_step(active: Option<usize>, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let len = len as isize;
    let next = match active {
        Some(current) => (current as isize + delta).rem_euclid(len),
        None if delta < 0 => len - 1,
        None => 0,
    };
    Some(next as usize)
}

/// What the menu is showing for the draft.
#[derive(Debug, Clone, Default)]
pub struct SlashState {
    token: Option<SlashToken>,
    /// Indices into the owner's command list, best match first.
    filtered: Vec<usize>,
    active: Option<usize>,
    /// The token text the user closed with Escape. It stays closed until the
    /// token's text changes.
    dismissed: Option<(Range<usize>, String)>,
}

impl SlashState {
    /// Recomputes the menu for the draft, the caret and the command list.
    pub fn refresh(&mut self, text: &str, cursor: usize, commands: &[SlashCommand]) {
        let token = slash_token(text, cursor);
        let full = token
            .as_ref()
            .map(|t| (t.range.clone(), text[t.range.clone()].to_string()));
        if self.dismissed != full {
            self.dismissed = None;
        }
        let previous = self.selected_command();
        self.filtered = match &token {
            Some(token) if self.dismissed.is_none() => filter_indices(commands, &token.query),
            _ => Vec::new(),
        };
        self.token = token;
        // Keep the highlight on the same command while the list narrows.
        self.active = previous
            .and_then(|command| self.filtered.iter().position(|index| *index == command))
            .or(if self.filtered.is_empty() {
                None
            } else {
                Some(0)
            });
    }

    /// Whether the menu is showing rows.
    pub fn is_open(&self) -> bool {
        self.token.is_some() && !self.filtered.is_empty()
    }

    pub fn token(&self) -> Option<&SlashToken> {
        self.token.as_ref()
    }

    /// Indices into the command list, in display order.
    pub fn rows(&self) -> &[usize] {
        &self.filtered
    }

    /// The highlighted row, as a position in [`rows`](Self::rows).
    pub fn active_row(&self) -> Option<usize> {
        self.active
    }

    /// The highlighted command's index in the owner's list.
    pub fn selected_command(&self) -> Option<usize> {
        self.active.and_then(|row| self.filtered.get(row).copied())
    }

    pub fn navigate(&mut self, delta: isize) {
        self.active = menu_step(self.active, self.filtered.len(), delta);
    }

    /// Highlights a row, as hovering one does.
    pub fn set_active_row(&mut self, row: usize) {
        if row < self.filtered.len() {
            self.active = Some(row);
        }
    }

    /// Closes the menu for the current token.
    pub fn dismiss(&mut self, text: &str) {
        if let Some(token) = &self.token {
            self.dismissed = Some((token.range.clone(), text[token.range.clone()].to_string()));
        }
        self.filtered.clear();
        self.active = None;
    }

    /// The range to replace and the text to put there for the command at
    /// `command` in `commands`: `/name `, without a second space when one
    /// already follows the word.
    pub fn insertion(
        &self,
        text: &str,
        commands: &[SlashCommand],
        command: usize,
    ) -> Option<(Range<usize>, String)> {
        let token = self.token.as_ref()?;
        let command = commands.get(command)?;
        let space_follows = text[token.range.end..].starts_with(char::is_whitespace);
        let suffix = if space_follows { "" } else { " " };
        Some((token.range.clone(), format!("/{}{suffix}", command.name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands() -> Vec<SlashCommand> {
        ["compact", "review", "init", "clear", "cost"]
            .into_iter()
            .map(|name| SlashCommand::new(name, format!("{name} description")))
            .collect()
    }

    fn names(commands: &[SlashCommand], rows: &[usize]) -> Vec<String> {
        rows.iter().map(|i| commands[*i].name.to_string()).collect()
    }

    #[test]
    fn slash_token_opens_at_prose_boundaries() {
        assert_eq!(
            slash_token("/comp", 5),
            Some(SlashToken {
                range: 0..5,
                query: "comp".into()
            })
        );
        // The range spans the whole command word even with the caret inside it.
        assert_eq!(
            slash_token("/compact now", 3),
            Some(SlashToken {
                range: 0..8,
                query: "co".into()
            })
        );
        assert_eq!(slash_token("run /compact", 12).unwrap().range, 4..12);
        assert_eq!(slash_token("(/x", 3).unwrap().range, 1..3);
        // The caret past the command word is typing its argument.
        assert!(slash_token("/goal ship it", 10).is_none());
        // A typed absolute path or a URL piece is not a command.
        assert!(slash_token("/usr/bin", 8).is_none());
        assert!(slash_token("a/b", 3).is_none());
        // A bare slash opens only once the caret is after it.
        assert!(slash_token("/", 0).is_none());
        assert_eq!(slash_token("/", 1).map(|t| t.query), Some(String::new()));
        // An offset inside a character is not a position.
        assert!(slash_token("/é", 2).is_none());
    }

    #[test]
    fn filtering_ranks_prefix_before_substring_before_letters_in_order() {
        let list = commands();
        assert_eq!(
            names(&list, &filter_indices(&list, "")),
            ["compact", "review", "init", "clear", "cost"]
        );
        assert_eq!(
            names(&list, &filter_indices(&list, "c")),
            ["compact", "clear", "cost"]
        );
        assert_eq!(
            names(&list, &filter_indices(&list, "CO")),
            ["compact", "cost"]
        );
        // "view" is inside "review"; "cpt" is only letters of "compact" in order.
        assert_eq!(names(&list, &filter_indices(&list, "view")), ["review"]);
        assert_eq!(names(&list, &filter_indices(&list, "cpt")), ["compact"]);
        assert!(filter_indices(&list, "zzz").is_empty());
    }

    #[test]
    fn menu_step_wraps_and_handles_an_empty_list() {
        assert_eq!(menu_step(None, 0, 1), None);
        assert_eq!(menu_step(None, 3, 1), Some(0));
        assert_eq!(menu_step(None, 3, -1), Some(2));
        assert_eq!(menu_step(Some(2), 3, 1), Some(0));
        assert_eq!(menu_step(Some(0), 3, -1), Some(2));
        assert_eq!(menu_step(Some(1), 3, 1), Some(2));
    }

    #[test]
    fn refresh_opens_narrows_and_keeps_the_highlight_on_one_command() {
        let list = commands();
        let mut state = SlashState::default();
        state.refresh("/", 1, &list);
        assert!(state.is_open());
        assert_eq!(state.rows().len(), 5);
        state.navigate(1);
        assert_eq!(state.selected_command(), Some(1));
        // Narrowing to a list that still holds "review" keeps it highlighted.
        state.refresh("/re", 3, &list);
        assert_eq!(state.selected_command(), Some(1));
        // Text that matches nothing closes the menu.
        state.refresh("/zzz", 4, &list);
        assert!(!state.is_open());
        // Leaving the token closes it.
        state.refresh("/zzz and more", 13, &list);
        assert!(state.token().is_none());
    }

    #[test]
    fn dismissed_menu_stays_closed_until_the_token_changes() {
        let list = commands();
        let mut state = SlashState::default();
        state.refresh("/co", 3, &list);
        assert!(state.is_open());
        state.dismiss("/co");
        assert!(!state.is_open());
        // Moving the caret inside the same word keeps it closed.
        state.refresh("/co", 2, &list);
        assert!(!state.is_open());
        // Typing changes the token and reopens it.
        state.refresh("/com", 4, &list);
        assert!(state.is_open());
    }

    #[test]
    fn insertion_replaces_the_word_and_adds_one_space() {
        let list = commands();
        let mut state = SlashState::default();
        state.refresh("run /co", 7, &list);
        let (range, text) = state.insertion("run /co", &list, 0).unwrap();
        assert_eq!(range, 4..7);
        assert_eq!(text, "/compact ");
        // A space that already follows the word is not doubled.
        state.refresh("/co rest", 3, &list);
        assert_eq!(state.insertion("/co rest", &list, 0).unwrap().1, "/compact");
        assert!(state.insertion("/co rest", &list, 99).is_none());
    }
}
