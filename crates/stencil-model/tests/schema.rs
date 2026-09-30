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

fn page_with_blocks() -> serde_json::Value {
    serde_json::json!({
        "title": "t", "kicker": "k", "lede": "l", "canvas": "internal", "theme": "dusk",
        "legend": [ { "kind": "blue", "text": "b" } ],
        "body": [
            { "tag": "Text", "id": "goals", "heading": "Goals", "body": ["a", "b"], "list": "numbered" },
            { "tag": "Callout", "id": "risk", "kind": "risk", "title": "Risk", "text": "c" },
            { "tag": "Frame", "label": "Screen", "height": 240 },
            { "tag": "Pipe", "id": "hop", "dir": "h", "kind": "blue", "label": "p", "arrow": "end" }
        ],
        "links": [ {
            "from": "goals", "to": "risk", "kind": "blue", "label": "1", "sub": "s",
            "arrow": "both", "from_side": "bottom", "to_side": "top",
            "via": [ { "x": 10, "y": 20.5 } ]
        } ]
    })
}

#[test]
fn section_11_fields_and_tags_validate() {
    let errors: Vec<String> = validator()
        .iter_errors(&page_with_blocks())
        .map(|error| error.to_string())
        .collect();
    assert_eq!(errors, Vec::<String>::new());
}

#[test]
fn malformed_id_fails_schema_validation() {
    for id in ["Goals", "-goals", "", "goals_1"] {
        let mut document = page_with_blocks();
        document["body"][0]["id"] = serde_json::json!(id);
        assert!(!validator().is_valid(&document), "{id:?}");
    }
    let mut document = page_with_blocks();
    document["body"][0]["id"] = serde_json::json!(format!("a{}", "-".repeat(63)));
    assert!(validator().is_valid(&document));
    document["body"][0]["id"] = serde_json::json!(format!("a{}", "-".repeat(64)));
    assert!(!validator().is_valid(&document));
}

#[test]
fn section_11_limits_and_enums_fail_schema_validation() {
    let edits: [(&str, serde_json::Value); 7] = [
        ("/theme", serde_json::json!("night")),
        ("/body/0/list", serde_json::json!("lettered")),
        ("/body/0/body", serde_json::json!([])),
        ("/body/1/kind", serde_json::json!("warning")),
        ("/body/2/height", serde_json::json!(39)),
        ("/body/3/arrow", serde_json::json!("tail")),
        ("/links/0/to_side", serde_json::json!("north")),
    ];
    for (pointer, value) in edits {
        let mut document = page_with_blocks();
        *document.pointer_mut(pointer).unwrap() = value;
        assert!(!validator().is_valid(&document), "{pointer}");
    }
    let mut document = page_with_blocks();
    document["links"][0]["via"] = serde_json::json!(vec![serde_json::json!({ "x": 1, "y": 1 }); 9]);
    assert!(!validator().is_valid(&document));
    let mut document = page_with_blocks();
    document["body"][0]["body"] = serde_json::json!(vec!["line"; 65]);
    assert!(!validator().is_valid(&document));
    for line in [String::new(), "x".repeat(401)] {
        let mut document = page_with_blocks();
        document["body"][0]["body"] = serde_json::json!(["first", line]);
        assert!(!validator().is_valid(&document));
    }
    let mut document = page_with_blocks();
    document["body"][0]["body"] = serde_json::json!(["first", "x".repeat(400)]);
    assert!(validator().is_valid(&document));
}

#[test]
fn onepager_validates() {
    let onepager: serde_json::Value =
        serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    let errors: Vec<String> = validator()
        .iter_errors(&onepager)
        .map(|error| error.to_string())
        .collect();
    assert_eq!(errors, Vec::<String>::new());
}
