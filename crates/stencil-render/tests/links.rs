#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Link drawing and the measured `links` (section 11.2).

mod common;

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{LinkRoute, PartName, RouteStatus};
use stencil_model::PagePoint;
use stencil_render::palette::Palette;
use stencil_render::{DeviceScale, measured_json, render_png};

const ONEPAGER_JSON: &str = include_str!("../../../examples/onepager.json");

fn card(id: &str) -> Value {
    json!({ "tag": "Pcard", "id": id, "fn": id })
}

/// Three cards in a row: a blue `end` link a to b with a label and sub, a gray `start` link
/// b to c without a label, and a gray `none` link a to c routed around b.
fn linked_document(theme: &str) -> Value {
    let mut document = common::page_document(
        json!([{ "tag": "Row", "gap": 64, "children": [card("a"), card("b"), card("c")] }]),
        json!([
            { "kind": "blue", "text": "blue" },
            { "kind": "gray", "text": "gray" }
        ]),
    );
    document["theme"] = json!(theme);
    document["links"] = json!([
        { "from": "a", "to": "b", "kind": "blue", "label": "call", "sub": "HTTPS" },
        { "from": "b", "to": "c", "kind": "gray", "arrow": "start" },
        { "from": "a", "to": "c", "kind": "gray", "arrow": "none", "from_side": "top", "to_side": "top" }
    ]);
    document
}

fn link_group<'a, 'input>(
    document: &'a roxmltree::Document<'input>,
    index: usize,
) -> roxmltree::Node<'a, 'input> {
    common::group(document, &format!("/links/{index}"))
}

/// The points of a `M x y L x y ...` path.
fn path_points(path: &str) -> Vec<PagePoint> {
    let numbers: Vec<f32> = path
        .split(' ')
        .filter(|token| *token != "M" && *token != "L")
        .map(|token| token.parse().unwrap())
        .collect();
    numbers
        .chunks(2)
        .map(|pair| PagePoint {
            x: pair[0],
            y: pair[1],
        })
        .collect()
}

fn number(node: roxmltree::Node<'_, '_>, attribute: &str) -> f32 {
    node.attribute(attribute).unwrap().parse().unwrap()
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 0.006
}

fn assert_point(actual: PagePoint, expected: PagePoint, what: &str) {
    assert!(
        close(actual.x, expected.x) && close(actual.y, expected.y),
        "{what}: {actual:?} is not {expected:?}"
    );
}

fn route(rendered: &common::Rendered, index: usize) -> &LinkRoute {
    rendered
        .geometry
        .links
        .iter()
        .find(|route| route.index == index)
        .unwrap()
}

#[test]
fn a_link_draws_its_path_the_marker_at_the_arrow_end_and_its_tag() {
    let rendered = common::render_document_with_fixed_metrics(linked_document("center"));
    let document = common::parse_xml(&rendered.svg.svg);
    let link = link_group(&document, 0);
    assert_eq!(link.attribute("data-tag"), Some("Link"));
    assert_eq!(link.attribute("data-kind"), Some("blue"));
    let routed = route(&rendered, 0);
    assert_eq!(routed.status, RouteStatus::Routed);

    let path = common::children_named(link, "path")[0];
    assert_eq!(path.attribute("stroke"), Some("#1A73E8"));
    assert_eq!(path.attribute("stroke-width"), Some("2"));
    assert_eq!(path.attribute("fill"), Some("none"));
    let drawn = path_points(path.attribute("d").unwrap());
    assert_eq!(drawn.len(), routed.points.len());
    let tip = *routed.points.last().unwrap();
    let before_tip = routed.points[routed.points.len() - 2];
    assert_point(drawn[0], routed.points[0], "path starts on the from edge");
    let segment_length = (tip.x - before_tip.x).abs() + (tip.y - before_tip.y).abs();
    assert!(segment_length > 10.0, "{segment_length}");
    let direction_x = (tip.x - before_tip.x) / segment_length;
    let direction_y = (tip.y - before_tip.y) / segment_length;
    let base = PagePoint {
        x: tip.x - direction_x * 10.0,
        y: tip.y - direction_y * 10.0,
    };
    assert_point(
        *drawn.last().unwrap(),
        base,
        "stroke ends at the arrowhead base",
    );

    let carriers = common::children_named(link, "line");
    assert_eq!(carriers.len(), 1, "one arrowhead for arrow end");
    let carrier = carriers[0];
    assert_eq!(carrier.attribute("stroke"), Some("none"));
    assert_eq!(
        carrier.attribute("marker-end"),
        Some("url(#arrow-center-blue)")
    );
    assert_point(
        PagePoint {
            x: number(carrier, "x2"),
            y: number(carrier, "y2"),
        },
        tip,
        "arrow tip on the to edge",
    );
    assert_point(
        PagePoint {
            x: number(carrier, "x1"),
            y: number(carrier, "y1"),
        },
        base,
        "arrow base",
    );
    let marker = document
        .descendants()
        .find(|node| {
            node.has_tag_name("marker") && node.attribute("id") == Some("arrow-center-blue")
        })
        .expect("the blue marker is defined");
    let triangle = common::children_named(marker, "path")[0];
    assert_eq!(triangle.attribute("fill"), Some("#1A73E8"));

    let tag = routed.tag.unwrap();
    let rect = common::children_named(link, "rect")[0];
    assert!(close(number(rect, "x"), tag.x + 0.75));
    assert!(close(number(rect, "y"), tag.y + 0.75));
    assert!(close(number(rect, "width"), tag.width - 1.5));
    assert_eq!(rect.attribute("fill"), Some("#FFFFFF"));
    assert_eq!(rect.attribute("stroke"), Some("#DADCE0"));
    let texts: Vec<&str> = common::children_named(link, "text")
        .iter()
        .map(|text| text.text().unwrap())
        .collect();
    assert_eq!(texts, ["call", "HTTPS"]);
    let elements: Vec<&str> = link
        .children()
        .filter(|child| child.is_element())
        .map(|child| child.tag_name().name())
        .collect();
    assert_eq!(
        elements,
        ["path", "line", "rect", "text", "text"],
        "tag over path"
    );
}

#[test]
fn arrow_start_marks_the_from_end_and_none_draws_no_marker() {
    let rendered = common::render_document_with_fixed_metrics(linked_document("wire"));
    let document = common::parse_xml(&rendered.svg.svg);

    let start_link = link_group(&document, 1);
    let carriers = common::children_named(start_link, "line");
    assert_eq!(carriers.len(), 1);
    assert_eq!(
        carriers[0].attribute("marker-end"),
        Some("url(#arrow-wire-gray)")
    );
    let first = route(&rendered, 1).points[0];
    assert_point(
        PagePoint {
            x: number(carriers[0], "x2"),
            y: number(carriers[0], "y2"),
        },
        first,
        "start arrow tip on the from edge",
    );
    assert!(
        common::children_named(start_link, "rect").is_empty(),
        "no label, no tag"
    );
    let path = common::children_named(start_link, "path")[0];
    assert_eq!(path.attribute("stroke"), Some("#222222"));
    assert_eq!(path.attribute("stroke-width"), Some("1.25"));

    let plain_link = link_group(&document, 2);
    assert!(common::children_named(plain_link, "line").is_empty());
    let drawn = path_points(
        common::children_named(plain_link, "path")[0]
            .attribute("d")
            .unwrap(),
    );
    let routed = &route(&rendered, 2).points;
    assert_eq!(drawn.len(), routed.len());
    for (drawn_point, routed_point) in drawn.iter().zip(routed) {
        assert_point(*drawn_point, *routed_point, "untrimmed without arrows");
    }
}

#[test]
fn links_are_drawn_after_every_node() {
    let rendered = common::render_document_with_fixed_metrics(linked_document("dusk"));
    let document = common::parse_xml(&rendered.svg.svg);
    let root = document.root_element();
    let groups: Vec<&str> = common::children_named(root, "g")
        .iter()
        .map(|group| group.attribute("data-id").unwrap())
        .collect();
    assert_eq!(groups, ["", "/links/0", "/links/1", "/links/2"]);
    let dusk_blue = Palette::new(stencil_model::Theme::Dusk)
        .wire_style(stencil_model::PipeKind::Blue)
        .stroke
        .color;
    let path = common::children_named(link_group(&document, 0), "path")[0];
    assert_eq!(path.attribute("stroke"), Some(dusk_blue));
}

#[test]
fn a_fallback_route_is_still_drawn() {
    let grid_row = |ids: [&str; 3]| json!({ "tag": "Row", "gap": 0, "children": [card(ids[0]), card(ids[1]), card(ids[2])] });
    let mut document = common::page_document(
        json!([{ "tag": "Col", "gap": 64, "children": [
            { "tag": "Col", "gap": 0, "children": [
                grid_row(["n1", "n2", "n3"]),
                grid_row(["w", "center", "e"]),
                grid_row(["s1", "s2", "s3"])
            ]},
            card("target")
        ]}]),
        json!([{ "kind": "blue", "text": "blue" }]),
    );
    document["links"] = json!([{ "from": "center", "to": "target", "kind": "blue" }]);
    let rendered = common::render_document_with_fixed_metrics(document);
    let routed = route(&rendered, 0);
    assert_eq!(routed.status, RouteStatus::Fallback);
    let svg = common::parse_xml(&rendered.svg.svg);
    let link = link_group(&svg, 0);
    let drawn = path_points(
        common::children_named(link, "path")[0]
            .attribute("d")
            .unwrap(),
    );
    assert_eq!(drawn.len(), routed.points.len());
    assert_eq!(common::children_named(link, "line").len(), 1, "arrowhead");
}

#[test]
fn measured_json_lists_links_with_points_tag_and_status() {
    let document = linked_document("center");
    let rendered = common::render_document_with_fixed_metrics(document.clone());
    let measured = measured_json(&document, &rendered.geometry);
    let links = measured["links"].as_array().unwrap();
    assert_eq!(links.len(), 3);
    for (entry, routed) in links.iter().zip(&rendered.geometry.links) {
        assert_eq!(entry["id"], format!("/links/{}", routed.index));
        assert_eq!(entry["status"], "routed");
        let from = &rendered.geometry.nodes[routed.from_node];
        assert_eq!(entry["from"], from.pointer.as_str());
        let points = entry["points"].as_array().unwrap();
        assert_eq!(points.len(), routed.points.len());
        for (point, expected) in points.iter().zip(&routed.points) {
            assert!((point["x"].as_f64().unwrap() - f64::from(expected.x)).abs() <= 0.005);
            assert!((point["y"].as_f64().unwrap() - f64::from(expected.y)).abs() <= 0.005);
        }
    }
    let tagged = &links[0];
    assert_eq!(tagged["kind"], "blue");
    assert_eq!(tagged["tag"], tagged["parts"]["tag"]);
    assert_eq!(tagged["parts"]["tag_label"]["line_count"], 1);
    assert!(tagged["parts"].get("tag_sub").is_some());
    let untagged = &links[1];
    assert!(untagged.get("tag").is_none());
    assert_eq!(untagged["parts"], json!({}));
    let tag_part = rendered.geometry.links[0]
        .parts
        .iter()
        .find(|part| part.name == PartName::Tag)
        .unwrap();
    assert_eq!(Some(tag_part.bounds), rendered.geometry.links[0].tag);
}

#[test]
fn a_page_without_links_writes_no_links_key() {
    let rendered = common::render_g7();
    let measured = measured_json(&common::g7_document(), &rendered.geometry);
    assert!(measured.get("links").is_none());
}

#[test]
fn onepager_links_render_to_png_in_every_theme() {
    for theme in ["center", "dusk", "wire"] {
        let mut document: Value = serde_json::from_str(ONEPAGER_JSON).unwrap();
        document["theme"] = json!(theme);
        let page: stencil_model::Page = serde_json::from_value(document).unwrap();
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = stencil_render::render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        for index in 0..page.links.len() {
            let link = link_group(&parsed, index);
            assert_eq!(
                common::children_named(link, "line").len(),
                1,
                "{theme} {index}"
            );
        }
        render_png(&svg.svg, svg.text_elements, DeviceScale::new(1).unwrap()).unwrap();
    }
}
