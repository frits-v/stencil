//! Findings as the model reports them, and the text the review posts for them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Credential prefixes a review comment must never carry onto a pull request: Anthropic keys
/// and OAuth tokens, GitHub installation, user and refresh tokens.
pub const POSTED_TEXT_PREFIXES: [&str; 7] = [
    "sk-ant-",
    "ghs_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghr_",
    "github_pat_",
];
pub const MARKER_PREFIX: &str = "<!-- stencil-review ";
const MARKER_SUFFIX: &str = " -->";
const FINGERPRINT_HEX_CHARS: usize = 16;
const MARKERS_PER_BODY_MAX: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Correctness,
    Security,
    RepositoryRule,
    TestCoverage,
}

impl Category {
    const fn label(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Security => "security",
            Self::RepositoryRule => "repository rule",
            Self::TestCoverage => "test coverage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub path: String,
    pub line: u32,
    pub severity: Severity,
    pub category: Category,
    pub summary: String,
    pub failure_scenario: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Findings {
    findings: Vec<Finding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Confirmed,
    Refuted,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerdictOutput {
    verdict: Verdict,
    reasoning: String,
}

/// The find pass's findings, most severe first; the CLI checked them against the schema, and
/// this decode refuses any field it does not know.
pub fn decode_findings(output: &Value) -> Result<Vec<Finding>, String> {
    let mut findings = Findings::deserialize(output)
        .map_err(|error| format!("the find pass's output does not decode: {error}"))?
        .findings;
    findings.sort_by_key(|finding| finding.severity);
    Ok(findings)
}

/// The verdict and its one-paragraph reasoning, which the job log shows.
pub fn decode_verdict(output: &Value) -> Result<(Verdict, String), String> {
    VerdictOutput::deserialize(output)
        .map(|decoded| (decoded.verdict, decoded.reasoning))
        .map_err(|error| format!("a verify pass's output does not decode: {error}"))
}

fn normalised(text: &str) -> String {
    text.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The SHA-256 of `bytes` as lowercase hex and a newline.
#[must_use]
pub fn sha256_line(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut line = Vec::with_capacity(digest.len().saturating_mul(2).saturating_add(1));
    for byte in digest {
        line.extend(
            [byte >> 4, byte & 0x0f]
                .iter()
                .filter_map(|nibble| HEX.get(usize::from(*nibble))),
        );
    }
    line.push(b'\n');
    line
}

/// Stable across pushes that move the line: the path, the category and the summary.
pub fn fingerprint(finding: &Finding) -> String {
    let line = sha256_line(
        format!(
            "{}\n{}\n{}",
            finding.path,
            finding.category.label(),
            normalised(&finding.summary)
        )
        .as_bytes(),
    );
    String::from_utf8_lossy(&line)
        .chars()
        .take(FINGERPRINT_HEX_CHARS)
        .collect()
}

/// Every marker line in a posted body, at most [`MARKERS_PER_BODY_MAX`], each with the text
/// before it back to the previous marker line: the finding it marks. A marker counts only as a
/// line of its own, which model text, posted as one line, never makes.
pub fn marked_entries(body: &str) -> Vec<(&str, &str)> {
    let mut entries = Vec::new();
    let mut entry_start = 0;
    let mut line_start: usize = 0;
    for line in body.split_inclusive('\n') {
        let line_end = line_start.saturating_add(line.len());
        if let Some(hex) = line_marker(line) {
            if entries.len() == MARKERS_PER_BODY_MAX {
                break;
            }
            entries.push((hex, body.get(entry_start..line_start).unwrap_or_default()));
            entry_start = line_end;
        }
        line_start = line_end;
    }
    entries
}

fn line_marker(line: &str) -> Option<&str> {
    let hex = line
        .trim()
        .strip_prefix(MARKER_PREFIX)?
        .strip_suffix(MARKER_SUFFIX)?;
    (hex.len() == FINGERPRINT_HEX_CHARS && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then_some(hex)
}

/// Text on one line: every markdown line ending (`\n`, `\r`) and the indentation after it
/// become one space, so no model text starts a block or a marker line.
fn one_line(text: &str) -> String {
    text.split(['\n', '\r'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Placed after `@`, and between `#`, `/` or `-` and a digit, so GitHub links no mention and
/// no issue or pull request reference: its autolinking runs on the decoded text, so an entity
/// still pings or links, which its markdown API shows.
const WORD_JOINER: char = '\u{2060}';

/// One character of model text outside a code span, written so it renders as itself: an
/// entity for each character that could start HTML, a code span, an escape or emphasis, so
/// the text around the review's own code spans holds none of them, and a word joiner after
/// each character that could start a mention or a reference.
fn push_inert(out: &mut String, character: char, following: Option<&char>) {
    match character {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '\\' => out.push_str("&#92;"),
        '`' => out.push_str("&#96;"),
        '*' => out.push_str("&#42;"),
        '_' => out.push_str("&#95;"),
        '[' => out.push_str("&#91;"),
        ']' => out.push_str("&#93;"),
        '~' => out.push_str("&#126;"),
        '|' => out.push_str("&#124;"),
        _ => out.push(character),
    }
    let starts_reference =
        matches!(character, '#' | '/' | '-') && following.is_some_and(char::is_ascii_digit);
    if character == '@' || starts_reference {
        out.push(WORD_JOINER);
    }
}

fn backtick_run(characters: &[char], from: usize) -> usize {
    characters
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|character| **character == '`')
        .count()
}

/// Where the code span opened by a run of `length` backticks ends: the next run of exactly
/// that length, as the markdown spec reads it.
fn closing_run(characters: &[char], from: usize, length: usize) -> Option<usize> {
    let mut at = from;
    while at < characters.len() {
        let run = backtick_run(characters, at);
        if run == length {
            return Some(at);
        }
        at = at.saturating_add(run.max(1));
    }
    None
}

fn longest_run(characters: &[char]) -> usize {
    characters
        .split(|character| *character != '`')
        .map(<[char]>::len)
        .max()
        .unwrap_or_default()
}

/// A code span the review writes around `content`, which GitHub shows as written: the spec
/// takes one space off each side of content that has one on both, the fence is longer than
/// any backtick run inside, and a pad keeps a backtick or an edge space off the fence.
fn push_code_span(out: &mut String, content: &[char]) {
    let blank = content.iter().all(|character| *character == ' ');
    let edged = |content: &[char], character: char| {
        content.first() == Some(&character) && content.last() == Some(&character)
    };
    let content = if !blank && edged(content, ' ') {
        content
            .get(1..content.len().saturating_sub(1))
            .unwrap_or_default()
    } else {
        content
    };
    let fence = "`".repeat(longest_run(content).saturating_add(1));
    let pad = if content.first() == Some(&'`')
        || content.last() == Some(&'`')
        || (!blank && edged(content, ' '))
    {
        " "
    } else {
        ""
    };
    out.push_str(&fence);
    out.push_str(pad);
    out.extend(content);
    out.push_str(pad);
    out.push_str(&fence);
}

/// Model text as one line of GitHub markdown that reads as written and does nothing. Its code
/// spans are found by the spec's rules (a backslash escapes the punctuation after it, a run
/// of backticks closes at the next run of the same length) and each is written again inside
/// a span the review builds; every other character that markdown or GitHub acts on becomes an
/// entity, and a mention or a reference is broken by a word joiner. Only the review's own
/// fences open a span, so a reading that differs from GitHub's changes how the text looks,
/// never what it does: no HTML, no marker, no mention and no reference.
fn inert(text: &str) -> String {
    let characters: Vec<char> = one_line(text).chars().collect();
    let mut out = String::with_capacity(characters.len());
    let mut at: usize = 0;
    while let Some(&character) = characters.get(at) {
        let next = at.saturating_add(1);
        let escaped = characters
            .get(next)
            .filter(|literal| character == '\\' && literal.is_ascii_punctuation());
        if let Some(&literal) = escaped {
            let after = next.saturating_add(1);
            push_inert(&mut out, literal, characters.get(after));
            at = after;
            continue;
        }
        if character != '`' {
            push_inert(&mut out, character, characters.get(next));
            at = next;
            continue;
        }
        let run = backtick_run(&characters, at);
        let opened_end = at.saturating_add(run);
        if let Some(close) = closing_run(&characters, opened_end, run) {
            push_code_span(
                &mut out,
                characters.get(opened_end..close).unwrap_or_default(),
            );
            at = close.saturating_add(run);
        } else {
            (0..run).for_each(|_| push_inert(&mut out, '`', None));
            at = opened_end;
        }
    }
    out
}

/// Text inside a code span the review writes itself: one line, no backtick to close it early.
fn code_text(text: &str) -> String {
    one_line(text).replace('`', "'")
}

fn texts(finding: &Finding) -> [&str; 3] {
    [&finding.path, &finding.summary, &finding.failure_scenario]
}

/// Whether a finding's text holds one of the run's own secrets; the run then posts nothing.
pub fn names_secret(finding: &Finding, secrets: &[&str]) -> bool {
    texts(finding).iter().any(|text| {
        secrets
            .iter()
            .any(|secret| !secret.is_empty() && text.contains(secret))
    })
}

/// Whether a finding's text holds a credential prefix; such a finding is withheld.
pub fn carries_credential_prefix(finding: &Finding) -> bool {
    texts(finding).iter().any(|text| {
        POSTED_TEXT_PREFIXES
            .iter()
            .any(|prefix| text.contains(prefix))
    })
}

fn heading(finding: &Finding) -> String {
    let severity = match finding.severity {
        Severity::High => "High",
        Severity::Medium => "Medium",
        Severity::Low => "Low",
    };
    format!("{severity}, {}", finding.category.label())
}

pub fn inline_body(finding: &Finding) -> String {
    format!(
        "**{}**: {}\n\n{}\n\n{MARKER_PREFIX}{}{MARKER_SUFFIX}",
        heading(finding),
        inert(&finding.summary),
        inert(&finding.failure_scenario),
        fingerprint(finding)
    )
}

/// A finding the diff does not show, listed in the review's body under its location.
pub fn outside_entry(finding: &Finding) -> String {
    format!(
        "- `{}:{}` ({}): {}\n  {}\n  {MARKER_PREFIX}{}{MARKER_SUFFIX}",
        code_text(&finding.path),
        finding.line,
        heading(finding),
        inert(&finding.summary),
        inert(&finding.failure_scenario),
        fingerprint(finding)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(summary: &str) -> Finding {
        Finding {
            path: "xtask/src/a.rs".to_string(),
            line: 7,
            severity: Severity::Medium,
            category: Category::Correctness,
            summary: summary.to_string(),
            failure_scenario: "Given x, y happens.".to_string(),
        }
    }

    #[test]
    fn findings_decode_most_severe_first_and_refuse_unknown_fields() {
        let output = serde_json::json!({"findings": [
            {"path": "a.rs", "line": 1, "severity": "low", "category": "test-coverage",
             "summary": "s", "failure_scenario": "f"},
            {"path": "b.rs", "line": 2, "severity": "high", "category": "repository-rule",
             "summary": "s", "failure_scenario": "f"},
        ]});
        let findings = decode_findings(&output).unwrap();
        assert_eq!(findings.first().unwrap().severity, Severity::High);
        assert_eq!(findings.first().unwrap().category, Category::RepositoryRule);
        let extra = serde_json::json!({"findings": [], "note": "x"});
        assert!(decode_findings(&extra).is_err());
        let bad_severity = serde_json::json!({"findings": [
            {"path": "a.rs", "line": 1, "severity": "urgent", "category": "security",
             "summary": "s", "failure_scenario": "f"}]});
        assert!(decode_findings(&bad_severity).is_err());
    }

    #[test]
    fn verdicts_decode_each_state() {
        let verdict =
            |text: &str| decode_verdict(&serde_json::json!({"verdict": text, "reasoning": "r"}));
        assert_eq!(
            verdict("confirmed"),
            Ok((Verdict::Confirmed, "r".to_string()))
        );
        assert_eq!(verdict("refuted"), Ok((Verdict::Refuted, "r".to_string())));
        assert!(verdict("maybe").is_err());
        assert!(decode_verdict(&serde_json::json!({"verdict": "confirmed"})).is_err());
    }

    #[test]
    fn a_fingerprint_ignores_the_line_case_and_spacing_and_reads_back_from_its_marker() {
        let first = finding("The  Bound is off by one");
        let mut moved = finding("the bound is off by one");
        moved.line = 90;
        assert_eq!(fingerprint(&first), fingerprint(&moved));
        assert_ne!(fingerprint(&first), fingerprint(&finding("another defect")));
        let body = inline_body(&first);
        assert_eq!(
            marker_fingerprint(&body),
            Some(fingerprint(&first).as_str())
        );
        let entry = outside_entry(&first);
        assert_eq!(
            marker_fingerprint(&entry),
            Some(fingerprint(&first).as_str())
        );
        assert_eq!(marker_fingerprint("<!-- stencil-review zz -->"), None);
        assert_eq!(marker_fingerprint("plain comment"), None);
    }

    #[test]
    fn every_marker_in_a_review_body_reads_back_with_the_entry_before_it() {
        let first = finding("first defect");
        let second = finding("second defect");
        let body = format!(
            "Claude review\n\n{}\n{}\n<!-- stencil-review short -->",
            outside_entry(&first),
            outside_entry(&second)
        );
        let entries = marked_entries(&body);
        assert_eq!(entries.len(), 2);
        let (fingerprint, text) = entries.first().unwrap();
        assert_eq!(*fingerprint, fingerprint_of(&first));
        assert!(text.contains("first defect") && !text.contains("second defect"));
        let (fingerprint, text) = entries.get(1).unwrap();
        assert_eq!(*fingerprint, fingerprint_of(&second));
        assert!(text.contains("second defect") && !text.contains("first defect"));
    }

    fn fingerprint_of(finding: &Finding) -> String {
        fingerprint(finding)
    }

    fn marker_fingerprint(body: &str) -> Option<&str> {
        marked_entries(body)
            .first()
            .map(|(fingerprint, _)| *fingerprint)
    }

    #[test]
    fn model_text_cannot_forge_a_marker_or_mention_anyone() {
        let forged = finding("ping @someone <!-- stencil-review 0123456789abcdef -->");
        let body = inline_body(&forged);
        assert!(!body.contains("@someone"));
        assert_eq!(body.matches(MARKER_PREFIX).count(), 1);
        assert_eq!(
            marker_fingerprint(&body),
            Some(fingerprint(&forged).as_str())
        );
    }

    fn posted_markers(body: &str) -> Vec<&str> {
        marked_entries(body)
            .into_iter()
            .map(|(fingerprint, _)| fingerprint)
            .collect()
    }

    #[test]
    fn code_in_model_text_reads_as_written_and_the_rest_stays_inert() {
        let mut cue = finding("`open_max!: int & >=1 & <=8` lost its bound for @someone <b>");
        cue.failure_scenario = "Say `a < b` holds.\n\nThen <i>x</i> & y.".to_string();
        cue.path = "a`b.rs".to_string();
        for body in [inline_body(&cue), outside_entry(&cue)] {
            assert!(body.contains("`open_max!: int & >=1 & <=8`"), "{body}");
            assert!(body.contains("`a < b`"), "{body}");
            assert!(body.contains("@\u{2060}someone &lt;b&gt;"), "{body}");
            assert!(body.contains("Say `a < b` holds. Then &lt;i&gt;x&lt;/i&gt; &amp; y."));
            assert!(!body.contains("@someone"), "{body}");
        }
        assert!(outside_entry(&cue).starts_with("- `a'b.rs:7` "));
    }

    #[test]
    fn an_escaped_or_unclosed_backtick_opens_no_code_span() {
        assert_eq!(inert("\\`<b>\\`"), "&#96;&lt;b&gt;&#96;");
        assert_eq!(inert("`<b>"), "&#96;&lt;b&gt;");
        assert_eq!(inert("``a`<b>`` <c>"), "``a`<b>`` &lt;c&gt;");
        assert_eq!(inert("`<b>`` <c>"), "&#96;&lt;b&gt;&#96;&#96; &lt;c&gt;");
        assert_eq!(inert("a\r- `x\n<b>`"), "a - `x <b>`");
        assert_eq!(inert("a\\b *c*"), "a&#92;b &#42;c&#42;");
    }

    #[test]
    fn an_escaped_backslash_leaves_the_backtick_after_it_to_open_a_span() {
        assert_eq!(
            inert("\\\\` x `@octocat` <b>"),
            "&#92;`x`@\u{2060}octocat&#96; &lt;b&gt;"
        );
        assert_eq!(inert("\\\\`<b>`"), "&#92;`<b>`");
    }

    #[test]
    fn a_rewritten_span_shows_its_content_with_the_spec_s_spacing() {
        assert_eq!(inert("`` `a ``"), "`` `a ``");
        assert_eq!(inert("`  x  `"), "`  x  `");
        assert_eq!(inert("` x `"), "`x`");
        assert_eq!(inert("```a``b```"), "```a``b```");
    }

    /// The text outside the code spans of posted text, each span a `C`: GitHub links neither a
    /// mention nor a reference across one. With no backslash outside a span, the spec finds
    /// the same openers, and a closer never depends on a backslash.
    fn outside_code(posted: &str) -> String {
        let characters: Vec<char> = posted.chars().collect();
        let mut outside = String::new();
        let mut at = 0_usize;
        while let Some(&character) = characters.get(at) {
            let run = backtick_run(&characters, at);
            let close = (run > 0)
                .then(|| closing_run(&characters, at.saturating_add(run), run))
                .flatten();
            if let Some(close) = close {
                outside.push('C');
                at = close.saturating_add(run);
            } else {
                outside.push(character);
                at = at.saturating_add(1);
            }
        }
        outside
    }

    #[test]
    fn no_short_string_of_markdown_characters_posts_one_of_them_live() {
        let alphabet = [
            '`', '\\', '<', '>', '@', '*', '_', '[', ']', '~', '|', 'a', ' ', '\n',
        ];
        let mut examined = 0_usize;
        let mut indices = vec![0_usize; 0];
        for length in 0..=5_u32 {
            let count = alphabet.len().pow(length);
            for number in 0..count {
                indices.clear();
                let mut rest = number;
                for _ in 0..length {
                    indices.push(rest % alphabet.len());
                    rest /= alphabet.len();
                }
                let text: String = indices
                    .iter()
                    .filter_map(|index| alphabet.get(*index))
                    .collect();
                let posted = inert(&text);
                assert!(!posted.contains(['\n', '\r']), "{text:?} -> {posted:?}");
                let outside = outside_code(&posted);
                assert!(
                    !outside.contains(['`', '\\', '<', '>', '*', '_', '[', ']', '~', '|']),
                    "{text:?} -> {posted:?}"
                );
                examined = examined.saturating_add(1);
            }
        }
        assert_eq!(examined, 579_195);
    }

    #[test]
    fn no_mention_or_reference_survives_outside_a_code_span() {
        let joined = |text: &str| text.replace('^', "\u{2060}");
        for (text, posted) in [
            ("@octocat", "@^octocat"),
            ("see #1 and gh-12", "see #^1 and gh-^12"),
            ("frits-v/stencil#1", "frits-v/stencil#^1"),
            ("org/repo/issues/7", "org/repo/issues/^7"),
            (
                "https://github.com/o/r/pull/3/files",
                "https://github.com/o/r/pull/^3/files",
            ),
            ("o/r@3cfcaa7", "o/r@^3cfcaa7"),
            ("\\#1", "#^1"),
            ("`@octocat #1`", "`@octocat #1`"),
            ("a-b c/d #x", "a-b c/d #x"),
        ] {
            assert_eq!(inert(text), joined(posted), "{text:?}");
        }
    }

    /// Whether `@` or a reference start followed by a digit appears without a word joiner in
    /// the text GitHub links on, which has the review's entities decoded to characters that
    /// start neither.
    fn live_link(outside: &str) -> bool {
        let decoded = [
            "&amp;", "&lt;", "&gt;", "&#92;", "&#96;", "&#42;", "&#95;", "&#91;", "&#93;",
            "&#126;", "&#124;",
        ]
        .iter()
        .fold(outside.to_string(), |text, entity| {
            text.replace(entity, "E")
        });
        let characters: Vec<char> = decoded.chars().collect();
        characters.windows(2).any(|pair| match pair {
            ['@', next] => *next != WORD_JOINER,
            ['#' | '/' | '-', next] => next.is_ascii_digit(),
            _ => false,
        }) || characters.last() == Some(&'@')
    }

    #[test]
    fn no_short_string_of_reference_characters_posts_a_live_mention_or_reference() {
        let alphabet = ['`', '\\', '@', '#', '/', '-', '1', 'a', ' '];
        let mut examined = 0_usize;
        let mut indices = Vec::new();
        for length in 0..=5_u32 {
            for number in 0..alphabet.len().pow(length) {
                indices.clear();
                let mut rest = number;
                for _ in 0..length {
                    indices.push(rest % alphabet.len());
                    rest /= alphabet.len();
                }
                let text: String = indices
                    .iter()
                    .filter_map(|index| alphabet.get(*index))
                    .collect();
                let posted = inert(&text);
                assert!(!live_link(&outside_code(&posted)), "{text:?} -> {posted:?}");
                examined = examined.saturating_add(1);
            }
        }
        assert_eq!(examined, 66_430);
    }

    #[test]
    fn a_marker_in_model_text_is_never_read_as_one() {
        let forged = "0123456789abcdef";
        for text in [
            format!("`<!-- stencil-review {forged} -->`"),
            format!("x\n<!-- stencil-review {forged} -->"),
            format!("x\r<!-- stencil-review {forged} -->\ry"),
            format!("x\n`\n<!-- stencil-review {forged} -->\n`"),
        ] {
            let mut planted = finding(&text);
            planted.failure_scenario.clone_from(&text);
            let real = fingerprint(&planted);
            assert_eq!(posted_markers(&inline_body(&planted)), [real.as_str()]);
            assert_eq!(posted_markers(&outside_entry(&planted)), [real.as_str()]);
        }
    }

    #[test]
    fn a_marker_counts_only_as_a_line_of_its_own() {
        let marker = "<!-- stencil-review 0123456789abcdef -->";
        assert_eq!(
            posted_markers(&format!("a\n  {marker}\n")),
            ["0123456789abcdef"]
        );
        assert!(posted_markers(&format!("a {marker}")).is_empty());
        assert!(posted_markers(&format!("`{marker}`")).is_empty());
    }

    #[test]
    fn a_finding_naming_a_secret_or_a_credential_prefix_is_caught() {
        assert!(names_secret(&finding("leaks abc123"), &["abc123"]));
        assert!(!names_secret(&finding("a plain defect"), &["abc123", ""]));
        assert!(carries_credential_prefix(&finding("token sk-ant-oat01-x")));
        assert!(carries_credential_prefix(&finding("ghs_16C7e42F")));
        assert!(!carries_credential_prefix(&finding("a plain defect")));
    }
}
