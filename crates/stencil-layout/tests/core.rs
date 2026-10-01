#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Layout of the core vocabulary (section 13.14): Boxes from their container kinds, Item
//! facts by source, chrome, and Lanes.

mod common;

use common::{assert_close, layout, node, page_from, page_with_body, part, plain};
use serde_json::{Value, json};
use stencil_layout::{NodeTag, PageGeometry, PartName, layout_page};
use stencil_model::grammar::{Role, Tone};
use stencil_model::text::FixedMetricsMeasurer;

fn single_box(kind: &str, tint: Option<u8>) -> PageGeometry {
    let mut box_node = json!({
        "tag": "Box", "kind": kind, "label": "Zone label",
        "children": [{ "tag": "Item", "kind": "product", "title": "Inside" }, { "tag": "Fact", "text": "Second" }]
    });
    if let Some(tint) = tint {
        box_node["tint"] = json!(tint);
    }
    layout(&page_with_body(1280, json!([box_node])))
}

/// Bounds and content box of `/body/0` and its two children.
fn box_boxes(geometry: &PageGeometry) -> Vec<[f32; 4]> {
    ["/body/0", "/body/0/children/0", "/body/0/children/1"]
        .iter()
        .flat_map(|pointer| {
            let node = node(geometry, pointer);
            [node.bounds, node.content]
        })
        .map(|rect| [rect.x, rect.y, rect.width, rect.height])
        .collect()
}

#[test]
fn a_region_of_any_tint_and_an_untinted_onprem_lay_out_as_region_tint_1() {
    let reference = single_box("region", Some(1));
    for (kind, tint) in [("region", Some(8)), ("region", None), ("onprem", None)] {
        let geometry = single_box(kind, tint);
        assert_eq!(
            box_boxes(&geometry),
            box_boxes(&reference),
            "{kind} {tint:?}"
        );
    }
}

#[test]
fn an_apis_box_lays_out_like_a_region() {
    assert_eq!(
        box_boxes(&single_box("apis", None)),
        box_boxes(&single_box("region", Some(1)))
    );
}

#[test]
fn a_box_records_its_key_tint_and_container_look() {
    let region = single_box("region", None);
    let zone = node(&region, "/body/0");
    assert_eq!(zone.tag, NodeTag::Zone);
    assert_eq!(zone.kind.as_deref(), Some("region-a"));
    assert_eq!(zone.tint, Some(1));
    let look = zone.container.unwrap();
    assert_eq!((look.role, look.tone), (Role::Group, Some(Tone::Neutral)));
    assert_eq!((look.border_width, look.radius), (1.5, 8.0));

    let region_h = single_box("region", Some(8));
    assert_eq!(node(&region_h, "/body/0").kind.as_deref(), Some("region-h"));
    let subnet = single_box("subnet", Some(3));
    let subnet_node = node(&subnet, "/body/0");
    assert_eq!(subnet_node.kind.as_deref(), Some("subnet"));
    assert_eq!(subnet_node.tint, None);
}

#[test]
fn plain_kinds_take_the_plain_grammar_padding_and_border() {
    let page = page_from(json!({
        "title": "Plain", "kicker": "Kicker", "lede": "Lede", "canvas": "internal",
        "grammar": "plain",
        "body": [
            { "tag": "Box", "kind": "boundary", "label": "Trust edge", "children": [
                { "tag": "Box", "kind": "group", "tint": 2, "label": "Site", "children": [
                    { "tag": "Item", "kind": "service", "title": "API" }
                ] }
            ] }
        ],
        "legend": []
    }));
    let geometry = layout_page(&page, &plain(), &mut FixedMetricsMeasurer::default()).unwrap();
    let boundary = node(&geometry, "/body/0");
    assert_close(
        boundary.content.x,
        boundary.bounds.x + 2.0 + 10.0,
        "boundary inset",
    );
    let group = node(&geometry, "/body/0/children/0");
    assert_close(group.content.x, group.bounds.x + 1.5 + 12.0, "group inset");
    assert_eq!(group.kind.as_deref(), Some("group-b"));
    assert_eq!(group.container.unwrap().radius, 8.0);
}

fn item_with_facts(facts: Value) -> PageGeometry {
    layout(&page_with_body(
        1280,
        json!([{ "tag": "Item", "kind": "product", "title": "Bucket", "subtitle": "Cloud Storage", "facts": facts }]),
    ))
}

#[test]
fn item_facts_add_their_parts_in_list_order_by_source() {
    let geometry = item_with_facts(json!([
        { "text": "dual-region" },
        { "text": "acme-raw", "source": "built" },
        { "text": "retention?", "source": "ask" }
    ]));
    let item = node(&geometry, "/body/0");
    let names: Vec<PartName> = item.parts.iter().map(|part| part.name).collect();
    assert_eq!(
        names,
        [
            PartName::Text,
            PartName::FunctionName,
            PartName::ProductName,
            PartName::FactBox,
            PartName::Fact,
            PartName::BuiltBox,
            PartName::Built,
            PartName::AskBox,
            PartName::Ask
        ]
    );
    let built = part(item, PartName::Built).text.as_ref().unwrap();
    assert_eq!(built.text, "\u{2022} acme-raw");
    assert_eq!(
        built.style,
        part(item, PartName::Fact).text.as_ref().unwrap().style
    );
    let ask = part(item, PartName::Ask).text.as_ref().unwrap();
    assert_eq!(ask.text, "Ask: retention?");
}

#[test]
fn a_fact_node_takes_the_style_and_prefix_of_its_source() {
    let geometry = layout(&page_with_body(
        1280,
        json!([
            { "tag": "Fact", "text": "read from the doc" },
            { "tag": "Fact", "text": "vlan-104", "source": "built" },
            { "tag": "Fact", "text": "which zone?", "source": "ask" }
        ]),
    ));
    let runs: Vec<(String, &str)> = ["/body/0", "/body/1", "/body/2"]
        .iter()
        .map(|pointer| {
            let run = part(node(&geometry, pointer), PartName::Text)
                .text
                .as_ref()
                .unwrap();
            (run.text.clone(), run.color)
        })
        .collect();
    assert_eq!(
        runs,
        [
            ("read from the doc".to_string(), "#5F6368"),
            ("\u{2022} vlan-104".to_string(), "#5F6368"),
            ("Ask: which zone?".to_string(), "#B06000")
        ]
    );
}

fn chrome_none_page() -> Value {
    json!({
        "title": "Captioned", "kicker": "Kicker", "lede": "Lede", "foot": "Foot",
        "canvas": "customer", "chrome": "none",
        "body": [{ "tag": "Pipe", "dir": "h", "line": "solid", "label": "hop" }],
        "legend": [{ "line": "solid", "text": "request path" }]
    })
}

#[test]
fn chrome_none_lays_out_body_legend_and_foot_only() {
    let geometry = layout(&page_from(chrome_none_page()));
    let pointers: Vec<&str> = geometry
        .nodes
        .iter()
        .map(|node| node.pointer.as_str())
        .collect();
    assert_eq!(
        pointers,
        ["", "/body", "/body/0", "/legend", "/legend/0", "/foot"]
    );
    assert_close(node(&geometry, "/body").bounds.y, 20.0, "body y");
    let tags: Vec<NodeTag> = geometry.nodes.iter().map(|node| node.tag).collect();
    for absent in [NodeTag::Kicker, NodeTag::Title, NodeTag::Lede] {
        assert!(!tags.contains(&absent), "{absent:?}");
    }
    let report = stencil_layout::checks::text_fits_box(&geometry);
    assert!(report.passed());
    let full = layout(&page_from({
        let mut document = chrome_none_page();
        document.as_object_mut().unwrap().remove("chrome");
        document
    }));
    let full_report = stencil_layout::checks::text_fits_box(&full);
    assert_eq!(full_report.examined, report.examined + 4);
}

#[test]
fn lanes_lay_their_heads_out_as_equal_columns_32_apart() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Lanes", "children": [
            { "tag": "Item", "kind": "product", "title": "Client" },
            { "tag": "Item", "kind": "product", "title": "A much longer service name" },
            { "tag": "Item", "kind": "product", "title": "Store" }
        ]}]),
    ));
    let lanes = node(&geometry, "/body/0");
    assert_eq!(lanes.tag, NodeTag::Lanes);
    assert_eq!(lanes.kind, None);
    let heads: Vec<_> = (0..3)
        .map(|index| node(&geometry, &format!("/body/0/children/{index}")).bounds)
        .collect();
    for head in &heads {
        assert_close(head.width, heads[0].width, "equal lane width");
    }
    for pair in heads.windows(2) {
        assert_close(pair[1].x - pair[0].right(), 32.0, "lane gap");
    }
}

/// Three lanes `a b c` and four ordered messages written out of order: link 0 has order 3,
/// link 1 order 1 without a label, link 2 order 2 with a sub, link 3 order 4.
fn lanes_with_messages() -> Value {
    json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede",
        "canvas": "internal", "grammar": "plain",
        "body": [{ "tag": "Lanes", "children": [
            { "tag": "Item", "id": "a", "kind": "person", "title": "Client" },
            { "tag": "Item", "id": "b", "kind": "service", "title": "A much longer service name" },
            { "tag": "Item", "id": "c", "kind": "store", "title": "Store" }
        ]}],
        "legend": [{ "line": "solid", "text": "call" }],
        "links": [
            { "from": "a", "to": "b", "line": "solid", "label": "three", "order": 3 },
            { "from": "b", "to": "c", "line": "solid", "order": 1 },
            { "from": "c", "to": "a", "line": "solid", "label": "two", "sub": "with a sub", "order": 2 },
            { "from": "a", "to": "c", "line": "solid", "label": "four", "order": 4 }
        ]
    })
}

#[test]
fn lanes_draw_messages_in_order_across_a_band_of_rows_with_lifelines() {
    let page = page_from(lanes_with_messages());
    let geometry = layout(&page);
    let lanes = node(&geometry, "/body/0");
    let heads: Vec<_> = (0..3)
        .map(|index| node(&geometry, &format!("/body/0/children/{index}")).bounds)
        .collect();
    for head in &heads {
        assert_close(head.width, heads[0].width, "equal lane width");
        assert_close(head.height, heads[0].height, "equal lane height");
    }
    for pair in heads.windows(2) {
        assert_close(pair[1].x - pair[0].right(), 32.0, "lane gap");
    }
    let parts: Vec<PartName> = lanes.parts.iter().map(|part| part.name).collect();
    assert_eq!(
        parts,
        [
            PartName::Heads,
            PartName::Band,
            PartName::Lifeline,
            PartName::Lifeline,
            PartName::Lifeline
        ]
    );
    assert!(lanes.parts.iter().all(|part| part.text.is_none()));
    let band = part(lanes, PartName::Band).bounds;
    assert_close(band.y, heads[0].bottom(), "band top");
    assert_close(band.width, lanes.bounds.width, "band width");
    assert_close(band.bottom(), lanes.bounds.bottom(), "band bottom");

    // Rows top to bottom: link 1, link 2, link 0, link 3.
    let mut row_top = band.y;
    let mut row_heights = Vec::new();
    for link_index in [1, 2, 0, 3] {
        let route = &geometry.links[link_index];
        assert_eq!(route.points.len(), 2, "link {link_index}");
        let (start, end) = (route.points[0], route.points[1]);
        assert_close(start.y, end.y, "a message is horizontal");
        let row_height = 2.0 * (start.y - row_top);
        assert!(row_height >= 36.0 - 0.01, "row of link {link_index}");
        row_heights.push(row_height);
        row_top += row_height;
        let from = &geometry.nodes[route.from_node].bounds;
        let to = &geometry.nodes[route.to_node].bounds;
        assert_close(start.x, from.x + from.width / 2.0, "from head center");
        assert_close(end.x, to.x + to.width / 2.0, "to head center");
        if let Some(tag) = route.tag {
            assert_close(tag.y + tag.height / 2.0, start.y, "tag on the message");
            assert_close(
                tag.x + tag.width / 2.0,
                (start.x + end.x) / 2.0,
                "tag centered",
            );
        }
    }
    assert_close(row_top, band.bottom(), "rows fill the band");
    assert_eq!(geometry.links[1].tag, None);
    assert_close(row_heights[0], 36.0, "a message without a label");
    let sub_tag = geometry.links[2].tag.unwrap();
    assert!(sub_tag.height + 8.0 > 36.0);
    assert_close(row_heights[1], sub_tag.height + 8.0, "a sub widens its row");

    for (index, lifeline) in lanes
        .parts
        .iter()
        .filter(|part| part.name == PartName::Lifeline)
        .enumerate()
    {
        let head = heads[index];
        assert_close(lifeline.bounds.x, head.x + head.width / 2.0, "lifeline x");
        assert_close(lifeline.bounds.y, head.bottom(), "lifeline top");
        assert_close(lifeline.bounds.width, 0.0, "lifeline width");
        assert_close(lifeline.bounds.bottom(), band.bottom(), "lifeline bottom");
    }

    let routed = stencil_layout::checks::links_routed(&geometry);
    assert!(routed.passed());
    assert_eq!(routed.examined, 4);
    let avoid = stencil_layout::checks::links_avoid_boxes(&geometry);
    assert!(avoid.passed(), "{:?}", avoid.defects);
    assert!(avoid.examined > 0);
    let legend = stencil_model::checks::legend_consistency(&page);
    assert!(legend.passed());
    assert_eq!(legend.examined, 5, "four messages and one legend entry");
    assert!(stencil_layout::checks::child_inside_container(&geometry).passed());
    assert!(stencil_layout::checks::siblings_do_not_overlap(&geometry).passed());
}

#[test]
fn lanes_without_messages_have_a_band_of_height_0() {
    let mut document = lanes_with_messages();
    document["links"] = json!([]);
    document["legend"] = json!([]);
    let geometry = layout(&page_from(document));
    let lanes = node(&geometry, "/body/0");
    let band = part(lanes, PartName::Band).bounds;
    assert_close(band.height, 0.0, "band height");
    let head = node(&geometry, "/body/0/children/0").bounds;
    assert_close(
        lanes.bounds.bottom(),
        head.bottom(),
        "lanes end at the heads",
    );
    let lifelines = lanes
        .parts
        .iter()
        .filter(|part| part.name == PartName::Lifeline)
        .count();
    assert_eq!(lifelines, 3);
}

#[test]
fn an_unordered_link_between_lane_heads_is_routed_by_the_router() {
    let mut document = lanes_with_messages();
    document["links"] = json!([{ "from": "a", "to": "b", "line": "solid" }]);
    let geometry = layout(&page_from(document));
    let lanes = node(&geometry, "/body/0");
    assert_close(
        part(lanes, PartName::Band).bounds.height,
        0.0,
        "band height",
    );
    let route = &geometry.links[0];
    let from = &geometry.nodes[route.from_node].bounds;
    assert_close(route.points[0].x, from.right(), "leaves the right side");
}
