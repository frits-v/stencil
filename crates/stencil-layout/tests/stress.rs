#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{STRESS_JSON, layout};
use serde_json::{Value, json};
use stencil_layout::checks::{
    child_inside_container, pipes_land, siblings_do_not_overlap, text_fits_box,
};
use stencil_layout::{NodeTag, PartName};
use stencil_model::checks::{legend_consistency, remembered_constants};
use stencil_model::{Page, body_nodes, parse_page};

fn stress_page() -> Page {
    parse_page(STRESS_JSON).expect("stress-dense.json parses and passes vet")
}

#[test]
fn stress_document_has_37_body_nodes() {
    assert_eq!(body_nodes(&stress_page()).len(), 37);
}

/// The three service titles and the two pipe labels of each stage row, in order.
const STAGES: [([&str; 3], [&str; 2]); 5] = [
    (
        ["Shipment booking", "Label printing", "Pickup scan"],
        ["label", "pickup"],
    ),
    (
        ["Hub arrival", "Sort planner", "Chute assignment"],
        ["arrival", "sort plan"],
    ),
    (
        ["Trailer loading", "Route tracker", "Depot arrival"],
        ["load list", "position"],
    ),
    (
        ["Courier manifest", "Delivery tracker", "Proof of delivery"],
        ["manifest", "status"],
    ),
    (
        ["Return request", "Return label", "Refund trigger"],
        ["return", "refund"],
    ),
];

/// The section 10 description of `examples/stress-dense.json`, field for field.
fn section_10_stress_document() -> Value {
    let card = |title: &str| {
        json!({
            "tag": "Item", "kind": "product",
            "icon": "cloud-run",
            "title": title,
            "subtitle": "Cloud Run",
            "facts": [{ "text": "Publishes each parcel event to the tracking topic" }]
        })
    };
    let solid_step = |label: &str| json!({ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": label });
    let gray_step =
        |label: &str| json!({ "tag": "Pipe", "dir": "h", "line": "gray", "label": label });
    let row = |(titles, labels): ([&str; 3], [&str; 2])| {
        let [first, second, third] = titles;
        let [request, internal] = labels;
        json!({
            "tag": "Row",
            "children": [
                card(first),
                solid_step(request),
                card(second),
                gray_step(internal),
                card(third)
            ]
        })
    };
    let [intake, sorting, linehaul, last_mile, returns] = STAGES.map(row);
    let failover = json!({ "tag": "Pipe", "dir": "v", "line": "dash", "label": "failover" });
    let tee = json!({
        "tag": "Tee",
        "line": "deny",
        "hub": "egress",
        "arms": [
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "allowed" },
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": "reply" }
        ]
    });
    json!({
        "title": "A parcel carrier tracks every shipment through five stages behind one service perimeter",
        "kicker": "Parcel tracking · Cloud Run",
        "lede": "Each parcel moves from intake through sorting, linehaul and last-mile delivery to returns, and the perimeter blocks egress from every stage.",
        "badge": "Internal",
        "grammar": "gcp",
        "body": [{
            "tag": "Box",
            "kind": "gcp",
            "label": "Google Cloud",
            "children": [{
                "tag": "Box",
                "kind": "perimeter",
                "label": "Service perimeter",
                "children": [{
                    "tag": "Col",
                    "children": [intake, failover, sorting, linehaul, tee, last_mile, returns]
                }]
            }]
        }],
        "legend": [
            { "line": "gray", "text": "internal call" },
            { "line": "solid", "tint": 1, "text": "request path" },
            { "line": "solid", "tint": 2, "text": "reply path" },
            { "line": "dash", "text": "failover" },
            { "line": "deny", "text": "blocked egress" }
        ]
    })
}

#[test]
fn stress_document_matches_section_10() {
    let document: Value = serde_json::from_str(STRESS_JSON).unwrap();
    assert_eq!(document, section_10_stress_document());
}

/// pipes-land examines 22 pipe ends: two Pipe h per Row with a card on each side (5 by 2 by
/// 2) and the Pipe v between Row 1 and Row 2 (2). The Tee arms have no Row ancestor.
#[test]
fn stress_layout_passes_every_check_on_the_page() {
    let page = stress_page();
    let geometry = layout(&page);
    let landing = pipes_land(&page, &geometry);
    assert_eq!(landing.examined, 22);
    let reports = [
        child_inside_container(&geometry),
        siblings_do_not_overlap(&geometry),
        text_fits_box(&geometry),
        remembered_constants(&page, &common::gcp()),
        legend_consistency(&page),
        landing,
    ];
    for report in &reports {
        assert!(
            report.passed(),
            "{:?} examined {} defects {:?}",
            report.check,
            report.examined,
            report.defects
        );
    }
}

#[test]
fn every_stress_card_fact_wraps_to_two_lines() {
    let geometry = layout(&stress_page());
    let cards: Vec<_> = geometry
        .nodes
        .iter()
        .filter(|node| node.tag == NodeTag::Pcard)
        .collect();
    assert_eq!(cards.len(), 15);
    for card in cards {
        let fact = card.part(PartName::Fact).unwrap().text.as_ref().unwrap();
        assert_eq!(fact.metrics.line_count, 2, "{}", card.pointer);
    }
}
