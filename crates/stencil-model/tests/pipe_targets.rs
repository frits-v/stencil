#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Pipe `from` and `to` (section 13.8): parsing, serialization and the three vet rules.

mod common;

use serde_json::{Value, json};
use stencil_model::{Page, validate_page};

fn page(body: Value) -> Page {
    serde_json::from_value(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "canvas": "customer",
        "body": body,
        "legend": [{ "line": "solid", "text": "request path" }]
    }))
    .unwrap()
}

fn rules(page: &Page) -> Vec<(String, &'static str, String)> {
    validate_page(page, &common::gcp())
        .into_iter()
        .map(|violation| {
            (
                violation.pointer.as_str().to_string(),
                violation.rule.as_str(),
                violation.message,
            )
        })
        .collect()
}

/// A Row of two Boxes with ids `left` and `right` and a Pipe between them whose fields
/// `pipe_fields` adds.
fn row_with_pipe(pipe_fields: Value) -> Value {
    let mut pipe = json!({ "tag": "Pipe", "dir": "h", "line": "solid", "label": "hop" });
    for (key, value) in pipe_fields.as_object().unwrap() {
        pipe[key] = value.clone();
    }
    json!([{ "tag": "Row", "children": [
        { "tag": "Box", "id": "left", "kind": "onprem", "label": "Site", "children": [
            { "tag": "Fact", "text": "router" }
        ] },
        pipe,
        { "tag": "Box", "id": "right", "kind": "project", "label": "Project", "children": [
            { "tag": "Fact", "text": "service" }
        ] }
    ] }])
}

#[test]
fn targets_parse_and_serialize_only_when_set() {
    let with_targets = page(row_with_pipe(json!({ "from": "left", "to": "right" })));
    let stencil_model::Node::Row(row) = &with_targets.body[0] else {
        panic!("body/0 is a Row");
    };
    let stencil_model::Node::Pipe(pipe) = &row.children[1] else {
        panic!("the middle child is a Pipe");
    };
    assert_eq!(pipe.from.as_deref(), Some("left"));
    assert_eq!(pipe.to.as_deref(), Some("right"));
    let serialized = serde_json::to_value(&with_targets).unwrap();
    assert_eq!(serialized["body"][0]["children"][1]["from"], json!("left"));

    let without = page(row_with_pipe(json!({})));
    let serialized = serde_json::to_value(&without).unwrap();
    let pipe_object = serialized["body"][0]["children"][1].as_object().unwrap();
    assert!(!pipe_object.contains_key("from"));
    assert!(!pipe_object.contains_key("to"));
}

#[test]
fn targets_naming_nodes_on_either_side_vet_clean() {
    assert_eq!(
        rules(&page(row_with_pipe(
            json!({ "from": "left", "to": "right" })
        ))),
        vec![]
    );
    assert_eq!(
        rules(&page(row_with_pipe(json!({ "to": "right" })))),
        vec![]
    );
}

#[test]
fn a_target_naming_no_id_is_pipe_target_unknown() {
    assert_eq!(
        rules(&page(row_with_pipe(
            json!({ "from": "nowhere", "to": "right" })
        ))),
        vec![(
            "/body/0/children/1/from".to_string(),
            "pipe-target-unknown",
            "pipe target \"nowhere\" names no node id".to_string()
        )]
    );
}

#[test]
fn both_targets_naming_one_id_is_pipe_targets_equal() {
    assert_eq!(
        rules(&page(row_with_pipe(
            json!({ "from": "left", "to": "left" })
        ))),
        vec![(
            "/body/0/children/1/to".to_string(),
            "pipe-targets-equal",
            "pipe from and to both name \"left\"".to_string()
        )]
    );
}

#[test]
fn a_tee_arm_with_a_target_is_pipe_target_on_tee_arm() {
    let body = json!([
        { "tag": "Item", "id": "router", "kind": "product", "title": "Router", "subtitle": "Cloud Router" },
        { "tag": "Tee", "line": "solid", "hub": "hub", "arms": [
            { "tag": "Pipe", "dir": "h", "line": "solid", "label": "a", "to": "router" },
            { "tag": "Pipe", "dir": "h", "line": "solid", "label": "b", "from": "router", "to": "router" }
        ] }
    ]);
    assert_eq!(
        rules(&page(body)),
        vec![
            (
                "/body/1/arms/0/to".to_string(),
                "pipe-target-on-tee-arm",
                "a Tee arm cannot name a target".to_string()
            ),
            (
                "/body/1/arms/1/from".to_string(),
                "pipe-target-on-tee-arm",
                "a Tee arm cannot name a target".to_string()
            ),
            (
                "/body/1/arms/1/to".to_string(),
                "pipe-target-on-tee-arm",
                "a Tee arm cannot name a target".to_string()
            ),
        ]
    );
}

#[test]
fn a_pipe_id_shares_the_id_namespace() {
    let body = json!([{ "tag": "Row", "children": [
        { "tag": "Box", "id": "hop", "kind": "onprem", "label": "Site", "children": [
            { "tag": "Fact", "text": "router" }
        ] },
        { "tag": "Pipe", "id": "hop", "dir": "h", "line": "solid", "label": "hop" }
    ] }]);
    let found = rules(&page(body));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "id-duplicate");
}
