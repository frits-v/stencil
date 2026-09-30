// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::g7_value;
use stencil_model::page_schema;

const COMMITTED_SCHEMA: &str = include_str!("../../../schema/stencil.schema.json");

fn validator() -> jsonschema::Validator {
    let schema = serde_json::to_value(page_schema()).unwrap();
    jsonschema::validator_for(&schema).expect("the generated schema compiles")
}

#[test]
fn page_schema_equals_the_committed_file() {
    let generated = serde_json::to_string_pretty(&page_schema()).unwrap() + "\n";
    assert!(
        generated == COMMITTED_SCHEMA,
        "schema/stencil.schema.json is out of date; regenerate it from page_schema() as section 8.3 describes"
    );
}

#[test]
fn g7_validates() {
    let validator = validator();
    let g7 = g7_value();
    let errors: Vec<String> = validator
        .iter_errors(&g7)
        .map(|error| error.to_string())
        .collect();
    assert_eq!(errors, Vec::<String>::new());
}

#[test]
fn unknown_zone_field_fails() {
    let mut document = g7_value();
    document["body"][0]["children"][2]["colour"] = serde_json::json!("blue");
    assert!(!validator().is_valid(&document));
}

#[test]
fn missing_tag_fails() {
    let mut document = g7_value();
    let removed = document["body"][0]["children"][2]
        .as_object_mut()
        .unwrap()
        .remove("tag");
    assert!(removed.is_some());
    assert!(!validator().is_valid(&document));
}

#[test]
fn tag_is_accepted_on_every_node_kind() {
    let document = serde_json::json!({
        "title": "t", "kicker": "k", "lede": "l", "canvas": "internal",
        "legend": [ { "kind": "blue", "text": "b" } ],
        "body": [
            { "tag": "Row", "children": [ { "tag": "Fact", "text": "f" } ] },
            { "tag": "Col", "children": [ { "tag": "Note", "kind": "h1", "text": "n" } ] },
            { "tag": "Zone", "kind": "vpc", "label": "z", "children": [ { "tag": "Pcard", "fn": "f" } ] },
            { "tag": "Pipe", "dir": "v", "kind": "blue", "label": "p" },
            { "tag": "Tee", "kind": "blue", "hub": "h", "arms": [
                { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "a" },
                { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "b" }
            ] }
        ]
    });
    assert!(validator().is_valid(&document));
}

#[test]
fn null_optional_fields_validate() {
    let mut document = g7_value();
    document["foot"] = serde_json::Value::Null;
    document["body"][0]["gap"] = serde_json::Value::Null;
    document["body"][0]["justify"] = serde_json::Value::Null;
    document["body"][0]["children"][0]["children"][0]["children"][0]["pn"] =
        serde_json::Value::Null;
    document["body"][0]["children"][0]["children"][0]["children"][0]["icon"] =
        serde_json::Value::Null;
    assert!(validator().is_valid(&document));
}

#[test]
fn null_width_fails() {
    let mut document = g7_value();
    document["width"] = serde_json::Value::Null;
    assert!(!validator().is_valid(&document));
}

#[test]
fn tee_arm_tagged_zone_fails() {
    let mut document = g7_value();
    document["body"][0] = serde_json::json!({ "tag": "Tee", "kind": "blue", "hub": "h", "arms": [
        { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "a" },
        { "tag": "Zone", "kind": "vpc", "label": "z", "children": [ { "tag": "Fact", "text": "f" } ] }
    ] });
    assert!(!validator().is_valid(&document));
}
