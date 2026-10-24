//! The event stream of `claude -p --output-format stream-json --verbose`: the assistant's turns
//! with their summarized thinking, text and tool calls, each tool call paired with its result,
//! and the final result line, which `claude_run::decode` reads.

use std::time::Duration;

use serde_json::Value;

/// How much of a tool's result the trace keeps; the full size is recorded beside it.
pub const TOOL_OUTPUT_KEPT_BYTES: usize = 16 * 1024;
/// Unrecognised events are counted in full and named up to this many.
const UNRECOGNISED_NAMED_MAX: usize = 32;
const NAME_MAX_CHARS: usize = 80;

/// A thinking block: the summary the CLI was asked for, or none when the server omitted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thinking {
    Summary(String),
    Omitted,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TurnUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
    /// The CLI's last running estimate of the thinking tokens before this turn.
    pub thinking_estimate: Option<u64>,
    /// Whether `output` is the message's final count, from its `message_delta` event, rather
    /// than the count at its first content block.
    pub output_final: bool,
}

/// One assistant message. The CLI writes one event per content block, each repeating the
/// message's id and usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub message_id: String,
    pub model: String,
    pub usage: TurnUsage,
    pub thinking: Vec<Thinking>,
    pub text: Vec<String>,
    /// Indexes into `Transcript::tool_calls`, in the order the turn made them.
    pub tool_calls: Vec<usize>,
    /// When the turn's first and last events arrived, from the start of the run.
    pub first_seen: Option<Duration>,
    pub last_seen: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    /// The first `TOOL_OUTPUT_KEPT_BYTES` of the result, cut at a character boundary.
    pub kept: String,
    pub bytes: usize,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// The tool's input as JSON.
    pub input: String,
    pub output: Option<ToolOutput>,
    pub called: Option<Duration>,
    pub answered: Option<Duration>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transcript {
    /// The model the CLI resolved the alias to, from its init event.
    pub model: Option<String>,
    pub turns: Vec<Turn>,
    pub tool_calls: Vec<ToolCall>,
    pub unrecognised: Vec<String>,
    pub unrecognised_count: usize,
    pub result: Option<String>,
}

impl Transcript {
    fn unrecognised(&mut self, what: &str) {
        self.unrecognised_count = self.unrecognised_count.saturating_add(1);
        if self.unrecognised.len() < UNRECOGNISED_NAMED_MAX {
            self.unrecognised
                .push(what.chars().take(NAME_MAX_CHARS).collect());
        }
    }

    fn turn_for(
        &mut self,
        message: &Value,
        seen: Option<Duration>,
        estimate: Option<u64>,
    ) -> usize {
        let id = message.get("id").and_then(Value::as_str).unwrap_or("");
        if let Some(index) = self.turns.iter().rposition(|turn| turn.message_id == id) {
            if let Some(turn) = self.turns.get_mut(index) {
                turn.last_seen = seen.or(turn.last_seen);
            }
            return index;
        }
        let count = |key: &str| {
            message
                .pointer(&format!("/usage/{key}"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        self.turns.push(Turn {
            message_id: id.to_string(),
            model: message
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            usage: TurnUsage {
                input: count("input_tokens"),
                output: count("output_tokens"),
                cache_read: count("cache_read_input_tokens"),
                cache_creation: count("cache_creation_input_tokens"),
                thinking_estimate: estimate,
                output_final: false,
            },
            thinking: Vec::new(),
            text: Vec::new(),
            tool_calls: Vec::new(),
            first_seen: seen,
            last_seen: seen,
        });
        self.turns.len().saturating_sub(1)
    }

    fn assistant(&mut self, event: &Value, seen: Option<Duration>, estimate: Option<u64>) {
        let Some(message) = event.get("message") else {
            self.unrecognised("an assistant event without a message");
            return;
        };
        let turn = self.turn_for(message, seen, estimate);
        let blocks = message
            .get("content")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for block in blocks {
            let kind = block.get("type").and_then(Value::as_str).unwrap_or("");
            let text_of = |key: &str| block.get(key).and_then(Value::as_str).unwrap_or("");
            match kind {
                "thinking" => {
                    let thinking = match text_of("thinking") {
                        "" => Thinking::Omitted,
                        summary => Thinking::Summary(summary.to_string()),
                    };
                    if let Some(turn) = self.turns.get_mut(turn) {
                        turn.thinking.push(thinking);
                    }
                }
                "redacted_thinking" => {
                    if let Some(turn) = self.turns.get_mut(turn) {
                        turn.thinking.push(Thinking::Omitted);
                    }
                }
                "text" => {
                    let text = text_of("text").to_string();
                    if let Some(turn) = self.turns.get_mut(turn) {
                        turn.text.push(text);
                    }
                }
                "tool_use" => {
                    self.tool_calls.push(ToolCall {
                        id: text_of("id").to_string(),
                        name: text_of("name").to_string(),
                        input: block
                            .get("input")
                            .map_or_else(String::new, Value::to_string),
                        output: None,
                        called: seen,
                        answered: None,
                    });
                    let index = self.tool_calls.len().saturating_sub(1);
                    if let Some(turn) = self.turns.get_mut(turn) {
                        turn.tool_calls.push(index);
                    }
                }
                other => self.unrecognised(&format!("assistant block {other}")),
            }
        }
    }

    fn user(&mut self, event: &Value, seen: Option<Duration>) {
        let Some(blocks) = event.pointer("/message/content").and_then(Value::as_array) else {
            self.unrecognised("a user event without content blocks");
            return;
        };
        for block in blocks {
            let kind = block.get("type").and_then(Value::as_str).unwrap_or("");
            if kind != "tool_result" {
                self.unrecognised(&format!("user block {kind}"));
                continue;
            }
            let id = block
                .get("tool_use_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let Some(call) = self.tool_calls.iter_mut().rev().find(|call| call.id == id) else {
                self.unrecognised(&format!("a result for unknown tool call {id}"));
                continue;
            };
            let text = result_text(block.get("content"));
            call.output = Some(ToolOutput {
                kept: kept_prefix(&text, TOOL_OUTPUT_KEPT_BYTES).to_string(),
                bytes: text.len(),
                is_error: block.get("is_error").and_then(Value::as_bool) == Some(true),
            });
            call.answered = seen;
        }
    }
}

/// A tool result's content: a string, or text blocks joined by newlines, with any other block
/// named in brackets.
fn result_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .map(|block| match block.get("type").and_then(Value::as_str) {
                Some("text") => block
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                Some(other) => format!("[{other}]"),
                None => "[block without a type]".to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// The longest prefix of `text` within `max` bytes that ends at a character boundary.
pub fn kept_prefix(text: &str, max: usize) -> &str {
    let end = (0..=max.min(text.len()))
        .rev()
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(0);
    text.get(..end).unwrap_or("")
}

/// What the partial-message events of `--include-partial-messages` add: each message's final
/// output count, which only its `message_delta` carries.
#[derive(Debug, Default)]
struct Partials {
    current: Option<String>,
    final_outputs: Vec<(String, u64)>,
}

impl Partials {
    fn event(&mut self, event: &Value, transcript: &mut Transcript) {
        let inner = event.get("event").unwrap_or(&Value::Null);
        match inner.get("type").and_then(Value::as_str).unwrap_or("") {
            "message_start" => {
                self.current = inner
                    .pointer("/message/id")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            "message_delta" => {
                let output = inner
                    .pointer("/usage/output_tokens")
                    .and_then(Value::as_u64);
                if let (Some(id), Some(output)) = (self.current.clone(), output) {
                    self.final_outputs.push((id, output));
                }
            }
            "content_block_start"
            | "content_block_delta"
            | "content_block_stop"
            | "message_stop" => {}
            other => transcript.unrecognised(&format!("stream event {other}")),
        }
    }

    fn apply(self, transcript: &mut Transcript) {
        for (id, output) in self.final_outputs {
            if let Some(turn) = transcript
                .turns
                .iter_mut()
                .find(|turn| turn.message_id == id)
            {
                turn.usage.output = output;
                turn.usage.output_final = true;
            }
        }
    }
}

/// Reads the stream; `line_ends` holds when each line arrived, from the start of the run.
pub fn read(stdout: &str, line_ends: &[Duration]) -> Transcript {
    let mut transcript = Transcript::default();
    let mut thinking_estimate = None;
    let mut partials = Partials::default();
    for (index, line) in stdout.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let seen = line_ends.get(index).copied();
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            transcript.unrecognised(&format!("line {} is not JSON", index.saturating_add(1)));
            continue;
        };
        let kind = event.get("type").and_then(Value::as_str).unwrap_or("");
        let subtype = event.get("subtype").and_then(Value::as_str).unwrap_or("");
        match (kind, subtype) {
            ("system", "init") => {
                transcript.model = event
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            ("system", "thinking_tokens") => {
                thinking_estimate = event.get("estimated_tokens").and_then(Value::as_u64);
            }
            ("system", "status") | ("rate_limit_event", _) => {}
            ("stream_event", _) => partials.event(&event, &mut transcript),
            ("assistant", _) => {
                transcript.assistant(&event, seen, thinking_estimate.take());
            }
            ("user", _) => transcript.user(&event, seen),
            ("result", _) => transcript.result = Some(line.to_string()),
            (kind, subtype) => transcript.unrecognised(&format!("event {kind}:{subtype}")),
        }
    }
    partials.apply(&mut transcript);
    transcript
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS: &str = include_str!("../../fixtures/review/claude-stream-success.jsonl");
    const MAX_TURNS: &str = include_str!("../../fixtures/review/claude-stream-max-turns.jsonl");
    const THINKING: &str = include_str!("../../fixtures/review/claude-stream-thinking.jsonl");
    const PARTIAL: &str = include_str!("../../fixtures/review/claude-stream-partial.jsonl");

    fn seconds(count: u64) -> Vec<Duration> {
        (0..count).map(Duration::from_secs).collect()
    }

    #[test]
    fn a_reviewing_run_pairs_each_tool_call_with_its_result_and_keeps_the_result_line() {
        let transcript = read(SUCCESS, &seconds(7));
        assert_eq!(transcript.model.as_deref(), Some("claude-opus-5-5"));
        assert_eq!(transcript.turns.len(), 2);
        let names: Vec<&str> = transcript
            .tool_calls
            .iter()
            .map(|call| call.name.as_str())
            .collect();
        assert_eq!(names, ["Read", "StructuredOutput"]);
        let read_call = transcript.tool_calls.first().unwrap();
        assert!(
            read_call.input.contains("/tmp/review-files/main.rs"),
            "{}",
            read_call.input
        );
        let output = read_call.output.as_ref().unwrap();
        assert!(output.kept.contains("v[3]"));
        assert_eq!(output.bytes, output.kept.len());
        assert!(!output.is_error);
        assert_eq!(read_call.called, Some(Duration::from_secs(1)));
        assert_eq!(read_call.answered, Some(Duration::from_secs(3)));
        let first = transcript.turns.first().unwrap();
        assert_eq!(first.tool_calls, [0]);
        assert_eq!(first.usage.cache_creation, 4694);
        assert!(
            transcript
                .result
                .as_deref()
                .unwrap()
                .contains("\"structured_output\"")
        );
        assert_eq!(
            transcript.unrecognised_count, 0,
            "{:?}",
            transcript.unrecognised
        );
    }

    #[test]
    fn each_message_takes_its_final_output_count_from_its_message_delta() {
        let transcript = read(PARTIAL, &[]);
        assert_eq!(
            transcript.unrecognised_count, 0,
            "{:?}",
            transcript.unrecognised
        );
        let outputs: Vec<(u64, bool)> = transcript
            .turns
            .iter()
            .map(|turn| (turn.usage.output, turn.usage.output_final))
            .collect();
        assert_eq!(outputs, [(119, true), (132, true)]);
        let result: Value = serde_json::from_str(transcript.result.as_deref().unwrap()).unwrap();
        let total: u64 = outputs.iter().map(|(output, _)| output).sum();
        assert_eq!(
            result.pointer("/usage/output_tokens"),
            Some(&Value::from(total))
        );
        let first = transcript.tool_calls.first().unwrap();
        assert_eq!(first.input, "{\"file_path\":\"/tmp/review-files/main.rs\"}");
        let without_partials = read(SUCCESS, &[]);
        assert!(
            without_partials
                .turns
                .iter()
                .all(|turn| !turn.usage.output_final)
        );
    }

    #[test]
    fn a_turn_written_as_several_events_is_one_turn_with_its_summary() {
        let transcript = read(THINKING, &seconds(28));
        assert_eq!(transcript.turns.len(), 1);
        let turn = transcript.turns.first().unwrap();
        assert_eq!(turn.text.len(), 1);
        let Some(Thinking::Summary(summary)) = turn.thinking.first() else {
            panic!("{:?}", turn.thinking);
        };
        assert!(summary.contains("Pell"), "{summary}");
        assert!(
            turn.usage
                .thinking_estimate
                .is_some_and(|tokens| tokens > 0)
        );
        assert!(turn.first_seen < turn.last_seen);
        assert_eq!(
            transcript.unrecognised_count, 0,
            "{:?}",
            transcript.unrecognised
        );
    }

    #[test]
    fn a_run_stopped_at_its_turn_limit_still_has_its_result_line() {
        let transcript = read(MAX_TURNS, &[]);
        assert_eq!(transcript.tool_calls.len(), 1);
        assert!(transcript.tool_calls.first().unwrap().called.is_none());
        assert!(
            transcript
                .result
                .as_deref()
                .unwrap()
                .contains("error_max_turns")
        );
    }

    #[test]
    fn omitted_thinking_and_every_unrecognised_event_are_recorded_not_dropped() {
        let stream = [
            r#"{"type":"assistant","message":{"id":"m1","model":"x","content":[{"type":"thinking","thinking":"","signature":"s"},{"type":"redacted_thinking","data":"d"},{"type":"image"}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"nobody","content":"x"},{"type":"text","text":"hi"}]}}"#,
            r#"{"type":"user","message":{"content":"plain"}}"#,
            r#"{"type":"stream_event","event":{"type":"ping"}}"#,
            r#"{"type":"system","subtype":"compact_boundary"}"#,
            "not json",
            r#"{"type":"assistant"}"#,
        ]
        .join("\n");
        let transcript = read(&stream, &[]);
        let turn = transcript.turns.first().unwrap();
        assert_eq!(turn.thinking, [Thinking::Omitted, Thinking::Omitted]);
        assert_eq!(
            transcript.unrecognised,
            [
                "assistant block image",
                "a result for unknown tool call nobody",
                "user block text",
                "a user event without content blocks",
                "stream event ping",
                "event system:compact_boundary",
                "line 6 is not JSON",
                "an assistant event without a message",
            ]
        );
        assert!(transcript.result.is_none());
    }

    #[test]
    fn a_long_tool_result_keeps_its_prefix_and_its_true_size() {
        let long = "é".repeat(TOOL_OUTPUT_KEPT_BYTES);
        let stream = [
            r#"{"type":"assistant","message":{"id":"m1","model":"x","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/a"}}]}}"#.to_string(),
            serde_json::json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":true,"content":[{"type":"text","text":long},{"type":"image"}]}]}}).to_string(),
        ]
        .join("\n");
        let transcript = read(&stream, &[]);
        let output = transcript
            .tool_calls
            .first()
            .unwrap()
            .output
            .as_ref()
            .unwrap();
        assert_eq!(output.bytes, long.len().saturating_add("\n[image]".len()));
        assert_eq!(output.kept.len(), TOOL_OUTPUT_KEPT_BYTES);
        assert!(output.is_error);
        assert_eq!(kept_prefix("aé", 2), "a");
    }

    #[test]
    fn unrecognised_events_are_all_counted_and_named_up_to_the_bound() {
        let stream =
            vec![r#"{"type":"mystery"}"#; UNRECOGNISED_NAMED_MAX.saturating_add(5)].join("\n");
        let transcript = read(&stream, &[]);
        assert_eq!(transcript.unrecognised.len(), UNRECOGNISED_NAMED_MAX);
        assert_eq!(
            transcript.unrecognised_count,
            UNRECOGNISED_NAMED_MAX.saturating_add(5)
        );
    }
}
