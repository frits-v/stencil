// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{callout, col, frame, page_with_body, pipe, text_block};
use stencil_model::{
    CalloutKind, FRAME_HEIGHT_DEFAULT, Line, ListKind, ModelError, Node, Page, TEXT_BODY_LINES_MAX,
    VetRule, body_nodes, parse_and_vet, text_fields, validate_page,
};

fn violations(page: &Page) -> Vec<(String, &'static str, String)> {
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

fn one(pointer: &str, rule: &'static str, message: &str) -> Vec<(String, &'static str, String)> {
    vec![(pointer.to_string(), rule, message.to_string())]
}

fn text_pointers(page: &Page) -> Vec<String> {
    text_fields(page)
        .iter()
        .map(|field| field.pointer.as_str().to_string())
        .collect()
}

/// A page whose body holds `node` and a blue pipe, so the page is valid apart from `node`.
fn page_with(node: Node) -> Page {
    page_with_body(vec![node, pipe(Line::Solid, Some(1), "hop")])
}

fn page_json(body: serde_json::Value) -> String {
    serde_json::json!({
        "title": "t", "kicker": "k", "lede": "l", "canvas": "internal",
        "body": body, "legend": []
    })
    .to_string()
}

#[test]
fn vet_rule_names_for_blocks() {
    let expected = [
        (VetRule::TextBodyEmpty, "text-body-empty"),
        (VetRule::TextBodyTooLong, "text-body-too-long"),
        (VetRule::FrameHeightOutOfRange, "frame-height-out-of-range"),
    ];
    for (rule, name) in expected {
        assert_eq!(rule.as_str(), name);
    }
}

#[test]
fn blocks_parse_with_their_tags_and_defaults() {
    let page = parse_and_vet(
        &page_json(serde_json::json!([
            { "tag": "Text", "id": "problem", "heading": "Problem", "body": ["One line."] },
            { "tag": "Callout", "kind": "risk", "text": "Clips can arrive late." },
            { "tag": "Frame", "label": "Trigger editor" }
        ])),
        &common::gcp(),
    )
    .unwrap();
    let tags: Vec<&str> = page.body.iter().map(Node::tag_name).collect();
    assert_eq!(tags, ["Text", "Callout", "Frame"]);
    match &page.body[..] {
        [Node::Text(text), Node::Callout(callout), Node::Frame(frame)] => {
            assert_eq!(text.id.as_deref(), Some("problem"));
            assert_eq!(text.list, ListKind::Plain);
            assert_eq!(callout.kind, CalloutKind::Risk);
            assert_eq!(callout.title, None);
            assert_eq!(frame.height, FRAME_HEIGHT_DEFAULT);
        }
        _ => panic!("body is Text, Callout, Frame"),
    }
    assert_eq!(page.body[0].id(), Some("problem"));
    assert_eq!(page.body[1].id(), None);
}

#[test]
fn block_enums_and_fields_reject_unknown_values() {
    for body in [
        serde_json::json!([{ "tag": "Text", "body": ["a"], "list": "lettered" }]),
        serde_json::json!([{ "tag": "Text", "body": ["a"], "colour": "red" }]),
        serde_json::json!([{ "tag": "Callout", "kind": "warning", "text": "a" }]),
        serde_json::json!([{ "tag": "Callout", "text": "a" }]),
        serde_json::json!([{ "tag": "Frame", "label": "a", "width": 300 }]),
        serde_json::json!([{ "tag": "Frame", "label": "a", "height": 70000 }]),
    ] {
        let parsed = parse_and_vet(&page_json(body.clone()), &common::gcp());
        assert!(
            matches!(parsed, Err(ModelError::Json { .. })),
            "{body} parsed: {parsed:?}"
        );
    }
}

#[test]
fn blocks_are_leaves_of_the_walk() {
    let page = page_with_body(vec![col(vec![
        text_block(Some("Goals"), &["a", "b"], ListKind::Numbered),
        callout(CalloutKind::Decision, Some("Queue"), "c"),
        frame("Screen"),
    ])]);
    let entries: Vec<(String, usize, &str)> = body_nodes(&page)
        .iter()
        .map(|entry| {
            (
                entry.pointer.as_str().to_string(),
                entry.depth,
                entry.node.tag_name(),
            )
        })
        .collect();
    assert_eq!(
        entries,
        vec![
            ("/body/0".to_string(), 1, "Col"),
            ("/body/0/children/0".to_string(), 2, "Text"),
            ("/body/0/children/1".to_string(), 2, "Callout"),
            ("/body/0/children/2".to_string(), 2, "Frame"),
        ]
    );
}

#[test]
fn block_text_fields_in_declaration_order() {
    let page = page_with_body(vec![
        text_block(Some("Goals"), &["first", "second"], ListKind::Bulleted),
        text_block(None, &["only"], ListKind::Plain),
        callout(CalloutKind::Risk, Some("Late clips"), "text"),
        callout(CalloutKind::Open, None, "text"),
        frame("Screen"),
    ]);
    assert_eq!(
        text_pointers(&page),
        [
            "/title",
            "/kicker",
            "/lede",
            "/body/0/heading",
            "/body/0/body/0",
            "/body/0/body/1",
            "/body/1/body/0",
            "/body/2/title",
            "/body/2/text",
            "/body/3/text",
            "/body/4/label",
            "/legend/0/text"
        ]
    );
}

#[test]
fn every_block_text_is_vetted() {
    let untrimmed = "x ";
    let page = page_with_body(vec![
        text_block(Some(untrimmed), &["ok", untrimmed], ListKind::Numbered),
        callout(CalloutKind::Note, Some(untrimmed), untrimmed),
        frame(untrimmed),
        pipe(Line::Solid, Some(1), "hop"),
    ]);
    let pointers: Vec<String> = violations(&page)
        .into_iter()
        .map(|(pointer, rule, _)| {
            assert_eq!(rule, "text-untrimmed");
            pointer
        })
        .collect();
    assert_eq!(
        pointers,
        [
            "/body/0/heading",
            "/body/0/body/1",
            "/body/1/title",
            "/body/1/text",
            "/body/2/label"
        ]
    );
}

#[test]
fn text_body_needs_a_line() {
    let page = page_with(text_block(Some("Problem"), &[], ListKind::Plain));
    assert_eq!(
        violations(&page),
        one("/body/0/body", "text-body-empty", "body has no lines")
    );
}

#[test]
fn text_body_of_64_lines_passes_and_65_fails() {
    let lines = vec!["line"; TEXT_BODY_LINES_MAX];
    let page = page_with(text_block(None, &lines, ListKind::Bulleted));
    assert_eq!(violations(&page), vec![]);
    let lines = vec!["line"; TEXT_BODY_LINES_MAX + 1];
    let page = page_with(text_block(None, &lines, ListKind::Bulleted));
    assert_eq!(
        violations(&page),
        one(
            "/body/0/body",
            "text-body-too-long",
            "body has 65 lines, above 64"
        )
    );
}

#[test]
fn text_body_length_sits_between_heading_and_lines() {
    let mut lines = vec![" untrimmed"; 100];
    lines[0] = "";
    let page = page_with(text_block(Some(" heading"), &lines, ListKind::Plain));
    let found = violations(&page);
    assert_eq!(found.len(), 1 + 1 + TEXT_BODY_LINES_MAX + 1);
    assert_eq!(
        found[..3],
        [
            (
                "/body/0/heading".to_string(),
                "text-untrimmed",
                "text starts or ends with whitespace".to_string()
            ),
            (
                "/body/0/body".to_string(),
                "text-body-too-long",
                "body has 100 lines, above 64".to_string()
            ),
            (
                "/body/0/body/0".to_string(),
                "text-empty",
                "text is empty".to_string()
            ),
        ]
    );
    assert_eq!(
        found.last().map(|last| last.0.as_str()),
        Some("/body/0/body/64")
    );
}

#[test]
fn frame_height_limits() {
    for (height, expected) in [(40, true), (1200, true), (39, false), (1201, false)] {
        let mut node = frame("Screen");
        if let Node::Frame(frame) = &mut node {
            frame.height = height;
        }
        let found = violations(&page_with(node));
        if expected {
            assert_eq!(found, vec![], "height {height}");
        } else {
            assert_eq!(
                found,
                one(
                    "/body/0/height",
                    "frame-height-out-of-range",
                    &format!("height {height} is outside 40 to 1200")
                )
            );
        }
    }
}

#[test]
fn block_ids_are_checked_like_every_node_id() {
    let mut first = frame("One");
    let mut second = callout(CalloutKind::Note, None, "Two");
    if let Node::Frame(frame) = &mut first {
        frame.id = Some("screen".to_string());
    }
    if let Node::Callout(callout) = &mut second {
        callout.id = Some("screen".to_string());
    }
    let page = page_with_body(vec![first, second, pipe(Line::Solid, Some(1), "hop")]);
    assert_eq!(
        violations(&page),
        one(
            "/body/1/id",
            "id-duplicate",
            "id \"screen\" is already used at /body/0"
        )
    );
}
