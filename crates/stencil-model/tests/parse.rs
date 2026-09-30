// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{G7_JSON, g7_page, g7_value};
use stencil_model::{ModelError, Node, PAGE_WIDTH_DEFAULT, Pcard, parse_page};

fn assert_json_error(json_text: &str) {
    match parse_page(json_text) {
        Err(ModelError::Json { .. }) => {}
        other => panic!("expected ModelError::Json for {json_text}, got {other:?}"),
    }
}

/// g7 with the first body node replaced by `node`, as JSON text.
fn g7_with_first_node(node: serde_json::Value) -> String {
    let mut document = g7_value();
    document["body"][0] = node;
    document.to_string()
}

#[test]
fn g7_parses_and_round_trips() {
    let page = parse_page(G7_JSON).expect("g7 is valid");
    let serialized = serde_json::to_string(&page).unwrap();
    let reparsed = parse_page(&serialized).unwrap();
    assert_eq!(page, reparsed);
    assert_eq!(page, g7_page());
}

#[test]
fn width_defaults_to_1280_when_absent() {
    let page = parse_page(G7_JSON).unwrap();
    assert!(g7_value().get("width").is_none());
    assert_eq!(page.width, PAGE_WIDTH_DEFAULT);
    assert_eq!(page.width, 1280);
}

#[test]
fn explicit_width_is_kept() {
    let mut document = g7_value();
    document["width"] = serde_json::json!(1600);
    let page = parse_page(&document.to_string()).unwrap();
    assert_eq!(page.width, 1600);
}

#[test]
fn unknown_field_on_zone_is_rejected() {
    let mut document = g7_value();
    document["body"][0]["children"][2]["colour"] = serde_json::json!("blue");
    assert_json_error(&document.to_string());
}

#[test]
fn unknown_field_on_pipe_is_rejected() {
    assert_json_error(&g7_with_first_node(serde_json::json!(
        { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "a", "weight": 2 }
    )));
}

#[test]
fn unknown_field_on_page_is_rejected() {
    let mut document = g7_value();
    document["subtitle"] = serde_json::json!("extra");
    assert_json_error(&document.to_string());
}

#[test]
fn unknown_tag_is_rejected() {
    assert_json_error(&g7_with_first_node(serde_json::json!(
        { "tag": "Box", "children": [] }
    )));
}

#[test]
fn unknown_icon_is_rejected() {
    assert_json_error(&g7_with_first_node(serde_json::json!(
        { "tag": "Pcard", "icon": "bigtable", "fn": "Store" }
    )));
}

#[test]
fn unknown_zone_kind_is_rejected() {
    assert_json_error(&g7_with_first_node(serde_json::json!(
        { "tag": "Zone", "kind": "region-c", "label": "Region C", "children": [ { "tag": "Fact", "text": "a" } ] }
    )));
}

#[test]
fn pipe_without_kind_is_rejected() {
    assert_json_error(&g7_with_first_node(serde_json::json!(
        { "tag": "Pipe", "dir": "h", "label": "a" }
    )));
}

fn tee_json(arms: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "tag": "Tee", "kind": "blue", "hub": "hub", "arms": arms })
}

fn arm_json(tag: &str) -> serde_json::Value {
    serde_json::json!({ "tag": tag, "dir": "h", "kind": "blue", "label": "arm" })
}

#[test]
fn tee_with_two_pipe_arms_parses() {
    let json_text = g7_with_first_node(tee_json(serde_json::json!([
        arm_json("Pipe"),
        arm_json("Pipe")
    ])));
    let page = parse_page(&json_text).expect("a Tee with two Pipe arms is valid");
    assert!(matches!(page.body.first(), Some(Node::Tee(_))));
}

#[test]
fn tee_with_three_arms_is_rejected() {
    assert_json_error(&g7_with_first_node(tee_json(serde_json::json!([
        arm_json("Pipe"),
        arm_json("Pipe"),
        arm_json("Pipe")
    ]))));
}

#[test]
fn tee_with_one_arm_is_rejected() {
    assert_json_error(&g7_with_first_node(tee_json(serde_json::json!([
        arm_json("Pipe")
    ]))));
}

#[test]
fn tee_arm_tagged_zone_is_rejected() {
    assert_json_error(&g7_with_first_node(tee_json(serde_json::json!([
        arm_json("Pipe"),
        { "tag": "Zone", "kind": "vpc", "label": "VPC", "children": [ { "tag": "Fact", "text": "a" } ] }
    ]))));
}

#[test]
fn null_optional_fields_parse_as_absent() {
    let json_text = g7_with_first_node(serde_json::json!(
        { "tag": "Row", "gap": null, "grow": null, "justify": null, "children": [
            { "tag": "Pcard", "icon": null, "fn": "Store", "pn": null, "fact": null, "ask": null }
        ] }
    ));
    let page = parse_page(&json_text).unwrap();
    let Some(Node::Row(row)) = page.body.first() else {
        panic!("first body node is a Row");
    };
    assert_eq!(
        (row.gap, row.grow.as_ref(), row.justify),
        (None, None, None)
    );
    assert_eq!(
        row.children.first(),
        Some(&Node::Pcard(Pcard {
            icon: None,
            function_name: "Store".to_string(),
            product_name: None,
            fact: None,
            ask: None,
        }))
    );
}

#[test]
fn null_foot_parses_as_absent() {
    let mut document = g7_value();
    document["foot"] = serde_json::Value::Null;
    assert_eq!(parse_page(&document.to_string()).unwrap().foot, None);
}

#[test]
fn null_width_is_a_json_error() {
    let mut document = g7_value();
    document["width"] = serde_json::Value::Null;
    assert_json_error(&document.to_string());
}

#[test]
fn json_error_carries_location_and_bare_message() {
    let json_text =
        "{\n  \"body\": [\n    { \"tag\": \"Pipe\", \"dir\": \"h\", \"label\": \"a\" }\n  ]\n}";
    let Err(ModelError::Json {
        line,
        column,
        message,
    }) = parse_page(json_text)
    else {
        panic!("expected a JSON error");
    };
    assert_eq!(message, "missing field `kind`");
    // serde_json reports a tagged node's error where the buffered node ends.
    assert!(line >= 3, "line {line}");
    assert!(column > 0);
    let display = ModelError::Json {
        line,
        column,
        message,
    }
    .to_string();
    assert_eq!(
        display,
        format!(
            "document is not valid stencil JSON at line {line}, column {column}: missing field `kind`"
        )
    );
}

#[test]
fn malformed_json_is_a_json_error() {
    assert_json_error("{ \"title\": ");
}

#[test]
fn vet_violation_is_model_error_invalid() {
    let mut document = g7_value();
    document["title"] = serde_json::json!(" untrimmed");
    match parse_page(&document.to_string()) {
        Err(ModelError::Invalid(violations)) => assert_eq!(violations.len(), 1),
        other => panic!("expected ModelError::Invalid, got {other:?}"),
    }
}
