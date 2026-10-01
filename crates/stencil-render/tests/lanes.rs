#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Lanes and the plain-grammar examples (sections 13.6 and 13.14): lifelines and messages in
//! the SVG and the measured JSON, every theme over the plain grammar, and the center
//! fixtures of the three plain examples.

mod common;

use serde_json::Value;
use stencil_layout::{PageGeometry, layout_page};
use stencil_model::{Page, builtin_grammar, parse_and_vet};
use stencil_render::{measured_json, render_svg};
use stencil_text::CosmicTextMeasurer;

struct Example {
    stem: &'static str,
    document: &'static str,
    svg: &'static str,
    geometry: &'static str,
}

const EXAMPLES: [Example; 3] = [
    Example {
        stem: "sequence",
        document: include_str!("../../../examples/sequence.json"),
        svg: include_str!("fixtures/sequence.center.svg"),
        geometry: include_str!("fixtures/sequence.center.geometry.json"),
    },
    Example {
        stem: "org",
        document: include_str!("../../../examples/org.json"),
        svg: include_str!("fixtures/org.center.svg"),
        geometry: include_str!("fixtures/org.center.geometry.json"),
    },
    Example {
        stem: "onprem-network",
        document: include_str!("../../../examples/onprem-network.json"),
        svg: include_str!("fixtures/onprem-network.center.svg"),
        geometry: include_str!("fixtures/onprem-network.center.geometry.json"),
    },
];

const THEMES: [&str; 3] = ["center", "dusk", "wire"];

/// The document with its `theme` set, vetted and laid out under the plain grammar.
fn page_and_geometry(document: &str, theme: &str) -> (Value, Page, PageGeometry) {
    let mut value: Value = serde_json::from_str(document).unwrap();
    value["theme"] = Value::String(theme.to_string());
    let grammar = builtin_grammar("plain").unwrap().unwrap();
    let page = parse_and_vet(&value.to_string(), &grammar).unwrap();
    assert_eq!(page.grammar.as_deref(), Some("plain"));
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    let geometry = layout_page(&page, &grammar, &mut measurer).unwrap();
    (value, page, geometry)
}

/// The measured JSON without `document`, serialized as `render` writes it.
fn geometry_bytes(document: &Value, geometry: &PageGeometry) -> String {
    let mut measured = measured_json(document, geometry, None);
    measured.as_object_mut().unwrap().remove("document");
    let mut bytes = serde_json::to_string_pretty(&measured).unwrap();
    bytes.push('\n');
    bytes
}

#[test]
fn every_plain_example_renders_its_center_fixtures_byte_for_byte() {
    let mut examined = 0;
    for example in &EXAMPLES {
        let original: Value = serde_json::from_str(example.document).unwrap();
        let (_, page, geometry) = page_and_geometry(example.document, "center");
        let rendered = render_svg(&page, &geometry).unwrap();
        assert!(
            rendered.svg == example.svg,
            "{}: center SVG differs from tests/fixtures/{}.center.svg",
            example.stem,
            example.stem
        );
        assert!(
            geometry_bytes(&original, &geometry) == example.geometry,
            "{}: measured JSON differs from tests/fixtures/{}.center.geometry.json",
            example.stem,
            example.stem
        );
        examined += 1;
    }
    assert_eq!(examined, EXAMPLES.len());
}

#[test]
fn every_theme_renders_every_plain_example_with_the_same_geometry() {
    for example in &EXAMPLES {
        let original: Value = serde_json::from_str(example.document).unwrap();
        let mut geometries = Vec::new();
        let mut svgs = Vec::new();
        for theme in THEMES {
            let (_, page, geometry) = page_and_geometry(example.document, theme);
            svgs.push(render_svg(&page, &geometry).unwrap().svg);
            geometries.push(geometry_bytes(&original, &geometry));
        }
        assert_eq!(geometries[0], geometries[1], "{} dusk", example.stem);
        assert_eq!(geometries[0], geometries[2], "{} wire", example.stem);
        assert_ne!(svgs[0], svgs[1], "{} dusk", example.stem);
        assert_ne!(svgs[0], svgs[2], "{} wire", example.stem);
    }
}

/// The `<line>` elements drawn directly in the Lanes group of the sequence example.
fn lifeline_elements(svg: &str) -> Vec<(f32, f32, f32, f32, String, String, String)> {
    let document = common::parse_xml(svg);
    let lanes = common::group(&document, "/body/0/children/0");
    assert_eq!(lanes.attribute("data-tag"), Some("Lanes"));
    common::children_named(lanes, "line")
        .into_iter()
        .map(|line| {
            let number = |name: &str| line.attribute(name).unwrap().parse::<f32>().unwrap();
            let text = |name: &str| line.attribute(name).unwrap_or_default().to_string();
            (
                number("x1"),
                number("y1"),
                number("x2"),
                number("y2"),
                text("stroke"),
                text("stroke-width"),
                text("stroke-dasharray"),
            )
        })
        .collect()
}

#[test]
fn each_head_draws_a_dashed_1_px_lifeline_to_the_band_bottom_in_every_theme() {
    let sequence = &EXAMPLES[0];
    for theme in THEMES {
        let (_, page, geometry) = page_and_geometry(sequence.document, theme);
        let svg = render_svg(&page, &geometry).unwrap().svg;
        let lifelines = lifeline_elements(&svg);
        assert_eq!(lifelines.len(), 5, "{theme}: one lifeline per head");
        let lanes = geometry
            .nodes
            .iter()
            .find(|node| node.pointer.as_str() == "/body/0/children/0")
            .unwrap();
        let band = lanes.part(stencil_layout::PartName::Band).unwrap().bounds;
        for (index, (x1, y1, x2, y2, stroke, width, dash)) in lifelines.iter().enumerate() {
            let head = &geometry
                .nodes
                .iter()
                .find(|node| {
                    node.pointer.as_str() == format!("/body/0/children/0/children/{index}")
                })
                .unwrap()
                .bounds;
            assert!((x1 - (head.x + head.width / 2.0)).abs() < 0.06, "{theme} x");
            assert_eq!(x1, x2, "{theme}: vertical");
            assert!((y1 - head.bottom()).abs() < 0.06, "{theme} top");
            assert!((y2 - band.bottom()).abs() < 0.06, "{theme} bottom");
            assert_eq!(width, "1", "{theme}");
            assert!(!dash.is_empty(), "{theme}: dashed");
            if theme == "center" {
                assert_eq!(stroke, "#9AA0A6", "center lifeline is section 13.5's gray");
            }
        }
    }
}

#[test]
fn messages_are_two_point_links_and_lifelines_are_keyed_by_head_in_the_measured_json() {
    let sequence = &EXAMPLES[0];
    let (document, _, geometry) = page_and_geometry(sequence.document, "center");
    let measured = measured_json(&document, &geometry, None);
    let lanes = measured["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "/body/0/children/0")
        .unwrap();
    assert_eq!(lanes["tag"], "Lanes");
    let keys: Vec<&str> = lanes["parts"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut expected = vec!["band", "heads"];
    expected.extend([
        "lifeline/0",
        "lifeline/1",
        "lifeline/2",
        "lifeline/3",
        "lifeline/4",
    ]);
    let mut sorted_keys = keys.clone();
    sorted_keys.sort_unstable();
    expected.sort_unstable();
    assert_eq!(sorted_keys, expected);
    assert_eq!(lanes["parts"]["lifeline/0"]["width"], 0);
    let links = measured["links"].as_array().unwrap();
    assert_eq!(links.len(), 10);
    let mut previous_y = f64::MIN;
    let mut by_order: Vec<(u64, &Value)> = links
        .iter()
        .enumerate()
        .map(|(index, link)| (document["links"][index]["order"].as_u64().unwrap(), link))
        .collect();
    by_order.sort_by_key(|(order, _)| *order);
    for (order, link) in by_order {
        let points = link["points"].as_array().unwrap();
        assert_eq!(points.len(), 2, "order {order}");
        assert_eq!(
            points[0]["y"], points[1]["y"],
            "order {order} is horizontal"
        );
        assert_eq!(link["status"], "routed");
        let y = points[0]["y"].as_f64().unwrap();
        assert!(
            y > previous_y,
            "order {order} sits below the message before it"
        );
        previous_y = y;
    }
}
