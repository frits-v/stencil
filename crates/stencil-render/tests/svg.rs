#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unreachable
)]

mod common;

use common::render_svg;
use common::{LineKey, ZoneKey};
use resvg::usvg;
use serde_json::json;
use stencil_layout::{NodeTag, PartName};
use stencil_model::LEGEND_ENTRIES_MAX;
use stencil_model::pointer::NodePointer;
use stencil_render::RenderError;

#[test]
fn g7_svg_parses_with_usvg() {
    let rendered = common::render_g7();
    usvg::Tree::from_str(&rendered.svg.svg, &usvg::Options::default()).unwrap();
}

#[test]
fn one_group_per_geometry_node_in_geometry_order() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    let group_pointers: Vec<&str> = document
        .descendants()
        .filter(|node| node.has_tag_name("g"))
        .map(|node| {
            node.attribute("data-id")
                .expect("every <g> carries data-id")
        })
        .collect();
    let geometry_pointers: Vec<&str> = rendered
        .geometry
        .nodes
        .iter()
        .map(|node| node.pointer.as_str())
        .collect();
    assert_eq!(group_pointers, geometry_pointers);
}

#[test]
fn groups_nest_like_the_node_tree_and_carry_tag_and_kind() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    for node in &rendered.geometry.nodes {
        let group = common::group(&document, node.pointer.as_str());
        assert_eq!(group.attribute("data-tag"), Some(node.tag.as_str()));
        assert_eq!(group.attribute("data-kind"), node.kind.as_deref());
        assert_eq!(group.attribute("transform"), None);
        let parent_pointer = node
            .parent
            .map(|parent| rendered.geometry.nodes[parent].pointer.as_str());
        let enclosing = group
            .parent_element()
            .filter(|element| element.has_tag_name("g"))
            .and_then(|element| element.attribute("data-id"));
        assert_eq!(enclosing, parent_pointer, "{}", node.pointer);
    }
    let kinded: Vec<NodeTag> = rendered
        .geometry
        .nodes
        .iter()
        .filter(|node| node.kind.is_some())
        .map(|node| node.tag)
        .collect();
    assert!(kinded.iter().all(|tag| matches!(
        tag,
        NodeTag::Zone | NodeTag::Pipe | NodeTag::Tee | NodeTag::LegendEntry
    )));
}

#[test]
fn the_only_http_substring_is_the_namespace() {
    let rendered = common::render_g7();
    let occurrences = rendered.svg.svg.matches("http").count();
    assert_eq!(occurrences, 1);
    assert!(
        rendered
            .svg
            .svg
            .contains(r#"xmlns="http://www.w3.org/2000/svg""#)
    );
}

#[test]
fn text_elements_count_every_line_of_every_run() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    let text_count = document
        .descendants()
        .filter(|node| node.has_tag_name("text"))
        .count();
    let line_total: u32 = common::text_runs(&rendered.geometry)
        .iter()
        .map(|run| run.metrics.line_count)
        .sum();
    assert_eq!(rendered.svg.text_elements, text_count);
    assert_eq!(text_count, line_total as usize);
}

#[test]
fn text_attributes_follow_section_5_2() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    let texts: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("text"))
        .collect();
    let lines: Vec<_> = common::text_runs(&rendered.geometry)
        .into_iter()
        .flat_map(|run| run.metrics.lines.iter().map(move |line| (run, line)))
        .collect();
    assert_eq!(texts.len(), lines.len());
    let mut spaced = 0;
    for (text, (run, line)) in texts.iter().zip(&lines) {
        assert_eq!(
            text.attribute(("http://www.w3.org/XML/1998/namespace", "space")),
            Some("preserve")
        );
        assert_eq!(text.attribute("font-family"), Some("Inter"));
        assert_eq!(text.attribute("text-anchor"), None);
        assert_eq!(
            text.attribute("font-weight"),
            Some(run.style.weight.css_value().to_string().as_str())
        );
        assert_eq!(text.attribute("fill"), Some(run.color));
        assert_eq!(text.text(), Some(&run.text[line.byte_start..line.byte_end]));
        let spacing = text.attribute("letter-spacing");
        if run.style.letter_spacing_em == 0.0 {
            assert_eq!(spacing, None, "{}", run.text);
        } else {
            spaced += 1;
            let expected = run.style.letter_spacing_em * run.style.size_px;
            let written: f32 = spacing.expect("letter-spacing present").parse().unwrap();
            assert!((written - expected).abs() < 0.006, "{}", run.text);
        }
    }
    // Badge, kicker, title and the gcp bar label are the spaced styles g7 draws.
    assert_eq!(spaced, 4);
}

#[test]
fn text_positions_follow_alignment_and_baseline() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    for node in &rendered.geometry.nodes {
        let group = common::group(&document, node.pointer.as_str());
        let texts = common::children_named(group, "text");
        let lines: Vec<_> = node
            .parts
            .iter()
            .filter_map(|part| part.text.as_ref().map(|run| (part, run)))
            .flat_map(|(part, run)| run.metrics.lines.iter().map(move |line| (part, run, line)))
            .collect();
        assert_eq!(texts.len(), lines.len(), "{}", node.pointer);
        for (text, (part, run, line)) in texts.iter().zip(lines) {
            let x: f32 = text.attribute("x").unwrap().parse().unwrap();
            let y: f32 = text.attribute("y").unwrap().parse().unwrap();
            let expected_x = match run.align {
                stencil_layout::TextAlign::Start => part.bounds.x,
                stencil_layout::TextAlign::Center => {
                    part.bounds.x + (part.bounds.width - line.width_px) / 2.0
                }
            };
            assert!((x - expected_x).abs() <= 0.006, "{}", node.pointer);
            assert!(
                (y - (part.bounds.y + line.baseline_px)).abs() <= 0.006,
                "{}",
                node.pointer
            );
        }
    }
}

/// Attributes that hold names, colors or payloads rather than numbers.
const NON_NUMERIC_ATTRIBUTES: [&str; 10] = [
    "xmlns",
    "data-id",
    "data-tag",
    "data-kind",
    "fill",
    "stroke",
    "font-family",
    "href",
    "space",
    "stroke-dasharray",
];

#[test]
fn numbers_have_at_most_two_decimals_and_never_negative_zero() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    let mut examined = 0;
    for element in document.descendants().filter(|node| node.is_element()) {
        for attribute in element.attributes() {
            if NON_NUMERIC_ATTRIBUTES.contains(&attribute.name()) {
                continue;
            }
            for token in attribute.value().split([' ', ',']) {
                let Ok(value) = token.parse::<f64>() else {
                    continue;
                };
                examined += 1;
                if value == 0.0 {
                    assert_eq!(token, "0", "{}={}", attribute.name(), attribute.value());
                }
                let decimals = token
                    .split_once('.')
                    .map_or(0, |(_, fraction)| fraction.len());
                assert!(decimals <= 2, "{}={}", attribute.name(), attribute.value());
                assert!(!token.ends_with('.'), "{token}");
                if token.contains('.') {
                    assert!(!token.ends_with('0'), "{token}");
                }
            }
        }
    }
    assert!(examined > 400, "examined {examined} numbers");
}

#[test]
fn icons_are_images_with_the_data_uri() {
    let rendered = common::render_g7();
    let document = common::parse_xml(&rendered.svg.svg);
    let images: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("image"))
        .collect();
    let icon_parts: Vec<_> = rendered
        .geometry
        .nodes
        .iter()
        .filter_map(|node| node.part(PartName::Icon))
        .collect();
    assert_eq!(images.len(), 6);
    assert_eq!(images.len(), icon_parts.len());
    for (image, part) in images.iter().zip(icon_parts) {
        let href = image.attribute("href").unwrap();
        assert!(href.starts_with("data:image/svg+xml;base64,"));
        assert_eq!(image.attribute("width"), Some("28"));
        assert_eq!(
            image.attribute("x").unwrap().parse::<f32>().unwrap(),
            (part.bounds.x * 100.0).round() / 100.0
        );
    }
}

/// A page using every ZoneKey, every LineKey as a Pipe, a Tee and a legend entry.
fn palette_document() -> serde_json::Value {
    let zones: Vec<_> = ZoneKey::ALL.iter().map(|kind| kind.box_json()).collect();
    let pipes: Vec<_> = LineKey::ALL
        .iter()
        .map(|kind| kind.with_line(json!({ "tag": "Pipe", "dir": "h", "label": kind.as_str() })))
        .collect();
    let tees: Vec<_> = LineKey::ALL
        .iter()
        .map(|kind| {
            kind.with_line(json!({
                "tag": "Tee",
                "hub": "hub",
                "arms": [
                    kind.with_line(json!({ "tag": "Pipe", "dir": "h", "label": "arm one" })),
                    kind.with_line(json!({ "tag": "Pipe", "dir": "h", "label": "arm two" }))
                ]
            }))
        })
        .collect();
    let legend: Vec<_> = LineKey::ALL
        .iter()
        .map(|kind| kind.with_line(json!({ "text": kind.as_str() })))
        .collect();
    common::page_document(
        json!([
            { "tag": "Col", "children": zones },
            { "tag": "Col", "children": pipes },
            { "tag": "Col", "children": tees }
        ]),
        json!(legend),
    )
}

/// (fill, stroke, stroke-width, dasharray) of section 2.4, gcp excluded.
fn expected_zone_rect(
    kind: ZoneKey,
) -> (
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
) {
    match kind {
        ZoneKey::Gcp => unreachable!("gcp is drawn with paths"),
        ZoneKey::Vpc => ("none", Some("#5F6368"), Some("2"), Some("6 5")),
        ZoneKey::RegionA => ("#D2E3FC", Some("#BDC1C6"), Some("1.5"), None),
        ZoneKey::RegionB => ("#FCE4EC", Some("#BDC1C6"), Some("1.5"), None),
        ZoneKey::Subnet => ("#EDE7F6", Some("#9AA0A6"), Some("1.5"), Some("6 5")),
        ZoneKey::OnpremA => ("#D2E3FC", Some("#D7CCC8"), Some("1.5"), None),
        ZoneKey::OnpremB => ("#FCE4EC", Some("#D7CCC8"), Some("1.5"), None),
        ZoneKey::Project => ("#FFF8E1", Some("#FFE082"), Some("1.5"), None),
        ZoneKey::Optional => ("#F8FBFF", Some("#4284F3"), Some("2"), Some("6 5")),
        ZoneKey::K8s => ("#FCE4EC", None, None, None),
        ZoneKey::Perimeter => ("#FFFBF5", Some("#E37400"), Some("2.5"), Some("6 5")),
    }
}

/// (color, dasharray) of section 5.2.
fn expected_wire(kind: LineKey) -> (&'static str, Option<&'static str>) {
    match kind {
        LineKey::Gray => ("#5F6368", None),
        LineKey::Blue => ("#1A73E8", None),
        LineKey::Pink => ("#C2185B", None),
        LineKey::Dash => ("#1A73E8", Some("6 5")),
        LineKey::Deny => ("#C5221F", Some("6 5")),
    }
}

#[test]
fn zone_boxes_use_the_section_2_4_colors() {
    let rendered = common::render_document_with_fixed_metrics(palette_document());
    let document = common::parse_xml(&rendered.svg.svg);
    for (index, kind) in ZoneKey::ALL.iter().enumerate() {
        let group = common::group(&document, &format!("/body/0/children/{index}"));
        assert_eq!(group.attribute("data-kind"), Some(kind.as_str()));
        if *kind == ZoneKey::Gcp {
            let paths = common::children_named(group, "path");
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0].attribute("fill"), Some("#FFFFFF"));
            assert_eq!(paths[1].attribute("fill"), Some("none"));
            assert_eq!(paths[1].attribute("stroke"), Some("#1A73E8"));
            assert_eq!(paths[1].attribute("stroke-width"), Some("3"));
            assert_eq!(paths[1].attribute("stroke-dasharray"), None);
            let rects = common::children_named(group, "rect");
            assert_eq!(rects[0].attribute("fill"), Some("#1A73E8"), "bar");
            assert_eq!(rects[1].attribute("fill"), Some("#FAFBFC"), "body");
            let stroke_position = group.children().position(|child| child == paths[1]);
            let bar_position = group.children().position(|child| child == rects[0]);
            assert!(
                bar_position < stroke_position,
                "bar is drawn before the frame stroke"
            );
            continue;
        }
        let rect = common::children_named(group, "rect")[0];
        let (fill, stroke, stroke_width, dash) = expected_zone_rect(*kind);
        assert_eq!(rect.attribute("fill"), Some(fill), "{}", kind.as_str());
        assert_eq!(rect.attribute("stroke"), stroke, "{}", kind.as_str());
        assert_eq!(
            rect.attribute("stroke-width"),
            stroke_width,
            "{}",
            kind.as_str()
        );
        assert_eq!(
            rect.attribute("stroke-dasharray"),
            dash,
            "{}",
            kind.as_str()
        );
    }
}

#[test]
fn stroked_rects_stay_inside_the_border_box() {
    let rendered = common::render_document_with_fixed_metrics(palette_document());
    let document = common::parse_xml(&rendered.svg.svg);
    let region = rendered
        .geometry
        .node(
            &NodePointer::root()
                .child("body")
                .index(0)
                .child("children")
                .index(2),
        )
        .unwrap();
    let group = common::group(&document, "/body/0/children/2");
    let rect = common::children_named(group, "rect")[0];
    let x: f32 = rect.attribute("x").unwrap().parse().unwrap();
    let width: f32 = rect.attribute("width").unwrap().parse().unwrap();
    assert!((x - (region.bounds.x + 0.75)).abs() <= 0.006);
    assert!((width - (region.bounds.width - 1.5)).abs() <= 0.006);
}

#[test]
fn pipes_tees_and_legend_swatches_use_the_section_5_2_colors() {
    let rendered = common::render_document_with_fixed_metrics(palette_document());
    let document = common::parse_xml(&rendered.svg.svg);
    for (index, kind) in LineKey::ALL.iter().enumerate() {
        let (color, dash) = expected_wire(*kind);

        let pipe = common::group(&document, &format!("/body/1/children/{index}"));
        let wires = common::children_named(pipe, "line");
        assert_eq!(wires.len(), 2, "{}", kind.as_str());
        for wire in wires {
            assert_eq!(wire.attribute("stroke"), Some(color));
            assert_eq!(wire.attribute("stroke-width"), Some("2"));
            assert_eq!(wire.attribute("stroke-dasharray"), dash);
            assert_eq!(
                wire.attribute("y1"),
                wire.attribute("y2"),
                "h wire is horizontal"
            );
        }
        let dots = common::children_named(pipe, "circle");
        assert_eq!(dots.len(), 2);
        for dot in dots {
            assert_eq!(dot.attribute("fill"), Some(color));
            assert_eq!(dot.attribute("r"), Some("4"));
        }

        let tee = common::group(&document, &format!("/body/2/children/{index}"));
        let spine = common::children_named(tee, "line");
        assert_eq!(spine.len(), 1);
        assert_eq!(spine[0].attribute("stroke"), Some(color));
        assert_eq!(spine[0].attribute("stroke-dasharray"), dash);
        assert_eq!(
            spine[0].attribute("x1"),
            spine[0].attribute("x2"),
            "spine is vertical"
        );

        let entry = common::group(&document, &format!("/legend/{index}"));
        let swatch = common::children_named(entry, "line");
        assert_eq!(swatch.len(), 1);
        assert_eq!(swatch[0].attribute("stroke"), Some(color));
        assert_eq!(swatch[0].attribute("stroke-width"), Some("2"));
        assert_eq!(swatch[0].attribute("stroke-dasharray"), dash);
    }
}

#[test]
fn tag_and_hub_borders_turn_red_only_for_deny() {
    let rendered = common::render_document_with_fixed_metrics(palette_document());
    let document = common::parse_xml(&rendered.svg.svg);
    for (index, kind) in LineKey::ALL.iter().enumerate() {
        let (border, label) = if *kind == LineKey::Deny {
            ("#F4C7C3", "#C5221F")
        } else {
            ("#DADCE0", "#202124")
        };
        for pointer in [
            format!("/body/1/children/{index}"),
            format!("/body/2/children/{index}"),
        ] {
            let group = common::group(&document, &pointer);
            let rect = common::children_named(group, "rect")[0];
            assert_eq!(rect.attribute("fill"), Some("#FFFFFF"), "{pointer}");
            assert_eq!(rect.attribute("stroke"), Some(border), "{pointer}");
            let text = common::children_named(group, "text")[0];
            assert_eq!(text.attribute("fill"), Some(label), "{pointer}");
        }
    }
}

#[test]
fn tee_draws_spine_then_hub_then_arm_groups() {
    let rendered = common::render_document_with_fixed_metrics(palette_document());
    let document = common::parse_xml(&rendered.svg.svg);
    let tee = common::group(&document, "/body/2/children/4");
    let names: Vec<&str> = tee
        .children()
        .filter(|child| child.is_element())
        .map(|child| child.tag_name().name())
        .collect();
    assert_eq!(names, ["line", "rect", "text", "g", "g"]);
}

#[test]
fn text_content_is_xml_escaped() {
    let body = json!([
        { "tag": "Fact", "text": "a < b & c > \"d\"" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" }
    ]);
    let legend = json!([{ "line": "solid", "tint": 1, "text": "request path" }]);
    let rendered = common::render_document_with_fixed_metrics(common::page_document(body, legend));
    assert!(
        rendered
            .svg
            .svg
            .contains("a &lt; b &amp; c &gt; &quot;d&quot;")
    );
    let document = common::parse_xml(&rendered.svg.svg);
    let fact = common::group(&document, "/body/0");
    assert_eq!(
        common::children_named(fact, "text")[0].text(),
        Some("a < b & c > \"d\"")
    );
}

#[test]
fn interior_space_runs_survive_in_the_svg() {
    let body = json!([
        { "tag": "Fact", "text": "a  b" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" }
    ]);
    let legend = json!([{ "line": "solid", "tint": 1, "text": "request path" }]);
    let rendered = common::render_document_with_fixed_metrics(common::page_document(body, legend));
    assert!(rendered.svg.svg.contains(r#"xml:space="preserve""#));
    assert!(rendered.svg.svg.contains(">a  b</text>"));
}

#[test]
fn kicker_badge_follows_the_canvas() {
    let customer = common::render_g7();
    let document = common::parse_xml(&customer.svg.svg);
    let kicker = common::group(&document, "/kicker");
    assert_eq!(
        common::children_named(kicker, "rect")[0].attribute("fill"),
        Some("#E8F0FE")
    );
    let texts = common::children_named(kicker, "text");
    assert_eq!(texts[0].text(), Some("CUSTOMER"));
    assert_eq!(texts[0].attribute("fill"), Some("#174EA6"));
    assert_eq!(texts[0].attribute("letter-spacing"), Some("0.7"));
    assert_eq!(texts[1].attribute("letter-spacing"), Some("0.88"));
}

#[test]
fn geometry_of_another_page_is_a_mismatch() {
    let g7 = common::render_g7();
    let body = json!([
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" },
        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" }
    ]);
    let other = common::render_document_with_fixed_metrics(common::page_document(body, json!([])));
    let error = render_svg(&other.page, &g7.geometry).unwrap_err();
    match error {
        RenderError::GeometryMismatch { expected, found } => {
            assert_eq!(expected.as_str(), "/body/1");
            assert_eq!(found.as_str(), "/body/0/children/0");
        }
        other => panic!("expected GeometryMismatch, got {other:?}"),
    }
}

#[test]
fn truncated_geometry_is_a_mismatch() {
    let mut rendered = common::render_g7();
    rendered.geometry.nodes.pop();
    let error = render_svg(&rendered.page, &rendered.geometry).unwrap_err();
    match error {
        RenderError::GeometryMismatch { expected, .. } => assert_eq!(expected.as_str(), "/foot"),
        other => panic!("expected GeometryMismatch, got {other:?}"),
    }
}

#[test]
fn rendering_twice_is_byte_identical() {
    let first = common::render_g7();
    let second = render_svg(&first.page, &first.geometry).unwrap();
    assert_eq!(first.svg, second);
}

/// A geometry with more legend nodes than the vet limit allows fails at the first one past
/// LEGEND_ENTRIES_MAX + 1, because the document side stops walking the legend there.
#[test]
fn legend_order_stops_one_past_the_vet_limit() {
    let entries: Vec<serde_json::Value> = (0..LEGEND_ENTRIES_MAX)
        .map(|_| json!({ "line": "solid", "tint": 1, "text": "request" }))
        .collect();
    let body = json!([{ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" }]);
    let mut rendered = common::render_document_with_fixed_metrics(common::page_document(
        body,
        serde_json::Value::Array(entries),
    ));
    let last_entry = rendered.geometry.nodes.last().unwrap().clone();
    assert_eq!(last_entry.pointer.as_str(), "/legend/15");
    let legend_pointer = NodePointer::root().child("legend");
    for index in LEGEND_ENTRIES_MAX..LEGEND_ENTRIES_MAX + 4 {
        let mut extra = last_entry.clone();
        extra.pointer = legend_pointer.index(index);
        rendered.geometry.nodes.push(extra);
        rendered.page.legend.push(rendered.page.legend[0].clone());
    }
    assert_eq!(rendered.page.legend.len(), 20);

    let error = render_svg(&rendered.page, &rendered.geometry).unwrap_err();
    match error {
        RenderError::GeometryMismatch { expected, found } => {
            assert_eq!(expected.as_str(), "/<absent>");
            assert_eq!(found.as_str(), "/legend/17");
        }
        other => panic!("expected GeometryMismatch, got {other:?}"),
    }
}
