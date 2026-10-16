//! Scored search over the session list.
//!
//! Same idea as a thread palette's scorer: every query word must match some field; a match
//! scores its field's weight, plus half again when it starts a word; a field's best match
//! counts once per word. A title hit outranks any prompt-text hit, ties go to the more
//! recently updated session, and an empty query keeps the given order (newest first).

use super::{SessionEntry, outcome_chip};

const TITLE: u32 = 100;
const AGENT: u32 = 40;
const PROMPT: u32 = 30;
const OUTCOME: u32 = 20;

/// Lowercased whitespace-separated query words.
pub fn search_terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// `None` when some term matches no field. With no terms every entry matches with score 0.
pub fn score(entry: &SessionEntry, terms: &[String]) -> Option<u32> {
    if terms.is_empty() {
        return Some(0);
    }
    let agent = entry.agent.as_deref().unwrap_or("").to_lowercase();
    let outcome = entry
        .outcome
        .map_or_else(String::new, |kind| outcome_chip(kind).0.to_lowercase());
    let haystacks = [
        (entry.title.to_lowercase(), TITLE),
        (agent, AGENT),
        (entry.prompt_text.to_lowercase(), PROMPT),
        (outcome, OUTCOME),
    ];
    let mut total = 0;
    for term in terms {
        total += haystacks
            .iter()
            .filter_map(|(haystack, weight)| field_score(haystack, term, *weight))
            .max()?;
    }
    Some(total)
}

fn field_score(haystack: &str, term: &str, weight: u32) -> Option<u32> {
    let mut best = None;
    for (at, _) in haystack.match_indices(term) {
        let starts_word = haystack[..at]
            .chars()
            .next_back()
            .is_none_or(|previous| !previous.is_alphanumeric());
        if starts_word {
            return Some(weight + weight / 2);
        }
        best = Some(weight);
    }
    best
}

/// Indices into `entries` of the matches, best first. An empty (or blank) query returns every
/// index in the given order.
pub fn matches(entries: &[SessionEntry], query: &str) -> Vec<usize> {
    let terms = search_terms(query);
    if terms.is_empty() {
        return (0..entries.len()).collect();
    }
    let mut scored: Vec<(usize, u32)> = entries
        .iter()
        .enumerate()
        .filter_map(|(ix, entry)| score(entry, &terms).map(|s| (ix, s)))
        .collect();
    scored.sort_by(|(a, score_a), (b, score_b)| {
        score_b
            .cmp(score_a)
            .then(entries[*b].updated_unix.cmp(&entries[*a].updated_unix))
            .then(a.cmp(b))
    });
    scored.into_iter().map(|(ix, _)| ix).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, prompt: &str, updated: u64) -> SessionEntry {
        SessionEntry {
            prompt_text: prompt.into(),
            updated_unix: updated,
            ..SessionEntry::new(title, title)
        }
    }

    #[test]
    fn every_word_must_match_some_field() {
        let mut e = entry("Fix the intro", "make it faster", 1);
        e.agent = Some("claude".into());
        assert!(score(&e, &search_terms("intro faster")).is_some());
        assert!(score(&e, &search_terms("claude fast")).is_some());
        assert!(score(&e, &search_terms("intro outro")).is_none());
    }

    #[test]
    fn a_title_match_outranks_a_prompt_match_and_word_starts_earn_a_bonus() {
        let terms = search_terms("logo");
        let title = score(&entry("Logo pass", "", 1), &terms).unwrap();
        let body = score(&entry("Pass", "add the logo", 1), &terms).unwrap();
        let prefix = score(&entry("Pass", "the logos", 1), &terms).unwrap();
        assert_eq!((title, body, prefix), (150, 45, 45));
        let mid_word = score(&entry("Pass", "technologo", 1), &terms).unwrap();
        assert_eq!(mid_word, 30);
    }

    #[test]
    fn matches_rank_by_score_then_recency_and_an_empty_query_keeps_order() {
        let entries = [
            entry("Newest", "mentions logo here", 30),
            entry("Logo title", "", 10),
            entry("Older body", "also logo", 20),
        ];
        assert_eq!(matches(&entries, "logo"), vec![1, 0, 2]);
        assert_eq!(matches(&entries, "  "), vec![0, 1, 2]);
        assert_eq!(matches(&entries, "zebra"), Vec::<usize>::new());
    }
}
