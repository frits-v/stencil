//! The review's prompt templates in Langfuse's prompt management. Each run looks up the
//! latest version of a template and, when its text or configuration differs from what the
//! run uses, creates a new version; the run's generations link to that version. The
//! repository stays the source of the templates.

use serde_json::{Value, json};

use super::langfuse::{Langfuse, PromptLink};
use crate::github_api::{Credential, Method, PlainClient, Reply};

const PROMPTS_PATH: &str = "/api/public/v2/prompts";

/// A template as the run uses it: its text with `{{name}}` variables, and the model,
/// limits and output schema it runs with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub name: &'static str,
    pub text: &'static str,
    pub config: Value,
}

#[derive(Debug, PartialEq, Eq)]
enum Stored {
    Same(u64),
    Different,
    Absent,
}

fn decode_lookup(status: u16, body: &str, template: &Template) -> Result<Stored, String> {
    match status {
        404 => Ok(Stored::Absent),
        200 => {
            let stored: Value = serde_json::from_str(body).map_err(|_| {
                format!(
                    "Langfuse answered the lookup of {} with no JSON",
                    template.name
                )
            })?;
            let version = stored.get("version").and_then(Value::as_u64);
            let text = stored.get("prompt").and_then(Value::as_str);
            let (Some(version), Some(text)) = (version, text) else {
                return Err(format!(
                    "Langfuse answered the lookup of {} without a text prompt and its version",
                    template.name
                ));
            };
            let config = stored.get("config").unwrap_or(&Value::Null);
            if text == template.text && *config == template.config {
                Ok(Stored::Same(version))
            } else {
                Ok(Stored::Different)
            }
        }
        other => Err(format!(
            "Langfuse answered the lookup of {} with HTTP {other}",
            template.name
        )),
    }
}

fn decode_created(status: u16, body: &str, name: &str) -> Result<u64, String> {
    if !(200..300).contains(&status) {
        return Err(format!(
            "Langfuse refused a new version of {name} with HTTP {status}"
        ));
    }
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|created| created.get("version")?.as_u64())
        .ok_or_else(|| format!("Langfuse created a version of {name} without saying which"))
}

fn sent(reply: Reply, what: &str) -> Result<(u16, String), String> {
    match reply {
        Reply::Response { status, body } => Ok((status, body)),
        Reply::BodyTooLarge => Err(format!("the Langfuse answer to {what} is over its cap")),
        Reply::Failed(reason) => Err(format!("{what}: {reason}")),
    }
}

/// The version of `template` the run's generations link to, created when the latest one
/// holds other text or configuration.
pub fn register(langfuse: &Langfuse, template: &Template, run: &str) -> Result<PromptLink, String> {
    register_with(&PlainClient::https(), langfuse, template, run)
}

fn register_with(
    client: &PlainClient,
    langfuse: &Langfuse,
    template: &Template,
    run: &str,
) -> Result<PromptLink, String> {
    let credential = || Credential::Basic(&langfuse.basic);
    let lookup = format!(
        "{}{PROMPTS_PATH}/{}?label=latest",
        langfuse.base, template.name
    );
    let what = format!("looking up prompt {}", template.name);
    let (status, body) = sent(
        client.request_with(Method::Get, &lookup, credential(), &[], None),
        &what,
    )?;
    let version = match decode_lookup(status, &body, template)? {
        Stored::Same(version) => version,
        Stored::Different | Stored::Absent => {
            let created = json!({
                "type": "text",
                "name": template.name,
                "prompt": template.text,
                "config": template.config,
                "labels": [],
                "commitMessage": format!("registered by xtask review ({run} run)"),
            });
            let what = format!("creating a version of prompt {}", template.name);
            let (status, body) = sent(
                client.request_with(
                    Method::Post,
                    &format!("{}{PROMPTS_PATH}", langfuse.base),
                    credential(),
                    &[],
                    Some(&created.to_string()),
                ),
                &what,
            )?;
            decode_created(status, &body, template.name)?
        }
    };
    Ok(PromptLink {
        name: template.name.to_string(),
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github_api::tests::listener;
    use crate::review::langfuse::tests::langfuse;

    fn template() -> Template {
        Template {
            name: "review-find",
            text: "Review {{diff}}.",
            config: json!({"model": "opus", "max_turns": 80}),
        }
    }

    fn stored(text: &str, version: u64) -> String {
        json!({"type": "text", "name": "review-find", "version": version, "prompt": text,
               "config": {"model": "opus", "max_turns": 80}, "labels": ["latest"]})
        .to_string()
    }

    fn body_of(raw: &str) -> Value {
        serde_json::from_str(raw.split("\r\n\r\n").nth(1).unwrap()).unwrap()
    }

    #[test]
    fn the_latest_version_is_linked_when_it_holds_the_same_text_and_configuration() {
        let (base, seen) = listener(vec![(200, stored("Review {{diff}}.", 3))]);
        let link = register_with(
            &PlainClient::loopback(),
            &langfuse(&base),
            &template(),
            "post",
        );
        assert_eq!(
            link,
            Ok(PromptLink {
                name: "review-find".to_string(),
                version: 3
            })
        );
        let requests = seen.recv().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(
            requests
                .first()
                .unwrap()
                .starts_with("GET /api/public/v2/prompts/review-find?label=latest HTTP/1.1\r\n")
        );
    }

    #[test]
    fn other_text_other_configuration_or_no_prompt_creates_a_version() {
        let mut other_config =
            serde_json::from_str::<Value>(&stored("Review {{diff}}.", 3)).unwrap();
        other_config["config"] = json!({"model": "opus", "max_turns": 30});
        for lookup in [
            (200, stored("Review the {{diff}}.", 3)),
            (200, other_config.to_string()),
            (404, r#"{"message":"Prompt not found"}"#.to_string()),
        ] {
            let (base, seen) = listener(vec![lookup, (201, r#"{"version":4}"#.to_string())]);
            let link = register_with(
                &PlainClient::loopback(),
                &langfuse(&base),
                &template(),
                "dry-run",
            );
            assert_eq!(link.map(|link| link.version), Ok(4));
            let requests = seen.recv().unwrap();
            let create = requests.get(1).unwrap();
            assert!(
                create.starts_with("POST /api/public/v2/prompts HTTP/1.1\r\n"),
                "{create}"
            );
            let body = body_of(create);
            assert_eq!(body.get("type"), Some(&json!("text")));
            assert_eq!(body.get("prompt"), Some(&json!("Review {{diff}}.")));
            assert_eq!(body.get("config"), Some(&template().config));
            assert_eq!(
                body.get("commitMessage"),
                Some(&json!("registered by xtask review (dry-run run)"))
            );
        }
    }

    #[test]
    fn a_refusal_or_an_answer_it_cannot_read_is_an_error() {
        for replies in [
            vec![(500, "{}".to_string())],
            vec![(200, "not json".to_string())],
            vec![(
                200,
                r#"{"type":"chat","version":2,"prompt":[]}"#.to_string(),
            )],
            vec![(404, "{}".to_string()), (400, "{}".to_string())],
            vec![(404, "{}".to_string()), (201, "{}".to_string())],
        ] {
            let (base, _seen) = listener(replies.clone());
            let refused = register_with(
                &PlainClient::loopback(),
                &langfuse(&base),
                &template(),
                "post",
            );
            assert!(refused.is_err(), "{replies:?}: {refused:?}");
        }
        let unreachable = register_with(
            &PlainClient::loopback(),
            &langfuse("http://127.0.0.1:1"),
            &template(),
            "post",
        );
        assert!(
            unreachable
                .unwrap_err()
                .starts_with("looking up prompt review-find: ")
        );
    }
}
