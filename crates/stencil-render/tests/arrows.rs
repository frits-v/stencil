#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unreachable
)]

//! Pipe arrowheads (section 11.2, last paragraph of the Pipe change).

mod common;

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{BoxRect, PartName};
use stencil_model::pointer::NodePointer;
use stencil_render::{DeviceScale, render_png};

/// Body: an `end` blue Pipe h, a `start` pink Pipe h, a `both` dash Pipe v, a gray Pipe h
/// with no arrow, and a deny Tee whose first arm has an `end` arrow.
fn arrow_document(theme: &str) -> Value {
    let mut document = common::page_document(
        json!([
            {
                "tag": "Col",
                "children": [
                    { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "end", "arrow": "end" },
                    { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": "start", "arrow": "start" },
                    { "tag": "Pipe", "dir": "v", "line": "dash", "label": "both", "arrow": "both" },
                    { "tag": "Pipe", "dir": "h", "line": "gray", "label": "none" },
                    {
                        "tag": "Tee",
                        "line": "deny",
                        "hub": "hub",
                        "arms": [
                            { "tag": "Pipe", "dir": "h", "line": "deny", "label": "arm", "arrow": "end" },
                            { "tag": "Pipe", "dir": "h", "line": "deny", "label": "arm" }
                        ]
                    }
                ]
            }
        ]),
        json!([
            { "line": "gray", "text": "gray" },
            { "line": "solid", "tint": 1, "text": "blue" },
            { "line": "solid", "tint": 2, "text": "pink" },
            { "line": "dash", "text": "dash" },
            { "line": "deny", "text": "deny" }
        ]),
    );
    document["theme"] = json!(theme);
    document
}

fn pipe_pointer(index: usize) -> String {
    format!("/body/0/children/{index}")
}

fn part_bounds(rendered: &common::Rendered, pointer: &str, part: PartName) -> BoxRect {
    let mut node_pointer = NodePointer::root();
    for token in pointer.trim_start_matches('/').split('/') {
        node_pointer = node_pointer.child(token);
    }
    rendered
        .geometry
        .node(&node_pointer)
        .unwrap()
        .part(part)
        .unwrap()
        .bounds
}

fn number(node: roxmltree::Node<'_, '_>, attribute: &str) -> f32 {
    node.attribute(attribute).unwrap().parse().unwrap()
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 0.006
}

/// Carrier lines of arrowheads directly in the group.
fn carriers<'a, 'input>(group: roxmltree::Node<'a, 'input>) -> Vec<roxmltree::Node<'a, 'input>> {
    common::children_named(group, "line")
        .into_iter()
        .filter(|line| line.attribute("marker-end").is_some())
        .collect()
}

/// The wire colors of solid slot 1, solid slot 2 and dash slot 1 under a theme.
fn marker_colors(theme: &str) -> [String; 3] {
    let theme = common::theme(theme);
    [
        &theme.tints[0].wire,
        &theme.tints[1].wire,
        &theme.tints[0].wire,
    ]
    .map(|color| color.as_str().to_string())
}

/// A deny line ends in a stop plate, so its arrowed arm adds no marker.
#[test]
fn defs_hold_one_marker_per_arrow_kind_in_the_wire_color() {
    assert_eq!(marker_colors("center"), ["#1A73E8", "#C2185B", "#1A73E8"]);
    assert_eq!(marker_colors("wire"), ["#222222"; 3]);
    for theme in common::THEMES {
        let colors = marker_colors(theme);
        let rendered = common::render_document_with_fixed_metrics(arrow_document(theme));
        let document = common::parse_xml(&rendered.svg.svg);
        let markers: Vec<_> = document
            .descendants()
            .filter(|node| node.has_tag_name("marker"))
            .collect();
        let ids: Vec<_> = markers
            .iter()
            .map(|marker| marker.attribute("id").unwrap())
            .collect();
        assert_eq!(
            ids,
            ["blue", "pink", "dash"].map(|kind| format!("arrow-{theme}-{kind}")),
            "{theme}"
        );
        for (marker, color) in markers.iter().zip(colors) {
            assert_eq!(marker.attribute("markerUnits"), Some("userSpaceOnUse"));
            assert_eq!(marker.attribute("orient"), Some("auto"));
            assert_eq!(marker.attribute("markerWidth"), Some("10"));
            assert_eq!(marker.attribute("markerHeight"), Some("8"));
            assert_eq!(marker.attribute("refX"), Some("10"));
            assert_eq!(marker.attribute("refY"), Some("4"));
            let triangle = common::children_named(*marker, "path")[0];
            assert_eq!(triangle.attribute("d"), Some("M 0 0 L 10 4 L 0 8 Z"));
            assert_eq!(triangle.attribute("fill"), Some(color.as_str()), "{theme}");
        }
    }
}

#[test]
fn an_arrow_replaces_the_dot_at_the_end_it_names() {
    let rendered = common::render_document_with_fixed_metrics(arrow_document("center"));
    let document = common::parse_xml(&rendered.svg.svg);

    // (pipe pointer, marker kind, arrow at start, arrow at end, horizontal)
    let cases = [
        (pipe_pointer(0), "blue", false, true, true),
        (pipe_pointer(1), "pink", true, false, true),
        (pipe_pointer(2), "dash", true, true, false),
        (pipe_pointer(3), "gray", false, false, true),
        (
            format!("{}/arms/0", pipe_pointer(4)),
            "deny",
            false,
            true,
            true,
        ),
        (
            format!("{}/arms/1", pipe_pointer(4)),
            "deny",
            false,
            false,
            true,
        ),
    ];
    for (pointer, kind, arrow_start, arrow_end, horizontal) in cases {
        let group = common::group(&document, &pointer);
        let dot_start = part_bounds(&rendered, &pointer, PartName::DotStart);
        let dot_end = part_bounds(&rendered, &pointer, PartName::DotEnd);
        let circles = common::children_named(group, "circle");
        let has_dot_at = |bounds: BoxRect| {
            circles.iter().any(|circle| {
                close(number(*circle, "cx"), bounds.x + bounds.width / 2.0)
                    && close(number(*circle, "cy"), bounds.y + bounds.height / 2.0)
            })
        };
        assert_eq!(has_dot_at(dot_start), !arrow_start, "{pointer} start dot");
        assert_eq!(has_dot_at(dot_end), !arrow_end, "{pointer} end dot");

        let lines = carriers(group);
        if kind == "deny" {
            assert!(lines.is_empty(), "{pointer}: a deny arm draws no arrowhead");
            continue;
        }
        let expected_count = usize::from(arrow_start) + usize::from(arrow_end);
        assert_eq!(lines.len(), expected_count, "{pointer}");
        for line in &lines {
            assert_eq!(
                line.attribute("marker-end"),
                Some(format!("url(#arrow-center-{kind})").as_str()),
                "{pointer}"
            );
            assert_eq!(line.attribute("stroke"), Some("none"), "{pointer}");
        }
        // Tip on the dot box's outer edge, base 10 px back along the run axis.
        let tip_at = |line: roxmltree::Node<'_, '_>| (number(line, "x2"), number(line, "y2"));
        let base_at = |line: roxmltree::Node<'_, '_>| (number(line, "x1"), number(line, "y1"));
        let mut remaining = lines.iter();
        if arrow_start {
            let line = *remaining.next().unwrap();
            let (tip_x, tip_y) = tip_at(line);
            let (base_x, base_y) = base_at(line);
            if horizontal {
                let center_y = dot_start.y + dot_start.height / 2.0;
                assert!(
                    close(tip_x, dot_start.x) && close(tip_y, center_y),
                    "{pointer}"
                );
                assert!(close(base_x, dot_start.x + 10.0) && close(base_y, center_y));
            } else {
                let center_x = dot_start.x + dot_start.width / 2.0;
                assert!(
                    close(tip_x, center_x) && close(tip_y, dot_start.y),
                    "{pointer}"
                );
                assert!(close(base_x, center_x) && close(base_y, dot_start.y + 10.0));
            }
        }
        if arrow_end {
            let line = *remaining.next().unwrap();
            let (tip_x, tip_y) = tip_at(line);
            let (base_x, base_y) = base_at(line);
            if horizontal {
                let center_y = dot_end.y + dot_end.height / 2.0;
                assert!(
                    close(tip_x, dot_end.right()) && close(tip_y, center_y),
                    "{pointer}"
                );
                assert!(close(base_x, dot_end.right() - 10.0) && close(base_y, center_y));
            } else {
                let center_x = dot_end.x + dot_end.width / 2.0;
                assert!(
                    close(tip_x, center_x) && close(tip_y, dot_end.bottom()),
                    "{pointer}"
                );
                assert!(close(base_x, center_x) && close(base_y, dot_end.bottom() - 10.0));
            }
        }
    }
}

#[test]
fn a_page_without_arrows_writes_no_defs() {
    let rendered = common::render_g7();
    assert!(!rendered.svg.svg.contains("<defs>"));
    assert!(!rendered.svg.svg.contains("marker"));
}

#[test]
fn the_arrowhead_paints_in_the_png() {
    const SCALE: u8 = 4;
    let rendered = common::render_document_with_fixed_metrics(arrow_document("center"));
    let png = render_png(
        &rendered.svg.svg,
        rendered.svg.text_elements,
        DeviceScale::new(SCALE).unwrap(),
    )
    .unwrap();
    let pixmap = common::decode_png(&png);
    let dot_end = part_bounds(&rendered, &pipe_pointer(0), PartName::DotEnd);
    // 9 px back from the tip the triangle is 7.2 px wide, so 3 px off the axis is inside it,
    // outside the 2 px wire and outside the 4 px dot a missing arrowhead would leave.
    let sample_x = dot_end.right() - 9.0;
    let sample_y = dot_end.y + dot_end.height / 2.0 + 3.0;
    let scale = f32::from(SCALE);
    let pixel = pixmap
        .pixel((sample_x * scale) as u32, (sample_y * scale) as u32)
        .unwrap();
    // #1A73E8
    assert_eq!(
        (pixel.red(), pixel.green(), pixel.blue()),
        (0x1A, 0x73, 0xE8),
        "pixel at ({sample_x}, {sample_y})"
    );
}

/// The filled rects directly in `group`, as (x, y, width, height, fill).
fn rects(group: roxmltree::Node<'_, '_>) -> Vec<(f32, f32, f32, f32, String)> {
    common::children_named(group, "rect")
        .into_iter()
        .map(|rect| {
            (
                number(rect, "x"),
                number(rect, "y"),
                number(rect, "width"),
                number(rect, "height"),
                rect.attribute("fill").unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// Section 11.2: a deny line ends in a stop plate at its arrowed end, a bar 16 across its
/// line and 3 thick in the deny color with its outer face on the end, never an arrowhead.
/// The line's stroke runs to the plate, and the legend sample shows a plate at its end.
#[test]
fn a_deny_line_ends_in_a_stop_plate_on_its_end_and_draws_no_arrowhead() {
    for theme in common::THEMES {
        let mut document = arrow_document(theme);
        document["body"] = json!([
            {
                "tag": "Row",
                "gap": 64,
                "children": [
                    { "tag": "Item", "kind": "product", "id": "left", "title": "left" },
                    { "tag": "Pipe", "dir": "h", "line": "deny", "label": "arm", "arrow": "end" },
                    { "tag": "Item", "kind": "product", "id": "right", "title": "right" }
                ]
            },
            { "tag": "Item", "kind": "product", "id": "below", "title": "below" }
        ]);
        document["links"] = json!([{ "from": "right", "to": "below", "line": "deny" }]);
        let rendered = common::render_document_with_fixed_metrics(document);
        let svg = &rendered.svg.svg;
        let deny = common::theme(theme).deny.color.as_str().to_string();
        let xml = common::parse_xml(svg);
        assert!(
            !svg.contains(&format!("arrow-{theme}-deny")),
            "{theme}: no deny marker"
        );

        let pipe = common::group(&xml, "/body/0/children/1");
        assert!(
            carriers(pipe).is_empty(),
            "{theme}: the pipe draws no arrowhead"
        );
        let dot_end = part_bounds(&rendered, "/body/0/children/1", PartName::DotEnd);
        let center_y = dot_end.y + dot_end.height / 2.0;
        let plate = (
            dot_end.right() - 3.0,
            center_y - 8.0,
            3.0,
            16.0,
            deny.clone(),
        );
        let pipe_rects = rects(pipe);
        assert!(
            pipe_rects.iter().any(|rect| close(rect.0, plate.0)
                && close(rect.1, plate.1)
                && close(rect.2, plate.2)
                && close(rect.3, plate.3)
                && rect.4 == plate.4),
            "{theme}: {pipe_rects:?} holds no plate {plate:?}"
        );
        let stub = common::children_named(pipe, "line")
            .into_iter()
            .find(|line| close(number(*line, "x2"), dot_end.right() - 3.0))
            .unwrap_or_else(|| panic!("{theme}: no stroke into the plate"));
        assert!(close(number(stub, "x1"), dot_end.right() - 10.0), "{theme}");
        assert_eq!(stub.attribute("stroke"), Some(deny.as_str()), "{theme}");

        let route = &rendered.geometry.links[0];
        let tip = *route.points.last().unwrap();
        let before = route.points[route.points.len() - 2];
        assert!(
            close(before.x, tip.x) && tip.y > before.y,
            "{theme}: a straight approach down"
        );
        let link = common::group(&xml, "/links/0");
        assert!(
            carriers(link).is_empty(),
            "{theme}: the link draws no arrowhead"
        );
        let link_rects = rects(link);
        assert!(
            link_rects.iter().any(|rect| close(rect.0, tip.x - 8.0)
                && close(rect.1, tip.y - 3.0)
                && close(rect.2, 16.0)
                && close(rect.3, 3.0)
                && rect.4 == deny),
            "{theme}: {link_rects:?} holds no plate at {tip:?}"
        );
        let path = common::children_named(link, "path")[0]
            .attribute("d")
            .unwrap();
        assert!(
            path.ends_with(&format!("{} {}", tip.x, tip.y - 3.0)),
            "{theme}: the stroke ends at the plate's inner face: {path}"
        );

        let entry = common::group(&xml, "/legend/4");
        let swatch = rendered
            .geometry
            .node(&NodePointer::root().child("legend").index(4))
            .unwrap()
            .part(PartName::Swatch)
            .unwrap()
            .bounds;
        let sample = (
            swatch.right() - 3.0,
            swatch.y + swatch.height / 2.0 - 6.0,
            3.0,
            12.0,
            deny.clone(),
        );
        let entry_rects = rects(entry);
        assert!(
            entry_rects.iter().any(|rect| close(rect.0, sample.0)
                && close(rect.1, sample.1)
                && close(rect.2, sample.2)
                && close(rect.3, sample.3)
                && rect.4 == sample.4),
            "{theme}: {entry_rects:?} holds no sample plate {sample:?}"
        );
    }
}
