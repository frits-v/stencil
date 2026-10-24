//! The two kinds of Claude pass: one find pass over the diff, then a verify pass per
//! candidate, each in a fresh session, with what each one spent recorded for the trace.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Value, json};

use super::claude_run::{self, Limits, RunOutcome};
use super::findings::{self, Finding, Verdict};
use super::langfuse::{Pass, PromptLink};
use super::prompt_registry::{self, Template};
use super::pull_request::Posted;
use super::stream::{self, Transcript};
use super::verdict::blocks;
use super::{Context, EXCLUDED_PATHSPECS, Grants, Tally, VERIFIED_MAX, git_in};
use crate::anthropic_token;
use crate::process::{Exit, run_bounded, run_bounded_timed_lines};

const FIND_SCHEMA: &str = include_str!("../../schemas/review_findings.json");
const VERDICT_SCHEMA: &str = include_str!("../../schemas/review_verdict.json");
const FIND_INSTRUCTIONS: &str = include_str!("../../prompts/review-find.md");
const VERIFY_INSTRUCTIONS: &str = include_str!("../../prompts/review-verify.md");
const DIFF_MAX_BYTES: usize = 512 * 1024;
/// The event stream of one pass, an 80-turn find pass with its tool results included.
const STREAM_MAX_BYTES: u64 = 64 << 20;
const GIT_TIMEOUT: Duration = Duration::from_secs(120);
const FIND_LIMITS: Limits = Limits {
    max_turns: 80,
    max_budget_usd: 10,
};
const VERIFY_LIMITS: Limits = Limits {
    max_turns: 30,
    max_budget_usd: 2,
};
const VERIFY_PARALLEL: usize = 4;
const ONE_RUN_MIN: Duration = Duration::from_secs(60);
/// Left unused at the end of a pass's token, so no request goes out on a token about to lapse.
const TOKEN_MARGIN: Duration = Duration::from_secs(60);
/// The longest a pass's exchange can take: the OIDC token and the exchange, each one request.
const MINT_TIME_MAX: Duration =
    Duration::from_secs(2 * crate::github_api::REQUEST_TIMEOUT.as_secs());
/// The shortest token a pass can use: its exchange, the margin and a minute of work.
pub const TOKEN_LIFETIME_MIN: Duration =
    Duration::from_secs(MINT_TIME_MAX.as_secs() + TOKEN_MARGIN.as_secs() + ONE_RUN_MIN.as_secs());

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// The diff of the reviewed checkout, through git without the run's secrets: it runs inside
/// the pull request's checkout.
fn diff_command(context: &Context) -> Command {
    let mut command = git_in(&context.checkout);
    command
        .args(["-c", "core.quotePath=false", "diff", "--no-color"])
        .args(["--no-ext-diff", "--no-textconv", "-M", "--unified=3"])
        .arg(format!("{}...{}", context.base, context.head))
        .args(["--", "."])
        .args(EXCLUDED_PATHSPECS);
    command
}

/// The diff, written where the model can read it with the earlier findings beside it.
pub fn prepare(context: &Context) -> Result<String, String> {
    let range = format!("{}...{}", context.base, context.head);
    let captured = run_bounded(&mut diff_command(context), GIT_TIMEOUT);
    if captured.exit != Exit::Code(0) {
        return Err(format!("git diff {range} ended {:?}", captured.exit));
    }
    if captured.stdout.len() > DIFF_MAX_BYTES {
        return Err(format!(
            "the diff is {} bytes; the review takes at most {DIFF_MAX_BYTES}",
            captured.stdout.len()
        ));
    }
    write_file(&context.files.join("diff.patch"), &captured.stdout)?;
    write_file(
        &context.files.join("posted.json"),
        &posted_listing(&context.posted),
    )?;
    Ok(captured.stdout)
}

fn posted_listing(posted: &[Posted]) -> String {
    let entries: Vec<Value> = posted
        .iter()
        .map(|entry| Value::from(entry.excerpt.as_str()))
        .collect();
    Value::Array(entries).to_string()
}

/// The values a template's `{{name}}` variables take in this run.
fn variables(context: &Context, finding: Option<&Finding>) -> Vec<(&'static str, String)> {
    let finding = finding
        .and_then(|finding| serde_json::to_string_pretty(finding).ok())
        .unwrap_or_default();
    let in_files = |name: &str| context.files.join(name).display().to_string();
    vec![
        ("head", context.head.as_str().to_string()),
        ("base", context.base.as_str().to_string()),
        ("files", context.files.display().to_string()),
        ("diff", in_files("diff.patch")),
        ("posted", in_files("posted.json")),
        ("finding", finding),
    ]
}

fn render(template: &str, variables: &[(&'static str, String)]) -> String {
    variables
        .iter()
        .fold(template.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{{{name}}}}}"), value)
        })
}

/// The variables a pass's template used, as the pass's input in the trace.
fn variables_json(variables: &[(&'static str, String)]) -> String {
    let object: serde_json::Map<String, Value> = variables
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| ((*name).to_string(), json!(value)))
        .collect();
    Value::Object(object).to_string()
}

fn template(
    name: &'static str,
    text: &'static str,
    schema: &str,
    limits: Limits,
) -> Result<Template, String> {
    let schema: Value = serde_json::from_str(&minified(schema)?)
        .map_err(|_| format!("the schema of {name} does not read back as JSON"))?;
    Ok(Template {
        name,
        text,
        config: json!({
            "model": claude_run::MODEL,
            "max_turns": limits.max_turns,
            "max_budget_usd": limits.max_budget_usd,
            "json_schema": schema,
        }),
    })
}

/// The templates as Langfuse's prompt management holds them.
fn templates() -> Result<[Template; 2], String> {
    Ok([
        template(
            "stencil-review-find",
            FIND_INSTRUCTIONS,
            FIND_SCHEMA,
            FIND_LIMITS,
        )?,
        template(
            "stencil-review-verify",
            VERIFY_INSTRUCTIONS,
            VERDICT_SCHEMA,
            VERIFY_LIMITS,
        )?,
    ])
}

/// The prompt versions this run's passes link to.
#[derive(Debug, Default)]
pub struct Prompts {
    pub find: Option<PromptLink>,
    pub verify: Option<PromptLink>,
}

/// Registers both templates when the run traces; a failure leaves that pass unlinked and is
/// kept for the run's exit code, like a failed export.
pub fn register_prompts(context: &Context, tally: &mut Tally) {
    let Some(langfuse) = &context.langfuse else {
        return;
    };
    let [find, verify] = match templates() {
        Ok(templates) => templates,
        Err(reason) => {
            tally.trace_failures.push(reason);
            return;
        }
    };
    let mut link = |template: &Template| {
        prompt_registry::register(langfuse, template, context.tag)
            .map_err(|reason| tally.trace_failures.push(reason))
            .ok()
    };
    let find = link(&find);
    let verify = link(&verify);
    tally.prompts = Prompts { find, verify };
}

/// The exported schema on one line without its `$schema` dialect: the CLI's validator refuses
/// the draft 2020-12 URI cue writes, and reads `$defs` and patterns without it.
pub fn minified(schema: &str) -> Result<String, String> {
    let mut value = serde_json::from_str::<Value>(schema)
        .map_err(|_| "an exported review schema is not JSON; run `mise run config-export`")?;
    value
        .as_object_mut()
        .ok_or("an exported review schema is not a JSON object")?
        .remove("$schema");
    Ok(value.to_string())
}

/// The time a pass has once its token is minted: until the run's deadline or the token's end
/// less the margin, whichever comes first, dated from before the exchange, which the token
/// was issued after. Less than a minute is none.
fn pass_time(
    run: Instant,
    started: Instant,
    minted: Instant,
    token_lifetime: Option<Duration>,
) -> Option<Duration> {
    token_lifetime
        .and_then(|lifetime| started.checked_add(lifetime.saturating_sub(TOKEN_MARGIN)))
        .map_or(run, |token_end| token_end.min(run))
        .checked_duration_since(minted)
        .filter(|remaining| *remaining >= ONE_RUN_MIN)
}

/// What one launch left: how the run ended, its transcript, and when the CLI started.
struct Launched {
    outcome: RunOutcome,
    transcript: Transcript,
    at: Option<SystemTime>,
}

impl Launched {
    fn before_start(outcome: RunOutcome) -> Self {
        Self {
            outcome,
            transcript: Transcript::default(),
            at: None,
        }
    }
}

/// Mints the pass's own token, when the run mints any, and runs the CLI under the earlier of
/// the run's deadline and the token's end.
fn launch(
    context: &Context,
    grants: &Grants,
    path: &Path,
    schema: &str,
    limits: Limits,
    deadline: Instant,
) -> Launched {
    let started = Instant::now();
    if deadline
        .checked_duration_since(started)
        .is_none_or(|remaining| remaining < ONE_RUN_MIN)
    {
        return Launched::before_start(RunOutcome::Ended(Exit::TimedOut));
    }
    let token = match grants
        .anthropic
        .as_ref()
        .map(anthropic_token::Minter::mint)
        .transpose()
    {
        Ok(token) => token,
        Err(reason) => {
            return Launched::before_start(RunOutcome::Unrecognised(format!(
                "the token exchange: {reason}"
            )));
        }
    };
    let lifetime = token
        .as_ref()
        .map(|token| Duration::from_secs(token.expires_in_seconds));
    let Some(remaining) = pass_time(deadline, started, Instant::now(), lifetime) else {
        return Launched::before_start(RunOutcome::Ended(Exit::TimedOut));
    };
    let environment = claude_run::environment(
        std::env::vars_os(),
        context.user.as_deref(),
        token.as_ref().map(|token| token.value.expose()),
    );
    let mut command = claude_run::command(
        &context.claude,
        path,
        &context.files,
        schema,
        limits,
        environment,
    );
    command.current_dir(&context.checkout);
    let at = SystemTime::now();
    let timed = run_bounded_timed_lines(&mut command, remaining, STREAM_MAX_BYTES);
    Launched {
        outcome: claude_run::decode(timed.captured.exit, &timed.captured.stdout),
        transcript: stream::read(&timed.captured.stdout, &timed.line_ends),
        at: Some(at),
    }
}

/// What a pass runs: its name, its template and the values it is rendered with, and the
/// schema and limits of its run.
struct Run<'a> {
    name: &'a str,
    template: &'static str,
    variables: Vec<(&'static str, String)>,
    prompt: Option<&'a PromptLink>,
    schema: &'a str,
    limits: Limits,
}

/// One pass: the instructions written to the run's directory, the CLI run under its
/// deadline, and the pass the trace records.
fn run_claude(
    context: &Context,
    grants: &Grants,
    run: &Run<'_>,
    deadline: Instant,
) -> (RunOutcome, Pass) {
    let started = SystemTime::now();
    let path = context.files.join(format!("{}.md", run.name));
    let launched = match write_file(&path, &render(run.template, &run.variables)) {
        Err(reason) => Launched::before_start(RunOutcome::Unrecognised(reason)),
        Ok(()) => launch(context, grants, &path, run.schema, run.limits, deadline),
    };
    let outcome = launched.outcome;
    let pass = Pass {
        name: run.name.to_string(),
        model: claude_run::MODEL,
        started,
        ended: SystemTime::now(),
        launched: launched.at,
        input: variables_json(&run.variables),
        output: match &outcome {
            RunOutcome::Answered { output, .. } => output.to_string(),
            other => other.label(),
        },
        outcome: outcome.label(),
        usage: outcome.usage(),
        prompt: run.prompt.cloned(),
        transcript: launched.transcript,
    };
    (outcome, pass)
}

pub fn find(
    context: &Context,
    grants: &Grants,
    deadline: Instant,
    tally: &mut Tally,
) -> Result<Vec<Finding>, String> {
    let schema = minified(FIND_SCHEMA)?;
    let run = Run {
        name: "find",
        template: FIND_INSTRUCTIONS,
        variables: variables(context, None),
        prompt: tally.prompts.find.as_ref(),
        schema: &schema,
        limits: FIND_LIMITS,
    };
    let (outcome, pass) = run_claude(context, grants, &run, deadline);
    tally.cost = tally.cost.saturating_add(pass.usage.cost);
    tally.passes.push(pass);
    match outcome {
        RunOutcome::Answered { output, .. } => {
            let candidates = findings::decode_findings(&output)?;
            tally.candidates = candidates.len();
            Ok(candidates)
        }
        other => Err(format!("the find pass ended {}", other.label())),
    }
}

/// One verify pass; `Ok` carries the verdict, `Err` why the pass gave none.
fn verify_one(
    context: &Context,
    grants: &Grants,
    index: usize,
    finding: &Finding,
    shared: &Shared<'_>,
    deadline: Instant,
) -> (Result<Verdict, String>, Pass) {
    let name = format!("verify-{index}");
    let run = Run {
        name: &name,
        template: VERIFY_INSTRUCTIONS,
        variables: variables(context, Some(finding)),
        prompt: shared.prompt,
        schema: shared.schema,
        limits: VERIFY_LIMITS,
    };
    let (outcome, pass) = run_claude(context, grants, &run, deadline);
    let verdict = match outcome {
        RunOutcome::Answered { output, .. } => {
            findings::decode_verdict(&output).map(|(verdict, reasoning)| {
                println!(
                    "xtask review: {}:{} {verdict:?}: {reasoning}",
                    finding.path, finding.line
                );
                verdict
            })
        }
        other => Err(other.label()),
    };
    (verdict, pass)
}

/// What every verify pass of a run shares: the output schema and the prompt version.
struct Shared<'a> {
    schema: &'a str,
    prompt: Option<&'a PromptLink>,
}

fn verify_chunk(
    context: &Context,
    grants: &Grants,
    first_index: usize,
    chunk: &[Finding],
    shared: &Shared<'_>,
    deadline: Instant,
) -> Vec<(Result<Verdict, String>, Option<Pass>)> {
    #[allow(
        clippy::needless_collect,
        reason = "every pass in the chunk starts before the first is joined"
    )]
    thread::scope(|scope| {
        let handles: Vec<_> = chunk
            .iter()
            .enumerate()
            .map(|(offset, finding)| {
                let index = first_index.saturating_add(offset);
                scope.spawn(move || verify_one(context, grants, index, finding, shared, deadline))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| match handle.join() {
                Ok((verdict, pass)) => (verdict, Some(pass)),
                Err(_) => (Err("the verify thread panicked".to_string()), None),
            })
            .collect()
    })
}

/// Verifies the most severe candidates, a few sessions at a time, and keeps the confirmed.
/// A blocking candidate over the cap or without a verdict is counted as unjudged.
pub fn verify(
    context: &Context,
    grants: &Grants,
    candidates: &[Finding],
    deadline: Instant,
    tally: &mut Tally,
) -> Vec<Finding> {
    let schema = match minified(VERDICT_SCHEMA) {
        Ok(schema) => schema,
        Err(reason) => {
            tally.failed.push(reason);
            tally.unjudged_blocking = candidates.iter().filter(|finding| blocks(finding)).count();
            return Vec::new();
        }
    };
    let over_cap = candidates.get(VERIFIED_MAX..).unwrap_or_default();
    tally.unverified = over_cap.len();
    tally.unjudged_blocking = over_cap.iter().filter(|finding| blocks(finding)).count();
    let checked = candidates.get(..VERIFIED_MAX).unwrap_or(candidates);
    let mut confirmed = Vec::new();
    for (batch, chunk) in checked.chunks(VERIFY_PARALLEL).enumerate() {
        let first = batch.saturating_mul(VERIFY_PARALLEL);
        let prompt = tally.prompts.verify.clone();
        let shared = Shared {
            schema: &schema,
            prompt: prompt.as_ref(),
        };
        let results = verify_chunk(context, grants, first, chunk, &shared, deadline);
        for (finding, (verdict, pass)) in chunk.iter().zip(results) {
            if let Some(pass) = pass {
                tally.cost = tally.cost.saturating_add(pass.usage.cost);
                tally.passes.push(pass);
            }
            match verdict {
                Ok(Verdict::Confirmed) => confirmed.push(finding.clone()),
                Ok(Verdict::Refuted) => tally.refuted = tally.refuted.saturating_add(1),
                Err(reason) => {
                    if blocks(finding) {
                        tally.unjudged_blocking = tally.unjudged_blocking.saturating_add(1);
                    }
                    tally
                        .failed
                        .push(format!("{}:{}: {reason}", finding.path, finding.line));
                }
            }
        }
    }
    tally.confirmed = confirmed.len();
    confirmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schemas_handed_to_the_cli_name_no_dialect_and_the_prompts_name_every_placeholder() {
        for schema in [FIND_SCHEMA, VERDICT_SCHEMA] {
            assert!(schema.contains("\"$schema\""));
            let handed = minified(schema).unwrap();
            assert!(!handed.contains("$schema"), "{handed}");
            assert!(handed.contains("\"$defs\""));
        }
        for placeholder in [
            "{{head}}",
            "{{base}}",
            "{{files}}",
            "{{diff}}",
            "{{posted}}",
        ] {
            assert!(FIND_INSTRUCTIONS.contains(placeholder), "{placeholder}");
        }
        for placeholder in [
            "{{head}}",
            "{{base}}",
            "{{files}}",
            "{{diff}}",
            "{{finding}}",
        ] {
            assert!(VERIFY_INSTRUCTIONS.contains(placeholder), "{placeholder}");
        }
    }

    #[test]
    fn the_diff_runs_in_the_reviewed_checkout_with_only_the_inherited_variables() {
        let context = crate::review::setup::tests::context(Vec::new());
        let command = diff_command(&context);
        for (name, _) in command.get_envs() {
            let name = name.to_string_lossy();
            assert!(crate::child::INHERITED.contains(&name.as_ref()), "{name}");
        }
        let words: Vec<String> = command
            .get_args()
            .map(|word| word.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            words.get(..2),
            Some(["-C".to_string(), "/r/pull-request".to_string()].as_slice())
        );
        assert!(words.contains(&"--no-ext-diff".to_string()));
        assert!(words.contains(&"--no-textconv".to_string()));
    }

    #[test]
    fn a_pass_ends_at_the_run_deadline_or_before_its_token_lapses() {
        let now = Instant::now();
        let run = now.checked_add(Duration::from_secs(1800)).unwrap();
        assert_eq!(
            pass_time(run, now, now, None),
            Some(Duration::from_secs(1800))
        );
        assert_eq!(
            pass_time(run, now, now, Some(Duration::from_secs(598))),
            Some(Duration::from_secs(538))
        );
        let soon = now.checked_add(Duration::from_secs(120)).unwrap();
        assert_eq!(
            pass_time(soon, now, now, Some(Duration::from_secs(598))),
            Some(Duration::from_secs(120))
        );
        assert_eq!(pass_time(soon, now, soon, None), None);
    }

    #[test]
    fn the_shortest_accepted_token_leaves_a_minute_after_the_slowest_exchange() {
        let started = Instant::now();
        let run = started.checked_add(Duration::from_secs(1800)).unwrap();
        let minted = started.checked_add(MINT_TIME_MAX).unwrap();
        assert_eq!(
            pass_time(run, started, minted, Some(TOKEN_LIFETIME_MIN)),
            Some(ONE_RUN_MIN)
        );
        let shorter = TOKEN_LIFETIME_MIN
            .checked_sub(Duration::from_secs(1))
            .unwrap();
        assert_eq!(pass_time(run, started, minted, Some(shorter)), None);
    }

    #[test]
    fn a_rendered_prompt_names_the_runs_own_files() {
        let context = crate::review::setup::tests::context(Vec::new());
        let rendered = render(FIND_INSTRUCTIONS, &variables(&context, None));
        assert!(rendered.contains("/t/stencil-review-1/diff.patch"));
        assert!(rendered.contains("/t/stencil-review-1/posted.json"));
        assert!(!rendered.contains("{{"));
        let input: Value =
            serde_json::from_str(&variables_json(&variables(&context, None))).unwrap();
        assert_eq!(
            input.get("diff"),
            Some(&json!("/t/stencil-review-1/diff.patch"))
        );
        assert!(input.get("finding").is_none());
    }

    #[test]
    fn a_failed_registration_leaves_the_pass_unlinked_and_is_kept_for_the_exit_code() {
        let mut tally = Tally::default();
        let untraced = crate::review::setup::tests::context(Vec::new());
        register_prompts(&untraced, &mut tally);
        assert_eq!(tally.prompts.find, None);
        assert!(tally.trace_failures.is_empty());
        let unreachable = Context {
            langfuse: Some(crate::review::langfuse::tests::langfuse(
                "https://127.0.0.1:1",
            )),
            ..crate::review::setup::tests::context(Vec::new())
        };
        register_prompts(&unreachable, &mut tally);
        assert_eq!(tally.prompts.find, None);
        assert_eq!(tally.prompts.verify, None);
        let [find, verify] = tally.trace_failures.as_slice() else {
            panic!("{:?}", tally.trace_failures);
        };
        assert!(
            find.starts_with("looking up prompt stencil-review-find: "),
            "{find}"
        );
        assert!(
            verify.starts_with("looking up prompt stencil-review-verify: "),
            "{verify}"
        );
    }

    #[test]
    fn the_registered_templates_are_the_prompt_files_with_their_run_settings() {
        let [find, verify] = templates().unwrap();
        assert_eq!(find.name, "stencil-review-find");
        assert_eq!(find.text, include_str!("../../prompts/review-find.md"));
        assert_eq!(verify.name, "stencil-review-verify");
        assert_eq!(verify.text, include_str!("../../prompts/review-verify.md"));
        assert_eq!(find.config.get("model"), Some(&json!(claude_run::MODEL)));
        assert_eq!(
            find.config.get("max_turns"),
            Some(&json!(FIND_LIMITS.max_turns))
        );
        assert_eq!(
            verify.config.get("max_budget_usd"),
            Some(&json!(VERIFY_LIMITS.max_budget_usd))
        );
        let schema = find.config.get("json_schema").unwrap();
        assert_eq!(schema.to_string(), minified(FIND_SCHEMA).unwrap());
    }

    #[test]
    fn a_pass_past_its_deadline_never_starts_and_is_recorded() {
        let context = crate::review::setup::tests::context(Vec::new());
        let dir = std::env::temp_dir().join(format!("stencil-review-pass-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let context = Context {
            files: dir.clone(),
            ..context
        };
        let link = PromptLink {
            name: "review-find".to_string(),
            version: 2,
        };
        let run = Run {
            name: "find",
            template: "text {{head}}",
            variables: vec![("head", "h".to_string())],
            prompt: Some(&link),
            schema: "{}",
            limits: FIND_LIMITS,
        };
        let (outcome, pass) = run_claude(
            &context,
            &crate::review::setup::tests::grants("t"),
            &run,
            Instant::now(),
        );
        assert_eq!(outcome, RunOutcome::Ended(Exit::TimedOut));
        assert_eq!(pass.name, "find");
        assert_eq!(pass.input, r#"{"head":"h"}"#);
        assert_eq!(pass.outcome, "ended TimedOut");
        assert_eq!(pass.prompt, Some(link));
        assert!(pass.launched.is_none());
        assert_eq!(fs::read_to_string(dir.join("find.md")).unwrap(), "text h");
        fs::remove_file(dir.join("find.md")).unwrap();
        fs::remove_dir(&dir).unwrap();
    }
}
