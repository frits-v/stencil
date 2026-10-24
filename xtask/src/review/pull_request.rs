//! The pull request under review: its event, its earlier review comments and the review this
//! run posts, all through the GitHub REST API in process with the job's token.

use serde_json::Value;

use super::verdict::Event;
use crate::github_api::{Client, Method, Reply, github_message};

const PER_PAGE: usize = 30;
const PAGES_MAX: usize = 10;
const BODY_EXCERPT_CHARS: usize = 300;
/// The mentions that ask for a review in a pull request comment.
const MENTIONS: [&str; 2] = ["@frits-code-review", "@claude"];
/// Who may ask: the repository's owner, its organisation's members and its collaborators.
const ASKERS: [&str; 3] = ["OWNER", "MEMBER", "COLLABORATOR"];
const COMMENT_MAX_CHARS: usize = 65_536;

/// The fields of a pull request the review reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub base: CommitId,
    pub head: CommitId,
    pub head_repository: String,
}

/// One finding an earlier review posted: its fingerprint and what the find pass is told
/// about it.
#[derive(Debug, PartialEq, Eq)]
pub struct Posted {
    pub fingerprint: String,
    pub excerpt: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PostOutcome {
    Posted { url: String },
    Failed(u16, Option<String>),
    Unrecognised(&'static str),
}

/// A full commit id, 40 or 64 lowercase hex digits, checked once where it enters: what the
/// review fetches, reads and posts on is always an exact commit, never a ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitId(String);

impl CommitId {
    pub fn parse(value: &str) -> Result<Self, String> {
        let full = matches!(value.len(), 40 | 64)
            && value
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
        if full {
            Ok(Self(value.to_string()))
        } else {
            Err(format!(
                "`{}` is not a full commit id",
                value.chars().take(80).collect::<String>()
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// What started a posting run: a pull request marked ready, whose event carries it, or a
/// comment asking for a review, which names only its number.
#[derive(Debug, PartialEq, Eq)]
pub enum Trigger {
    Ready(PullRequest),
    Asked { number: u64 },
}

/// Whether `body` mentions the reviewer as a whole word.
fn mentions(body: &str) -> bool {
    MENTIONS.iter().any(|mention| {
        body.match_indices(mention).any(|(start, _)| {
            let before = body.get(..start).and_then(|text| text.chars().next_back());
            let after = body
                .get(start.saturating_add(mention.len())..)
                .and_then(|text| text.chars().next());
            before.is_none_or(char::is_whitespace)
                && after.is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        })
    })
}

/// Decodes the event of a posting run. Only `pull_request_target` marked ready and a new
/// pull request comment from a writer that mentions the reviewer start a review; every other
/// event is refused, so pushes and the review's own comments never start one.
pub fn decode_trigger(event_name: &str, text: &str) -> Result<Trigger, String> {
    let event: Value =
        serde_json::from_str(text).map_err(|_| "the event file is not JSON".to_string())?;
    let action = event.get("action").and_then(Value::as_str);
    match (event_name, action) {
        ("pull_request_target", Some("ready_for_review")) => {
            let pull_request = event
                .get("pull_request")
                .ok_or("the event has no pull request")?;
            let association = pull_request
                .get("author_association")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !ASKERS.contains(&association) {
                return Err(format!(
                    "the pull request's author association is `{association}`; a pull request \
                     from {} is reviewed when marked ready, any other when one of them asks",
                    ASKERS.join(", ")
                ));
            }
            decode_pull_request(pull_request).map(Trigger::Ready)
        }
        ("issue_comment", Some("created")) => decode_ask(&event),
        _ => Err(format!(
            "a {event_name} event ({action:?}) does not start a review; mark the pull request \
             ready or mention {} in a comment",
            MENTIONS.join(" or ")
        )),
    }
}

fn decode_ask(event: &Value) -> Result<Trigger, String> {
    let text_at = |pointer: &str| event.pointer(pointer).and_then(Value::as_str);
    if event.pointer("/issue/pull_request").is_none() {
        return Err("the comment is on an issue, not a pull request".to_string());
    }
    let body = text_at("/comment/body").unwrap_or_default();
    if body.chars().count() > COMMENT_MAX_CHARS || !mentions(body) {
        return Err(format!(
            "the comment does not mention {}",
            MENTIONS.join(" or ")
        ));
    }
    if text_at("/comment/user/type") != Some("User") {
        return Err("the comment is not from a user account".to_string());
    }
    let association = text_at("/comment/author_association").unwrap_or_default();
    if !ASKERS.contains(&association) {
        return Err(format!(
            "the commenter's association is `{association}`; only {} ask for a review",
            ASKERS.join(", ")
        ));
    }
    event
        .pointer("/issue/number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .map(|number| Trigger::Asked { number })
        .ok_or_else(|| "the comment's issue has no number".to_string())
}

/// The commits and head repository of a pull request object, from an event or the API.
fn decode_pull_request(pull_request: &Value) -> Result<PullRequest, String> {
    let text_at = |pointer: &str| pull_request.pointer(pointer).and_then(Value::as_str);
    let number = pull_request
        .get("number")
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or("the event has no pull request number")?;
    let (Some(base), Some(head)) = (text_at("/base/sha"), text_at("/head/sha")) else {
        return Err("the event has no base or head commit".to_string());
    };
    let (base, head) = (CommitId::parse(base)?, CommitId::parse(head)?);
    let head_repository = text_at("/head/repo/full_name")
        .ok_or("the event has no head repository")?
        .to_string();
    Ok(PullRequest {
        number,
        base,
        head,
        head_repository,
    })
}

/// The pull request as the API reports it now, for a run a comment started.
pub fn fetch(
    client: &Client,
    token: &str,
    repository: &str,
    number: u64,
) -> Result<PullRequest, String> {
    let path = format!("/repos/{repository}/pulls/{number}");
    match client.request(Method::Get, &path, token, None) {
        Reply::Response { status: 200, body } => {
            let value: Value = serde_json::from_str(&body)
                .map_err(|_| "the pull request response is not JSON".to_string())?;
            decode_pull_request(&value)
        }
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

fn excerpt(prefix: &str, text: &str) -> String {
    let text = text.trim();
    let skipped = text.chars().count().saturating_sub(BODY_EXCERPT_CHARS);
    format!("{prefix}{}", text.chars().skip(skipped).collect::<String>())
}

fn ours(item: &Value, login: &str) -> bool {
    item.pointer("/user/login").and_then(Value::as_str) == Some(login)
}

/// Our findings in one inline comment, located by its path and line.
fn from_comment(comment: &Value, login: &str) -> Vec<Posted> {
    if !ours(comment, login) {
        return Vec::new();
    }
    let body = comment
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let path = comment
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let line = comment
        .get("line")
        .and_then(Value::as_u64)
        .map_or_else(String::new, |line| format!(":{line}"));
    super::findings::marked_entries(body)
        .into_iter()
        .map(|(fingerprint, text)| Posted {
            fingerprint: fingerprint.to_string(),
            excerpt: excerpt(&format!("{path}{line}: "), text),
        })
        .collect()
}

/// Our findings in one review's body, each located by its own entry.
fn from_review(review: &Value, login: &str) -> Vec<Posted> {
    if !ours(review, login) {
        return Vec::new();
    }
    let body = review
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default();
    super::findings::marked_entries(body)
        .into_iter()
        .map(|(fingerprint, text)| Posted {
            fingerprint: fingerprint.to_string(),
            excerpt: excerpt("", text),
        })
        .collect()
}

/// Every item of a paged listing; a listing that would go past [`PAGES_MAX`] is an error,
/// not a truncation.
fn listing(client: &Client, token: &str, path: &str) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    for page in 1..=PAGES_MAX {
        let paged = format!("{path}?per_page={PER_PAGE}&page={page}");
        let body = match client.request(Method::Get, &paged, token, None) {
            Reply::Response { status: 200, body } => body,
            Reply::Response { status, body } => {
                return Err(format!(
                    "listing {path} ended HTTP {status}{}",
                    github_message(&body)
                        .map(|m| format!(", {m}"))
                        .unwrap_or_default()
                ));
            }
            Reply::BodyTooLarge => return Err(format!("a page of {path} is over its cap")),
            Reply::Failed(reason) => return Err(format!("listing {path}: {reason}")),
        };
        let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&body) else {
            return Err(format!("a page of {path} is not a JSON array"));
        };
        let full = items.len() >= PER_PAGE;
        all.extend(items);
        if !full {
            return Ok(all);
        }
    }
    Err(format!(
        "{path} lists more than {} items; the review reads at most that many",
        PER_PAGE.saturating_mul(PAGES_MAX)
    ))
}

/// What the review App posted on the pull request before: its findings, inline and in
/// review bodies, and how many reviews it posted.
#[derive(Debug, PartialEq, Eq)]
pub struct Earlier {
    pub posted: Vec<Posted>,
    pub reviews: usize,
}

/// Reads what `login` posted on the pull request before; anyone else's text is ignored.
pub fn earlier(
    client: &Client,
    token: &str,
    repository: &str,
    number: u64,
    login: &str,
) -> Result<Earlier, String> {
    let base = format!("/repos/{repository}/pulls/{number}");
    let comments = listing(client, token, &format!("{base}/comments"))?;
    let reviews = listing(client, token, &format!("{base}/reviews"))?;
    let mut posted: Vec<Posted> = comments
        .iter()
        .flat_map(|comment| from_comment(comment, login))
        .collect();
    posted.extend(reviews.iter().flat_map(|review| from_review(review, login)));
    Ok(Earlier {
        posted,
        reviews: reviews.iter().filter(|review| ours(review, login)).count(),
    })
}

pub fn decode_post(status: u16, body: &str) -> PostOutcome {
    if status != 200 {
        return PostOutcome::Failed(status, github_message(body));
    }
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return PostOutcome::Unrecognised("the review response is not JSON");
    };
    value
        .get("html_url")
        .and_then(Value::as_str)
        .filter(|url| url.starts_with("https://"))
        .map_or(
            PostOutcome::Unrecognised("the review response has no html_url"),
            |url| PostOutcome::Posted {
                url: url.to_string(),
            },
        )
}

/// Posts one review with the verdict's event on the head commit the review read.
pub fn post_review(
    client: &Client,
    token: &str,
    repository: &str,
    pull_request: &PullRequest,
    review: (Event, &str),
    comments: &[(String, u32, String)],
) -> PostOutcome {
    let (event, body) = review;
    let comments: Vec<Value> = comments
        .iter()
        .map(|(path, line, text)| {
            serde_json::json!({"path": path, "line": line, "side": "RIGHT", "body": text})
        })
        .collect();
    let request = serde_json::json!({
        "commit_id": pull_request.head.as_str(),
        "event": event.api_name(),
        "body": body,
        "comments": comments,
    })
    .to_string();
    let path = format!("/repos/{repository}/pulls/{}/reviews", pull_request.number);
    match client.request(Method::Post, &path, token, Some(&request)) {
        Reply::Response { status, body } => decode_post(status, &body),
        Reply::BodyTooLarge => PostOutcome::Unrecognised("the review response is over its cap"),
        Reply::Failed(reason) => PostOutcome::Unrecognised(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github_api::tests::listener;

    const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
    const BASE: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const BOT: &str = "frits-code-review[bot]";

    fn pull_request(head: &str) -> Value {
        serde_json::json!({
            "number": 12,
            "base": {"sha": BASE},
            "head": {"sha": head, "repo": {"full_name": "o/r"}},
        })
    }

    fn ready(head: &str) -> String {
        let mut pull_request = pull_request(head);
        pull_request["author_association"] = Value::from("MEMBER");
        serde_json::json!({"action": "ready_for_review", "pull_request": pull_request}).to_string()
    }

    fn asked(body: &str, association: &str, kind: &str) -> String {
        serde_json::json!({
            "action": "created",
            "issue": {"number": 12, "pull_request": {"url": "u"}},
            "comment": {"body": body, "author_association": association,
                        "user": {"login": "someone", "type": kind}},
        })
        .to_string()
    }

    #[test]
    fn a_pull_request_marked_ready_starts_a_review_with_its_commits() {
        assert_eq!(
            decode_trigger("pull_request_target", &ready(HEAD)),
            Ok(Trigger::Ready(PullRequest {
                number: 12,
                base: CommitId::parse(BASE).unwrap(),
                head: CommitId::parse(HEAD).unwrap(),
                head_repository: "o/r".to_string(),
            }))
        );
        assert!(decode_trigger("pull_request_target", &ready("main")).is_err());
        assert!(decode_trigger("pull_request_target", &ready(&HEAD.to_uppercase())).is_err());
        assert!(decode_trigger("pull_request_target", "nope").is_err());
    }

    #[test]
    fn an_outsiders_pull_request_marked_ready_starts_nothing() {
        for association in ["CONTRIBUTOR", "FIRST_TIME_CONTRIBUTOR", "NONE"] {
            let outsider = ready(HEAD).replace("\"MEMBER\"", &format!("\"{association}\""));
            let refused = decode_trigger("pull_request_target", &outsider).unwrap_err();
            assert!(refused.contains(association), "{refused}");
        }
    }

    #[test]
    fn opening_or_pushing_never_starts_a_review() {
        for action in ["opened", "synchronize", "reopened", "edited"] {
            let event = serde_json::json!({"action": action, "pull_request": pull_request(HEAD)});
            let refused = decode_trigger("pull_request_target", &event.to_string()).unwrap_err();
            assert!(refused.contains("does not start a review"), "{refused}");
        }
        assert!(decode_trigger("push", &ready(HEAD)).is_err());
        assert!(decode_trigger("pull_request", &ready(HEAD)).is_err());
    }

    #[test]
    fn a_writers_comment_mentioning_the_reviewer_asks_for_a_review() {
        for body in [
            "@frits-code-review please",
            "can you look, @claude?",
            "@claude",
            "line one\n@frits-code-review",
        ] {
            assert_eq!(
                decode_trigger("issue_comment", &asked(body, "MEMBER", "User")),
                Ok(Trigger::Asked { number: 12 }),
                "{body}"
            );
        }
    }

    #[test]
    fn a_comment_without_the_mention_from_an_outsider_or_a_bot_or_on_an_issue_is_refused() {
        for body in [
            "no mention",
            "@claudette",
            "mail@claude",
            "@frits-code-reviewer",
        ] {
            assert!(
                decode_trigger("issue_comment", &asked(body, "OWNER", "User")).is_err(),
                "{body}"
            );
        }
        for association in ["CONTRIBUTOR", "FIRST_TIME_CONTRIBUTOR", "NONE"] {
            assert!(
                decode_trigger("issue_comment", &asked("@claude", association, "User")).is_err()
            );
        }
        assert!(decode_trigger("issue_comment", &asked("@claude", "OWNER", "Bot")).is_err());
        let on_issue = serde_json::json!({"action": "created", "issue": {"number": 3},
            "comment": {"body": "@claude", "author_association": "OWNER",
                        "user": {"type": "User"}}});
        assert!(decode_trigger("issue_comment", &on_issue.to_string()).is_err());
        let edited = asked("@claude", "OWNER", "User").replace("\"created\"", "\"edited\"");
        assert!(decode_trigger("issue_comment", &edited).is_err());
    }

    #[test]
    fn only_a_full_lowercase_object_name_is_a_commit_id() {
        assert_eq!(CommitId::parse(HEAD).unwrap().as_str(), HEAD);
        assert!(CommitId::parse(&"c".repeat(64)).is_ok());
        for refused in [
            "main",
            "refs/heads/main",
            "0123abc",
            "",
            &HEAD.to_uppercase(),
        ] {
            assert!(CommitId::parse(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn a_pull_request_is_fetched_by_number() {
        let (base, seen) = listener(vec![(200, pull_request(HEAD).to_string())]);
        let fetched = fetch(&Client::loopback(&base), "t", "o/r", 12).unwrap();
        assert_eq!(fetched.head.as_str(), HEAD);
        let requests = seen.recv().unwrap();
        assert!(
            requests
                .first()
                .unwrap()
                .starts_with("GET /repos/o/r/pulls/12 ")
        );
    }

    fn comment(body: &str) -> Value {
        serde_json::json!({"body": body, "path": "a.rs", "line": 3, "user": {"login": BOT}})
    }

    #[test]
    fn only_the_apps_marked_findings_and_reviews_count() {
        let ours = "x\n<!-- stencil-review 0123456789abcdef -->";
        let full: Vec<Value> = (0..PER_PAGE).map(|_| comment("no marker")).collect();
        let planted = serde_json::json!({"body": "<!-- stencil-review 1111111111111111 -->",
            "path": "a.rs", "line": 3, "user": {"login": "someone"}});
        let last = vec![
            comment(ours),
            comment("<!-- stencil-review nothex -->"),
            planted,
        ];
        let review = serde_json::json!({"user": {"login": BOT}, "body":
            "- `b.rs:9` (Low, correctness): s\n  f\n  <!-- stencil-review fedcba9876543210 -->"});
        let planted_review = serde_json::json!({"user": {"login": "someone"},
            "body": "<!-- stencil-review 2222222222222222 -->"});
        let plain_review = serde_json::json!({"user": {"login": BOT}, "body": "Approve."});
        let (base, seen) = listener(vec![
            (200, Value::Array(full).to_string()),
            (200, Value::Array(last).to_string()),
            (
                200,
                Value::Array(vec![review, planted_review, plain_review]).to_string(),
            ),
        ]);
        let earlier = earlier(&Client::loopback(&base), "t", "o/r", 12, BOT).unwrap();
        let fingerprints: Vec<&str> = earlier
            .posted
            .iter()
            .map(|entry| entry.fingerprint.as_str())
            .collect();
        assert_eq!(fingerprints, ["0123456789abcdef", "fedcba9876543210"]);
        assert_eq!(earlier.reviews, 2);
        assert_eq!(earlier.posted.first().unwrap().excerpt, "a.rs:3: x");
        let requests = seen.recv().unwrap();
        assert!(
            requests
                .get(1)
                .unwrap()
                .starts_with("GET /repos/o/r/pulls/12/comments?per_page=30&page=2 ")
        );
        assert!(
            requests
                .get(2)
                .unwrap()
                .starts_with("GET /repos/o/r/pulls/12/reviews?per_page=30&page=1 ")
        );
    }

    #[test]
    fn a_listing_past_its_bound_or_a_refusal_is_an_error() {
        let full = Value::Array((0..PER_PAGE).map(|_| comment("c")).collect());
        let (base, _seen) = listener(vec![(200, full.to_string()); PAGES_MAX]);
        let error = earlier(&Client::loopback(&base), "t", "o/r", 1, BOT).unwrap_err();
        assert!(error.contains("more than 300"), "{error}");
        let (base, _seen) = listener(vec![(
            403,
            r#"{"message":"Resource not accessible"}"#.into(),
        )]);
        let error = earlier(&Client::loopback(&base), "t", "o/r", 1, BOT).unwrap_err();
        assert!(
            error.contains("HTTP 403, Resource not accessible"),
            "{error}"
        );
    }

    #[test]
    fn the_review_carries_the_verdicts_event_on_the_head_commit() {
        let (base, seen) = listener(vec![(
            200,
            r#"{"id":1,"html_url":"https://github.com/o/r/pull/12#pullrequestreview-1"}"#.into(),
        )]);
        let Ok(Trigger::Ready(pull_request)) = decode_trigger("pull_request_target", &ready(HEAD))
        else {
            panic!("not ready");
        };
        let outcome = post_review(
            &Client::loopback(&base),
            "t",
            "o/r",
            &pull_request,
            (Event::RequestChanges, "body"),
            &[("a.rs".to_string(), 4, "text".to_string())],
        );
        assert!(matches!(outcome, PostOutcome::Posted { .. }), "{outcome:?}");
        let requests = seen.recv().unwrap();
        let raw = requests.first().unwrap();
        assert!(raw.starts_with("POST /repos/o/r/pulls/12/reviews HTTP/1.1\r\n"));
        let sent: Value = serde_json::from_str(raw.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(
            sent,
            serde_json::json!({"commit_id": HEAD, "event": "REQUEST_CHANGES", "body": "body",
                "comments": [{"path": "a.rs", "line": 4, "side": "RIGHT", "body": "text"}]})
        );
    }

    #[test]
    fn post_decoder_covers_each_state() {
        assert_eq!(
            decode_post(422, r#"{"message":"Unprocessable Entity"}"#),
            PostOutcome::Failed(422, Some("Unprocessable Entity".to_string()))
        );
        assert!(matches!(
            decode_post(200, "x"),
            PostOutcome::Unrecognised(_)
        ));
        assert!(matches!(
            decode_post(200, r#"{"html_url":"javascript:x"}"#),
            PostOutcome::Unrecognised(_)
        ));
    }
}
