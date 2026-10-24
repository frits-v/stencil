//! `xtask review`: a code review of a pull request by the Claude CLI, with a verdict.
//!
//! A find pass reports candidate findings against the diff; each candidate then gets a verify
//! pass in a fresh session that tries to refute it. A rule over the confirmed findings decides
//! the verdict; the other checks on the head commit are waited for and reported, never weighed.
//! `--post` runs from the base branch's checkout, reviews a pull request checked out beside it
//! as data, and posts the verdict as the review App; `--dry-run` reviews the checkout it runs
//! in and posts nothing. The model reads only inside the reviewed checkout and the run's own
//! directory, and never holds a GitHub credential. Each run is traced to Langfuse when it is
//! configured.

mod checks;
mod claude_run;
mod diff;
mod findings;
mod langfuse;
mod passes;
mod prompt_registry;
mod pull_request;
mod report;
mod setup;
mod stream;
mod verdict;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

use crate::anthropic_token;
use crate::app_token::{self, InstallationToken};
use langfuse::{Langfuse, Pass, Trace};
use pull_request::{Posted, PullRequest};
use verdict::Event;

/// Generated or derived files: a review of them restates their sources, and the render
/// fixtures alone can pass the diff cap.
const EXCLUDED_PATHSPECS: [&str; 4] = [
    ":(exclude)Cargo.lock",
    ":(exclude)crates/stencil-render/tests/fixtures",
    ":(exclude)docs/gallery",
    ":(exclude)docs/themes",
];
const VERIFIED_MAX: usize = 12;
/// How long after the review starts the other checks are waited for; the App's token lasts
/// an hour from the start.
const CHECKS_WAIT: Duration = Duration::from_secs(45 * 60);
const SHORT_COMMIT_CHARS: usize = 12;
/// git in `dir`, with only the variables `child` passes on.
fn git_in(dir: &Path) -> Command {
    let mut command = crate::child::command("git");
    command.arg("-C").arg(dir);
    command
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    DryRun,
    /// The pull request's head is fetched by its commit, reviewed and never run.
    Post,
}

/// Where a posting run reads and writes on GitHub, as the review App.
#[derive(Debug)]
struct GitHub {
    repository: String,
    pull_request: PullRequest,
}

/// What the run was granted: the means to mint Anthropic tokens, the App's token, and the run
/// time. Kept apart from [`Context`], which the run prints from, and handed only to the code
/// that uses a token.
#[derive(Debug)]
struct Grants {
    /// Mints each pass its own Anthropic token; `None` uses the developer's own login.
    anthropic: Option<anthropic_token::Minter>,
    app: Option<InstallationToken>,
    run_time: Duration,
}

#[derive(Debug)]
struct Context {
    tag: &'static str,
    /// The tree the model reads and the diff is taken in.
    checkout: PathBuf,
    /// The pull request's head as a worktree the posting run made, removed when it ends.
    worktree: Option<setup::Worktree>,
    base: pull_request::CommitId,
    head: pull_request::CommitId,
    claude: PathBuf,
    github: Option<GitHub>,
    /// `owner/name#number` in Actions; Langfuse groups a pull request's runs by it.
    session: String,
    posted: Vec<Posted>,
    user: Option<String>,
    langfuse: Option<Langfuse>,
    /// A directory private to this run, outside the checkout: the CLI refuses reads of
    /// gitignored paths under `dontAsk`, so `target/` cannot hold what the model reads.
    files: PathBuf,
}

#[derive(Debug, Default)]
struct Tally {
    files: usize,
    hunks: usize,
    candidates: usize,
    confirmed: usize,
    refuted: usize,
    unverified: usize,
    unjudged_blocking: usize,
    still_holding: usize,
    withheld: usize,
    inline: usize,
    outside: usize,
    failed: Vec<String>,
    cost: u64,
    passes: Vec<Pass>,
    prompts: passes::Prompts,
    /// Why part of the trace could not be recorded; each one fails the run, like a failed export.
    trace_failures: Vec<String>,
    /// The reviewed diff, the trace's input.
    diff: String,
}

/// `xtask review --dry-run` or `xtask review --post`.
pub fn command(arguments: &[String]) -> i32 {
    let mode = match arguments {
        [flag] if flag == "--dry-run" => Mode::DryRun,
        [flag] if flag == "--post" => Mode::Post,
        _ => {
            eprintln!("usage: xtask review --dry-run | --post");
            return 2;
        }
    };
    let grants = match setup::grants(&mode) {
        Ok(grants) => grants,
        Err(reason) => {
            eprintln!("xtask review: {reason}");
            return 1;
        }
    };
    let context = match setup::context(&mode, &grants) {
        Ok(context) => context,
        Err(reason) => {
            eprintln!("xtask review: {reason}");
            revoke(&grants);
            return 1;
        }
    };
    let started = SystemTime::now();
    let mut tally = Tally::default();
    let reviewed = review(&context, &grants, &mut tally);
    setup::remove_files(&context.files);
    if let Some(worktree) = &context.worktree {
        setup::remove_worktree(worktree);
    }
    finish(&context, &grants, started, &tally, &reviewed)
}

fn review(
    context: &Context,
    grants: &Grants,
    tally: &mut Tally,
) -> Result<(Event, String), String> {
    let started = Instant::now();
    let deadline = started
        .checked_add(grants.run_time)
        .ok_or("the run deadline overflows")?;
    tally.diff = passes::prepare(context)?;
    let map = diff::parse_diff(&tally.diff)?;
    if map.commentable.is_empty() {
        return Err(format!(
            "the diff {}...{} has no file with head-side lines to review once {} are excluded",
            short(context.base.as_str()),
            short(context.head.as_str()),
            EXCLUDED_PATHSPECS.join(", ")
        ));
    }
    tally.files = map.files;
    tally.hunks = map.hunks;
    passes::register_prompts(context, tally);
    let candidates = passes::find(context, grants, deadline, tally)?;
    let confirmed = passes::verify(context, grants, &candidates, deadline, tally);
    let checks = match (&context.github, &grants.app) {
        (Some(github), Some(token)) => checks::wait(
            &crate::github_api::Client::github(),
            token.expose(),
            &github.repository,
            context.head.as_str(),
            started.checked_add(CHECKS_WAIT).unwrap_or(started),
        ),
        _ => checks::Seen::default(),
    };
    let (event, reason) = verdict::decide(&verdict::Evidence {
        confirmed: &confirmed,
        unjudged_blocking: tally.unjudged_blocking,
    });
    report::publish(
        context,
        grants,
        &map,
        &confirmed,
        &checks,
        (event, &reason),
        tally,
    )?;
    Ok((event, reason))
}

fn short(commit: &str) -> &str {
    commit.get(..SHORT_COMMIT_CHARS).unwrap_or(commit)
}

fn trace(
    context: &Context,
    started: SystemTime,
    tally: &Tally,
    reviewed: &Result<(Event, String), String>,
) -> Trace {
    let output = match reviewed {
        Ok((event, reason)) => format!("{}: {reason}", event.label()),
        Err(reason) => format!("no verdict: {reason}"),
    };
    let verdict = reviewed.as_ref().map_or("none", |(event, _)| event.label());
    Trace {
        name: "code-review".to_string(),
        session: context.session.clone(),
        tags: vec![context.tag.to_string()],
        started,
        ended: SystemTime::now(),
        input: serde_json::json!({
            "pull_request": context.session,
            "base": context.base.as_str(),
            "head": context.head.as_str(),
            "diff": tally.diff,
        })
        .to_string(),
        output,
        metadata: vec![
            ("verdict".to_string(), verdict.to_string()),
            ("head".to_string(), context.head.to_string()),
            ("counts".to_string(), report::counts(tally)),
            ("candidates".to_string(), tally.candidates.to_string()),
            ("confirmed".to_string(), tally.confirmed.to_string()),
        ],
        passes: tally.passes.clone(),
    }
}

/// Revokes the App's token when the run holds one; a failure is printed and returns false.
fn revoke(grants: &Grants) -> bool {
    grants.app.as_ref().is_none_or(|token| {
        app_token::revoke(token)
            .map_err(|reason| eprintln!("xtask review: {reason}"))
            .is_ok()
    })
}

/// Exports the trace and revokes the App's token; any failure on the way is the exit code.
fn finish(
    context: &Context,
    grants: &Grants,
    started: SystemTime,
    tally: &Tally,
    reviewed: &Result<(Event, String), String>,
) -> i32 {
    if let Err(reason) = reviewed {
        eprintln!("xtask review: {reason}");
    }
    for failure in &tally.failed {
        eprintln!("xtask review: a verify pass gave no verdict: {failure}");
    }
    for failure in &tally.trace_failures {
        eprintln!("xtask review: the trace: {failure}");
    }
    let traced = context.langfuse.as_ref().map_or_else(
        || {
            println!("xtask review: Langfuse is not configured; no trace sent");
            true
        },
        |langfuse| {
            langfuse::export(langfuse, &trace(context, started, tally, reviewed))
                .map_err(|reason| eprintln!("xtask review: {reason}"))
                .is_ok()
        },
    );
    let revoked = revoke(grants);
    exit_code(reviewed.is_ok(), tally, traced, revoked)
}

/// 0 only when the review gave a verdict, every verify pass did, the trace was exported with
/// every part of it recorded, and the App's token was revoked.
fn exit_code(reviewed: bool, tally: &Tally, exported: bool, revoked: bool) -> i32 {
    let traced = exported && tally.trace_failures.is_empty();
    i32::from(!reviewed || !tally.failed.is_empty() || !traced || !revoked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arguments_choose_the_mode_and_anything_else_is_a_usage_error() {
        assert_eq!(command(&[]), 2);
        assert_eq!(command(&["--post".to_string(), "x".to_string()]), 2);
        assert_eq!(command(&["--dry-run".to_string(), "x".to_string()]), 2);
    }

    fn step_running(workflow: &str, needle: &str) -> Option<String> {
        let lines: Vec<&str> = workflow.lines().collect();
        let at = lines.iter().position(|line| line.contains(needle))?;
        lines
            .get(..at)?
            .iter()
            .rev()
            .find(|line| line.contains("run:"))
            .map(|line| line.trim().to_string())
    }

    #[test]
    fn the_trusted_review_starts_only_on_ready_or_a_comment_and_runs_the_base_xtask() {
        let workflow = include_str!("../../.github/workflows/review.yml");
        let triggers: Vec<&str> = workflow
            .lines()
            .skip_while(|line| !line.starts_with("on:"))
            .skip(1)
            .take_while(|line| line.starts_with(' ') || line.starts_with('#'))
            .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
            .map(str::trim)
            .collect();
        assert_eq!(
            triggers,
            [
                "# The workflow and the xtask come from the base branch; xtask review fetches the pull",
                "# request's head by its commit as data the model reads, and never builds or runs it.",
                "pull_request_target: # zizmor: ignore[dangerous-triggers]",
                "issue_comment:",
            ]
        );
        assert!(workflow.contains("    types: [ready_for_review]\n"));
        assert!(workflow.contains("    types: [created]\n"));
        assert_eq!(
            step_running(workflow, "secrets.FRITS_CODE_REVIEW_APP_PRIVATE_KEY").as_deref(),
            Some("- run: target/debug/xtask review --post")
        );
        assert_eq!(
            workflow
                .matches("secrets.FRITS_CODE_REVIEW_APP_PRIVATE_KEY")
                .count(),
            1
        );
        let build = workflow
            .find("- run: cargo build --locked -p xtask")
            .unwrap();
        let review = workflow
            .find("- run: target/debug/xtask review --post")
            .unwrap();
        assert!(build < review);
        assert_eq!(workflow.matches("uses: actions/checkout@").count(), 1);
        assert_eq!(workflow.matches("persist-credentials: false").count(), 1);
        assert!(!workflow.contains("refs/pull/"));
        assert!(!workflow.contains("ref:"));
        assert!(!workflow.contains("working-directory"));
        assert_eq!(workflow.matches("- run:").count(), 2);
        assert!(
            workflow.contains("    environment:\n      name: review\n      deployment: false\n")
        );
    }

    #[test]
    fn the_dry_run_is_started_by_hand_and_never_holds_the_app_key() {
        let workflow = include_str!("../../.github/workflows/review-dry-run.yml");
        assert!(workflow.contains("on:\n  workflow_dispatch:\n"));
        assert!(!workflow.contains("pull_request"));
        assert!(!workflow.contains("FRITS_CODE_REVIEW"));
        assert!(!workflow.contains("pull-requests: write"));
        assert!(workflow.contains("- run: target/debug/xtask review --dry-run"));
        assert!(
            workflow.contains(
                "    environment:\n      name: review-dry-run\n      deployment: false\n"
            )
        );
    }

    #[test]
    fn a_part_of_the_trace_that_could_not_be_recorded_fails_the_run() {
        let clean = Tally::default();
        assert_eq!(exit_code(true, &clean, true, true), 0);
        let unregistered = Tally {
            trace_failures: vec!["looking up prompt review-find: refused".to_string()],
            ..Tally::default()
        };
        assert_eq!(exit_code(true, &unregistered, true, true), 1);
        assert_eq!(exit_code(true, &clean, false, true), 1);
        assert_eq!(exit_code(false, &clean, true, true), 1);
        assert_eq!(exit_code(true, &clean, true, false), 1);
    }

    #[test]
    fn the_trace_names_the_verdict_or_why_there_is_none() {
        let context = setup::tests::context(Vec::new());
        let tally = Tally::default();
        let started = SystemTime::now();
        let approved = trace(
            &context,
            started,
            &tally,
            &Ok((
                Event::Approve,
                "no confirmed high or medium findings".to_string(),
            )),
        );
        assert_eq!(approved.session, "o/r#1");
        assert_eq!(approved.tags, ["post"]);
        assert!(approved.output.starts_with("approve: "));
        let failed = trace(
            &context,
            started,
            &tally,
            &Err("the find pass ended".into()),
        );
        assert!(failed.output.starts_with("no verdict: "));
        assert!(
            failed
                .metadata
                .contains(&("verdict".to_string(), "none".to_string()))
        );
    }
}
