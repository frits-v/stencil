#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Rounded link bends (section 11.6): the writers draw them from the routed polyline, so
//! routing, the checks and the measured JSON never see the radius.

mod common;

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{PageGeometry, layout_page};
use stencil_model::text::FixedMetricsMeasurer;
use stencil_model::{
    Bend, Grammar, Line, LineKind, Page, Projection, builtin_grammar, parse_and_vet,
};
use stencil_render::iso::{SolidInputs, project_page};
use stencil_render::{measured_json, render_svg};
use stencil_text::CosmicTextMeasurer;

fn card(id: &str) -> Value {
    json!({ "tag": "Item", "kind": "product", "id": id, "title": id, "subtitle": "service" })
}

/// `a` top left and `b` bottom right at their content widths, and a blue link from a's
/// right side into b's top: one leg right, one bend, one leg down.
fn dogleg_document() -> Value {
    let mut document = common::page_document(
        json!([{ "tag": "Col", "gap": 48, "children": [
            { "tag": "Row", "grow": [0], "children": [card("a")] },
            { "tag": "Row", "grow": [0], "justify": "end", "children": [card("b")] }
        ]}]),
        json!([{ "line": "solid", "tint": 1, "text": "request path" }]),
    );
    document["links"] = json!([
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "from_side": "right", "to_side": "top" }
    ]);
    document
}

fn link_path(svg: &str, index: usize) -> String {
    let document = common::parse_xml(svg);
    let link = common::group(&document, &format!("/links/{index}"));
    common::children_named(link, "path")[0]
        .attribute("d")
        .unwrap()
        .to_string()
}

fn commands(path: &str, command: &str) -> Vec<Vec<f32>> {
    let tokens: Vec<&str> = path.split(' ').collect();
    let width = match command {
        "A" => 7,
        "C" => 6,
        "Q" => 4,
        _ => 2,
    };
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| **token == command)
        .map(|(index, _)| {
            tokens[index + 1..=index + width]
                .iter()
                .map(|token| token.parse().unwrap())
                .collect()
        })
        .collect()
}

fn render_with(mut document: Value, corner: Option<f32>, bend: Option<&str>) -> common::Rendered {
    if let Some(corner) = corner {
        document["corner"] = json!(corner);
    }
    if let Some(bend) = bend {
        document["bend"] = json!(bend);
    }
    common::render_document_with_fixed_metrics(document)
}

#[test]
fn a_link_with_one_bend_draws_one_arc_of_the_default_radius_and_none_at_corner_0() {
    let rendered = render_with(dogleg_document(), None, None);
    assert_eq!(rendered.geometry.links[0].points.len(), 3);
    let path = link_path(&rendered.svg.svg, 0);
    let arcs = commands(&path, "A");
    assert_eq!(arcs.len(), 1, "{path}");
    assert_eq!(&arcs[0][..5], &[6.0, 6.0, 0.0, 0.0, 1.0], "{path}");

    let square = render_with(dogleg_document(), Some(0.0), None);
    let path = link_path(&square.svg.svg, 0);
    assert!(!path.contains('A') && !path.contains('C'), "{path}");
    assert_eq!(commands(&path, "L").len(), 2, "{path}");
}

#[test]
fn a_page_corner_sets_the_radius_and_a_curve_bend_draws_a_cubic_on_the_corner() {
    let rendered = render_with(dogleg_document(), Some(12.0), Some("curve"));
    let route = &rendered.geometry.links[0];
    let corner = route.points[1];
    let path = link_path(&rendered.svg.svg, 0);
    assert!(commands(&path, "A").is_empty(), "{path}");
    let curves = commands(&path, "C");
    assert_eq!(curves.len(), 1, "{path}");
    assert_eq!(
        curves[0][..4],
        [corner.x, corner.y, corner.x, corner.y],
        "{path}"
    );
    assert_eq!(curves[0][4..], [corner.x, corner.y + 12.0], "{path}");
}

#[test]
fn the_arrowhead_and_the_trimmed_path_end_are_the_same_at_every_radius() {
    let carriers = |rendered: &common::Rendered| -> Vec<String> {
        let document = common::parse_xml(&rendered.svg.svg);
        let link = common::group(&document, "/links/0");
        common::children_named(link, "line")
            .iter()
            .map(|line| {
                ["x1", "y1", "x2", "y2"]
                    .map(|name| line.attribute(name).unwrap())
                    .join(" ")
            })
            .collect()
    };
    let last_point = |rendered: &common::Rendered| -> Vec<f32> {
        let path = link_path(&rendered.svg.svg, 0);
        commands(&path, "L").last().unwrap().clone()
    };
    let square = render_with(dogleg_document(), Some(0.0), None);
    assert_eq!(carriers(&square).len(), 1);
    for corner in [6.0, 16.0] {
        let rounded = render_with(dogleg_document(), Some(corner), None);
        assert_eq!(carriers(&rounded), carriers(&square), "corner {corner}");
        assert_eq!(last_point(&rounded), last_point(&square), "corner {corner}");
    }
}

/// The plain grammar with a house style on solid lines.
fn plain_with_solid_kind(corner: f32, bend: Bend) -> Grammar {
    let mut grammar = builtin_grammar("plain").unwrap().unwrap();
    grammar.lines.push(LineKind {
        line: Line::Solid,
        corner: Some(corner),
        bend: Some(bend),
    });
    grammar
}

#[test]
fn a_line_kind_of_the_grammar_wins_over_the_page() {
    let node = |id: &str| json!({ "tag": "Item", "kind": "service", "id": id, "title": id });
    let mut document = common::page_document(
        json!([{ "tag": "Row", "gap": 64, "children": [node("a"), node("b"), node("c")] }]),
        json!([
            { "line": "solid", "text": "calls" },
            { "line": "dash", "text": "replicates" }
        ]),
    );
    document["grammar"] = json!("plain");
    document["corner"] = json!(10);
    document["bend"] = json!("arc");
    document["links"] = json!([
        { "from": "a", "to": "b", "line": "solid" },
        { "from": "b", "to": "c", "line": "dash" }
    ]);
    let page: Page = serde_json::from_value(document).unwrap();
    let grammar = plain_with_solid_kind(3.0, Bend::Curve);
    let geometry = layout_page(&page, &grammar, &mut FixedMetricsMeasurer::default()).unwrap();
    let solid = &geometry.links[0];
    assert_eq!((solid.corner, solid.bend), (3.0, Bend::Curve));
    let dash = &geometry.links[1];
    assert_eq!((dash.corner, dash.bend), (10.0, Bend::Arc));

    let mut unset = page.clone();
    unset.corner = None;
    unset.bend = None;
    let geometry = layout_page(&unset, &grammar, &mut FixedMetricsMeasurer::default()).unwrap();
    assert_eq!(
        (geometry.links[1].corner, geometry.links[1].bend),
        (6.0, Bend::Arc)
    );
}

struct Example {
    stem: &'static str,
    document: &'static str,
    square_svg: &'static str,
}

/// Examples with links, flat solid, deny and dash, and iso solid, dash and gray, with
/// their center SVG as drawn before links had rounded bends.
const SQUARE_EXAMPLES: [Example; 3] = [
    Example {
        stem: "onepager",
        document: include_str!("../../../examples/onepager.json"),
        square_svg: include_str!("fixtures/onepager.square.svg"),
    },
    Example {
        stem: "org",
        document: include_str!("../../../examples/org.json"),
        square_svg: include_str!("fixtures/org.square.svg"),
    },
    Example {
        stem: "hero-iso",
        document: include_str!("../../../examples/hero-iso.json"),
        square_svg: include_str!("fixtures/hero-iso.square.svg"),
    },
];

fn vetted(example: &Example) -> (Page, Grammar) {
    let named: Page = serde_json::from_str(example.document).unwrap();
    let grammar = match named.grammar.as_deref() {
        Some("plain") => builtin_grammar("plain").unwrap().unwrap(),
        _ => common::gcp(),
    };
    (parse_and_vet(example.document, &grammar).unwrap(), grammar)
}

fn layout(page: &Page, grammar: &Grammar) -> PageGeometry {
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    layout_page(page, grammar, &mut measurer).unwrap()
}

#[test]
fn corner_0_draws_every_example_as_before_byte_for_byte() {
    let mut examined = 0;
    for example in &SQUARE_EXAMPLES {
        let (mut page, grammar) = vetted(example);
        page.corner = Some(0.0);
        let geometry = layout(&page, &grammar);
        assert!(!geometry.links.is_empty(), "{}", example.stem);
        let rendered = render_svg(&page, &common::theme("center"), &geometry).unwrap();
        assert!(
            rendered.svg == example.square_svg,
            "{}: corner 0 differs from tests/fixtures/{}.square.svg",
            example.stem,
            example.stem
        );
        examined += 1;
    }
    assert_eq!(examined, SQUARE_EXAMPLES.len());
}

#[test]
fn the_measured_json_is_the_same_at_every_radius_and_bend() {
    let measured = |page: &Page, grammar: &Grammar, document: &Value| -> String {
        let geometry = layout(page, grammar);
        let scene = match page.projection {
            Projection::Flat => None,
            Projection::Iso => {
                let inputs = SolidInputs::new(
                    &geometry,
                    &page.links,
                    common::theme("center").iso.slab_thickness,
                );
                Some(project_page(&geometry, &inputs).unwrap())
            }
        };
        serde_json::to_string(&measured_json(document, &geometry, scene.as_ref())).unwrap()
    };
    let mut examined = 0;
    for example in &SQUARE_EXAMPLES {
        let (page, grammar) = vetted(example);
        let document: Value = serde_json::from_str(example.document).unwrap();
        let mut square = page.clone();
        square.corner = Some(0.0);
        let expected = measured(&square, &grammar, &document);
        for (corner, bend) in [
            (6.0, Bend::Arc),
            (16.0, Bend::Arc),
            (16.0, Bend::Curve),
            (6.0, Bend::Spline),
        ] {
            let mut rounded = page.clone();
            rounded.corner = Some(corner);
            rounded.bend = Some(bend);
            assert!(
                measured(&rounded, &grammar, &document) == expected,
                "{}: measured JSON moves at corner {corner} {bend:?}",
                example.stem
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 4 * SQUARE_EXAMPLES.len());
}

/// Every `A` of the patterned links under iso: a floor quarter circle projects to an
/// ellipse with its axes on the screen's, rx over ry the square root of 3. A link drawn as a
/// tube per leg with round joints is the same at every radius.
#[test]
fn an_iso_dashed_link_bends_on_a_projected_ellipse_and_a_tube_keeps_its_joints() {
    let (page, grammar) = vetted(&SQUARE_EXAMPLES[2]);
    let geometry = layout(&page, &grammar);
    let rendered = render_svg(&page, &common::theme("center"), &geometry).unwrap();
    let mut square_page = page.clone();
    square_page.corner = Some(0.0);
    let square = render_svg(&square_page, &common::theme("center"), &geometry).unwrap();
    let document = common::parse_xml(&rendered.svg);
    let square_document = common::parse_xml(&square.svg);
    let mut arcs = 0;
    let mut tubes = 0;
    for index in 0..geometry.links.len() {
        let pointer = format!("/links/{index}");
        let link = common::group(&document, &pointer);
        let stroked: Vec<roxmltree::Node<'_, '_>> = common::children_named(link, "path")
            .into_iter()
            .filter(|path| path.attribute("fill") == Some("none"))
            .collect();
        let Some(stroked) = stroked.first() else {
            let square_link = common::group(&square_document, &pointer);
            assert_eq!(
                &rendered.svg[link.range()],
                &square.svg[square_link.range()],
                "{pointer}"
            );
            tubes += 1;
            continue;
        };
        for arc in commands(stroked.attribute("d").unwrap(), "A") {
            let ratio = arc[0] / arc[1];
            assert!((ratio - 3.0_f32.sqrt()).abs() < 0.01, "{arc:?}");
            assert_eq!(arc[2..4], [0.0, 0.0]);
            arcs += 1;
        }
    }
    assert!(tubes > 0, "hero-iso has no link drawn as a tube");
    assert!(arcs > 0, "no patterned link of hero-iso has a bend");
}

#[test]
fn a_spline_bend_runs_from_the_middle_of_one_leg_to_the_middle_of_the_next() {
    let rendered = render_with(dogleg_document(), None, Some("spline"));
    let route = &rendered.geometry.links[0];
    let [start, corner, tip] = [route.points[0], route.points[1], route.points[2]];
    let path = link_path(&rendered.svg.svg, 0);
    assert!(
        commands(&path, "A").is_empty() && commands(&path, "C").is_empty(),
        "{path}"
    );
    let splines = commands(&path, "Q");
    assert_eq!(splines.len(), 1, "{path}");
    // The end leg is drawn up to the arrowhead base, 10 px short of the tip.
    let base_y = tip.y - 10.0;
    let close = |actual: f32, expected: f32| (actual - expected).abs() <= 0.006;
    assert!(
        close(splines[0][0], corner.x) && close(splines[0][1], corner.y),
        "{path}"
    );
    assert!(close(splines[0][2], corner.x), "{path}");
    assert!(close(splines[0][3], (corner.y + base_y) / 2.0), "{path}");
    let entry = commands(&path, "L")[0].clone();
    assert!(close(entry[0], (start.x + corner.x) / 2.0), "{path}");
    let report = stencil_layout::checks::links_avoid_boxes(&rendered.geometry);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
}

/// `a` and `b` at their content widths either side of a frame taller than both, linked
/// bottom to bottom: the route runs under the frame, and a spline at either elbow would cut
/// the frame's lower corner.
fn under_the_frame_document() -> Value {
    let mut document = common::page_document(
        json!([{ "tag": "Row", "gap": 16, "grow": [0, 1, 0], "children": [
            { "tag": "Col", "children": [card("a")] },
            { "tag": "Col", "children": [{ "tag": "Frame", "label": "console", "height": 160 }] },
            { "tag": "Col", "children": [card("b")] }
        ]}]),
        json!([{ "line": "solid", "tint": 1, "text": "request path" }]),
    );
    document["bend"] = json!("spline");
    document["links"] = json!([
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": "none", "from_side": "bottom", "to_side": "bottom" }
    ]);
    document
}

#[test]
fn a_spline_bend_that_would_cross_a_box_falls_back_to_a_curve() {
    let rendered = common::render_document_with_fixed_metrics(under_the_frame_document());
    let route = &rendered.geometry.links[0];
    assert_eq!(route.points.len(), 4, "{:?}", route.points);
    let path = link_path(&rendered.svg.svg, 0);
    assert!(commands(&path, "Q").is_empty(), "{path}");
    let curves = commands(&path, "C");
    assert_eq!(curves.len(), 2, "{path}");
    assert_eq!(curves[0][..2], [route.points[1].x, route.points[1].y]);
    let report = stencil_layout::checks::links_avoid_boxes(&rendered.geometry);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
}

#[test]
fn under_iso_a_spline_draws_as_a_curve() {
    let (page, grammar) = vetted(&SQUARE_EXAMPLES[2]);
    let mut spline = page.clone();
    spline.bend = Some(Bend::Spline);
    let mut curve = page;
    curve.bend = Some(Bend::Curve);
    let geometry = layout(&spline, &grammar);
    let theme = common::theme("center");
    assert_eq!(
        render_svg(&spline, &theme, &geometry).unwrap().svg,
        render_svg(&curve, &theme, &layout(&curve, &grammar))
            .unwrap()
            .svg
    );
}
