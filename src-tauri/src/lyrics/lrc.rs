//! LRC lyrics parser (`lyrics::lrc`).
//!
//! Grammar we accept (standard + enhanced LRC):
//!
//! ```text
//! [ti:Song Title]              metadata: ti / ar / al / by / offset / any "key:value"
//! [offset:+300]                global shift applied to every timestamp (ms, may be negative)
//! [00:12.00]Hello               line-level: [mm:ss.xx] / [mm:ss.xxx] / [mm:ss:xxx] / [mm:ss]
//! [00:12.00][00:30.00]Hello     multiple timestamps → one LyricLine per stamp, same text
//! [00:12.00]<00:12.20>All<00:13.50>of<00:14.00>me   enhanced/karaoke word timings
//! ```
//!
//! Semantics worth spelling out:
//! - Numbers: fraction is normalised to milliseconds by digit count (1 digit
//!   → centiseconds skip *100, 2 → *10, 3 → as-is, >3 truncated).
//! - "offset" is *applied*, not just recorded: `shown_time = stamp + offset`
//!   (the common modern interpretation); lines clamp at 0. It is also kept in
//!   `metadata["offset"]` so editors can round-trip it.
//! - Bare text lines (no brackets) are kept as untimed entries with
//!   `time_ms: 0` in file order — real LRC is either fully stamped or fully
//!   plain; the UI renders plain documents from the raw text, not from `lines`.
//! - `[now-timestamp brackets]` such as `[feat. Other Artist]` remain literal
//!   lyric text (they only fail the timestamp / "key:value" probes).
//! - A timestamp with no text keeps its place in the timeline with `text: ""`
//!   (real files use those as pacing separators).
//! - Word timings are closed by the next inline tag; the last word of a line
//!   stays open (`endMs: null`) unless the line ends with a tag.

use std::collections::HashMap;

use serde::Serialize;

/// One timed lyric line. `words` is `None` for plain (non-enhanced) lines.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub time_ms: u32,
    pub text: String,
    /// Word-level karaoke timings (enhanced LRC `<mm:ss.xx>word`). `None`
    /// when the line has no inline timestamps.
    pub words: Option<Vec<WordTiming>>,
}

/// Timing window for one word inside an en enhanced line.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordTiming {
    pub start_ms: u32,
    /// End of the word = start of the next inline tag in the same line,
    /// `None` when the last word stays open.
    pub end_ms: Option<u32>,
    pub word: String,
}

/// Parsed LRC document: tag metadata (`ti`, `ar`, `al`, `by`, `offset`, …)
/// and lyric lines sorted by time.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedLyrics {
    pub metadata: HashMap<String, String>,
    pub lines: Vec<LyricLine>,
}

/// Parse a whole LRC document. Never fails: anything it cannot interpret
/// degrades to the closest sane representation (see module docs).
pub fn parse_lrc(text: &str) -> ParsedLyrics {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut offset_ms: i64 = 0;

    // Phase 1 — bracket headers: timestamps and `key:value` tags.
    // `times` gathers every `[..]` stamp found before the lyric text; the
    // remainder (after the last header bracket) is the lyric content.
    let mut raw_lines: Vec<(Vec<u32>, String)> = Vec::new();
    for raw in text.lines() {
        let mut times: Vec<u32> = Vec::new();
        let mut rest = raw.trim();
        loop {
            let (inner, after) = match take_bracket_group(rest) {
                Some(pair) => pair,
                None => break,
            };
            if let Some(stamp) = parse_timestamp(&inner) {
                times.push(stamp);
            } else if let Some((key, value)) = split_metadata_tag(&inner) {
                if key.eq_ignore_ascii_case("offset") {
                    offset_ms = parse_signed_ms(&value).unwrap_or(0);
                    // keep it visible for round-trips, normalised
                    metadata.insert("offset".into(), format_offset(offset_ms));
                } else {
                    metadata.insert(key, value);
                }
            } else {
                // Not a timestamp, not a tag — literal text from here on.
                break;
            }
            rest = after;
        }
        let content = rest.trim();
        if times.is_empty() && content.is_empty() {
            continue; // blank or metadata-only line
        }
        raw_lines.push((times, content.to_string()));
    }

    // Phase 2 — build lines, apply the offset, word timings, sort.
    let mut lines: Vec<LyricLine> = Vec::new();
    for (times, content) in raw_lines {
        let stamps: Vec<u32> = times
            .into_iter()
            .map(|t| clamp_ms(t as i64 + offset_ms))
            .collect();
        if stamps.is_empty() {
            // Untimed (plain) line — preserved verbatim at time 0.
            let (plain, words) = split_inline(&content, 0);
            lines.push(LyricLine {
                time_ms: 0,
                text: plain,
                words,
            });
        } else {
            let (plain, shared_words) = split_inline(&content, stamps[0]);
            for stamp in stamps {
                // With several stamps per line the word tags are absolute, so
                // entries beyond the first simply share the same timings.
                lines.push(LyricLine {
                    time_ms: stamp,
                    text: plain.clone(),
                    words: shared_words.clone(),
                });
            }
        }
    }
    lines.sort_by_key(|l| l.time_ms);

    ParsedLyrics { metadata, lines }
}

/// Split `"..[..]"` → `(inside, rest)` when the line starts with a full
/// bracket group, else `None`.
fn take_bracket_group(line: &str) -> Option<(String, &str)> {
    let after_open = line.strip_prefix('[')?;
    let close = after_open.find(']')?;
    Some((after_open[..close].to_string(), &after_open[close + 1..]))
}

/// Split `"key: value"` (any leading/trailing spaces trimmed). Returns `None`
/// for text without a colon or with an empty key — those stay literal lyrics.
fn split_metadata_tag(inner: &str) -> Option<(String, String)> {
    let colon = inner.find(':')?;
    let key = inner[..colon].trim();
    if key.is_empty() {
        return None;
    }
    let value = inner[colon + 1..].trim();
    Some((key.to_lowercase(), value.to_string()))
}

/// `+/-ms` values from `[offset:..]`; accepts `+300`, `-120`, `300`.
fn parse_signed_ms(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    let (sign, digits) = match trimmed.as_bytes().first() {
        Some(b'+') => (1i64, &trimmed[1..]),
        Some(b'-') => (-1i64, &trimmed[1..]),
        _ => (1i64, trimmed),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<i64>().ok().map(|n| sign * n)
}

fn format_offset(ms: i64) -> String {
    if ms < 0 {
        format!("-{ms}")
    } else {
        format!("+{ms}")
    }
}

fn clamp_ms(ms: i64) -> u32 {
    ms.clamp(0, u32::MAX as i64) as u32
}

/// Parse a stamp body (the text inside `[..]` or `<..>`):
/// `mm:ss` | `mm:ss.xx` | `mm:ss.xxx` | `mm:ss:xxx`.
///
/// The minutes count is capped at 999 to keep the arithmetic safe and keep
/// `[garbage]` out of the timeline; fractional digits after the third are
/// truncated.
pub fn parse_timestamp(inner: &str) -> Option<u32> {
    let s = inner.trim();
    if s.is_empty() {
        return None;
    }

    let (mins, mut rest) = scan_digits(s);
    if mins.is_empty() {
        return None;
    }
    rest = rest.strip_prefix(':')?;
    let (secs, mut rest) = scan_digits(rest);
    if secs.is_empty() {
        return None;
    }

    // Optional fraction after '.' or ':' — `[00:12.45]` / `[00:12:456]`.
    let mut frac_ms = 0u32;
    if let Some(after_sep) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(':')) {
        if !after_sep.is_empty() && after_sep.as_bytes()[0].is_ascii_digit() {
            let (frac, tail) = scan_digits(after_sep);
            rest = tail;
            frac_ms = fraction_to_ms(frac);
        }
    }

    // Reject `[00:12 junk]` — only whitespace may trail the stamp.
    if !rest.trim().is_empty() {
        return None;
    }

    let mins = mins.parse::<i64>().ok()?;
    let secs = secs.parse::<i64>().ok()?;
    if mins > 999 || secs > 599 {
        return None;
    }
    let total = (mins * 60 + secs)
        .checked_mul(1000)
        .and_then(|v| v.checked_add(frac_ms as i64))?;
    if total > u32::MAX as i64 {
        return None;
    }
    Some(total as u32)
}

fn scan_digits(s: &str) -> (&str, &str) {
    let mut end = 0;
    for (i, b) in s.bytes().enumerate() {
        if b.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    (&s[..end], &s[end..])
}

/// Fractional digits → ms: `5` → 500, `45` → 450, `345` → 345,
/// `3456` → 345 (truncated).
fn fraction_to_ms(frac: &str) -> u32 {
    let digits: String = frac.chars().take(3).filter(|c| c.is_ascii_digit()).collect();
    let extra_ms = match digits.len() {
        0 => 0,
        1 => digits[0..1].parse::<u32>().unwrap_or(0) * 100,
        2 => digits[0..2].parse::<u32>().unwrap_or(0) * 10,
        _ => digits[0..3].parse::<u32>().unwrap_or(0),
    };
    extra_ms
}

/// Walk inline enhanced-LRC tags `<mm:ss.xx>` inside a line's content.
///
/// Returns the plain text (tags stripped) and, whenever the content contains
/// at least one inline tag, the word timings; leading untagged text belongs to
/// the first words of the line and starts at `line_time_ms`.
fn split_inline(content: &str, line_time_ms: u32) -> (String, Option<Vec<WordTiming>>) {
    let mut segments: Vec<Segment> = scan_inline(content);
    let has_tags = segments.iter().any(|s| matches!(s, Segment::Tag(_)));
    if !has_tags {
        return (content.to_string(), None);
    }

    let mut words: Vec<WordTiming> = Vec::new();
    let mut pending_start: Option<u32> = None;
    let mut plain = String::new();
    for seg in segments.drain(..) {
        match seg {
            Segment::Tag(t) => {
                if let Some(last) = words.last_mut() {
                    if last.end_ms.is_none() {
                        last.end_ms = Some(t);
                    }
                }
                pending_start = Some(t);
            }
            Segment::Text(chunk) => {
                // Join chunks so removing tags cannot glue words together:
                // `All<00:12.5>of` still reads `All of`.
                if !plain.is_empty()
                    && !plain.ends_with(char::is_whitespace)
                    && !chunk.starts_with(char::is_whitespace)
                {
                    plain.push(' ');
                }
                plain.push_str(&chunk); // keep raw spacing for the plain text
                let trimmed = chunk.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let start = pending_start.take().unwrap_or(line_time_ms);
                words.push(WordTiming {
                    start_ms: start,
                    end_ms: None,
                    word: trimmed.to_string(),
                });
            }
        }
    }
    (plain, Some(words))
}

enum Segment {
    Tag(u32),
    Text(String),
}

/// `<00:12.34>` tags and the text between them, in order. `<` that is not a
/// complete, parseable tag is literal text (scanning is strict: the whole
/// bracket body must parse as a timestamp, otherwise it stays in the text).
fn scan_inline(content: &str) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    let bytes = content.as_bytes();
    let mut text_start = 0usize; // start of the current plain-text run
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        match content[i + 1..].find('>') {
            Some(rel_close) => {
                let close = i + 1 + rel_close;
                match parse_timestamp(&content[i + 1..close]) {
                    Some(t) => {
                        if text_start < i {
                            segments.push(Segment::Text(content[text_start..i].to_string()));
                        }
                        segments.push(Segment::Tag(t));
                        text_start = close + 1;
                    }
                    None => { /* literal `<...>` text; keep scanning inside */ }
                }
                i = close + 1;
            }
            None => break, // no closing '>' anywhere — rest is literal
        }
    }
    if text_start < bytes.len() {
        segments.push(Segment::Text(content[text_start..].to_string()));
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_parsing_and_sorting() {
        let src = "[00:12.00]This line at 12s\n\
                   [00:01.50]Aly first [with brackets ignored]\n\
                   [03:20]Later";
        let parsed = parse_lrc(src);
        let times: Vec<u32> = parsed.lines.iter().map(|l| l.time_ms).collect();
        assert_eq!(times, vec![1500, 12000, 200_000]);
        assert_eq!(parsed.lines[0].text, "Aly first [with brackets ignored]");
        assert_eq!(parsed.lines[1].text, "This line at 12s");
        assert_eq!(parsed.metadata.get("offset"), None);
    }

    #[test]
    fn metadata_tags_are_captured() {
        let src = "[ti:Song Title]\n[ar:Artist]\n[al:Album]\n[by:Editor]\n\
                   [00:10.00]line";
        let parsed = parse_lrc(src);
        assert_eq!(parsed.metadata.get("ti").map(String::as_str), Some("Song Title"));
        assert_eq!(parsed.metadata.get("ar").map(String::as_str), Some("Artist"));
        assert_eq!(parsed.metadata.get("al").map(String::as_str), Some("Album"));
        assert_eq!(parsed.metadata.get("by").map(String::as_str), Some("Editor"));
        assert!(parsed.metadata.contains_key("offset") == false);
        assert_eq!(parsed.lines.len(), 1);
    }

    #[test]
    fn stamp_variants() {
        // .xx centiseconds / .xxx milliseconds / :xxx variant / no fraction
        assert_eq!(parse_timestamp("00:12.34"), Some(12_340));
        assert_eq!(parse_timestamp("00:12.345"), Some(12_345));
        assert_eq!(parse_timestamp("00:12:345"), Some(12_345));
        assert_eq!(parse_timestamp("00:12"), Some(12_000));
        assert_eq!(parse_timestamp(" 01:02"), Some(62_000));
        // overlong minutes / stray junk are rejected
        assert_eq!(parse_timestamp("1000:00"), None);
        assert_eq!(parse_timestamp("00:12junk"), None);
        assert_eq!(parse_timestamp(""), None);
    }

    #[test]
    fn offset_shifts_lines_and_clamps_at_zero() {
        let positive = parse_lrc("[offset:+1000]\n[00:10.00]A");
        assert_eq!(positive.lines[0].time_ms, 11_000);
        assert_eq!(positive.metadata.get("offset").map(String::as_str), Some("+1000"));

        let negative = parse_lrc("[offset:-9510]\n[00:10.00]A");
        assert_eq!(negative.lines[0].time_ms, 490);

        let clamped = parse_lrc("[offset:-91000]\n[00:10.00]A");
        assert_eq!(clamped.lines[0].time_ms, 0);
    }

    #[test]
    fn multi_timestamp_per_line() {
        let parsed = parse_lrc("[00:12.00][01:30.00]Same text\n[00:15]Other");
        assert_eq!(parsed.lines.len(), 3);
        assert_eq!(parsed.lines[0].time_ms, 12_000);
        assert_eq!(parsed.lines[0].text, "Same text");
        assert_eq!(parsed.lines[1].time_ms, 15_000);
        assert_eq!(parsed.lines[1].text, "Other");
        assert_eq!(parsed.lines[2].time_ms, 90_000);
    }

    #[test]
    fn enhanced_word_level_karaoke() {
        let parsed = parse_lrc("[00:12.00]<00:12.20>All<00:12.50>of<00:12.80>me");
        assert_eq!(parsed.lines.len(), 1);
        assert_eq!(parsed.lines[0].text, "All of me");
        let words = parsed.lines[0].words.as_ref().expect("word timings");
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].word, "All");
        assert_eq!(words[0].start_ms, 12_200);
        assert_eq!(words[0].end_ms, Some(12_500));
        assert_eq!(words[1].word, "of");
        assert_eq!(words[1].end_ms, Some(12_800));
        // last word of the line stays open
        assert_eq!(words[2].word, "me");
        assert_eq!(words[2].end_ms, None);
    }

    #[test]
    fn enhanced_degrades_to_line_level() {
        let parsed = parse_lrc("[00:12.00]No word tags here");
        assert_eq!(parsed.lines[0].words, None);
        assert_eq!(parsed.lines[0].text, "No word tags here");
    }

    #[test]
    fn enhanced_leading_untagged_words_start_with_line() {
        let parsed = parse_lrc("[00:12.00]First <00:13.00>tagged");
        let words = parsed.lines[0].words.as_ref().expect("word timings");
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].word, "First");
        assert_eq!(words[0].start_ms, 12_000);
        assert_eq!(words[0].end_ms, Some(13_000));
        assert_eq!(words[1].word, "tagged");
        assert_eq!(words[1].start_ms, 13_000);
        assert_eq!(words[1].end_ms, None);
    }

    #[test]
    fn plain_lyrics_without_timestamps() {
        let parsed = parse_lrc("First verse plain\n\nSecond line");
        assert!(parsed.metadata.is_empty());
        assert_eq!(parsed.lines.len(), 2);
        assert_eq!(parsed.lines[0].text, "First verse plain");
        assert_eq!(parsed.lines[1].text, "Second line");
        assert_eq!(parsed.lines[0].words, None);
    }

    #[test]
    fn timed_empty_lines_are_kept_as_pacing() {
        let parsed = parse_lrc("[00:10]Then\n[00:20]\n[00:30]After");
        assert_eq!(parsed.lines.len(), 3);
        assert_eq!(parsed.lines[1].time_ms, 20_000);
        assert_eq!(parsed.lines[1].text, "");
    }

    #[test]
    fn status_metadata_only_lines_are_dropped() {
        let parsed = parse_lrc("[ti:T]\n[00:05]lyric\n[by:E]");
        assert_eq!(parsed.lines.len(), 1);
        assert_eq!(parsed.lines[0].text, "lyric");
    }

    #[test]
    fn crlf_and_bom_tolerated() {
        let src = "\u{feff}[00:01.00]first\r\n[00:02.00]second\r\n";
        let parsed = parse_lrc(src);
        assert_eq!(parsed.lines.len(), 2);
        assert_eq!(parsed.lines[0].text, "first");
        assert_eq!(parsed.lines[1].text, "second");
    }
}
