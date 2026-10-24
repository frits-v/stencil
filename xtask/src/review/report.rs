//! The review as posted: which confirmed findings are new and safe to show, where each one
//! goes, and the body that carries the verdict, its reason and the other checks.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Write as _;

use super::checks::{self, Seen};
use super::claude_run;
use super::diff::DiffMap;
use super::findings::{self, Finding};
use super::pull_request::{self, PostOutcome};
use super::verdict::Event;
use super::{Context, Grants, Tally, VERIFIED_MAX, short};
use crate::anthropic_token;
use crate::app_token::InstallationToken;
use crate::github_api::Client;

/// The confirmed findings that are new and safe to post, split by whether the diff shows
/// their line. One that names the run's own credential stops the whole post.
pub fn select(
    context: &Context,
    grants: &Grants,
    map: &DiffMap,
    confirmed: &[Finding],
    tally: &mut Tally,
) -> Result<(Vec<Finding>, Vec<Finding>), String> {
    let minted = grants
        .anthropic
        .as_ref()
        .map(anthropic_token::Minter::minted)
        .unwrap_or_default();
    let secrets: Vec<&str> = grants
        .app
        .as_ref()
        .map(InstallationToken::expose)
        .into_iter()
        .chain(minted.iter().map(anthropic_token::Secret::expose))
        .collect();
    let posted: BTreeSet<&str> = context
        .posted
        .iter()
        .map(|entry| entry.fingerprint.as_str())
        .collect();
    let mut inline = Vec::new();
    let mut outside = Vec::new();
    for finding in confirmed {
        if findings::names_secret(finding, &secrets) {
            return Err(
                "a confirmed finding holds the run's own credential; nothing is posted".into(),
            );
        }
        if findings::carries_credential_prefix(finding) {
            tally.withheld = tally.withheld.saturating_add(1);
        } else if posted.contains(findings::fingerprint(finding).as_str()) {
            tally.still_holding = tally.still_holding.saturating_add(1);
        } else if map.anchors(&finding.path, finding.line) {
            inline.push(finding.clone());
        } else {
            outside.push(finding.clone());
        }
    }
    tally.inline = inline.len();
    tally.outside = outside.len();
    Ok((inline, outside))
}

pub fn counts(tally: &Tally) -> String {
    format!(
        "examined {} files and {} hunks; {} candidates, {} confirmed, {} refuted, {} not \
         verified (over the cap of {VERIFIED_MAX}), {} without a verdict, {} posted before and \
         still holding, {} withheld; {} new inline, {} new outside the diff; cost ${}.{:02}",
        tally.files,
        tally.hunks,
        tally.candidates,
        tally.confirmed,
        tally.refuted,
        tally.unverified,
        tally.failed.len(),
        tally.still_holding,
        tally.withheld,
        tally.inline,
        tally.outside,
        tally.cost / 1_000_000,
        (tally.cost % 1_000_000) / 10_000
    )
}

/// The review body. It names no account and holds no `@`, so it never asks for another review.
pub fn body(
    context: &Context,
    verdict: (Event, &str),
    outside: &[Finding],
    checks: &Seen,
    tally: &Tally,
) -> String {
    let (event, reason) = verdict;
    let mut body = format!(
        "Claude review (`{}`) of {}: {}, {reason}.\n",
        claude_run::MODEL,
        short(context.head.as_str()),
        event.label()
    );
    if !outside.is_empty() {
        body.push_str("\nNew findings on lines the diff does not show:\n\n");
        for finding in outside {
            body.push_str(&findings::outside_entry(finding));
            body.push('\n');
        }
    }
    if let Some(summary) = checks::summary(checks) {
        body.push_str("\nThe other checks on this commit, which the verdict does not weigh:\n\n");
        body.push_str(&summary);
        body.push('\n');
    }
    body.push_str("\nThe review ");
    body.push_str(&counts(tally));
    body.push_str(".\n");
    body
}

fn write_step_summary(text: &str) {
    let Ok(path) = env::var("GITHUB_STEP_SUMMARY") else {
        return;
    };
    let written = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| writeln!(file, "{text}"));
    if let Err(error) = written {
        eprintln!("xtask review: cannot write the step summary: {error}");
    }
}

/// Posts the review as the App, or prints it in a dry run; the body goes to the step summary
/// either way.
pub fn publish(
    context: &Context,
    grants: &Grants,
    map: &DiffMap,
    confirmed: &[Finding],
    checks: &Seen,
    verdict: (Event, &str),
    tally: &mut Tally,
) -> Result<(), String> {
    let (inline, outside) = select(context, grants, map, confirmed, tally)?;
    let text = body(context, verdict, &outside, checks, tally);
    println!("{text}");
    write_step_summary(&text);
    let (Some(github), Some(token)) = (&context.github, &grants.app) else {
        for finding in &inline {
            println!(
                "{}:{}\n{}\n",
                finding.path,
                finding.line,
                findings::inline_body(finding)
            );
        }
        println!("xtask review: dry run; nothing posted");
        return Ok(());
    };
    let comments: Vec<(String, u32, String)> = inline
        .iter()
        .map(|finding| {
            (
                finding.path.clone(),
                finding.line,
                findings::inline_body(finding),
            )
        })
        .collect();
    match pull_request::post_review(
        &Client::github(),
        token.expose(),
        &github.repository,
        &github.pull_request,
        (verdict.0, &text),
        &comments,
    ) {
        PostOutcome::Posted { url } => {
            println!("xtask review: posted {url}");
            Ok(())
        }
        other => Err(format!("posting the review ended {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff;
    use crate::review::findings::{Category, Severity};
    use crate::review::pull_request::Posted;
    use crate::review::setup::tests::{context, grants};

    fn finding(path: &str, line: u32, summary: &str) -> Finding {
        Finding {
            path: path.to_string(),
            line,
            severity: Severity::Medium,
            category: Category::Correctness,
            summary: summary.to_string(),
            failure_scenario: "Given x, y happens.".to_string(),
        }
    }

    #[test]
    fn select_withholds_skips_and_splits_the_confirmed_findings() {
        let map =
            diff::parse_diff("diff --git a/a.rs b/a.rs\n+++ b/a.rs\n@@ -1,0 +1,2 @@\n+x\n+y\n")
                .unwrap();
        let seen = finding("a.rs", 2, "posted before");
        let posted = vec![Posted {
            fingerprint: findings::fingerprint(&seen),
            excerpt: String::new(),
        }];
        let confirmed = [
            finding("a.rs", 1, "anchors"),
            finding("a.rs", 9, "outside the hunk"),
            finding("a.rs", 2, "token sk-ant-oat01-x in a message"),
            finding("a.rs", 1, "Posted   before"),
        ];
        let mut tally = Tally::default();
        let (inline, outside) = select(
            &context(posted),
            &grants("ghs_job"),
            &map,
            &confirmed,
            &mut tally,
        )
        .unwrap();
        assert_eq!(inline, [finding("a.rs", 1, "anchors")]);
        assert_eq!(outside, [finding("a.rs", 9, "outside the hunk")]);
        assert_eq!((tally.withheld, tally.still_holding), (1, 1));
        assert_eq!((tally.inline, tally.outside), (1, 1));
    }

    #[test]
    fn a_finding_holding_a_minted_anthropic_token_stops_the_whole_post_whatever_its_form() {
        let grants = Grants {
            anthropic: Some(anthropic_token::Minter::with_minted(&["opaque-token-4f2a"])),
            ..grants("t")
        };
        let error = select(
            &context(Vec::new()),
            &grants,
            &DiffMap::default(),
            &[finding("a.rs", 1, "the pass saw opaque-token-4f2a")],
            &mut Tally::default(),
        )
        .unwrap_err();
        assert!(error.contains("nothing is posted"), "{error}");
    }

    #[test]
    fn a_finding_holding_the_job_token_stops_the_whole_post() {
        let confirmed = [
            finding("a.rs", 1, "fine"),
            finding("a.rs", 1, "leaks job-token-value"),
        ];
        let error = select(
            &context(Vec::new()),
            &grants("job-token-value"),
            &DiffMap::default(),
            &confirmed,
            &mut Tally::default(),
        )
        .unwrap_err();
        assert!(error.contains("nothing is posted"), "{error}");
    }

    #[test]
    fn the_body_states_the_verdict_lists_the_checks_and_never_mentions_anyone() {
        let checks = Seen {
            checks: vec![checks::Check {
                name: "ci".to_string(),
                outcome: checks::Outcome::Failed,
            }],
            unreadable: None,
        };
        let text = body(
            &context(Vec::new()),
            (Event::Approve, "no confirmed high or medium findings"),
            &[finding("b.rs", 4, "ping @claude here")],
            &checks,
            &Tally::default(),
        );
        assert!(
            text.starts_with("Claude review (`opus`) of aaaaaaaaaaaa: approve, "),
            "{text}"
        );
        assert!(text.contains("- `ci`: failed"), "{text}");
        assert!(text.contains("does not weigh"));
        assert!(!text.contains("@claude"), "{text}");
        assert!(text.contains("@\u{2060}claude"), "{text}");
    }
}
