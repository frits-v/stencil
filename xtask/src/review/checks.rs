//! The other checks on the head commit: check runs and commit statuses, read with the App's
//! token and polled until every one has finished or the wait ends. They are reported beside
//! the verdict and never decide it.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::github_api::{Client, Method, Reply, github_message};

const PER_PAGE: usize = 100;
const PAGES_MAX: usize = 5;
const NAME_MAX_CHARS: usize = 100;
pub const POLL_INTERVAL: Duration = Duration::from_secs(30);
/// The review's own jobs, which would otherwise wait on themselves.
const OWN_CHECKS: [&str; 2] = ["review", "review-dry-run"];

/// How one check ended, or that it has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    Passed,
    Failed,
    /// Ended without a pass or a failure: neutral, skipped, cancelled, stale.
    Inconclusive,
    Unrecognised,
}

impl Outcome {
    const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
            Self::Unrecognised => "unrecognised",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub outcome: Outcome,
}

fn check_run_outcome(status: Option<&str>, conclusion: Option<&str>) -> Outcome {
    match (status, conclusion) {
        (Some("queued" | "in_progress" | "waiting" | "requested" | "pending"), _) => {
            Outcome::Pending
        }
        (Some("completed"), Some("success")) => Outcome::Passed,
        (
            Some("completed"),
            Some("failure" | "timed_out" | "action_required" | "startup_failure"),
        ) => Outcome::Failed,
        (Some("completed"), Some("neutral" | "skipped" | "cancelled" | "stale")) => {
            Outcome::Inconclusive
        }
        _ => Outcome::Unrecognised,
    }
}

fn status_outcome(state: Option<&str>) -> Outcome {
    match state {
        Some("pending") => Outcome::Pending,
        Some("success") => Outcome::Passed,
        Some("failure" | "error") => Outcome::Failed,
        _ => Outcome::Unrecognised,
    }
}

fn plain_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .take(NAME_MAX_CHARS)
        .collect()
}

fn own(name: &str) -> bool {
    OWN_CHECKS.contains(&name)
}

/// The other check runs of one page, how many runs the page held, and the count GitHub
/// reports in all.
pub fn decode_check_runs(body: &str) -> Result<(Vec<Check>, usize, usize), String> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| "a check runs page is not JSON".to_string())?;
    let total = value
        .get("total_count")
        .and_then(Value::as_u64)
        .and_then(|total| usize::try_from(total).ok())
        .ok_or("a check runs page has no total_count")?;
    let runs = value
        .get("check_runs")
        .and_then(Value::as_array)
        .ok_or("a check runs page has no check_runs")?;
    let checks = runs
        .iter()
        .map(|run| {
            let text = |key: &str| run.get(key).and_then(Value::as_str);
            Check {
                name: plain_name(text("name").unwrap_or_default()),
                outcome: check_run_outcome(text("status"), text("conclusion")),
            }
        })
        .filter(|check| !own(&check.name))
        .collect();
    Ok((checks, runs.len(), total))
}

pub fn decode_statuses(body: &str) -> Result<Vec<Check>, String> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| "the combined status is not JSON".to_string())?;
    let statuses = value
        .get("statuses")
        .and_then(Value::as_array)
        .ok_or("the combined status has no statuses")?;
    Ok(statuses
        .iter()
        .map(|status| Check {
            name: plain_name(
                status
                    .get("context")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            outcome: status_outcome(status.get("state").and_then(Value::as_str)),
        })
        .collect())
}

fn get(client: &Client, token: &str, path: &str) -> Result<String, String> {
    match client.request(Method::Get, path, token, None) {
        Reply::Response { status: 200, body } => Ok(body),
        Reply::Response { status, body } => Err(format!(
            "{path} ended HTTP {status}{}",
            github_message(&body)
                .map(|message| format!(", {message}"))
                .unwrap_or_default()
        )),
        Reply::BodyTooLarge => Err(format!("{path} is over its cap")),
        Reply::Failed(reason) => Err(format!("{path}: {reason}")),
    }
}

/// Every other check on `head` once: the check runs, paged up to the bound, then the
/// commit statuses. More check runs than the bound reads is an error, not a truncation.
pub fn read(
    client: &Client,
    token: &str,
    repository: &str,
    head: &str,
) -> Result<Vec<Check>, String> {
    let mut checks = Vec::new();
    let mut seen = 0_usize;
    for page in 1..=PAGES_MAX {
        let path = format!(
            "/repos/{repository}/commits/{head}/check-runs?per_page={PER_PAGE}&page={page}"
        );
        let body = get(client, token, &path)?;
        let (mut runs, on_page, total) = decode_check_runs(&body)?;
        seen = seen.saturating_add(on_page);
        checks.append(&mut runs);
        if seen >= total || on_page < PER_PAGE {
            let status = get(
                client,
                token,
                &format!("/repos/{repository}/commits/{head}/status"),
            )?;
            checks.extend(decode_statuses(&status)?);
            return Ok(checks);
        }
    }
    Err(format!(
        "the head commit has more than {} check runs; the review reads at most that many",
        PER_PAGE.saturating_mul(PAGES_MAX)
    ))
}

/// What the polling saw: the last checks it read, and why the last read failed when it did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Seen {
    pub checks: Vec<Check>,
    pub unreadable: Option<String>,
}

/// Reads the checks until none is pending or `until` passes, sleeping between reads. A read
/// that fails is kept as the reason and polled again; it never stops the verdict.
pub fn wait(client: &Client, token: &str, repository: &str, head: &str, until: Instant) -> Seen {
    let polls = until
        .saturating_duration_since(Instant::now())
        .as_secs()
        .checked_div(POLL_INTERVAL.as_secs())
        .unwrap_or(0)
        .saturating_add(1);
    let mut seen = Seen::default();
    for _ in 0..polls {
        match read(client, token, repository, head) {
            Ok(checks) => {
                seen.checks = checks;
                seen.unreadable = None;
            }
            Err(reason) => seen.unreadable = Some(reason),
        }
        let waiting = seen.unreadable.is_some()
            || seen
                .checks
                .iter()
                .any(|check| check.outcome == Outcome::Pending);
        let no_time = Instant::now()
            .checked_add(POLL_INTERVAL)
            .is_none_or(|next| next > until);
        if !waiting || no_time {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    seen
}

/// One line per check for the review body, and why the checks could not be read when the
/// last read failed; `None` when there is nothing to say.
pub fn summary(seen: &Seen) -> Option<String> {
    let mut lines: Vec<String> = seen
        .unreadable
        .iter()
        .map(|reason| {
            format!(
                "- the checks could not be read: {}",
                reason.replace('`', "'")
            )
        })
        .collect();
    lines.extend(seen.checks.iter().map(|check| {
        format!(
            "- `{}`: {}",
            check.name.replace('`', "'"),
            check.outcome.label()
        )
    }));
    (!lines.is_empty()).then(|| lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github_api::tests::listener;

    #[test]
    fn check_run_states_decode_and_an_unknown_one_is_unrecognised() {
        for (status, conclusion, outcome) in [
            ("queued", None, Outcome::Pending),
            ("in_progress", None, Outcome::Pending),
            ("completed", Some("success"), Outcome::Passed),
            ("completed", Some("failure"), Outcome::Failed),
            ("completed", Some("timed_out"), Outcome::Failed),
            ("completed", Some("skipped"), Outcome::Inconclusive),
            ("completed", Some("cancelled"), Outcome::Inconclusive),
            ("completed", Some("glorious"), Outcome::Unrecognised),
            ("completed", None, Outcome::Unrecognised),
            ("exploded", None, Outcome::Unrecognised),
        ] {
            assert_eq!(
                check_run_outcome(Some(status), conclusion),
                outcome,
                "{status}"
            );
        }
        assert_eq!(status_outcome(Some("error")), Outcome::Failed);
        assert_eq!(status_outcome(Some("odd")), Outcome::Unrecognised);
    }

    #[test]
    fn the_reviews_own_jobs_are_left_out() {
        let body = serde_json::json!({"total_count": 3, "check_runs": [
            {"name": "ci", "status": "completed", "conclusion": "success"},
            {"name": "review", "status": "in_progress", "conclusion": null},
            {"name": "review-dry-run", "status": "queued", "conclusion": null},
        ]})
        .to_string();
        let (checks, on_page, total) = decode_check_runs(&body).unwrap();
        assert_eq!((on_page, total), (3, 3));
        assert_eq!(
            checks,
            [Check {
                name: "ci".to_string(),
                outcome: Outcome::Passed
            }]
        );
    }

    #[test]
    fn read_takes_the_check_runs_then_the_statuses() {
        let runs = serde_json::json!({"total_count": 1, "check_runs": [
            {"name": "ci", "status": "completed", "conclusion": "failure"}]});
        let statuses = serde_json::json!({"state": "pending", "statuses": [
            {"context": "deploy", "state": "pending"}]});
        let (base, seen) = listener(vec![(200, runs.to_string()), (200, statuses.to_string())]);
        let sha = "a".repeat(40);
        let checks = read(&Client::loopback(&base), "t", "o/r", &sha).unwrap();
        let outcomes: Vec<Outcome> = checks.iter().map(|check| check.outcome).collect();
        assert_eq!(outcomes, [Outcome::Failed, Outcome::Pending]);
        let requests = seen.recv().unwrap();
        assert!(requests.first().unwrap().starts_with(&format!(
            "GET /repos/o/r/commits/{sha}/check-runs?per_page=100&page=1 "
        )));
        assert!(
            requests
                .get(1)
                .unwrap()
                .starts_with(&format!("GET /repos/o/r/commits/{sha}/status "))
        );
    }

    #[test]
    fn a_wait_whose_bound_has_passed_reads_once_and_returns_what_is_pending() {
        let runs = serde_json::json!({"total_count": 1, "check_runs": [
            {"name": "ci", "status": "in_progress", "conclusion": null}]});
        let statuses = serde_json::json!({"statuses": []});
        let (base, _seen) = listener(vec![(200, runs.to_string()), (200, statuses.to_string())]);
        let seen = wait(
            &Client::loopback(&base),
            "t",
            "o/r",
            &"a".repeat(40),
            Instant::now(),
        );
        assert_eq!(seen.checks.first().unwrap().outcome, Outcome::Pending);
        assert_eq!(seen.unreadable, None);
    }

    #[test]
    fn a_failed_read_is_kept_as_the_reason_and_never_an_error() {
        let (base, _seen) = listener(vec![(502, "{}".to_string())]);
        let seen = wait(
            &Client::loopback(&base),
            "t",
            "o/r",
            &"a".repeat(40),
            Instant::now(),
        );
        assert!(seen.checks.is_empty());
        assert!(seen.unreadable.unwrap().contains("HTTP 502"));
    }

    #[test]
    fn a_refused_read_is_an_error() {
        let (base, _seen) = listener(vec![(
            403,
            r#"{"message":"Resource not accessible"}"#.into(),
        )]);
        let error = read(&Client::loopback(&base), "t", "o/r", &"a".repeat(40)).unwrap_err();
        assert!(
            error.contains("HTTP 403, Resource not accessible"),
            "{error}"
        );
    }

    #[test]
    fn the_summary_lists_each_check_and_why_a_read_failed() {
        assert_eq!(summary(&Seen::default()), None);
        let seen = Seen {
            checks: vec![Check {
                name: "c`i".to_string(),
                outcome: Outcome::Failed,
            }],
            unreadable: Some("HTTP 502".to_string()),
        };
        assert_eq!(
            summary(&seen).unwrap(),
            "- the checks could not be read: HTTP 502\n- `c'i`: failed"
        );
    }
}
