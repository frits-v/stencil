//! The examples are figures for readers, not fixtures for tests: their copy describes the
//! system, never the test it serves.

use std::fs;
use std::path::Path;

use serde_json::Value;

/// Words that name a fixture or a test case rather than a system.
const FIXTURE_WORDS: [&str; 7] = [
    "fixture",
    "example",
    "covers",
    "narrow-zone",
    "long-name",
    "test figure",
    "two narrow regions",
];

fn copy_fields(node: &Value, out: &mut Vec<(String, String)>) {
    match node {
        Value::Object(map) => {
            for field in [
                "title", "kicker", "lede", "subtitle", "label", "hub", "text",
            ] {
                if let Some(Value::String(text)) = map.get(field) {
                    out.push((field.to_string(), text.clone()));
                }
            }
            for value in map.values() {
                copy_fields(value, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                copy_fields(item, out);
            }
        }
        _ => {}
    }
}

#[test]
fn no_example_copy_names_a_fixture() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut examined = 0;
    for entry in fs::read_dir(&examples).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let document: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let mut fields = Vec::new();
        copy_fields(&document, &mut fields);
        assert!(!fields.is_empty(), "{}: no copy fields", path.display());
        for (field, text) in &fields {
            let lower = text.to_lowercase();
            for word in FIXTURE_WORDS {
                assert!(
                    !lower.contains(word),
                    "{}: {field} \"{text}\" names a fixture ({word})",
                    path.display()
                );
            }
            examined += 1;
        }
    }
    assert!(examined > 0);
}
