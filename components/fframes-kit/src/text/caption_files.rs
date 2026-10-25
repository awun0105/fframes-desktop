//! Caption files as timed words: SRT, VTT and Whisper transcripts.

use std::{error::Error, fmt, fmt::Write as _};

use serde_json::Value;

/// One timed piece of a transcript: a word, or a whole cue of a subtitle file (Remotion's
/// `Caption`).
#[derive(Debug, Clone, PartialEq)]
pub struct CaptionWord {
    /// The text. A word of a transcript keeps its leading space (`" world"`): words join into
    /// a line without a separator, and a page of `tiktok_pages` only breaks before a space.
    /// A cue of a subtitle file is trimmed.
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
    /// When the word is said (Remotion's `timestampMs`). The readers here give the middle of
    /// the word or cue.
    pub timestamp_ms: Option<f64>,
    /// How sure the transcriber is, from 0 to 1. Subtitle cues have 1, transcripts without
    /// probabilities `None`.
    pub confidence: Option<f64>,
}

impl CaptionWord {
    /// A word from `start_ms` to `end_ms`, said in the middle, with no confidence.
    pub fn new(text: impl Into<String>, start_ms: f64, end_ms: f64) -> Self {
        Self {
            text: text.into(),
            start_ms,
            end_ms,
            timestamp_ms: Some(start_ms.midpoint(end_ms)),
            confidence: None,
        }
    }
}

/// Why a caption file could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptionFileError {
    /// The file does not start with the `WEBVTT` line.
    Vtt,
    /// The file is not JSON.
    Json(String),
    /// JSON, but neither an openai-whisper nor a whisper.cpp transcript.
    Whisper(&'static str),
}

impl fmt::Display for CaptionFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vtt => write!(f, "a WebVTT file starts with a WEBVTT line"),
            Self::Json(message) => write!(f, "invalid JSON: {message}"),
            Self::Whisper(message) => write!(f, "not a Whisper transcript: {message}"),
        }
    }
}

impl Error for CaptionFileError {}

/// The cues of an SRT file, one `CaptionWord` each (Remotion's `parseSrt`).
///
/// Lenient: a byte order mark and `\r\n` line ends are accepted, a cue is the text after a
/// `start --> end` line up to the next blank line, so a missing or wrong index does not
/// matter. Times are `HH:MM:SS,mmm`, with `.` also accepted and the hours optional. Cues
/// without text are left out.
pub fn parse_srt(text: &str) -> Vec<CaptionWord> {
    let text = normalize(text);
    let mut words = Vec::new();
    let mut cue: Option<(f64, f64, Vec<&str>)> = None;
    for line in text.lines() {
        if let Some((start, end)) = timing(line) {
            push_cue(&mut words, cue.take());
            cue = Some((start, end, Vec::new()));
        } else if line.trim().is_empty() {
            push_cue(&mut words, cue.take());
        } else if let Some((_, _, lines)) = &mut cue {
            lines.push(line);
        }
    }
    push_cue(&mut words, cue);
    words
}

/// An SRT file with one cue per line of words (Remotion's `serializeSrt`): from the first
/// word's start to the last word's end, the words joined without a separator. Empty lines
/// are left out.
pub fn serialize_srt(lines: &[Vec<CaptionWord>]) -> String {
    let mut out = String::new();
    for (index, line) in lines.iter().filter(|l| !l.is_empty()).enumerate() {
        let start = line.first().map_or(0.0, |w| w.start_ms);
        let end = line.last().map_or(0.0, |w| w.end_ms);
        let text: String = line.iter().map(|w| w.text.as_str()).collect();
        if index > 0 {
            out.push('\n');
        }
        let _ = write!(
            out,
            "{}\n{} --> {}\n{}\n",
            index + 1,
            srt_time(start),
            srt_time(end),
            text.trim()
        );
    }
    out
}

/// The cues of a VTT file, one `CaptionWord` each, read like `parse_srt`: cue
/// identifiers, cue settings and `NOTE`, `STYLE` and `REGION` blocks are skipped. A cue's
/// text is trimmed (tags such as `<v Speaker>` stay) and its confidence is 1.
pub fn parse_vtt(text: &str) -> Result<Vec<CaptionWord>, CaptionFileError> {
    let header = normalize(text);
    let header = header.lines().next().unwrap_or_default();
    if header != "WEBVTT" && !header.starts_with("WEBVTT ") && !header.starts_with("WEBVTT\t") {
        return Err(CaptionFileError::Vtt);
    }
    Ok(parse_srt(text))
}

/// The words of a Whisper transcript (Remotion's `openAiWhisperApiToCaptions` and
/// `toCaptions` for whisper.cpp).
///
/// - **openai-whisper** (`--word_timestamps True`, or the API's `verbose_json` with word
///   timestamps): `segments[].words[]` or `words[]`, each `{ word, start, end }` in seconds
///   with an optional `probability`. A segment without words becomes one caption.
/// - **whisper.cpp** (`-oj`): `transcription[]`, each `{ text, offsets: { from, to } }` in
///   milliseconds. Run it with `-ml 1` for one entry per word.
///
/// Word text keeps its leading space.
pub fn from_whisper_json(json: &str) -> Result<Vec<CaptionWord>, CaptionFileError> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| CaptionFileError::Json(e.to_string()))?;
    if let Some(entries) = value.get("transcription").and_then(Value::as_array) {
        return entries.iter().map(whisper_cpp_entry).collect();
    }
    if let Some(words) = value.get("words").and_then(Value::as_array) {
        return words.iter().map(|w| openai_word(w, "word")).collect();
    }
    let segments =
        value
            .get("segments")
            .and_then(Value::as_array)
            .ok_or(CaptionFileError::Whisper(
                "no `segments`, `words` or `transcription` array",
            ))?;
    let mut words = Vec::new();
    for segment in segments {
        if let Some(list) = segment.get("words").and_then(Value::as_array) {
            for word in list {
                words.push(openai_word(word, "word")?);
            }
        } else {
            words.push(openai_word(segment, "text")?);
        }
    }
    Ok(words)
}

/// An openai-whisper word, or a segment (`text_key` `"text"`), timed in seconds.
fn openai_word(word: &Value, text_key: &'static str) -> Result<CaptionWord, CaptionFileError> {
    let text = string(word, text_key)?;
    let mut caption = CaptionWord::new(
        text,
        number(word, "start")? * 1000.0,
        number(word, "end")? * 1000.0,
    );
    caption.confidence = word.get("probability").and_then(Value::as_f64);
    Ok(caption)
}

fn whisper_cpp_entry(entry: &Value) -> Result<CaptionWord, CaptionFileError> {
    let text = string(entry, "text")?;
    let offsets = entry.get("offsets").ok_or(CaptionFileError::Whisper(
        "a transcription entry has no `offsets`",
    ))?;
    Ok(CaptionWord::new(
        text,
        number(offsets, "from")?,
        number(offsets, "to")?,
    ))
}

fn string<'v>(value: &'v Value, key: &'static str) -> Result<&'v str, CaptionFileError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(CaptionFileError::Whisper("an entry misses its text"))
}

fn number(value: &Value, key: &'static str) -> Result<f64, CaptionFileError> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or(CaptionFileError::Whisper("an entry misses a time"))
}

/// Without a byte order mark, with `\n` line ends.
fn normalize(text: &str) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn cue_word(text: &str, start_ms: f64, end_ms: f64) -> CaptionWord {
    CaptionWord {
        confidence: Some(1.0),
        ..CaptionWord::new(text, start_ms, end_ms)
    }
}

fn push_cue(words: &mut Vec<CaptionWord>, cue: Option<(f64, f64, Vec<&str>)>) {
    let Some((start, end, lines)) = cue else {
        return;
    };
    let text = lines.join("\n");
    let text = text.trim();
    if !text.is_empty() {
        words.push(cue_word(text, start, end));
    }
}

/// The start and end of a `start --> end [settings]` line.
fn timing(line: &str) -> Option<(f64, f64)> {
    let (start, rest) = line.split_once("-->")?;
    let end = rest.split_whitespace().next()?;
    Some((timestamp(start.trim())?, timestamp(end)?))
}

/// `HH:MM:SS,mmm`, `HH:MM:SS.mmm` or `MM:SS.mmm` in milliseconds.
fn timestamp(text: &str) -> Option<f64> {
    let (clock, millis) = text.split_once([',', '.'])?;
    let millis: u64 = millis.parse().ok()?;
    let mut seconds = 0u64;
    let parts: Vec<&str> = clock.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    for part in parts {
        seconds = seconds * 60 + part.parse::<u64>().ok()?;
    }
    Some((seconds * 1000 + millis) as f64)
}

fn srt_time(ms: f64) -> String {
    let ms = ms.max(0.0).round() as u64;
    let (hours, rest) = (ms / 3_600_000, ms % 3_600_000);
    let (minutes, rest) = (rest / 60_000, rest % 60_000);
    let (seconds, millis) = (rest / 1000, rest % 1000);
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRT: &str = "\u{feff}1\r\n00:00:01,000 --> 00:00:02,500\r\nHello there\r\n\r\n\
                       2\r\n00:00:03.000 --> 01:00:04,250 X1:40\r\nTwo\r\nlines\r\n\r\n\
                       3\r\n00:00:05,000 --> 00:00:06,000\r\n\r\n";

    #[test]
    fn srt_cues_are_read_leniently() {
        let words = parse_srt(SRT);
        assert_eq!(words.len(), 2, "the cue without text is left out");
        assert_eq!(words[0].text, "Hello there");
        assert_eq!((words[0].start_ms, words[0].end_ms), (1000.0, 2500.0));
        assert_eq!(words[0].timestamp_ms, Some(1750.0));
        assert_eq!(words[0].confidence, Some(1.0));
        assert_eq!(words[1].text, "Two\nlines");
        assert_eq!(words[1].end_ms, 3_604_250.0);
    }

    #[test]
    fn srt_round_trips() {
        let words = parse_srt(SRT);
        let lines: Vec<Vec<CaptionWord>> = words.iter().map(|w| vec![w.clone()]).collect();
        let text = serialize_srt(&lines);
        assert!(text.starts_with("1\n00:00:01,000 --> 00:00:02,500\nHello there\n\n2\n"));
        assert_eq!(parse_srt(&text), words);
        assert_eq!(serialize_srt(&lines), serialize_srt(&lines));
    }

    #[test]
    fn serialize_joins_the_words_of_a_line() {
        let line = vec![
            CaptionWord::new("Hello", 0.0, 400.0),
            CaptionWord::new(" world", 400.0, 900.0),
        ];
        let text = serialize_srt(&[
            line,
            Vec::new(),
            vec![CaptionWord::new(" again", 1000.0, 1500.0)],
        ]);
        assert_eq!(
            text,
            "1\n00:00:00,000 --> 00:00:00,900\nHello world\n\n2\n00:00:01,000 --> 00:00:01,500\nagain\n"
        );
    }

    #[test]
    fn vtt_cues_are_read() {
        let vtt = "WEBVTT - demo\r\n\r\nSTYLE\r\n::cue { color: red }\r\n\r\nNOTE a comment\r\n\r\nintro\r\n00:01.000 --> 00:02.000\r\nHi\r\n\r\n\
                   00:00:02.500 --> 00:00:04.000 align:end\r\nThere you are\r\n";
        let words = parse_vtt(vtt).expect("valid vtt");
        let texts: Vec<_> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, ["Hi", "There you are"]);
        assert_eq!((words[1].start_ms, words[1].end_ms), (2500.0, 4000.0));
        assert!(parse_vtt("not a subtitle file").is_err());
    }

    #[test]
    fn vtt_and_srt_agree() {
        let srt = parse_srt("1\n00:00:01,000 --> 00:00:02,000\nSame\n");
        let vtt = parse_vtt("WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nSame\n").expect("valid vtt");
        assert_eq!(srt, vtt);
    }

    #[test]
    fn openai_whisper_words_keep_their_spaces() {
        let json = r#"{"text": " Hello world.", "segments": [
            {"start": 0.0, "end": 1.2, "text": " Hello world.", "words": [
                {"word": " Hello", "start": 0.0, "end": 0.5, "probability": 0.9},
                {"word": " world.", "start": 0.5, "end": 1.2, "probability": 0.75}]},
            {"start": 1.5, "end": 2.0, "text": " Bye."}]}"#;
        let words = from_whisper_json(json).expect("transcript");
        let texts: Vec<_> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, [" Hello", " world.", " Bye."]);
        assert_eq!((words[1].start_ms, words[1].end_ms), (500.0, 1200.0));
        assert_eq!(words[1].confidence, Some(0.75));
        assert_eq!(words[2].confidence, None);
        let api = r#"{"words": [{"word": "Hi", "start": 0.25, "end": 0.5}]}"#;
        assert_eq!(
            from_whisper_json(api).expect("api transcript")[0].start_ms,
            250.0
        );
    }

    #[test]
    fn whisper_cpp_entries_are_words() {
        let json = r#"{"result": {"language": "en"}, "transcription": [
            {"timestamps": {"from": "00:00:00,000", "to": "00:00:00,320"},
             "offsets": {"from": 0, "to": 320}, "text": " And"},
            {"timestamps": {"from": "00:00:00,320", "to": "00:00:00,800"},
             "offsets": {"from": 320, "to": 800}, "text": " so"}]}"#;
        let words = from_whisper_json(json).expect("transcript");
        assert_eq!(words[1], CaptionWord::new(" so", 320.0, 800.0));
    }

    #[test]
    fn other_json_is_rejected() {
        assert!(matches!(
            from_whisper_json("{"),
            Err(CaptionFileError::Json(_))
        ));
        assert!(matches!(
            from_whisper_json("{}"),
            Err(CaptionFileError::Whisper(_))
        ));
        assert!(matches!(
            from_whisper_json(r#"{"words": [{"word": "x"}]}"#),
            Err(CaptionFileError::Whisper(_))
        ));
    }
}
