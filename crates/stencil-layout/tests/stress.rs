#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{STRESS_JSON, layout};
use stencil_layout::checks::{child_inside_container, siblings_do_not_overlap, text_fits_box};
use stencil_layout::{NodeTag, PartName};
use stencil_model::checks::{legend_consistency, remembered_constants};
use serde_json::{Value, json};
use stencil_model::{Page, body_nodes, parse_page};

fn stress_page() -> Page {
    parse_page(STRESS_JSON).expect("stress-dense.json parses and passes vet")
}

#[test]
fn stress_document_has_37_body_nodes() {
    assert_eq!(body_nodes(&stress_page()).len(), 37);
}

/// The section 10 description of `examples/stress-dense.json`, field for field.
fn section_10_stress_document() -> Value {
    let card = |row: u32, column: u32| {
        json!({
            "tag": "Pcard",
            "icon": "cloud-run",
            "fn": format!("Service {row}.{column}"),
            "pn": "Cloud Run",
            "fact": "Reads its config from a bucket in the same project"
        })
    };
    let step = |row: u32, step: u32, kind: &str| {
        json!({ "tag": "Pipe", "dir": "h", "kind": kind, "label": format!("step {row}.{step}") })
    };
    let row = |row: u32| {
        json!({
            "tag": "Row",
            "children": [
                card(row, 1),
                step(row, 1, "blue"),
                card(row, 2),
                step(row, 2, "gray"),
                card(row, 3)
            ]
        })
    };
    let failover = json!({ "tag": "Pipe", "dir": "v", "kind": "dash", "label": "failover" });
    let tee = json!({
        "tag": "Tee",
        "kind": "deny",
        "hub": "egress",
        "arms": [
            { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "allowed" },
            { "tag": "Pipe", "dir": "h", "kind": "pink", "label": "reply" }
        ]
    });
    json!({
        "title": "Dense stress figure",
        "kicker": "Stress · dense layout",
        "lede": "Five rows of cards and pipes inside a service perimeter.",
        "canvas": "internal",
        "body": [{
            "tag": "Zone",
            "kind": "gcp",
            "label": "Google Cloud",
            "children": [{
                "tag": "Zone",
                "kind": "perimeter",
                "label": "Service perimeter",
                "children": [{
                    "tag": "Col",
                    "children": [row(1), failover, row(2), row(3), tee, row(4), row(5)]
                }]
            }]
        }],
        "legend": [
            { "kind": "gray", "text": "internal call" },
            { "kind": "blue", "text": "request path" },
            { "kind": "pink", "text": "reply path" },
            { "kind": "dash", "text": "failover" },
            { "kind": "deny", "text": "blocked egress" }
        ]
    })
}

#[test]
fn stress_document_matches_section_10() {
    let document: Value = serde_json::from_str(STRESS_JSON).unwrap();
    assert_eq!(document, section_10_stress_document());
}

#[test]
fn stress_layout_passes_all_five_checks() {
    let page = stress_page();
    let geometry = layout(&page);
    let reports = [
        child_inside_container(&geometry),
        siblings_do_not_overlap(&geometry),
        text_fits_box(&geometry),
        remembered_constants(&page),
        legend_consistency(&page),
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
