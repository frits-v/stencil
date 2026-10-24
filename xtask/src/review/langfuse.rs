//! One Langfuse trace per review, sent as OTLP/JSON to Langfuse's OpenTelemetry endpoint in
//! process: a root span for the review, an agent span for each Claude pass, a generation for
//! each assistant message of the pass, linked to the prompt version it ran, and a tool span
//! for each tool call. The keys never reach the Claude CLI's environment.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde_json::{Value, json};

use super::claude_run::Usage;
use super::findings::sha256_line;
use super::stream::{Thinking, ToolCall, Transcript, Turn};
use crate::github_api::{Credential, Method, PlainClient, Reply};

pub const BASE_URL_VARIABLE: &str = "LANGFUSE_BASE_URL";
pub const PUBLIC_KEY_VARIABLE: &str = "LANGFUSE_PUBLIC_KEY";
pub const SECRET_KEY_VARIABLE: &str = "LANGFUSE_SECRET_KEY";
const TRACES_PATH: &str = "/api/public/otel/v1/traces";
const INGESTION_VERSION: (&str, &str) = ("x-langfuse-ingestion-version", "4");
const KEY_MAX_CHARS: usize = 256;
const BASE_URL_MAX_CHARS: usize = 512;
const SERVICE_NAME: &str = "stencil-review";
const TRACE_ID_HEX_CHARS: usize = 32;
const SPAN_ID_HEX_CHARS: usize = 16;
/// Spans sent in one request, so a long review is several requests of a bounded size.
const SPANS_PER_REQUEST: usize = 100;
/// Token kinds weighted by their price relative to input, in tenths: output 5x, cache reads
/// 0.1x and one-hour cache writes 2x, the writes the review's runs make.
const WEIGHT_INPUT: u128 = 10;
const WEIGHT_OUTPUT: u128 = 50;
const WEIGHT_CACHE_READ: u128 = 1;
const WEIGHT_CACHE_WRITE: u128 = 20;

/// Where traces and prompts go and the Basic credential they carry.
pub struct Langfuse {
    pub(super) base: String,
    pub(super) basic: String,
}

impl std::fmt::Debug for Langfuse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Langfuse")
            .field("base", &self.base)
            .field("basic", &"[redacted]")
            .finish()
    }
}

/// A prompt version in Langfuse's prompt management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptLink {
    pub name: String,
    pub version: u64,
}

/// One Claude pass: an agent in the trace, with its messages and tool calls under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    pub name: String,
    pub model: &'static str,
    pub started: SystemTime,
    pub ended: SystemTime,
    /// When the CLI started; the transcript's times count from it.
    pub launched: Option<SystemTime>,
    /// The values the prompt was rendered with, as JSON.
    pub input: String,
    pub output: String,
    pub outcome: String,
    pub usage: Usage,
    pub prompt: Option<PromptLink>,
    pub transcript: Transcript,
}

/// The review as Langfuse shows it: grouped into a session per pull request, tagged with
/// the mode, with the verdict and counts as metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub name: String,
    pub session: String,
    pub tags: Vec<String>,
    pub started: SystemTime,
    pub ended: SystemTime,
    pub input: String,
    pub output: String,
    pub metadata: Vec<(String, String)>,
    pub passes: Vec<Pass>,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn usable_key(name: &str, value: &str) -> Result<(), String> {
    if value.len() <= KEY_MAX_CHARS && value.chars().all(|c| c.is_ascii_graphic()) {
        Ok(())
    } else {
        Err(format!("{name} is not a Langfuse key"))
    }
}

/// The Langfuse settings: all three set traces, none set turns tracing off, and anything
/// between is an error naming what is missing.
pub fn read(
    base_url: Option<String>,
    public_key: Option<String>,
    secret_key: Option<String>,
) -> Result<Option<Langfuse>, String> {
    match (
        non_empty(base_url),
        non_empty(public_key),
        non_empty(secret_key),
    ) {
        (None, None, None) => Ok(None),
        (Some(base_url), Some(public_key), Some(secret_key)) => {
            let base_url = base_url.trim_end_matches('/');
            let url_valid = base_url.starts_with("https://")
                && base_url.len() <= BASE_URL_MAX_CHARS
                && base_url.chars().all(|c| c.is_ascii_graphic())
                && !base_url.contains(['?', '#']);
            if !url_valid {
                return Err(format!("{BASE_URL_VARIABLE} is not an HTTPS base URL"));
            }
            usable_key(PUBLIC_KEY_VARIABLE, &public_key)?;
            usable_key(SECRET_KEY_VARIABLE, &secret_key)?;
            Ok(Some(Langfuse {
                base: base_url.to_string(),
                basic: base64::engine::general_purpose::STANDARD
                    .encode(format!("{public_key}:{secret_key}")),
            }))
        }
        _ => Err(format!(
            "{BASE_URL_VARIABLE}, {PUBLIC_KEY_VARIABLE} and {SECRET_KEY_VARIABLE} are set \
             together or not at all"
        )),
    }
}

fn hex_id(seed: &str, path: &str, chars: usize) -> String {
    String::from_utf8_lossy(&sha256_line(format!("{seed}/{path}").as_bytes()))
        .chars()
        .take(chars)
        .collect()
}

/// Nanoseconds since the epoch as a decimal string, the OTLP/JSON form of a 64-bit integer.
fn nanos(time: SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos())
        .to_string()
}

fn text(key: &str, value: &str) -> Value {
    json!({"key": key, "value": {"stringValue": value}})
}

fn integer(key: &str, value: u64) -> Value {
    json!({"key": key, "value": {"intValue": value.to_string()}})
}

/// The trace-level attributes Langfuse wants on every span, so filters work per observation.
fn trace_attributes(trace: &Trace) -> Vec<Value> {
    let tags: Vec<Value> = trace
        .tags
        .iter()
        .map(|tag| json!({"stringValue": tag}))
        .collect();
    let mut attributes = vec![
        text("langfuse.trace.name", &trace.name),
        text("langfuse.session.id", &trace.session),
        json!({"key": "langfuse.trace.tags", "value": {"arrayValue": {"values": tags}}}),
    ];
    attributes.extend(
        trace
            .metadata
            .iter()
            .map(|(key, value)| text(&format!("langfuse.trace.metadata.{key}"), value)),
    );
    attributes
}

#[allow(
    clippy::cast_precision_loss,
    reason = "micro-dollars below 2^53 convert exactly; the bound is a million dollars"
)]
fn dollars(micro_dollars: u64) -> f64 {
    micro_dollars as f64 / 1e6
}

/// Builds the spans of one trace with ids derived from `seed` and a path per span.
struct Spans<'a> {
    trace: &'a Trace,
    seed: &'a str,
    trace_id: String,
    built: Vec<Value>,
}

impl Spans<'_> {
    fn span_id(&self, path: &str) -> String {
        hex_id(self.seed, path, SPAN_ID_HEX_CHARS)
    }

    fn push(
        &mut self,
        path: &str,
        parent: Option<&str>,
        name: &str,
        times: (SystemTime, SystemTime),
        attributes: Vec<Value>,
    ) {
        let mut all = trace_attributes(self.trace);
        all.extend(attributes);
        let mut span = json!({
            "traceId": self.trace_id,
            "spanId": self.span_id(path),
            "name": name,
            "kind": 1,
            "startTimeUnixNano": nanos(times.0),
            "endTimeUnixNano": nanos(times.1.max(times.0)),
            "attributes": all,
        });
        if let (Some(parent), Some(object)) = (parent, span.as_object_mut()) {
            object.insert("parentSpanId".to_string(), json!(self.span_id(parent)));
        }
        self.built.push(span);
    }

    fn pass(&mut self, index: usize, pass: &Pass) {
        let path = format!("pass/{index}");
        self.push(
            &path,
            Some("root"),
            &pass.name,
            (pass.started, pass.ended),
            pass_attributes(pass),
        );
        let transcript = &pass.transcript;
        let at = |offset: Option<std::time::Duration>, fallback: SystemTime| {
            pass.launched
                .zip(offset)
                .and_then(|(launched, offset)| launched.checked_add(offset))
                .unwrap_or(fallback)
        };
        let mut boundary = pass.launched.unwrap_or(pass.started);
        let shares = cost_shares(pass.usage.cost, &transcript.turns);
        for (turn_index, turn) in transcript.turns.iter().enumerate() {
            let ended = at(turn.last_seen, pass.ended);
            let turn_path = format!("{path}/turn/{turn_index}");
            let model = if turn.model.is_empty() {
                pass.model
            } else {
                turn.model.as_str()
            };
            self.push(
                &turn_path,
                Some(&path),
                &format!("{} turn {}", pass.name, turn_index.saturating_add(1)),
                (boundary, ended),
                turn_attributes(
                    turn,
                    transcript,
                    model,
                    pass.prompt.as_ref(),
                    shares.get(turn_index).copied().unwrap_or(0),
                ),
            );
            boundary = ended;
            for &call_index in &turn.tool_calls {
                let Some(call) = transcript.tool_calls.get(call_index) else {
                    continue;
                };
                let called = at(call.called, ended);
                let answered = at(call.answered, called);
                self.push(
                    &format!("{path}/tool/{call_index}"),
                    Some(&turn_path),
                    &call.name,
                    (called, answered),
                    tool_attributes(call),
                );
                boundary = boundary.max(answered);
            }
        }
    }
}

fn pass_attributes(pass: &Pass) -> Vec<Value> {
    let transcript = &pass.transcript;
    let metadata =
        |key: &str, value: &str| text(&format!("langfuse.observation.metadata.{key}"), value);
    let mut attributes = vec![
        text("langfuse.observation.type", "agent"),
        text("langfuse.observation.input", &pass.input),
        text("langfuse.observation.output", &pass.output),
        metadata("outcome", &pass.outcome),
        metadata("model", transcript.model.as_deref().unwrap_or(pass.model)),
        metadata("cost_usd", &dollars(pass.usage.cost).to_string()),
        metadata(
            "tokens",
            &json!({"input": pass.usage.input_tokens, "output": pass.usage.output_tokens})
                .to_string(),
        ),
        metadata("turns", &transcript.turns.len().to_string()),
        metadata("tool_calls", &transcript.tool_calls.len().to_string()),
    ];
    if transcript.unrecognised_count > 0 {
        attributes.push(metadata(
            "unrecognised_events",
            &json!({"count": transcript.unrecognised_count, "named": transcript.unrecognised})
                .to_string(),
        ));
    }
    if pass.outcome != "answered" {
        attributes.push(text("langfuse.observation.level", "WARNING"));
        attributes.push(text("langfuse.observation.status_message", &pass.outcome));
    }
    attributes
}

/// The pass's reported cost in micro-dollars shared across its messages by their tokens at
/// price-ratio weights; the shares add up to the reported total exactly, so the trace's cost
/// is the CLI's figure whatever price Langfuse knows for the model.
fn cost_shares(total: u64, turns: &[Turn]) -> Vec<u64> {
    let weights: Vec<u128> = turns
        .iter()
        .map(|turn| {
            let usage = turn.usage;
            u128::from(usage.input)
                .saturating_mul(WEIGHT_INPUT)
                .saturating_add(u128::from(usage.output).saturating_mul(WEIGHT_OUTPUT))
                .saturating_add(u128::from(usage.cache_read).saturating_mul(WEIGHT_CACHE_READ))
                .saturating_add(u128::from(usage.cache_creation).saturating_mul(WEIGHT_CACHE_WRITE))
        })
        .collect();
    let sum = weights
        .iter()
        .fold(0_u128, |sum, weight| sum.saturating_add(*weight));
    let mut shares: Vec<u64> = weights
        .iter()
        .map(|weight| {
            u128::from(total)
                .saturating_mul(*weight)
                .checked_div(sum)
                .and_then(|share| u64::try_from(share).ok())
                .unwrap_or(0)
        })
        .collect();
    let allocated = shares
        .iter()
        .fold(0_u64, |sum, share| sum.saturating_add(*share));
    if let Some(last) = shares.last_mut() {
        *last = last.saturating_add(total.saturating_sub(allocated));
    }
    shares
}

fn turn_attributes(
    turn: &Turn,
    transcript: &Transcript,
    model: &str,
    prompt: Option<&PromptLink>,
    cost: u64,
) -> Vec<Value> {
    let thinking: Vec<Value> = turn
        .thinking
        .iter()
        .map(|thinking| match thinking {
            Thinking::Summary(summary) => json!(summary),
            Thinking::Omitted => json!("(omitted by the server)"),
        })
        .collect();
    let tool_calls: Vec<Value> = turn
        .tool_calls
        .iter()
        .filter_map(|index| transcript.tool_calls.get(*index))
        .map(|call| {
            let input =
                serde_json::from_str::<Value>(&call.input).unwrap_or_else(|_| json!(call.input));
            json!({"name": call.name, "input": input})
        })
        .collect();
    let output = json!({"thinking": thinking, "text": turn.text, "tool_calls": tool_calls});
    let usage = json!({
        "input": turn.usage.input,
        "output": turn.usage.output,
        "cache_read_input_tokens": turn.usage.cache_read,
        "cache_creation_input_tokens": turn.usage.cache_creation,
    });
    let mut attributes = vec![
        text("langfuse.observation.type", "generation"),
        text("langfuse.observation.model.name", model),
        text("langfuse.observation.output", &output.to_string()),
        text("langfuse.observation.usage_details", &usage.to_string()),
        text(
            "langfuse.observation.cost_details",
            &json!({"total": dollars(cost)}).to_string(),
        ),
        text(
            "langfuse.observation.metadata.cost_source",
            "a share of the pass's total_cost_usd, by tokens at price-ratio weights",
        ),
        text("langfuse.observation.metadata.message_id", &turn.message_id),
    ];
    if let Some(estimate) = turn.usage.thinking_estimate {
        attributes.push(text(
            "langfuse.observation.metadata.thinking_tokens_estimate",
            &estimate.to_string(),
        ));
    }
    if !turn.usage.output_final {
        attributes.push(text(
            "langfuse.observation.metadata.output_tokens",
            "partial: the run ended before the message's final count",
        ));
    }
    if let Some(prompt) = prompt {
        attributes.push(text("langfuse.observation.prompt.name", &prompt.name));
        attributes.push(integer(
            "langfuse.observation.prompt.version",
            prompt.version,
        ));
    }
    attributes
}

fn tool_attributes(call: &ToolCall) -> Vec<Value> {
    let mut attributes = vec![
        text("langfuse.observation.type", "tool"),
        text("langfuse.observation.input", &call.input),
    ];
    let Some(output) = &call.output else {
        attributes.push(text("langfuse.observation.level", "WARNING"));
        attributes.push(text(
            "langfuse.observation.status_message",
            "no result arrived before the run ended",
        ));
        return attributes;
    };
    attributes.push(text("langfuse.observation.output", &output.kept));
    attributes.push(text(
        "langfuse.observation.metadata.output_bytes",
        &output.bytes.to_string(),
    ));
    if output.kept.len() < output.bytes {
        attributes.push(text(
            "langfuse.observation.metadata.output_kept_bytes",
            &output.kept.len().to_string(),
        ));
    }
    if output.is_error {
        attributes.push(text("langfuse.observation.level", "ERROR"));
    }
    attributes
}

/// The trace's spans: the review first, then each pass with its messages and tool calls.
pub fn spans(trace: &Trace, seed: &str) -> Vec<Value> {
    let mut spans = Spans {
        trace,
        seed,
        trace_id: hex_id(seed, "trace", TRACE_ID_HEX_CHARS),
        built: Vec::new(),
    };
    spans.push(
        "root",
        None,
        &trace.name,
        (trace.started, trace.ended),
        vec![
            text("langfuse.observation.input", &trace.input),
            text("langfuse.observation.output", &trace.output),
        ],
    );
    for (index, pass) in trace.passes.iter().enumerate() {
        spans.pass(index, pass);
    }
    spans.built
}

/// The OTLP/JSON request bodies, `SPANS_PER_REQUEST` spans each; `seed` makes the ids.
pub fn bodies(trace: &Trace, seed: &str) -> Vec<Value> {
    spans(trace, seed)
        .chunks(SPANS_PER_REQUEST)
        .map(|chunk| {
            json!({"resourceSpans": [{
                "resource": {"attributes": [text("service.name", SERVICE_NAME)]},
                "scopeSpans": [{"scope": {"name": SERVICE_NAME}, "spans": chunk}],
            }]})
        })
        .collect()
}

/// A seed no two runs share: the process id and the clock.
pub fn seed() -> String {
    format!("{}-{}", std::process::id(), nanos(SystemTime::now()))
}

/// Langfuse answers 2xx; a body that reports rejected spans is a failure too.
fn decode_export(status: u16, body: &str) -> Result<(), String> {
    if !(200..300).contains(&status) {
        return Err(format!("Langfuse refused the trace with HTTP {status}"));
    }
    let rejected = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.pointer("/partialSuccess/rejectedSpans")?.as_u64())
        .unwrap_or(0);
    if rejected > 0 {
        return Err(format!("Langfuse rejected {rejected} spans of the trace"));
    }
    Ok(())
}

pub fn export(langfuse: &Langfuse, trace: &Trace) -> Result<(), String> {
    export_with(&PlainClient::https(), langfuse, trace, &seed())
}

fn export_with(
    client: &PlainClient,
    langfuse: &Langfuse,
    trace: &Trace,
    seed: &str,
) -> Result<(), String> {
    let endpoint = format!("{}{TRACES_PATH}", langfuse.base);
    let all = bodies(trace, seed);
    let count = all.len();
    for (index, body) in all.iter().enumerate() {
        let reply = client.request_with(
            Method::Post,
            &endpoint,
            Credential::Basic(&langfuse.basic),
            &[INGESTION_VERSION],
            Some(&body.to_string()),
        );
        let part = format!("part {} of {count}", index.saturating_add(1));
        match reply {
            Reply::Response { status, body } => {
                decode_export(status, &body).map_err(|reason| format!("{reason} ({part})"))?;
            }
            Reply::BodyTooLarge => {
                return Err(format!("the Langfuse response to {part} is over its cap"));
            }
            Reply::Failed(reason) => {
                return Err(format!("sending {part} of the trace to Langfuse: {reason}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::github_api::tests::listener;
    use crate::review::stream;
    use std::time::Duration;

    pub fn langfuse(base: &str) -> Langfuse {
        Langfuse {
            base: base.to_string(),
            basic: "cGs6c2s=".to_string(),
        }
    }

    fn trace() -> Trace {
        let started = UNIX_EPOCH
            .checked_add(Duration::from_secs(1_800_000_000))
            .unwrap();
        let fixture = include_str!("../../fixtures/review/claude-stream-success.jsonl");
        let line_ends: Vec<Duration> = (0..7).map(Duration::from_secs).collect();
        Trace {
            name: "code-review".to_string(),
            session: "o/r#12".to_string(),
            tags: vec!["post".to_string()],
            started,
            ended: started.checked_add(Duration::from_secs(90)).unwrap(),
            input: r#"{"diff":"d"}"#.to_string(),
            output: "approve".to_string(),
            metadata: vec![("verdict".to_string(), "approve".to_string())],
            passes: vec![Pass {
                name: "find".to_string(),
                model: "opus",
                started,
                ended: started.checked_add(Duration::from_secs(60)).unwrap(),
                launched: started.checked_add(Duration::from_secs(2)),
                input: r#"{"head":"h"}"#.to_string(),
                output: r#"{"findings":[]}"#.to_string(),
                outcome: "answered".to_string(),
                usage: Usage {
                    input_tokens: 1000,
                    output_tokens: 200,
                    cost: 1_500_000,
                },
                prompt: Some(PromptLink {
                    name: "review-find".to_string(),
                    version: 3,
                }),
                transcript: stream::read(fixture, &line_ends),
            }],
        }
    }

    fn attribute<'a>(span: &'a Value, key: &str) -> Option<&'a Value> {
        span.get("attributes")?
            .as_array()?
            .iter()
            .find(|attribute| attribute.get("key").and_then(Value::as_str) == Some(key))?
            .get("value")
    }

    fn string<'a>(span: &'a Value, key: &str) -> &'a str {
        attribute(span, key)
            .and_then(|value| value.get("stringValue"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{key} missing from {span}"))
    }

    #[test]
    fn the_secret_key_is_taken_out_of_the_environment_at_startup() {
        assert!(crate::secrets::SECRET_VARIABLES.contains(&SECRET_KEY_VARIABLE));
    }

    #[test]
    fn all_three_settings_trace_none_turns_tracing_off_and_some_is_an_error() {
        assert!(read(None, None, None).unwrap().is_none());
        let langfuse = read(
            Some("https://us.cloud.langfuse.com/".into()),
            Some("pk-lf-1".into()),
            Some("sk-lf-2".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(langfuse.base, "https://us.cloud.langfuse.com");
        assert!(!format!("{langfuse:?}").contains("sk-lf-2"));
        let partial = read(Some("https://x".into()), None, Some("sk".into())).unwrap_err();
        assert!(partial.contains(PUBLIC_KEY_VARIABLE), "{partial}");
        assert!(read(Some("http://x".into()), Some("p".into()), Some("s".into())).is_err());
    }

    #[test]
    fn every_span_has_its_place_in_the_tree_and_the_trace_attributes() {
        let spans = spans(&trace(), "seed");
        let [root, find, first_turn, read_call, second_turn, structured] = spans.as_slice() else {
            panic!("{spans:#?}");
        };
        let trace_id = root.get("traceId").and_then(Value::as_str).unwrap();
        assert_eq!(trace_id.len(), 32);
        assert!(root.get("parentSpanId").is_none());
        for (child, parent) in [
            (find, root),
            (first_turn, find),
            (read_call, first_turn),
            (second_turn, find),
            (structured, second_turn),
        ] {
            assert_eq!(child.get("traceId").and_then(Value::as_str), Some(trace_id));
            assert_eq!(child.get("parentSpanId"), parent.get("spanId"));
        }
        for span in &spans {
            assert_eq!(string(span, "langfuse.session.id"), "o/r#12");
            assert_eq!(
                attribute(span, "langfuse.trace.tags"),
                Some(&json!({"arrayValue": {"values": [{"stringValue": "post"}]}}))
            );
        }
    }

    #[test]
    fn the_review_and_its_passes_carry_their_input_output_and_cost() {
        let spans = spans(&trace(), "seed");
        let [root, find, ..] = spans.as_slice() else {
            panic!("{spans:#?}");
        };
        assert_eq!(
            string(root, "langfuse.observation.input"),
            r#"{"diff":"d"}"#
        );
        assert_eq!(string(root, "langfuse.observation.output"), "approve");
        assert_eq!(string(find, "langfuse.observation.type"), "agent");
        assert_eq!(
            string(find, "langfuse.observation.input"),
            r#"{"head":"h"}"#
        );
        assert_eq!(
            string(find, "langfuse.observation.output"),
            r#"{"findings":[]}"#
        );
        assert_eq!(
            string(find, "langfuse.observation.metadata.cost_usd"),
            "1.5"
        );
        assert_eq!(
            string(find, "langfuse.observation.metadata.model"),
            "claude-opus-5-5"
        );
    }

    #[test]
    fn each_message_links_its_prompt_version_and_each_tool_call_its_result() {
        let spans = spans(&trace(), "seed");
        let [_, _, first_turn, read_call, second_turn, _] = spans.as_slice() else {
            panic!("{spans:#?}");
        };
        assert_eq!(
            string(first_turn, "langfuse.observation.type"),
            "generation"
        );
        assert_eq!(
            string(first_turn, "langfuse.observation.model.name"),
            "claude-opus-5-5"
        );
        assert_eq!(
            string(first_turn, "langfuse.observation.prompt.name"),
            "review-find"
        );
        assert_eq!(
            attribute(first_turn, "langfuse.observation.prompt.version"),
            Some(&json!({"intValue": "3"}))
        );
        let output: Value =
            serde_json::from_str(string(first_turn, "langfuse.observation.output")).unwrap();
        assert_eq!(output.pointer("/tool_calls/0/name"), Some(&json!("Read")));
        let usage: Value =
            serde_json::from_str(string(first_turn, "langfuse.observation.usage_details")).unwrap();
        assert_eq!(usage.get("cache_creation_input_tokens"), Some(&json!(4694)));
        assert_eq!(string(read_call, "langfuse.observation.type"), "tool");
        assert_eq!(read_call.get("name"), Some(&json!("Read")));
        assert!(string(read_call, "langfuse.observation.input").contains("main.rs"));
        assert!(string(read_call, "langfuse.observation.output").contains("v[3]"));
        assert_eq!(
            read_call.get("startTimeUnixNano"),
            Some(&json!("1800000003000000000"))
        );
        assert_eq!(
            read_call.get("endTimeUnixNano"),
            Some(&json!("1800000005000000000"))
        );
        assert_eq!(
            second_turn.get("startTimeUnixNano"),
            read_call.get("endTimeUnixNano")
        );
    }

    #[test]
    fn the_messages_costs_add_up_to_the_pass_s_reported_cost() {
        let spans = spans(&trace(), "seed");
        let costs: Vec<f64> = spans
            .iter()
            .filter_map(|span| attribute(span, "langfuse.observation.cost_details"))
            .filter_map(|value| value.get("stringValue")?.as_str())
            .filter_map(|cost| {
                serde_json::from_str::<Value>(cost)
                    .ok()?
                    .get("total")?
                    .as_f64()
            })
            .collect();
        assert_eq!(costs.len(), 2);
        assert!((costs.iter().sum::<f64>() - 1.5).abs() < 1e-9, "{costs:?}");
        for span in &spans {
            if let Some(usage) = attribute(span, "langfuse.observation.usage_details") {
                assert!(!usage.to_string().contains("thinking"), "{usage}");
            }
        }
    }

    #[test]
    fn cost_shares_follow_the_token_weights_and_sum_exactly() {
        let turn = |input: u64, output: u64, cache_read: u64, cache_creation: u64| {
            let mut turn = stream::read(
                r#"{"type":"assistant","message":{"id":"m","model":"x","content":[]}}"#,
                &[],
            )
            .turns
            .remove(0);
            turn.usage = stream::TurnUsage {
                input,
                output,
                cache_read,
                cache_creation,
                ..turn.usage
            };
            turn
        };
        assert_eq!(cost_shares(1000, &[]), Vec::<u64>::new());
        assert_eq!(
            cost_shares(1000, &[turn(0, 0, 0, 0), turn(0, 0, 0, 0)]),
            [0, 1000]
        );
        assert_eq!(
            cost_shares(600, &[turn(0, 10, 0, 0), turn(50, 0, 0, 0)]),
            [300, 300]
        );
        assert_eq!(
            cost_shares(7, &[turn(1, 0, 0, 0), turn(1, 0, 0, 0), turn(1, 0, 0, 0)]),
            [2, 2, 3]
        );
        assert_eq!(
            cost_shares(30, &[turn(0, 0, 20, 0), turn(0, 0, 0, 1)]),
            [15, 15]
        );
    }

    #[test]
    fn a_pass_that_did_not_answer_and_a_failed_or_missing_tool_result_are_marked() {
        let mut trace = trace();
        let pass = trace.passes.first_mut().unwrap();
        pass.outcome = "max_turns".to_string();
        let calls = &mut pass.transcript.tool_calls;
        calls.first_mut().unwrap().output.as_mut().unwrap().is_error = true;
        calls.get_mut(1).unwrap().output = None;
        let spans = spans(&trace, "seed");
        let level = |index: usize| {
            string(spans.get(index).unwrap(), "langfuse.observation.level").to_string()
        };
        assert_eq!(level(1), "WARNING");
        assert_eq!(level(3), "ERROR");
        assert_eq!(level(5), "WARNING");
    }

    #[test]
    fn a_long_trace_is_sent_in_parts_of_bounded_size() {
        let mut trace = trace();
        let pass = trace.passes.first().unwrap().clone();
        trace.passes = vec![pass; 40];
        let bodies = bodies(&trace, "seed");
        let counts: Vec<usize> = bodies
            .iter()
            .map(|body| {
                body.pointer("/resourceSpans/0/scopeSpans/0/spans")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len)
            })
            .collect();
        assert_eq!(counts, [100, 100, 1]);
    }

    #[test]
    fn export_posts_each_part_with_basic_auth_and_the_ingestion_version() {
        let (base, seen) = listener(vec![
            (200, "{}".to_string()),
            (200, r#"{"partialSuccess":{"rejectedSpans":1}}"#.to_string()),
            (401, "{}".to_string()),
        ]);
        let langfuse = langfuse(&base);
        let client = PlainClient::loopback();
        assert_eq!(export_with(&client, &langfuse, &trace(), "s"), Ok(()));
        assert!(
            export_with(&client, &langfuse, &trace(), "s")
                .unwrap_err()
                .contains("rejected 1 spans of the trace (part 1 of 1)")
        );
        assert!(
            export_with(&client, &langfuse, &trace(), "s")
                .unwrap_err()
                .contains("HTTP 401")
        );
        let requests = seen.recv().unwrap();
        let raw = requests.first().unwrap();
        let lower = raw.to_ascii_lowercase();
        assert!(
            raw.starts_with("POST /api/public/otel/v1/traces HTTP/1.1\r\n"),
            "{raw}"
        );
        assert!(lower.contains("\r\nauthorization: basic cgs6c2s=\r\n"));
        assert!(lower.contains("\r\nx-langfuse-ingestion-version: 4\r\n"));
        assert!(lower.contains("\r\ncontent-type: application/json\r\n"));
    }
}
