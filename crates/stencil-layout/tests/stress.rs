#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{STRESS_JSON, layout};
use stencil_layout::checks::{child_inside_container, siblings_do_not_overlap, text_fits_box};
use stencil_layout::{NodeTag, PartName};
use stencil_model::checks::{legend_consistency, remembered_constants};
use stencil_model::{Canvas, Node, Page, PipeKind, body_nodes, parse_page};

fn stress_page() -> Page {
    parse_page(STRESS_JSON).expect("stress-dense.json parses and passes vet")
}

#[test]
fn stress_document_has_37_body_nodes() {
    assert_eq!(body_nodes(&stress_page()).len(), 37);
}

#[test]
fn stress_document_matches_section_10() {
    let document: serde_json::Value = serde_json::from_str(STRESS_JSON).unwrap();
    assert!(document.get("width").is_none());
    assert!(document.get("foot").is_none());
    let page = stress_page();
    assert_eq!(page.title, "Dense stress figure");
    assert_eq!(page.kicker, "Stress · dense layout");
    assert_eq!(
        page.lede,
        "Five rows of cards and pipes inside a service perimeter."
    );
    assert_eq!(page.canvas, Canvas::Internal);
    let legend: Vec<(PipeKind, &str)> = page
        .legend
        .iter()
        .map(|entry| (entry.kind, entry.text.as_str()))
        .collect();
    assert_eq!(
        legend,
        [
            (PipeKind::Gray, "internal call"),
            (PipeKind::Blue, "request path"),
            (PipeKind::Pink, "reply path"),
            (PipeKind::Dash, "failover"),
            (PipeKind::Deny, "blocked egress"),
        ]
    );
    let Some(Node::Zone(gcp)) = page.body.first() else {
        panic!("body starts with the gcp zone")
    };
    let Some(Node::Zone(perimeter)) = gcp.children.first() else {
        panic!("perimeter zone")
    };
    let Some(Node::Col(column)) = perimeter.children.first() else {
        panic!("Col")
    };
    assert!(column.gap.is_none() && column.grow.is_none() && column.justify.is_none());
    let tags: Vec<&str> = column.children.iter().map(Node::tag_name).collect();
    assert_eq!(tags, ["Row", "Pipe", "Row", "Row", "Tee", "Row", "Row"]);
    let rows = column.children.iter().filter_map(|child| match child {
        Node::Row(row) => Some(row),
        _ => None,
    });
    for (row_index, row) in rows.enumerate() {
        let row_number = row_index + 1;
        assert!(row.gap.is_none() && row.grow.is_none() && row.justify.is_none());
        let cards: Vec<&str> = row
            .children
            .iter()
            .filter_map(|child| match child {
                Node::Pcard(card) => Some(card.function_name.as_str()),
                _ => None,
            })
            .collect();
        let expected: Vec<String> = (1..=3)
            .map(|card| format!("Service {row_number}.{card}"))
            .collect();
        assert_eq!(cards, expected);
    }
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
