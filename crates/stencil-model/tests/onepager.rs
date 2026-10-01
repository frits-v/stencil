// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use stencil_model::checks::{legend_consistency, remembered_constants};
use stencil_model::{Line, Node, NodeRef, body_nodes, builtin_grammar, parse_and_vet, text_fields};

const ONEPAGER_JSON: &str = include_str!("../../../examples/onepager.json");

fn gcp() -> stencil_model::Grammar {
    builtin_grammar("gcp").unwrap().unwrap()
}

#[test]
fn onepager_vets_and_passes_both_model_checks() {
    let page = parse_and_vet(ONEPAGER_JSON, &gcp()).unwrap();
    assert_eq!(page.width, 1440);
    let constants = remembered_constants(&page, &gcp());
    assert_eq!(constants.examined, text_fields(&page).len() as u64);
    assert!(constants.passed(), "{:?}", constants.defects);
    let legend = legend_consistency(&page);
    assert_eq!(legend.examined, 7 + 2);
    assert!(legend.passed(), "{:?}", legend.defects);
}

#[test]
fn onepager_has_the_section_11_5_shape() {
    let page = parse_and_vet(ONEPAGER_JSON, &gcp()).unwrap();
    let entries = body_nodes(&page);
    let count = |tag: &str| {
        entries
            .iter()
            .filter(|entry| entry.node.tag_name() == tag)
            .count()
    };
    assert_eq!(count("Text"), 4);
    assert_eq!(count("Callout"), 2);
    assert_eq!(count("Frame"), 2);
    assert_eq!(count("Pipe") + count("Tee"), 0);

    let lists: BTreeSet<String> = entries
        .iter()
        .filter_map(|entry| match entry.node {
            NodeRef::Node(Node::Text(text)) => Some(format!("{:?}", text.list)),
            _ => None,
        })
        .collect();
    assert!(lists.contains("Numbered") && lists.contains("Bulleted"));

    let request_links: Vec<&str> = page
        .links
        .iter()
        .filter(|link| (link.line, link.tint) == (Line::Solid, Some(1)))
        .filter_map(|link| link.label.as_deref())
        .collect();
    assert_eq!(request_links.len(), 6);
    for (index, label) in request_links.iter().enumerate() {
        assert!(label.starts_with(&format!("{} ", index + 1)), "{label}");
    }
    assert_eq!(
        page.links
            .iter()
            .filter(|link| (link.line, link.tint) == (Line::Deny, None))
            .count(),
        1
    );
    let hops: Vec<(&str, &str)> = page
        .links
        .iter()
        .take(6)
        .map(|link| (link.from.as_str(), link.to.as_str()))
        .collect();
    assert_eq!(
        hops,
        [
            ("console", "gateway"),
            ("gateway", "api"),
            ("api", "queue"),
            ("queue", "worker"),
            ("worker", "evaluator"),
            ("worker", "database"),
        ]
    );
}
