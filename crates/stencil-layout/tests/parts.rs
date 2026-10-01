#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{layout, node, page_from};
use serde_json::json;
use stencil_layout::{NodeTag, PageGeometry, PartName, TextAlign, WRAP_EPSILON_PX};
use stencil_model::text::{FixedMetricsMeasurer, TextMeasurer};

/// A page holding every NodeTag, every optional part and both Zone variants.
fn every_tag_geometry() -> PageGeometry {
    let page = page_from(json!({
        "title": "Every tag",
        "kicker": "Parts",
        "lede": "One of each.",
        "foot": "Foot text",
        "canvas": "customer",
        "body": [
            {
                "tag": "Row",
                "children": [
                    {
                        "tag": "Col",
                        "children": [
                            {
                                "tag": "Box", "kind": "onprem", "tint": 1, "label": "Region A",
                                "children": [{
                                    "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster", "subtitle": "GKE",
                                    "facts": [{ "text": "Three zones" }, { "text": "confirm", "source": "ask" }]
                                }]
                            },
                            {
                                "tag": "Box", "kind": "gcp", "label": "Google Cloud",
                                "children": [{ "tag": "Item", "kind": "product", "title": "Bare card" }]
                            }
                        ]
                    },
                    { "tag": "Note", "kind": "h1", "text": "A note" },
                    { "tag": "Fact", "text": "A fact" }
                ]
            },
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "VLAN 1", "sub": "EAD 1" },
            { "tag": "Pipe", "dir": "v", "line": "dash", "label": "failover" },
            {
                "tag": "Tee", "line": "deny", "hub": "egress",
                "arms": [
                    { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "allowed" },
                    { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": "reply" }
                ]
            },
            {
                "tag": "Text", "heading": "Goals", "list": "numbered",
                "body": ["First goal", "Second goal"]
            },
            { "tag": "Callout", "kind": "risk", "title": "Drift", "text": "Plans change." },
            { "tag": "Frame", "label": "Console screen", "height": 120 }
        ],
        "legend": [{ "line": "solid", "tint": 1, "text": "request path" }]
    }));
    layout(&page)
}

fn part_names(geometry: &PageGeometry, pointer: &str) -> Vec<PartName> {
    node(geometry, pointer)
        .parts
        .iter()
        .map(|part| part.name)
        .collect()
}

fn text_part_names(geometry: &PageGeometry, pointer: &str) -> Vec<PartName> {
    node(geometry, pointer)
        .parts
        .iter()
        .filter(|part| part.text.is_some())
        .map(|part| part.name)
        .collect()
}

#[test]
fn every_node_tag_is_present() {
    let geometry = every_tag_geometry();
    let all = [
        NodeTag::Page,
        NodeTag::Kicker,
        NodeTag::Title,
        NodeTag::Lede,
        NodeTag::Body,
        NodeTag::Legend,
        NodeTag::LegendEntry,
        NodeTag::Foot,
        NodeTag::Row,
        NodeTag::Col,
        NodeTag::Zone,
        NodeTag::Pcard,
        NodeTag::Fact,
        NodeTag::Note,
        NodeTag::Pipe,
        NodeTag::Tee,
        NodeTag::Text,
        NodeTag::Callout,
        NodeTag::Frame,
    ];
    for tag in all {
        assert!(
            geometry.nodes.iter().any(|node| node.tag == tag),
            "{tag:?} missing"
        );
    }
}

#[test]
fn parts_follow_section_2_11_order_and_text_runs() {
    use PartName::*;
    let geometry = every_tag_geometry();
    let pipe_with_sub = vec![DotStart, WireStart, Tag, TagLabel, TagSub, WireEnd, DotEnd];
    let pipe_without_sub = vec![DotStart, WireStart, Tag, TagLabel, WireEnd, DotEnd];
    let region = "/body/0/children/0/children/0";
    let gcp = "/body/0/children/0/children/1";
    let expected: Vec<(&str, Vec<PartName>, Vec<PartName>)> = vec![
        ("", vec![], vec![]),
        (
            "/kicker",
            vec![Badge, BadgeText, Text],
            vec![BadgeText, Text],
        ),
        ("/title", vec![Text], vec![Text]),
        ("/lede", vec![Text], vec![Text]),
        ("/body", vec![], vec![]),
        ("/body/0", vec![], vec![]),
        ("/body/0/children/0", vec![], vec![]),
        (region, vec![Label], vec![Label]),
        (
            "/body/0/children/0/children/0/children/0",
            vec![
                Icon,
                Text,
                FunctionName,
                ProductName,
                FactBox,
                Fact,
                AskBox,
                Ask,
            ],
            vec![FunctionName, ProductName, Fact, Ask],
        ),
        (gcp, vec![Bar, Label, Body], vec![Label]),
        (
            "/body/0/children/0/children/1/children/0",
            vec![Text, FunctionName],
            vec![FunctionName],
        ),
        ("/body/0/children/1", vec![Text], vec![Text]),
        ("/body/0/children/2", vec![Text], vec![Text]),
        ("/body/1", pipe_with_sub, vec![TagLabel, TagSub]),
        ("/body/2", pipe_without_sub.clone(), vec![TagLabel]),
        ("/body/3", vec![Spine, Hub, HubText], vec![HubText]),
        ("/body/3/arms/0", pipe_without_sub.clone(), vec![TagLabel]),
        ("/body/3/arms/1", pipe_without_sub, vec![TagLabel]),
        (
            "/body/4",
            vec![Heading, Marker, BodyLine, Marker, BodyLine],
            vec![Heading, Marker, BodyLine, Marker, BodyLine],
        ),
        ("/body/5", vec![Accent, Heading, Text], vec![Heading, Text]),
        ("/body/6", vec![LabelChip, Label], vec![Label]),
        ("/legend", vec![], vec![]),
        (
            "/legend/0",
            vec![Swatch, LegendLabel, LegendText],
            vec![LegendLabel, LegendText],
        ),
        ("/foot", vec![Text], vec![Text]),
    ];
    assert_eq!(geometry.nodes.len(), expected.len());
    for (pointer, parts, text_parts) in &expected {
        assert_eq!(
            &part_names(&geometry, pointer),
            parts,
            "parts of {pointer:?}"
        );
        assert_eq!(
            &text_part_names(&geometry, pointer),
            text_parts,
            "text parts of {pointer:?}"
        );
    }
}

#[test]
fn text_leaf_nodes_carry_one_text_part_equal_to_their_border_box() {
    let geometry = every_tag_geometry();
    for pointer in ["/title", "/lede", "/foot", "/body/0/children/1"] {
        let text_node = node(&geometry, pointer);
        assert_eq!(text_node.parts.len(), 1);
        assert_eq!(text_node.parts[0].bounds, text_node.bounds, "{pointer}");
    }
}

#[test]
fn kinds_are_set_for_zone_pipe_tee_and_legend_entry_only() {
    let geometry = every_tag_geometry();
    for geometry_node in &geometry.nodes {
        let expects_kind = matches!(
            geometry_node.tag,
            NodeTag::Zone | NodeTag::Pipe | NodeTag::Tee | NodeTag::LegendEntry
        );
        assert_eq!(
            geometry_node.kind.is_some(),
            expects_kind,
            "{}",
            geometry_node.pointer
        );
    }
    assert_eq!(
        node(&geometry, "/body/0/children/0/children/0")
            .kind
            .as_deref(),
        Some("onprem-a")
    );
    assert_eq!(node(&geometry, "/body/3").kind.as_deref(), Some("deny"));
    assert_eq!(
        node(&geometry, "/body/3/arms/1").kind.as_deref(),
        Some("pink")
    );
    assert_eq!(node(&geometry, "/legend/0").kind.as_deref(), Some("blue"));
}

#[test]
fn every_text_run_is_measured_at_its_final_width() {
    let geometry = every_tag_geometry();
    let mut measurer = FixedMetricsMeasurer::default();
    let mut runs = 0;
    for geometry_node in &geometry.nodes {
        for part in &geometry_node.parts {
            let Some(run) = &part.text else { continue };
            runs += 1;
            let expected = measurer
                .measure(
                    &run.text,
                    &run.style,
                    Some(part.bounds.width + WRAP_EPSILON_PX),
                )
                .unwrap();
            assert_eq!(
                run.metrics, expected,
                "{} {:?}",
                geometry_node.pointer, part.name
            );
        }
    }
    assert_eq!(runs, 30);
}

#[test]
fn tag_label_tag_sub_hub_text_and_frame_label_are_centered() {
    let geometry = every_tag_geometry();
    for geometry_node in &geometry.nodes {
        for part in &geometry_node.parts {
            let Some(run) = &part.text else { continue };
            let expected = match (part.name, geometry_node.tag) {
                (PartName::TagLabel | PartName::TagSub | PartName::HubText, _)
                | (PartName::Label, NodeTag::Frame) => TextAlign::Center,
                _ => TextAlign::Start,
            };
            assert_eq!(
                run.align, expected,
                "{} {:?}",
                geometry_node.pointer, part.name
            );
        }
    }
}

#[test]
fn deny_tee_hub_text_is_red_and_other_hubs_are_dark() {
    let geometry = every_tag_geometry();
    let hub_text = node(&geometry, "/body/3").part(PartName::HubText).unwrap();
    assert_eq!(hub_text.text.as_ref().unwrap().color, "#C5221F");
    let arm_label = node(&geometry, "/body/3/arms/0")
        .part(PartName::TagLabel)
        .unwrap();
    assert_eq!(arm_label.text.as_ref().unwrap().color, "#202124");
}

#[test]
fn legend_label_names_the_kind() {
    let geometry = every_tag_geometry();
    let label = node(&geometry, "/legend/0")
        .part(PartName::LegendLabel)
        .unwrap();
    assert_eq!(label.text.as_ref().unwrap().text, "Solid blue");
    let swatch = node(&geometry, "/legend/0").part(PartName::Swatch).unwrap();
    assert_eq!((swatch.bounds.width, swatch.bounds.height), (26.0, 2.0));
}
