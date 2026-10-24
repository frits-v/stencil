//! One headless Claude CLI run of the review: its command line, its environment and the
//! decoding of the result line that ends its event stream.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::process::Exit;

/// Model aliases, so the review follows the newest model of each tier.
pub const MODEL: &str = "opus";
/// Read-only tools; under `dontAsk` they reach only the working directory.
const TOOLS: &str = "Read,Grep,Glob";
const PROMPT: &str = "Carry out the review the appended system instructions describe.";
pub const AUTH_TOKEN_VARIABLE: &str = "ANTHROPIC_AUTH_TOKEN";
const INHERITED: [&str; 4] = ["PATH", "HOME", "LANG", "TMPDIR"];
/// The largest cost a run may report, a million dollars; anything above is unrecognised.
const COST_MICRO_DOLLARS_MAX: f64 = 1e12;
const SUBTYPE_MAX_CHARS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_turns: u32,
    pub max_budget_usd: u32,
}

/// What a run spent: input counts cache reads and writes too; cost is in micro-dollars.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: u64,
}

/// How one run ended. Every result the CLI wrote carries what it spent.
#[derive(Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Answered { output: Value, usage: Usage },
    Unanswered { usage: Usage },
    MaxTurns { usage: Usage },
    BudgetExhausted { usage: Usage },
    ApiError { status: Option<u64>, usage: Usage },
    Ended(Exit),
    Unrecognised(String),
}

impl RunOutcome {
    pub const fn usage(&self) -> Usage {
        match self {
            Self::Answered { usage, .. }
            | Self::Unanswered { usage }
            | Self::MaxTurns { usage }
            | Self::BudgetExhausted { usage }
            | Self::ApiError { usage, .. } => *usage,
            Self::Ended(_) | Self::Unrecognised(_) => Usage {
                input_tokens: 0,
                output_tokens: 0,
                cost: 0,
            },
        }
    }

    /// A short name for the trace and the log, never the output itself.
    pub fn label(&self) -> String {
        match self {
            Self::Answered { .. } => "answered".to_string(),
            Self::Unanswered { .. } => "unanswered".to_string(),
            Self::MaxTurns { .. } => "max_turns".to_string(),
            Self::BudgetExhausted { .. } => "budget_exhausted".to_string(),
            Self::ApiError { status, .. } => format!("api_error {status:?}"),
            Self::Ended(exit) => format!("ended {exit:?}"),
            Self::Unrecognised(reason) => format!("unrecognised: {reason}"),
        }
    }
}

/// The command line of a run. Setting sources are empty so the checkout's
/// `.claude/settings.json`, which the pull request controls, adds no hook or permission.
/// `--add-dir` takes several values, so another flag always follows it.
pub fn command(
    program: &Path,
    instructions: &Path,
    files: &Path,
    schema: &str,
    limits: Limits,
    environment: Vec<(OsString, OsString)>,
) -> Command {
    let mut command = crate::child::empty(program);
    command
        .args(["-p", "--output-format", "stream-json", "--verbose"])
        .arg("--include-partial-messages")
        .args(["--thinking-display", "summarized", "--model", MODEL])
        .args(["--tools", TOOLS, "--permission-mode", "dontAsk"])
        .args(["--setting-sources", "", "--strict-mcp-config"])
        .arg("--no-session-persistence")
        .arg("--add-dir")
        .arg(files)
        .args(["--max-turns", &limits.max_turns.to_string()])
        .args(["--max-budget-usd", &limits.max_budget_usd.to_string()])
        .args(["--json-schema", schema])
        .arg("--append-system-prompt-file")
        .arg(instructions)
        .arg(PROMPT)
        .envs(environment);
    command
}

/// The run's environment: a few inherited variables, the user's name from the password
/// database (the macOS keychain login needs it), and the access token when there is one.
/// Never the GitHub token or the Actions OIDC request variables.
pub fn environment(
    parent: impl IntoIterator<Item = (OsString, OsString)>,
    user: Option<&str>,
    access_token: Option<&str>,
) -> Vec<(OsString, OsString)> {
    let mut kept: Vec<(OsString, OsString)> = parent
        .into_iter()
        .filter(|(name, _)| INHERITED.iter().any(|kept| name == *kept))
        .collect();
    if let Some(user) = user {
        kept.push(("USER".into(), user.into()));
        kept.push(("LOGNAME".into(), user.into()));
    }
    kept.push((
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".into(),
        "1".into(),
    ));
    if let Some(token) = access_token {
        kept.push((AUTH_TOKEN_VARIABLE.into(), token.into()));
    }
    kept
}

/// `total_cost_usd` rounded to micro-dollars; `None` when it is not a finite non-negative
/// number within the bound.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is finite, non-negative and below 1e12, so it fits a u64 exactly"
)]
fn micro_dollars(cost: &Value) -> Option<u64> {
    let micro = (cost.as_f64()? * 1e6).round();
    (micro.is_finite() && (0.0..=COST_MICRO_DOLLARS_MAX).contains(&micro)).then_some(micro as u64)
}

fn plain_word(text: &str) -> String {
    text.chars()
        .take(SUBTYPE_MAX_CHARS)
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// Decodes the last `result` line of the run's output, the event stream of
/// `--output-format stream-json` or the one line of `--output-format json`. The CLI exits 0
/// with `is_error` false and 1 with it true; any other pairing is unrecognised.
pub fn decode(exit: Exit, stdout: &str) -> RunOutcome {
    let code = match exit {
        Exit::Code(code @ (0 | 1)) => code,
        other => return RunOutcome::Ended(other),
    };
    let result = stdout
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .find(|value| value.get("type").and_then(Value::as_str) == Some("result"));
    let Some(result) = result else {
        return RunOutcome::Unrecognised(format!(
            "exit {code} with no result line in the output; the CLI's message is above"
        ));
    };
    let Some(cost) = result.get("total_cost_usd").and_then(micro_dollars) else {
        return RunOutcome::Unrecognised("the result has no usable total_cost_usd".to_string());
    };
    let count = |key: &str| {
        result
            .pointer(&format!("/usage/{key}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let usage = Usage {
        input_tokens: count("input_tokens")
            .saturating_add(count("cache_read_input_tokens"))
            .saturating_add(count("cache_creation_input_tokens")),
        output_tokens: count("output_tokens"),
        cost,
    };
    let subtype = result.get("subtype").and_then(Value::as_str).unwrap_or("");
    let is_error = result.get("is_error").and_then(Value::as_bool);
    let api_status = result.get("api_error_status").and_then(Value::as_u64);
    match (code, subtype, is_error) {
        (0, "success", Some(false)) => result
            .get("structured_output")
            .filter(|output| output.is_object())
            .map_or(RunOutcome::Unanswered { usage }, |output| {
                RunOutcome::Answered {
                    output: output.clone(),
                    usage,
                }
            }),
        (1, "success", Some(true)) if api_status.is_some() => RunOutcome::ApiError {
            status: api_status,
            usage,
        },
        (1, "error_during_execution", Some(true)) => RunOutcome::ApiError {
            status: api_status,
            usage,
        },
        (1, "error_max_turns", Some(true)) => RunOutcome::MaxTurns { usage },
        (1, "error_max_budget_usd", Some(true)) => RunOutcome::BudgetExhausted { usage },
        (code, subtype, is_error) => RunOutcome::Unrecognised(format!(
            "exit {code}, subtype `{}`, is_error {is_error:?}",
            plain_word(subtype)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from the pinned CLI with a small schema, a turn cap, a budget cap and a bogus
    /// `ANTHROPIC_AUTH_TOKEN`.
    const SUCCESS: &str = include_str!("../../fixtures/review/claude-success.json");
    const MAX_TURNS: &str = include_str!("../../fixtures/review/claude-max-turns.json");
    const MAX_BUDGET: &str = include_str!("../../fixtures/review/claude-max-budget.json");
    const UNAUTHORIZED: &str = include_str!("../../fixtures/review/claude-unauthorized.json");
    /// Event streams captured from the pinned CLI with `--thinking-display summarized`.
    const STREAM_SUCCESS: &str = include_str!("../../fixtures/review/claude-stream-success.jsonl");
    const STREAM_MAX_TURNS: &str =
        include_str!("../../fixtures/review/claude-stream-max-turns.jsonl");

    #[test]
    fn the_command_line_has_exact_text() {
        let command = command(
            Path::new("/opt/claude/2.1.280/claude"),
            Path::new("/t/stencil-review-7/find.md"),
            Path::new("/t/stencil-review-7"),
            "{}",
            Limits {
                max_turns: 80,
                max_budget_usd: 10,
            },
            vec![("PATH".into(), "/usr/bin".into())],
        );
        let words: Vec<String> = std::iter::once(command.get_program())
            .chain(command.get_args())
            .map(|word| word.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            words,
            [
                "/opt/claude/2.1.280/claude",
                "-p",
                "--output-format",
                "stream-json",
                "--verbose",
                "--include-partial-messages",
                "--thinking-display",
                "summarized",
                "--model",
                "opus",
                "--tools",
                "Read,Grep,Glob",
                "--permission-mode",
                "dontAsk",
                "--setting-sources",
                "",
                "--strict-mcp-config",
                "--no-session-persistence",
                "--add-dir",
                "/t/stencil-review-7",
                "--max-turns",
                "80",
                "--max-budget-usd",
                "10",
                "--json-schema",
                "{}",
                "--append-system-prompt-file",
                "/t/stencil-review-7/find.md",
                PROMPT,
            ]
        );
        assert!(!words.iter().any(|word| word == "--bare"));
    }

    #[test]
    fn the_run_never_receives_the_github_token_or_the_oidc_request() {
        let parent: Vec<(OsString, OsString)> = [
            ("PATH", "/usr/bin"),
            ("HOME", "/home/runner"),
            ("GITHUB_TOKEN", "ghs_x"),
            ("GH_TOKEN", "ghs_x"),
            ("ACTIONS_ID_TOKEN_REQUEST_URL", "https://x"),
            ("ACTIONS_ID_TOKEN_REQUEST_TOKEN", "t"),
            ("ACTIONS_RUNTIME_TOKEN", "t"),
            ("ANTHROPIC_FEDERATION_RULE_ID", "fdrl_x"),
            ("ANTHROPIC_API_KEY", "k"),
            ("LANGFUSE_SECRET_KEY", "sk-lf-x"),
            ("LANGFUSE_PUBLIC_KEY", "pk-lf-x"),
            ("FRITS_CODE_REVIEW_APP_PRIVATE_KEY", "key"),
        ]
        .into_iter()
        .map(|(name, value)| (name.into(), value.into()))
        .collect();
        let kept = environment(parent, Some("runner"), Some("sk-ant-oat01-x"));
        let names: Vec<String> = kept
            .iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "PATH",
                "HOME",
                "USER",
                "LOGNAME",
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
                AUTH_TOKEN_VARIABLE
            ]
        );
        let local = environment(Vec::new(), None, None);
        assert!(
            !local
                .iter()
                .any(|(name, _)| name == AUTH_TOKEN_VARIABLE || name == "USER")
        );
    }

    #[test]
    fn each_captured_result_decodes_to_its_state() {
        match decode(Exit::Code(0), SUCCESS) {
            RunOutcome::Answered { output, usage } => {
                assert!(output.get("findings").is_some_and(Value::is_array));
                assert_eq!(
                    usage,
                    Usage {
                        input_tokens: 28 + 7368 + 13_532,
                        output_tokens: 771,
                        cost: 19_972
                    }
                );
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            decode(Exit::Code(1), MAX_TURNS),
            RunOutcome::MaxTurns {
                usage: Usage {
                    input_tokens: 6381,
                    output_tokens: 218,
                    cost: 13_842
                }
            }
        );
        assert_eq!(
            decode(Exit::Code(1), MAX_BUDGET),
            RunOutcome::BudgetExhausted {
                usage: Usage {
                    cost: 5_114,
                    ..Usage::default()
                }
            }
        );
        assert_eq!(
            decode(Exit::Code(1), UNAUTHORIZED),
            RunOutcome::ApiError {
                status: Some(401),
                usage: Usage::default()
            }
        );
    }

    #[test]
    fn a_stream_decodes_by_its_last_result_line() {
        match decode(Exit::Code(0), STREAM_SUCCESS) {
            RunOutcome::Answered { output, usage } => {
                assert!(output.get("findings").is_some_and(Value::is_array));
                assert_eq!(usage.output_tokens, 291);
                assert_eq!(usage.cost, 46_079);
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            decode(Exit::Code(1), STREAM_MAX_TURNS),
            RunOutcome::MaxTurns { .. }
        ));
        let without_result: String = STREAM_SUCCESS
            .lines()
            .filter(|line| !line.contains(r#""type":"result""#))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(matches!(
            decode(Exit::Code(0), &without_result),
            RunOutcome::Unrecognised(reason) if reason.contains("no result line")
        ));
    }

    #[test]
    fn an_exit_that_disagrees_with_the_result_or_a_malformed_result_is_unrecognised() {
        for (exit, stdout) in [
            (Exit::Code(1), SUCCESS),
            (Exit::Code(0), MAX_TURNS),
            (Exit::Code(2), SUCCESS),
            (Exit::Code(0), "not json"),
            (Exit::Code(0), r#"{"type":"assistant"}"#),
            (
                Exit::Code(0),
                r#"{"type":"result","subtype":"success","is_error":false}"#,
            ),
            (
                Exit::Code(1),
                r#"{"type":"result","subtype":"weird\u001b[31m","is_error":true,"total_cost_usd":0}"#,
            ),
        ] {
            let outcome = decode(exit, stdout);
            assert!(
                matches!(
                    outcome,
                    RunOutcome::Unrecognised(_) | RunOutcome::Ended(Exit::Code(2))
                ),
                "{outcome:?}"
            );
            assert!(!format!("{outcome:?}").contains('\u{1b}'));
        }
        assert_eq!(
            decode(Exit::TimedOut, SUCCESS),
            RunOutcome::Ended(Exit::TimedOut)
        );
        let unanswered = SUCCESS.replace("\"structured_output\"", "\"other_output\"");
        assert!(matches!(
            decode(Exit::Code(0), &unanswered),
            RunOutcome::Unanswered { .. }
        ));
    }
}
