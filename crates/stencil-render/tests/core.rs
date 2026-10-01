#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Rendering of the core vocabulary (section 13.14): keys and colors for every tint slot,
//! fact boxes by source with their measured keys, chrome, and a plain grammar page.

mod common;

use serde_json::{Value, json};
use stencil_layout::NodeTag;
use stencil_render::iso::is_ring_zone;
use stencil_render::measured_json;
use stencil_render::palette::{TINT_FILLS, TINT_WIRES};

fn solid_pipe(tint: u8) -> Value {
    json!({ "tag": "Pipe", "dir": "h", "line": "solid", "tint": tint, "label": format!("slot {tint}") })
}

/// Eight groups, one per slot, and a solid pipe, a dash pipe and a legend entry per slot.
fn slots_document() -> Value {
    let groups: Vec<Value> = (1..=8)
        .map(|tint| {
            json!({
                "tag": "Box", "kind": "region", "tint": tint, "label": format!("Region {tint}"),
                "children": [{ "tag": "Fact", "text": "fact" }]
            })
        })
        .collect();
    let solid: Vec<Value> = (1..=8).map(solid_pipe).collect();
    let dash: Vec<Value> = (1..=8)
        .map(|tint| json!({ "tag": "Pipe", "dir": "h", "line": "dash", "tint": tint, "label": format!("dash {tint}") }))
        .collect();
    let mut legend: Vec<Value> = (1..=8)
        .map(|tint| json!({ "line": "solid", "tint": tint, "text": format!("solid {tint}") }))
        .collect();
    legend.extend(
        (1..=8).map(|tint| json!({ "line": "dash", "tint": tint, "text": format!("dash {tint}") })),
    );
    common::page_document(
        json!([
            { "tag": "Col", "children": groups },
            { "tag": "Col", "children": solid },
            { "tag": "Col", "children": dash }
        ]),
        json!(legend),
    )
}

#[test]
fn every_tint_slot_draws_its_key_fill_and_wire_under_center() {
    let rendered = common::render_document_with_fixed_metrics(slots_document());
    let document = common::parse_xml(&rendered.svg.svg);
    let region_keys = [
        "region-a", "region-b", "region-c", "region-d", "region-e", "region-f", "region-g",
        "region-h",
    ];
    let solid_keys = [
        "blue", "pink", "solid-3", "solid-4", "solid-5", "solid-6", "solid-7", "solid-8",
    ];
    let dash_keys = [
        "dash", "dash-2", "dash-3", "dash-4", "dash-5", "dash-6", "dash-7", "dash-8",
    ];
    for index in 0..8 {
        let region = common::group(&document, &format!("/body/0/children/{index}"));
        assert_eq!(region.attribute("data-kind"), Some(region_keys[index]));
        let rect = common::children_named(region, "rect")[0];
        assert_eq!(rect.attribute("fill"), Some(TINT_FILLS[index]));
        assert_eq!(rect.attribute("stroke"), Some("#BDC1C6"));

        let solid = common::group(&document, &format!("/body/1/children/{index}"));
        assert_eq!(solid.attribute("data-kind"), Some(solid_keys[index]));
        for wire in common::children_named(solid, "line") {
            assert_eq!(wire.attribute("stroke"), Some(TINT_WIRES[index]));
            assert_eq!(wire.attribute("stroke-dasharray"), None);
        }

        let dash = common::group(&document, &format!("/body/2/children/{index}"));
        assert_eq!(dash.attribute("data-kind"), Some(dash_keys[index]));
        for wire in common::children_named(dash, "line") {
            assert_eq!(wire.attribute("stroke"), Some(TINT_WIRES[index]));
            assert_eq!(wire.attribute("stroke-dasharray"), Some("6 5"));
        }
    }
    assert_eq!(TINT_WIRES[0], "#1A73E8");
    assert_eq!(TINT_WIRES[1], "#C2185B");
    assert_eq!(TINT_FILLS[0], "#D2E3FC");
    assert_eq!(TINT_FILLS[1], "#FCE4EC");
}

#[test]
fn the_legend_measures_canonical_labels_per_slot() {
    let rendered = common::render_document_with_fixed_metrics(slots_document());
    let labels: Vec<String> = rendered
        .geometry
        .nodes
        .iter()
        .filter(|node| node.tag == NodeTag::LegendEntry)
        .map(|node| {
            node.part(stencil_layout::PartName::LegendLabel)
                .unwrap()
                .text
                .as_ref()
                .unwrap()
                .text
                .clone()
        })
        .collect();
    let expected: Vec<String> = stencil_model::TINT_NAMES
        .iter()
        .map(|name| format!("Solid {name}"))
        .chain(
            stencil_model::TINT_NAMES
                .iter()
                .map(|name| format!("Dashed {name}")),
        )
        .collect();
    assert_eq!(labels, expected);
}

fn item_document(facts: Value) -> Value {
    common::page_document(
        json!([
            { "tag": "Item", "kind": "product", "title": "Bucket", "subtitle": "Cloud Storage", "facts": facts },
            { "tag": "Pipe", "dir": "h", "line": "gray", "label": "read" }
        ]),
        json!([{ "line": "gray", "text": "internal call" }]),
    )
}

#[test]
fn measured_fact_keys_are_bare_for_the_first_of_each_source_and_indexed_after() {
    let document = item_document(json!([
        { "text": "dual-region" },
        { "text": "acme-raw", "source": "built" },
        { "text": "retention?", "source": "ask" },
        { "text": "versioned" },
        { "text": "acme-logs", "source": "built" }
    ]));
    let rendered = common::render_document_with_fixed_metrics(document.clone());
    let measured = measured_json(&document, &rendered.geometry, None);
    let item = measured["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "/body/0")
        .unwrap();
    assert_eq!(item["tag"], "Pcard");
    let keys: Vec<&str> = item["parts"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for key in [
        "fact_box",
        "fact",
        "built_box",
        "built",
        "ask_box",
        "ask",
        "fact_box/3",
        "fact/3",
        "built_box/4",
        "built/4",
        "function_name",
        "product_name",
    ] {
        assert!(keys.contains(&key), "{key} missing from {keys:?}");
    }
    assert_eq!(keys.len(), 13, "{keys:?}");
}

#[test]
fn a_built_fact_is_unfilled_and_a_doc_and_ask_fact_are_filled() {
    let rendered = common::render_document_with_fixed_metrics(item_document(json!([
        { "text": "dual-region" },
        { "text": "acme-raw", "source": "built" },
        { "text": "retention?", "source": "ask" }
    ])));
    let document = common::parse_xml(&rendered.svg.svg);
    let item = common::group(&document, "/body/0");
    let fills: Vec<&str> = common::children_named(item, "rect")
        .iter()
        .filter_map(|rect| rect.attribute("fill"))
        .collect();
    assert_eq!(fills, ["#FFFFFF", "#F1F3F4", "#FEF7E0"], "card, fact, ask");
    let texts: Vec<(String, &str)> = common::children_named(item, "text")
        .iter()
        .map(|text| {
            (
                text.text().unwrap().to_string(),
                text.attribute("fill").unwrap(),
            )
        })
        .collect();
    assert!(
        texts.contains(&("\u{2022} acme-raw".to_string(), "#5F6368")),
        "{texts:?}"
    );
    assert!(
        texts.contains(&("Ask: retention?".to_string(), "#B06000")),
        "{texts:?}"
    );
}

#[test]
fn chrome_none_renders_no_kicker_title_or_lede() {
    let mut document = common::page_document(
        json!([{ "tag": "Pipe", "dir": "h", "line": "solid", "label": "hop" }]),
        json!([{ "line": "solid", "text": "request path" }]),
    );
    document["chrome"] = json!("none");
    let rendered = common::render_document_with_fixed_metrics(document);
    let parsed = common::parse_xml(&rendered.svg.svg);
    let tags: Vec<&str> = parsed
        .descendants()
        .filter_map(|node| node.attribute("data-tag"))
        .collect();
    assert_eq!(tags, ["Page", "Body", "Pipe", "Legend", "LegendEntry"]);
}

fn plain_document() -> Value {
    let mut document = common::page_document(
        json!([
            { "tag": "Box", "kind": "system", "label": "Platform", "children": [
                { "tag": "Box", "kind": "boundary", "label": "Trust edge", "children": [
                    { "tag": "Box", "kind": "group", "tint": 2, "label": "Site B", "children": [
                        { "tag": "Item", "kind": "service", "title": "API" }
                    ] },
                    { "tag": "Box", "kind": "group", "label": "Shared", "children": [
                        { "tag": "Item", "kind": "store", "title": "Ledger" }
                    ] }
                ] }
            ] },
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": "call" }
        ]),
        json!([{ "line": "solid", "tint": 2, "text": "request path" }]),
    );
    document["grammar"] = json!("plain");
    document
}

#[test]
fn a_plain_page_paints_through_roles_tones_and_tints() {
    let rendered = common::render_document_with_fixed_metrics(plain_document());
    let parsed = common::parse_xml(&rendered.svg.svg);
    let system = common::group(&parsed, "/body/0");
    assert_eq!(system.attribute("data-kind"), Some("system"));
    assert_eq!(
        common::children_named(system, "rect")[0].attribute("fill"),
        Some("#1A73E8")
    );
    let boundary = common::group(&parsed, "/body/0/children/0");
    let boundary_rect = common::children_named(boundary, "rect")[0];
    assert_eq!(boundary_rect.attribute("fill"), Some("none"));
    assert_eq!(boundary_rect.attribute("stroke-dasharray"), Some("6 5"));
    let tinted = common::group(&parsed, "/body/0/children/0/children/0");
    assert_eq!(tinted.attribute("data-kind"), Some("group-b"));
    assert_eq!(
        common::children_named(tinted, "rect")[0].attribute("fill"),
        Some("#FCE4EC")
    );
    let untinted = common::group(&parsed, "/body/0/children/0/children/1");
    assert_eq!(untinted.attribute("data-kind"), Some("group"));
    assert_eq!(
        common::children_named(untinted, "rect")[0].attribute("fill"),
        Some("#EDF3F0")
    );
}

#[test]
fn a_box_is_a_ring_when_its_tone_is_strong_and_it_is_untinted() {
    let rendered = common::render_document_with_fixed_metrics(plain_document());
    let ring = |pointer: &str| {
        let node = rendered
            .geometry
            .nodes
            .iter()
            .find(|node| node.pointer.as_str() == pointer)
            .unwrap();
        is_ring_zone(node)
    };
    assert!(ring("/body/0/children/0"), "the plain boundary");
    assert!(!ring("/body/0"), "the frame");
    assert!(!ring("/body/0/children/0/children/0"), "a group");
}
