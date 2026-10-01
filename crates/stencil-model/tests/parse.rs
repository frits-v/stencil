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
use stencil_model::{
    Arrow, BUILTIN_THEMES, Item, ModelError, Node, PAGE_WIDTH_DEFAULT, VetRule, parse_and_vet,
    theme_reference,
};

fn assert_json_error(json_text: &str) {
    match parse_and_vet(json_text, &common::gcp()) {
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
    let page = parse_and_vet(G7_JSON, &common::gcp()).expect("g7 is valid");
    let serialized = serde_json::to_string(&page).unwrap();
    let reparsed = parse_and_vet(&serialized, &common::gcp()).unwrap();
    assert_eq!(page, reparsed);
    assert_eq!(page, g7_page());
}

#[test]
fn width_defaults_to_1280_when_absent() {
    let page = parse_and_vet(G7_JSON, &common::gcp()).unwrap();
    assert!(g7_value().get("width").is_none());
    assert_eq!(page.width, PAGE_WIDTH_DEFAULT);
    assert_eq!(page.width, 1280);
}

#[test]
fn explicit_width_is_kept() {
    let mut document = g7_value();
    document["width"] = serde_json::json!(1600);
    let page = parse_and_vet(&document.to_string(), &common::gcp()).unwrap();
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
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "a", "weight": 2 }
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
        { "tag": "Item", "kind": "product", "icon": "bigtable", "title": "Store" }
    )));
}

#[test]
fn retired_tags_and_fields_are_json_errors() {
    for node in [
        serde_json::json!({ "tag": "Zone", "kind": "region-a", "label": "Region A", "children": [ { "tag": "Fact", "text": "a" } ] }),
        serde_json::json!({ "tag": "Pcard", "fn": "Store" }),
        serde_json::json!({ "tag": "Pipe", "dir": "h", "kind": "blue", "label": "a" }),
        serde_json::json!({ "tag": "Item", "kind": "product", "title": "Store", "fn": "Store" }),
        serde_json::json!({ "tag": "Item", "kind": "product", "title": "Store", "ask": "Which?" }),
    ] {
        assert_json_error(&g7_with_first_node(node));
    }
}

#[test]
fn a_box_kind_outside_the_grammar_is_a_vet_violation() {
    let json_text = g7_with_first_node(serde_json::json!(
        { "tag": "Box", "kind": "region-c", "label": "Region C", "children": [ { "tag": "Fact", "text": "a" } ] }
    ));
    let Err(ModelError::Invalid(violations)) = parse_and_vet(&json_text, &common::gcp()) else {
        panic!("expected vet violations");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].pointer.as_str(), "/body/0/kind");
    assert_eq!(
        violations[0].message,
        "kind \"region-c\" is not a container kind of grammar gcp"
    );
}

#[test]
fn pipe_without_line_is_rejected() {
    assert_json_error(&g7_with_first_node(serde_json::json!(
        { "tag": "Pipe", "dir": "h", "label": "a" }
    )));
}

fn tee_json(arms: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "tag": "Tee", "line": "solid", "tint": 1, "hub": "hub", "arms": arms })
}

fn arm_json(tag: &str) -> serde_json::Value {
    serde_json::json!({ "tag": tag, "dir": "h", "line": "solid", "tint": 1, "label": "arm" })
}

#[test]
fn tee_with_two_pipe_arms_parses() {
    let json_text = g7_with_first_node(tee_json(serde_json::json!([
        arm_json("Pipe"),
        arm_json("Pipe")
    ])));
    let page =
        parse_and_vet(&json_text, &common::gcp()).expect("a Tee with two Pipe arms is valid");
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
        { "tag": "Box", "kind": "vpc", "label": "VPC", "children": [ { "tag": "Fact", "text": "a" } ] }
    ]))));
}

#[test]
fn null_optional_fields_parse_as_absent() {
    let json_text = g7_with_first_node(serde_json::json!(
        { "tag": "Row", "gap": null, "grow": null, "justify": null, "children": [
            { "tag": "Item", "kind": "product", "icon": null, "title": "Store", "subtitle": null }
        ] }
    ));
    let page = parse_and_vet(&json_text, &common::gcp()).unwrap();
    let Some(Node::Row(row)) = page.body.first() else {
        panic!("first body node is a Row");
    };
    assert_eq!(
        (row.gap, row.grow.as_ref(), row.justify),
        (None, None, None)
    );
    assert_eq!(
        row.children.first(),
        Some(&Node::Item(Item {
            id: None,
            kind: "product".to_string(),
            icon: None,
            title: "Store".to_string(),
            subtitle: None,
            facts: Vec::new(),
        }))
    );
}

#[test]
fn null_foot_parses_as_absent() {
    let mut document = g7_value();
    document["foot"] = serde_json::Value::Null;
    assert_eq!(
        parse_and_vet(&document.to_string(), &common::gcp())
            .unwrap()
            .foot,
        None
    );
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
    }) = parse_and_vet(json_text, &common::gcp())
    else {
        panic!("expected a JSON error");
    };
    assert_eq!(message, "missing field `line`");
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
            "document is not valid stencil JSON at line {line}, column {column}: missing field `line`"
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
    match parse_and_vet(&document.to_string(), &common::gcp()) {
        Err(ModelError::Invalid(violations)) => assert_eq!(violations.len(), 1),
        other => panic!("expected ModelError::Invalid, got {other:?}"),
    }
}

#[test]
fn g7_serializes_to_its_input_plus_the_width_default() {
    // `width` is the one field serialized at its default; theme, links, id and a Pipe's
    // arrow are skipped at theirs, so a document without them keeps its bytes.
    let page = parse_and_vet(G7_JSON, &common::gcp()).unwrap();
    let mut expected = g7_value();
    expected["width"] = serde_json::json!(PAGE_WIDTH_DEFAULT);
    assert_eq!(serde_json::to_value(&page).unwrap(), expected);
}

#[test]
fn theme_defaults_to_center_and_round_trips_as_written() {
    let page = parse_and_vet(G7_JSON, &common::gcp()).unwrap();
    assert_eq!(page.theme, None);
    assert_eq!(theme_reference(&page), "center");

    for name in BUILTIN_THEMES.iter().copied().chain(["themes/brand.json"]) {
        let mut document = g7_value();
        document["theme"] = serde_json::json!(name);
        let page = parse_and_vet(&document.to_string(), &common::gcp()).unwrap();
        assert_eq!(page.theme.as_deref(), Some(name));
        assert_eq!(theme_reference(&page), name);
        let serialized = serde_json::to_value(&page).unwrap();
        assert_eq!(serialized.get("theme"), Some(&serde_json::json!(name)));
    }
}

#[test]
fn a_theme_that_is_neither_built_in_nor_json_is_theme_unknown() {
    let mut document = g7_value();
    document["theme"] = serde_json::json!("night");
    match parse_and_vet(&document.to_string(), &common::gcp()) {
        Err(ModelError::Invalid(violations)) => {
            assert_eq!(violations.len(), 1);
            assert_eq!(violations[0].rule, VetRule::ThemeUnknown);
            assert_eq!(violations[0].rule.as_str(), "theme-unknown");
            assert_eq!(violations[0].pointer.as_str(), "/theme");
            assert_eq!(
                violations[0].message,
                "theme \"night\" is neither a built-in theme nor a .json path"
            );
        }
        other => panic!("expected ModelError::Invalid, got {other:?}"),
    }
    document["theme"] = serde_json::json!(7);
    assert_json_error(&document.to_string());
}

#[test]
fn theme_overrides_is_an_object_kept_as_written() {
    let mut document = g7_value();
    document["theme_overrides"] = serde_json::json!({ "tints": { "3": { "wire": "#00796B" } } });
    let page = parse_and_vet(&document.to_string(), &common::gcp()).unwrap();
    let overrides = page.theme_overrides.as_ref().unwrap();
    assert_eq!(overrides["tints"]["3"]["wire"], "#00796B");
    let serialized = serde_json::to_value(&page).unwrap();
    assert_eq!(serialized["theme_overrides"], document["theme_overrides"]);
    document["theme_overrides"] = serde_json::json!("dusk");
    assert_json_error(&document.to_string());
}

#[test]
fn pipe_arrow_defaults_to_none_and_link_arrow_to_end() {
    let page = parse_and_vet(G7_JSON, &common::gcp()).unwrap();
    let pipe_json =
        serde_json::json!({ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "a" });
    let pipe: Node = serde_json::from_value(pipe_json.clone()).unwrap();
    let Node::Pipe(pipe_value) = &pipe else {
        panic!("a Pipe node");
    };
    assert_eq!(pipe_value.arrow, Arrow::None);
    assert_eq!(serde_json::to_value(&pipe).unwrap(), pipe_json);

    let arrowed_json = serde_json::json!(
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "a", "arrow": "both" }
    );
    let arrowed: Node = serde_json::from_value(arrowed_json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&arrowed).unwrap(), arrowed_json);

    let link: stencil_model::Link = serde_json::from_value(
        serde_json::json!({ "from": "a", "to": "b", "line": "solid", "tint": 1 }),
    )
    .unwrap();
    assert_eq!(link.arrow, Arrow::End);
    assert!(page.links.is_empty());
}

#[test]
fn unknown_arrow_side_or_link_field_is_a_json_error() {
    for links in [
        serde_json::json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": "tail" }]),
        serde_json::json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "from_side": "north" }]),
        serde_json::json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "colour": "red" }]),
        serde_json::json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "via": [{ "x": 1, "y": 2, "z": 3 }] }]),
        serde_json::json!([{ "from": "a", "line": "solid", "tint": 1 }]),
        serde_json::Value::Null,
    ] {
        let mut document = g7_value();
        document["links"] = links;
        assert_json_error(&document.to_string());
    }
}

#[test]
fn links_with_sides_and_via_round_trip() {
    let mut document = g7_value();
    document["body"][0]["children"][0]["children"][0]["children"][0]["id"] =
        serde_json::json!("router-1");
    document["body"][0]["children"][2]["id"] = serde_json::json!("cloud");
    document["links"] = serde_json::json!([{
        "from": "router-1", "to": "cloud", "line": "solid", "tint": 1, "label": "1", "sub": "request",
        "arrow": "both", "from_side": "right", "to_side": "left",
        "via": [ { "x": 400.5, "y": 120.25 } ]
    }]);
    document["width"] = serde_json::json!(1440);
    let page = parse_and_vet(&document.to_string(), &common::gcp()).unwrap();
    assert_eq!(page.links.len(), 1);
    assert_eq!(serde_json::to_value(&page).unwrap(), document);
}
