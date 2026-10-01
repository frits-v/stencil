#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{layout, page_with_body};
use serde_json::json;
use stencil_model::body_nodes;

fn pointers(geometry: &stencil_layout::PageGeometry) -> Vec<&str> {
    geometry
        .nodes
        .iter()
        .map(|node| node.pointer.as_str())
        .collect()
}

#[test]
fn g7_geometry_order_starts_with_page_nodes_and_ends_with_legend_and_foot() {
    let geometry = layout(&common::g7_page());
    let order = pointers(&geometry);
    assert_eq!(
        order[..6],
        ["", "/kicker", "/title", "/lede", "/body", "/body/0"]
    );
    assert_eq!(
        order[order.len() - 5..],
        ["/legend", "/legend/0", "/legend/1", "/legend/2", "/foot"]
    );
}

#[test]
fn body_portion_equals_body_nodes() {
    let page = common::g7_page();
    let geometry = layout(&page);
    let order = pointers(&geometry);
    let expected: Vec<String> = body_nodes(&page)
        .iter()
        .map(|entry| entry.pointer.as_str().to_string())
        .collect();
    assert_eq!(order[5..5 + expected.len()], expected);
}

#[test]
fn tee_arms_follow_the_tee_before_its_next_sibling() {
    let page = page_with_body(
        1280,
        json!([{
            "tag": "Col",
            "children": [
                {
                    "tag": "Tee", "line": "solid", "tint": 1, "hub": "hub",
                    "arms": [
                        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "up" },
                        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "down" }
                    ]
                },
                { "tag": "Fact", "text": "after the tee" }
            ]
        }]),
    );
    let geometry = layout(&page);
    let order = pointers(&geometry);
    assert_eq!(
        order[5..],
        [
            "/body/0",
            "/body/0/children/0",
            "/body/0/children/0/arms/0",
            "/body/0/children/0/arms/1",
            "/body/0/children/1",
        ]
    );
    let expected: Vec<String> = body_nodes(&page)
        .iter()
        .map(|entry| entry.pointer.as_str().to_string())
        .collect();
    assert_eq!(order[5..], expected);
    let tee_index = 6;
    assert_eq!(geometry.children(tee_index), vec![7, 8]);
    assert_eq!(geometry.nodes[7].parent, Some(tee_index));
    assert_eq!(geometry.nodes[7].kind.as_deref(), Some("blue"));
}

#[test]
fn every_pointer_resolves_in_the_input_document() {
    let document: serde_json::Value = serde_json::from_str(common::G7_JSON).unwrap();
    let geometry = layout(&common::g7_page());
    for node in &geometry.nodes {
        assert!(
            document.pointer(node.pointer.as_str()).is_some(),
            "{} does not resolve",
            node.pointer
        );
    }
}
