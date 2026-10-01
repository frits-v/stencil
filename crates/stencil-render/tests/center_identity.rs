//! Identity proofs (section 13.14). The fixtures are the center SVG and the measured JSON of
//! every example, rendered from the documents before the core vocabulary replaced Zone,
//! Pcard and the color-named kinds. The SVG must match byte for byte, and the measured JSON
//! must match byte for byte once its `document` key, which echoes the input, is removed.

#![allow(clippy::unwrap_used)]

mod common;

use stencil_layout::{PageGeometry, layout_page};
use stencil_model::{Page, Projection, parse_and_vet};
use stencil_render::iso::project_page;
use stencil_render::{measured_json, render_svg};
use stencil_text::CosmicTextMeasurer;

struct Example {
    stem: &'static str,
    document: &'static str,
    svg: &'static str,
    geometry: &'static str,
}

const EXAMPLES: [Example; 6] = [
    Example {
        stem: "g7",
        document: include_str!("../../../examples/g7.json"),
        svg: include_str!("fixtures/g7.center.svg"),
        geometry: include_str!("fixtures/g7.center.geometry.json"),
    },
    Example {
        stem: "hero-iso",
        document: include_str!("../../../examples/hero-iso.json"),
        svg: include_str!("fixtures/hero-iso.center.svg"),
        geometry: include_str!("fixtures/hero-iso.center.geometry.json"),
    },
    Example {
        stem: "hybrid-ai",
        document: include_str!("../../../examples/hybrid-ai.json"),
        svg: include_str!("fixtures/hybrid-ai.center.svg"),
        geometry: include_str!("fixtures/hybrid-ai.center.geometry.json"),
    },
    Example {
        stem: "network-hub-spoke",
        document: include_str!("../../../examples/network-hub-spoke.json"),
        svg: include_str!("fixtures/network-hub-spoke.center.svg"),
        geometry: include_str!("fixtures/network-hub-spoke.center.geometry.json"),
    },
    Example {
        stem: "onepager",
        document: include_str!("../../../examples/onepager.json"),
        svg: include_str!("fixtures/onepager.center.svg"),
        geometry: include_str!("fixtures/onepager.center.geometry.json"),
    },
    Example {
        stem: "stress-dense",
        document: include_str!("../../../examples/stress-dense.json"),
        svg: include_str!("fixtures/stress-dense.center.svg"),
        geometry: include_str!("fixtures/stress-dense.center.geometry.json"),
    },
];

/// The example vetted and laid out under the built-in gcp grammar it names.
fn page_and_geometry(example: &Example) -> (Page, PageGeometry) {
    let grammar = common::gcp();
    let page = parse_and_vet(example.document, &grammar).unwrap();
    assert_eq!(page.grammar.as_deref(), Some("gcp"), "{}", example.stem);
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    let geometry = layout_page(&page, &grammar, &mut measurer).unwrap();
    (page, geometry)
}

#[test]
fn every_example_renders_its_center_svg_fixture_byte_for_byte() {
    let mut examined = 0;
    for example in &EXAMPLES {
        let (page, geometry) = page_and_geometry(example);
        let rendered = render_svg(&page, &geometry).unwrap();
        assert!(
            rendered.svg == example.svg,
            "{}: center SVG differs from tests/fixtures/{}.center.svg",
            example.stem,
            example.stem
        );
        examined += 1;
    }
    assert_eq!(examined, EXAMPLES.len());
}

#[test]
fn every_example_writes_its_geometry_fixture_byte_for_byte() {
    let mut examined = 0;
    for example in &EXAMPLES {
        let (page, geometry) = page_and_geometry(example);
        let document: serde_json::Value = serde_json::from_str(example.document).unwrap();
        let scene = match page.projection {
            Projection::Flat => None,
            Projection::Iso => Some(project_page(&geometry).unwrap()),
        };
        let mut measured = measured_json(&document, &geometry, scene.as_ref());
        let removed = measured.as_object_mut().unwrap().remove("document");
        assert!(
            removed.is_some(),
            "{}: measured JSON has no document key",
            example.stem
        );
        let mut bytes = serde_json::to_string_pretty(&measured).unwrap();
        bytes.push('\n');
        assert!(
            bytes == example.geometry,
            "{}: measured JSON differs from tests/fixtures/{}.center.geometry.json",
            example.stem,
            example.stem
        );
        examined += 1;
    }
    assert_eq!(examined, EXAMPLES.len());
}
