//! Bounded, source-preserving cleanup for the local Light profile.

use std::ops::Range;

const MAX_BYTES: usize = 65_536;
const MAX_WORDS: usize = 8_192;

#[derive(Debug)]
struct Word {
    span: Range<usize>,
    key: String,
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '\'' | '’' | '-')
}

fn words(text: &str) -> Vec<Word> {
    let mut result = Vec::new();
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if word_char(c) {
            start.get_or_insert(i);
        } else if let Some(begin) = start.take() {
            result.push(Word {
                span: begin..i,
                key: text[begin..i].to_ascii_lowercase(),
            });
        }
    }
    result
}

fn budget(text: &str, terms: &[&str]) -> bool {
    text.len() <= MAX_BYTES
        && terms.len() <= 256
        && terms.iter().map(|t| t.len()).sum::<usize>() <= MAX_BYTES * 4
}

// Boundary and quote handling adapted from upstream Verenu text_rules.rs,
// b35900ce0c76417e6f909f54008bc97bcca45d78 (MIT). Keep the stricter cleanup
// rules below independent of its broader filler/correction heuristics.
fn protected_ranges(text: &str, terms: &[&str]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let c = rest.chars().next().unwrap();
        if rest.starts_with("[[VERENU_") {
            let end = i + rest.find("]]").map_or(rest.len(), |n| n + 2);
            ranges.push(i..end);
            i = end;
            continue;
        }
        let boundary = i == 0 || !text[..i].chars().next_back().is_some_and(word_char);
        let close = match c {
            '`' => Some('`'),
            '“' => Some('”'),
            '‘' => Some('’'),
            '"' | '\'' if boundary => Some(c),
            _ => None,
        };
        if let Some(close) = close {
            let end = rest[c.len_utf8()..]
                .find(close)
                .map_or(text.len(), |n| i + c.len_utf8() + n + close.len_utf8());
            ranges.push(i..end);
            i = end;
            continue;
        }
        if boundary {
            if let Some(length) = terms
                .iter()
                .filter(|t| !t.is_empty())
                .filter_map(|term| {
                    let mut actual = rest.char_indices();
                    let mut length = 0;
                    for wanted in term.chars() {
                        let (offset, got) = actual.next()?;
                        if !got.eq_ignore_ascii_case(&wanted) {
                            return None;
                        }
                        length = offset + got.len_utf8();
                    }
                    rest[length..]
                        .chars()
                        .next()
                        .is_none_or(|next| !word_char(next))
                        .then_some(length)
                })
                .max()
            {
                ranges.push(i..i + length);
                i += length;
                continue;
            }
        }
        i += c.len_utf8();
    }
    let mut chunk_start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if !c.is_whitespace() {
            chunk_start.get_or_insert(i);
        } else if let Some(start) = chunk_start.take() {
            let chunk = &text[start..i];
            let technical = chunk.contains(['/', '\\', '@', '_', '=']);
            if technical || chunk.chars().any(|c| c.is_ascii_digit()) {
                // Sentence punctuation after a number is not part of the
                // numeric literal; a following spoken cue may replace it.
                let end = if technical {
                    i
                } else {
                    start + chunk.trim_end_matches(['.', ',', ':', ';', '?', '!']).len()
                };
                ranges.push(start..end);
            }
        }
    }
    for token in words(text) {
        let original = &text[token.span.clone()];
        let upper = original.chars().filter(|c| c.is_ascii_uppercase()).count();
        let camel_case = original
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
            && upper > 0;
        if upper >= 2 || camel_case {
            ranges.push(token.span);
        }
    }
    ranges
}

fn overlaps(span: &Range<usize>, ranges: &[Range<usize>]) -> bool {
    ranges
        .iter()
        .any(|p| span.start < p.end && p.start < span.end)
}

struct Edit {
    span: Range<usize>,
    replacement: String,
}

fn apply(text: &str, mut edits: Vec<Edit>) -> (String, usize) {
    edits.sort_by_key(|edit| edit.span.start);
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut count = 0;
    for edit in edits {
        if edit.span.start < cursor {
            continue;
        }
        result.push_str(&text[cursor..edit.span.start]);
        result.push_str(&edit.replacement);
        cursor = edit.span.end;
        count += 1;
    }
    result.push_str(&text[cursor..]);
    (result, count)
}

fn weekday(key: &str) -> bool {
    matches!(
        key,
        "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday"
    )
}

fn mechanical_words(text: &str, terms: &[&str]) -> (String, usize) {
    let tokens = words(text);
    let protected = protected_ranges(text, terms);
    let mut edits = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        if overlaps(&token.span, &protected) {
            i += 1;
            continue;
        }
        if weekday(&token.key) {
            for cue in [
                &["actually", "i", "mean"][..],
                &["i", "mean"],
                &["sorry"],
                &["actually"],
            ] {
                let next = i + 1 + cue.len();
                if let Some(replacement) = tokens.get(next) {
                    let span = token.span.start..replacement.span.start;
                    if weekday(&replacement.key)
                        && replacement.key != token.key
                        && tokens[i + 1..next]
                            .iter()
                            .map(|t| t.key.as_str())
                            .eq(cue.iter().copied())
                        && !text[token.span.end..replacement.span.start]
                            .contains(['.', '!', '?', '\n'])
                        && !overlaps(&(span.start..replacement.span.end), &protected)
                    {
                        edits.push(Edit {
                            span,
                            replacement: String::new(),
                        });
                        i = next;
                        break;
                    }
                }
            }
            if i < tokens.len() && tokens[i].span.start != token.span.start {
                continue;
            }
        }
        if matches!(token.key.as_str(), "called" | "named" | "identifier") {
            let start = i + 1;
            let mut end = start + 1;
            while end + 1 < tokens.len()
                && end < start + 20
                && tokens[end].key == "underscore"
                && !matches!(
                    tokens[end + 1].key.as_str(),
                    "and" | "then" | "return" | "with"
                )
                && tokens[end + 1]
                    .key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric())
                && tokens[end + 1].span.end - tokens[start].span.start < 256
            {
                end += 2;
            }
            if end > start + 1 {
                let span = tokens[start].span.start..tokens[end - 1].span.end;
                if !overlaps(&span, &protected)
                    && text[span.clone()]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
                {
                    let replacement = tokens[start..end]
                        .iter()
                        .step_by(2)
                        .map(|t| &text[t.span.clone()])
                        .collect::<Vec<_>>()
                        .join("_");
                    edits.push(Edit { span, replacement });
                    i = end;
                    continue;
                }
            }
        }
        if i + 1 < tokens.len()
            && matches!(token.key.as_str(), "the" | "a" | "an")
            && token.key == tokens[i + 1].key
            && text[token.span.end..tokens[i + 1].span.start]
                .chars()
                .all(char::is_whitespace)
            && !overlaps(&tokens[i + 1].span, &protected)
        {
            edits.push(Edit {
                span: token.span.start..tokens[i + 1].span.start,
                replacement: String::new(),
            });
        }
        i += 1;
    }
    apply(text, edits)
}

fn punctuation(text: &str, terms: &[&str]) -> (String, usize) {
    let tokens = words(text);
    let protected = protected_ranges(text, terms);
    let mut edits = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        let prev = i
            .checked_sub(1)
            .map(|p| tokens[p].key.as_str())
            .unwrap_or("");
        let next = tokens.get(i + 1).map(|t| t.key.as_str()).unwrap_or("");
        let mut start = token.span.start;
        let mut explicit = false;
        if matches!(prev, "add" | "insert" | "put") {
            explicit = true;
            start = tokens[i - 1].span.start;
        } else if prev == "a"
            && i > 1
            && matches!(tokens[i - 2].key.as_str(), "add" | "insert" | "put")
        {
            explicit = true;
            start = tokens[i - 2].span.start;
        }
        let literal = !explicit
            && (matches!(
                prev,
                "my" | "your"
                    | "his"
                    | "her"
                    | "our"
                    | "their"
                    | "the"
                    | "a"
                    | "an"
                    | "word"
                    | "literal"
                    | "explain"
                    | "said"
                    | "say"
                    | "type"
                    | "spell"
                    | "named"
                    | "called"
                    | "does"
                    | "learned"
                    | "studied"
                    | "discussed"
                    | "discuss"
                    | "mention"
                    | "mentioned"
            ) || matches!(
                next,
                "screening"
                    | "cancer"
                    | "mean"
                    | "means"
                    | "symbol"
                    | "character"
                    | "usage"
                    | "works"
            ));
        let mark = match (token.key.as_str(), next) {
            ("colon", _)
                if !literal
                    && (explicit
                        || matches!(
                            prev,
                            "items"
                                | "priorities"
                                | "following"
                                | "support"
                                | "saying"
                                | "list"
                                | "colon"
                        )) =>
            {
                Some((":", 1))
            }
            ("semicolon", _) if !literal => Some((";", 1)),
            ("comma", _) if !literal => Some((",", 1)),
            ("period", _)
                if !literal
                    && (explicit
                        || (!matches!(prev, "time" | "trial" | "billing" | "grace")
                            && (next == "end"
                                || matches!(prev, "hello" | "goodbye" | "thanks")))) =>
            {
                Some((".", 1))
            }
            ("question", "mark") if !literal => Some(("?", 2)),
            ("exclamation", "mark" | "point") if !literal => Some(("!", 2)),
            ("new", "line") if !literal => Some(("\n", 2)),
            ("new", "paragraph") if !literal => Some(("\n\n", 2)),
            ("dash" | "hyphen", _) if explicit => Some(("-", 1)),
            _ => None,
        };
        if let Some((symbol, count)) = mark {
            let mut end = tokens[i + count - 1].span.end;
            // Consume an STT-added terminator on the cue itself, not both
            // the dictated symbol and its recognition punctuation.
            if text[end..].starts_with(['.', '!', '?', ','])
                || (matches!(symbol, ":" | ";") && text[end..].starts_with(symbol))
            {
                end += 1;
            }
            while start > 0
                && text[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c == ' ' || c == '\t')
            {
                start -= 1;
            }
            // Recognition can punctuate both sides of a spoken cue, e.g.
            // "bananas, comma," or "priorities. Colon". Once the cue is
            // unambiguous, its symbol replaces that recognition punctuation.
            if matches!(symbol, "," | ":" | ";" | "." | "?" | "!")
                && text[..start].ends_with(['.', ',', ':', ';', '?', '!'])
            {
                start -= 1;
            }
            if symbol.starts_with('\n') {
                while end < text.len() && text[end..].starts_with([' ', '\t']) {
                    end += 1;
                }
            }
            let span = start..end;
            if !overlaps(&span, &protected) {
                edits.push(Edit {
                    span,
                    replacement: symbol.into(),
                });
                i += count;
                continue;
            }
        }
        i += 1;
    }
    apply(text, edits)
}

#[derive(Debug)]
pub(super) struct Prepared {
    pub text: String,
    pub edits: usize,
}

pub(super) fn prepare(text: &str, protected_terms: &[&str], english: bool) -> Prepared {
    if !english || !budget(text, protected_terms) || words(text).len() > MAX_WORDS {
        return Prepared {
            text: text.to_owned(),
            edits: 0,
        };
    }
    let (text, first) = mechanical_words(text, protected_terms);
    let (text, second) = punctuation(&text, protected_terms);
    Prepared {
        text,
        edits: first + second,
    }
}

fn gap_allowed(source: &str, proposed: &str) -> bool {
    let mandatory = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '.' | '?' | '!' | ','))
            .collect::<String>()
    };
    mandatory(source) == mandatory(proposed)
        && source.matches('\n').count() == proposed.matches('\n').count()
        && ['?', '!', ','].iter().all(|c| {
            !source.contains(*c) || source.matches(*c).count() == proposed.matches(*c).count()
        })
        && proposed
            .chars()
            .filter(|c| matches!(c, '.' | '?' | '!' | ','))
            .count()
            <= source
                .chars()
                .filter(|c| matches!(c, '.' | '?' | '!' | ','))
                .count()
                .max(1)
}

pub(super) fn validate(
    reference: &str,
    candidate: &str,
    protected_terms: &[&str],
) -> Option<String> {
    if !budget(reference, protected_terms) || !budget(candidate, protected_terms) {
        return None;
    }
    let original = words(reference);
    let proposed = words(candidate);
    if original.len() > MAX_WORDS || original.len() != proposed.len() {
        return None;
    }
    if original.is_empty() {
        return (reference == candidate).then(|| reference.to_owned());
    }
    let mut output = String::with_capacity(candidate.len());
    let (mut from, mut to) = (0, 0);
    for (source, proposal) in original.iter().zip(&proposed) {
        if !reference[source.span.clone()].eq_ignore_ascii_case(&candidate[proposal.span.clone()])
            || !gap_allowed(
                &reference[from..source.span.start],
                &candidate[to..proposal.span.start],
            )
        {
            return None;
        }
        output.push_str(&candidate[to..proposal.span.start]);
        output.push_str(&candidate[proposal.span.clone()]);
        from = source.span.end;
        to = proposal.span.end;
    }
    if !gap_allowed(&reference[from..], &candidate[to..]) {
        return None;
    }
    output.push_str(&candidate[to..]);
    for range in protected_ranges(reference, protected_terms) {
        let Some((index, first)) = original
            .iter()
            .enumerate()
            .find(|(_, t)| t.span.end > range.start && t.span.start < range.end)
        else {
            continue;
        };
        let delta = first.span.start.checked_sub(range.start)?;
        let start = proposed[index].span.start.checked_sub(delta)?;
        if candidate.get(start..start.checked_add(range.len())?)? != &reference[range] {
            return None;
        }
    }
    Some(output)
}

#[cfg(test)]
#[path = "fidelity_tests.rs"]
mod tests;
