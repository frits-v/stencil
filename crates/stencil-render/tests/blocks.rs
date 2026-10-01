#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Text, Callout and Frame drawing and their measured parts (section 11.3).

mod common;

use common::THEMES;
use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{BoxRect, PartName};
use stencil_model::Projection;
use stencil_model::pointer::NodePointer;
use stencil_render::measured_json;
use stencil_render::palette::Palette;

/// Body: a numbered Text, a bulleted Text, a plain Text, a risk and a decision Callout, and
/// a Frame.
fn block_document(theme: &str) -> Value {
    let mut document = common::page_document(
        json!([{ "tag": "Col", "children": [
            { "tag": "Text", "heading": "Goals", "list": "numbered", "body": ["first", "second", "third"] },
            { "tag": "Text", "list": "bulleted", "body": ["one", "two"] },
            { "tag": "Text", "heading": "Plain", "body": ["only line"] },
            { "tag": "Callout", "kind": "risk", "title": "Risk title", "text": "risk text" },
            { "tag": "Callout", "kind": "decision", "text": "decision text" },
            { "tag": "Frame", "label": "Screen", "height": 120 }
        ]}]),
        json!([]),
    );
    document["theme"] = json!(theme);
    document
}

fn block_pointer(index: usize) -> String {
    format!("/body/0/children/{index}")
}

fn node_bounds(rendered: &common::Rendered, pointer: &str) -> BoxRect {
    let mut node_pointer = NodePointer::root();
    for token in pointer.trim_start_matches('/').split('/') {
        node_pointer = node_pointer.child(token);
    }
    rendered.geometry.node(&node_pointer).unwrap().bounds
}

fn number(node: roxmltree::Node<'_, '_>, attribute: &str) -> f32 {
    node.attribute(attribute).unwrap().parse().unwrap()
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 0.006
}

fn texts<'a>(group: roxmltree::Node<'a, '_>) -> Vec<&'a str> {
    common::children_named(group, "text")
        .iter()
        .map(|text| text.text().unwrap())
        .collect()
}

#[test]
fn text_blocks_draw_box_heading_lines_and_markers() {
    for name in THEMES {
        let theme = common::theme(name);
        let palette = Palette::new(&theme, Projection::Flat);
        let rendered = common::render_document_with_fixed_metrics(block_document(name));
        let document = common::parse_xml(&rendered.svg.svg);

        let numbered = common::group(&document, &block_pointer(0));
        let block_box = common::children_named(numbered, "rect")[0];
        let card = palette.card();
        assert_eq!(block_box.attribute("fill"), Some(card.fill), "{name}");
        assert_eq!(
            block_box.attribute("stroke"),
            Some(card.border.unwrap().color),
            "{name}"
        );
        assert_eq!(block_box.attribute("stroke-width"), Some("1.25"), "{name}");
        assert_eq!(
            texts(numbered),
            ["Goals", "1.", "first", "2.", "second", "3.", "third"],
            "{name}"
        );

        let bulleted = common::group(&document, &block_pointer(1));
        assert_eq!(texts(bulleted), ["one", "two"], "{name}");
        let bullets = common::children_named(bulleted, "circle");
        assert_eq!(bullets.len(), 2, "{name}");
        let markers: Vec<BoxRect> = rendered
            .geometry
            .node(
                &NodePointer::root()
                    .child("body")
                    .index(0)
                    .child("children")
                    .index(1),
            )
            .unwrap()
            .parts
            .iter()
            .filter(|part| part.name == PartName::Marker)
            .map(|part| part.bounds)
            .collect();
        for (bullet, marker) in bullets.iter().zip(&markers) {
            assert_eq!(number(*bullet, "r"), 2.0);
            assert!(close(number(*bullet, "cx"), marker.x + 5.0));
            assert!(close(number(*bullet, "cy"), marker.y + marker.height / 2.0));
            assert_eq!(
                bullet.attribute("fill"),
                Some(palette.list_bullet(stencil_model::Canvas::Customer)),
                "{name}"
            );
        }

        let plain = common::group(&document, &block_pointer(2));
        assert_eq!(texts(plain), ["Plain", "only line"], "{name}");
        assert!(common::children_named(plain, "circle").is_empty());
    }
}

#[test]
fn callouts_draw_the_kind_tint_and_accent_bar() {
    for name in THEMES {
        let theme = common::theme(name);
        let fills = [&theme.callout.risk.fill, &theme.callout.decision.fill];
        let accents = [&theme.callout.risk.accent, &theme.callout.decision.accent];
        let rendered = common::render_document_with_fixed_metrics(block_document(name));
        let document = common::parse_xml(&rendered.svg.svg);
        for (offset, (fill, accent)) in fills.iter().zip(accents).enumerate() {
            let pointer = block_pointer(3 + offset);
            let callout = common::group(&document, &pointer);
            let callout_box = common::children_named(callout, "rect")[0];
            assert_eq!(
                callout_box.attribute("fill"),
                Some(fill.as_str()),
                "{name} {pointer}"
            );
            assert_eq!(
                callout_box.attribute("stroke"),
                Some(theme.card.border.color.as_str()),
                "{name} {pointer}"
            );
            let bar = common::children_named(callout, "path")[0];
            assert_eq!(
                bar.attribute("fill"),
                Some(accent.as_str()),
                "{name} {pointer}"
            );
        }
        let titled = common::group(&document, &block_pointer(3));
        assert_eq!(texts(titled), ["Risk title", "risk text"], "{name}");
        let untitled = common::group(&document, &block_pointer(4));
        assert_eq!(texts(untitled), ["decision text"], "{name}");
    }
}

#[test]
fn center_and_wire_callouts_keep_the_section_11_3_colors() {
    let center = common::theme("center");
    assert_eq!(center.callout.risk.fill.as_str(), "#FCE8E6");
    assert_eq!(center.callout.decision.fill.as_str(), "#E6F4EA");
    assert_eq!(center.callout.risk.accent.as_str(), "#C5221F");
    assert_eq!(center.callout.decision.accent.as_str(), "#188038");
    let wire = common::theme("wire");
    for accent in [
        &wire.callout.note,
        &wire.callout.risk,
        &wire.callout.decision,
        &wire.callout.open,
    ] {
        assert_eq!(accent.fill.as_str(), "#FFFFFF");
        assert_eq!(accent.accent.as_str(), "#222222");
    }
}

/// The endpoints of an `M`, `H`, `V`, `A` and `Z` path, in order.
fn path_vertices(path: &str) -> Vec<(f32, f32)> {
    let tokens: Vec<&str> = path.split(' ').collect();
    let mut vertices = Vec::new();
    let (mut x, mut y) = (0.0_f32, 0.0_f32);
    let mut index = 0;
    let value = |token: &str| -> f32 { token.parse().unwrap() };
    while index < tokens.len() {
        match tokens[index] {
            "M" => {
                x = value(tokens[index + 1]);
                y = value(tokens[index + 2]);
                index += 3;
            }
            "H" => {
                x = value(tokens[index + 1]);
                index += 2;
            }
            "V" => {
                y = value(tokens[index + 1]);
                index += 2;
            }
            "A" => {
                x = value(tokens[index + 6]);
                y = value(tokens[index + 7]);
                index += 8;
            }
            "Z" => {
                index += 1;
                continue;
            }
            other => panic!("unexpected path token {other}"),
        }
        vertices.push((x, y));
    }
    vertices
}

#[test]
fn the_callout_accent_bar_is_four_px_wide_inside_the_border() {
    let rendered = common::render_document_with_fixed_metrics(block_document("center"));
    let document = common::parse_xml(&rendered.svg.svg);
    for pointer in [block_pointer(3), block_pointer(4)] {
        let bounds = node_bounds(&rendered, &pointer);
        let bar = common::children_named(common::group(&document, &pointer), "path")[0];
        let vertices = path_vertices(bar.attribute("d").unwrap());
        assert_eq!(vertices.len(), 5, "{pointer}");
        let left = bounds.x + 1.25;
        let top = bounds.y + 1.25;
        let bottom = bounds.bottom() - 1.25;
        for (x, y) in &vertices {
            assert!(*x >= left - 0.006 && *x <= left + 4.006, "{pointer} x {x}");
            assert!(*y >= top - 0.006 && *y <= bottom + 0.006, "{pointer} y {y}");
        }
        let xs: Vec<f32> = vertices.iter().map(|(x, _)| *x).collect();
        assert!(
            xs.iter().any(|x| close(*x, left)),
            "{pointer} reaches the border"
        );
        assert!(
            xs.iter().any(|x| close(*x, left + 4.0)),
            "{pointer} is 4 px wide"
        );
        let ys: Vec<f32> = vertices.iter().map(|(_, y)| *y).collect();
        let span = ys.iter().cloned().fold(f32::MIN, f32::max)
            - ys.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            span > bounds.height - 2.5 - 2.0,
            "{pointer} runs top to bottom"
        );
    }
}

#[test]
fn a_frame_draws_two_diagonals_under_a_dashed_border_and_a_label_chip() {
    for name in THEMES {
        let theme = common::theme(name);
        let palette = Palette::new(&theme, Projection::Flat);
        let rendered = common::render_document_with_fixed_metrics(block_document(name));
        let document = common::parse_xml(&rendered.svg.svg);
        let pointer = block_pointer(5);
        let bounds = node_bounds(&rendered, &pointer);
        let frame = common::group(&document, &pointer);

        let diagonals = common::children_named(frame, "line");
        assert_eq!(diagonals.len(), 2, "{name}");
        let secondary = palette.frame_diagonal().color;
        let mut slopes = Vec::new();
        for diagonal in &diagonals {
            assert_eq!(diagonal.attribute("stroke"), Some(secondary), "{name}");
            assert_eq!(diagonal.attribute("stroke-width"), Some("1"), "{name}");
            let (x1, y1, x2, y2) = (
                number(*diagonal, "x1"),
                number(*diagonal, "y1"),
                number(*diagonal, "x2"),
                number(*diagonal, "y2"),
            );
            for (x, y) in [(x1, y1), (x2, y2)] {
                assert!(
                    x > bounds.x + 1.25 && x < bounds.x + 3.0
                        || x < bounds.right() - 1.25 && x > bounds.right() - 3.0,
                    "{name} x {x}"
                );
                assert!(
                    y > bounds.y + 1.25 && y < bounds.y + 3.0
                        || y < bounds.bottom() - 1.25 && y > bounds.bottom() - 3.0,
                    "{name} y {y}"
                );
            }
            slopes.push(((y2 - y1) / (x2 - x1)).signum());
        }
        assert_eq!(slopes, [1.0, -1.0], "{name}: one down, one up");

        let rects = common::children_named(frame, "rect");
        let border = rects[0];
        assert_eq!(border.attribute("fill"), Some("none"), "{name}");
        assert_eq!(border.attribute("stroke-dasharray"), Some("6 5"), "{name}");
        assert_eq!(border.attribute("stroke-width"), Some("1.25"), "{name}");
        let chip = rects[1];
        assert_eq!(
            chip.attribute("fill"),
            Some(palette.page_background()),
            "{name}"
        );

        let order: Vec<&str> = frame
            .children()
            .filter(|child| child.is_element())
            .map(|child| child.tag_name().name())
            .collect();
        assert_eq!(order, ["line", "line", "rect", "rect", "text"], "{name}");
        assert_eq!(texts(frame), ["Screen"], "{name}");
    }
}

#[test]
fn measured_parts_key_each_body_line_and_marker_by_line() {
    let document = block_document("center");
    let rendered = common::render_document_with_fixed_metrics(document.clone());
    let measured = measured_json(&document, &rendered.geometry, None);
    let nodes = measured["nodes"].as_array().unwrap();
    let parts_of = |pointer: &str| {
        nodes.iter().find(|node| node["id"] == pointer).unwrap()["parts"]
            .as_object()
            .unwrap()
            .clone()
    };
    let numbered = parts_of(&block_pointer(0));
    let keys: Vec<&str> = numbered.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "body_line/0",
            "body_line/1",
            "body_line/2",
            "heading",
            "marker/0",
            "marker/1",
            "marker/2"
        ]
    );
    for key in ["body_line/0", "marker/0", "heading"] {
        assert_eq!(numbered[key]["line_count"], 1, "{key}");
    }
    let first_line_y = numbered["body_line/0"]["y"].as_f64().unwrap();
    let second_line_y = numbered["body_line/1"]["y"].as_f64().unwrap();
    assert!(second_line_y > first_line_y);

    let bulleted = parts_of(&block_pointer(1));
    assert!(
        bulleted["marker/1"].get("line_count").is_none(),
        "a bullet has no text"
    );
    let plain = parts_of(&block_pointer(2));
    assert!(plain.contains_key("body_line/0"));
    assert!(!plain.contains_key("marker/0"));

    let callout = parts_of(&block_pointer(3));
    let keys: Vec<&str> = callout.keys().map(String::as_str).collect();
    assert_eq!(keys, ["accent", "heading", "text"]);
    let frame = parts_of(&block_pointer(5));
    let keys: Vec<&str> = frame.keys().map(String::as_str).collect();
    assert_eq!(keys, ["label", "label_chip"]);
}

#[test]
fn every_block_part_is_accounted_for_in_measured_json() {
    let document = block_document("center");
    let rendered = common::render_document_with_fixed_metrics(document.clone());
    let measured = measured_json(&document, &rendered.geometry, None);
    let nodes = measured["nodes"].as_array().unwrap();
    let mut examined = 0;
    for (node, geometry_node) in nodes.iter().zip(&rendered.geometry.nodes) {
        assert_eq!(
            node["parts"].as_object().unwrap().len(),
            geometry_node.parts.len(),
            "{}",
            node["id"]
        );
        examined += geometry_node.parts.len();
    }
    assert!(examined > 20, "examined {examined} parts");
}
