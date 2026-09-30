#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{layout, page_with_body, pointer};
use serde_json::json;
use stencil_layout::checks::{child_inside_container, siblings_do_not_overlap, text_fits_box};
use stencil_layout::{
    BoxRect, NodeGeometry, NodeTag, PageGeometry, Part, PartName, Size, TextAlign, TextRun,
};
use stencil_model::checks::CheckName;
use stencil_model::text::{TextLine, TextMetrics, TextStyleName};

fn rect(x: f32, y: f32, width: f32, height: f32) -> BoxRect {
    BoxRect {
        x,
        y,
        width,
        height,
    }
}

fn geometry_node(pointer_text: &str, parent: Option<usize>, bounds: BoxRect) -> NodeGeometry {
    NodeGeometry {
        pointer: pointer(pointer_text),
        tag: NodeTag::Fact,
        kind: None,
        parent,
        bounds,
        content: bounds,
        parts: Vec::new(),
    }
}

fn page(nodes: Vec<NodeGeometry>) -> PageGeometry {
    PageGeometry {
        canvas: Size {
            width: 100.0,
            height: 100.0,
        },
        nodes,
    }
}

fn root() -> NodeGeometry {
    let mut root = geometry_node("", None, rect(0.0, 0.0, 100.0, 100.0));
    root.tag = NodeTag::Page;
    root
}

fn run_of_width(width_px: f32, height_px: f32) -> TextRun {
    TextRun {
        text: "label".to_string(),
        style: TextStyleName::TagLabel.text_style().style,
        color: "#202124",
        align: TextAlign::Center,
        metrics: TextMetrics {
            width_px,
            height_px,
            line_count: 1,
            lines: vec![TextLine {
                byte_start: 0,
                byte_end: 5,
                width_px,
                baseline_px: 12.48,
            }],
        },
    }
}

#[test]
fn overlap_above_epsilon_is_a_defect_at_the_later_sibling() {
    let geometry = page(vec![
        root(),
        geometry_node("/kicker", Some(0), rect(10.0, 10.0, 30.0, 20.0)),
        geometry_node("/title", Some(0), rect(39.98, 10.0, 30.0, 20.0)),
    ]);
    let report = siblings_do_not_overlap(&geometry);
    assert_eq!(report.check, CheckName::SiblingsDoNotOverlap);
    assert_eq!(report.examined, 1);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/title");
    assert!(report.defects[0].message.contains("/kicker"));
    assert!(!report.passed());
}

#[test]
fn touching_siblings_pass() {
    let geometry = page(vec![
        root(),
        geometry_node("/kicker", Some(0), rect(10.0, 10.0, 30.0, 20.0)),
        geometry_node("/title", Some(0), rect(40.0, 10.0, 30.0, 20.0)),
        geometry_node("/lede", Some(0), rect(10.0, 30.0, 30.0, 20.0)),
    ]);
    let report = siblings_do_not_overlap(&geometry);
    assert_eq!(report.examined, 3);
    assert!(report.passed(), "{:?}", report.defects);
}

#[test]
fn overlap_in_one_axis_only_passes() {
    let geometry = page(vec![
        root(),
        geometry_node("/kicker", Some(0), rect(10.0, 10.0, 30.0, 20.0)),
        geometry_node("/title", Some(0), rect(20.0, 40.0, 30.0, 20.0)),
    ]);
    assert!(siblings_do_not_overlap(&geometry).passed());
}

#[test]
fn child_outside_parent_content_by_0_02_is_a_defect() {
    let mut parent = geometry_node("/body", Some(0), rect(10.0, 10.0, 80.0, 80.0));
    parent.content = rect(20.0, 20.0, 60.0, 60.0);
    let geometry = page(vec![
        root(),
        parent.clone(),
        geometry_node("/body/0", Some(1), rect(20.0, 20.0, 60.02, 30.0)),
    ]);
    let report = child_inside_container(&geometry);
    assert_eq!(report.examined, 2);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0");

    let inside = page(vec![
        root(),
        parent,
        geometry_node("/body/0", Some(1), rect(20.0, 20.0, 60.005, 60.0)),
    ]);
    assert!(child_inside_container(&inside).passed());
}

#[test]
fn text_run_wider_than_its_part_by_0_02_is_a_defect() {
    let mut pipe = geometry_node("/body/0", Some(0), rect(0.0, 0.0, 100.0, 40.0));
    pipe.parts = vec![Part {
        name: PartName::TagLabel,
        bounds: rect(10.0, 10.0, 38.0, 15.6),
        text: Some(run_of_width(38.02, 15.6)),
    }];
    let geometry = page(vec![root(), pipe.clone()]);
    let report = text_fits_box(&geometry);
    assert_eq!(report.examined, 1);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0");
    assert!(
        report.defects[0]
            .message
            .starts_with("tag_label \"label\" measured 38.02x15.60 in box 38.00x15.60")
    );

    pipe.parts[0].text = Some(run_of_width(38.005, 15.6));
    assert!(text_fits_box(&page(vec![root(), pipe])).passed());
}

#[test]
fn text_part_outside_its_node_is_a_defect() {
    let mut note = geometry_node("/body/0", Some(0), rect(0.0, 0.0, 50.0, 20.0));
    note.parts = vec![Part {
        name: PartName::Text,
        bounds: rect(0.0, 0.0, 50.02, 20.0),
        text: Some(run_of_width(40.0, 15.6)),
    }];
    let report = text_fits_box(&page(vec![root(), note]));
    assert_eq!(report.defects.len(), 1);
    assert!(report.defects[0].message.starts_with("text box"));
}

#[test]
fn root_only_geometry_examines_nothing_and_fails() {
    let geometry = page(vec![root()]);
    for report in [
        child_inside_container(&geometry),
        siblings_do_not_overlap(&geometry),
        text_fits_box(&geometry),
    ] {
        assert_eq!(report.examined, 0, "{:?}", report.check);
        assert!(report.defects.is_empty());
        assert!(!report.passed(), "{:?}", report.check);
    }
}

/// Section 2.5 keeps the widest word as the text column's floor, so the card grows to fit
/// the word and overflows the Row: child-inside-container reports it, and the run still fits
/// its own part box, so text-fits-box does not.
#[test]
fn card_with_a_120_character_word_overflows_its_row() {
    let word = "w".repeat(120);
    let page = page_with_body(
        640,
        json!([{ "tag": "Row", "children": [
            { "tag": "Pcard", "fn": word },
            { "tag": "Pcard", "fn": "Second" },
            { "tag": "Pcard", "fn": "Third" }
        ]}]),
    );
    let geometry = layout(&page);
    let inside = child_inside_container(&geometry);
    assert!(!inside.passed());
    let pointers: Vec<&str> = inside
        .defects
        .iter()
        .map(|defect| defect.pointer.as_str())
        .collect();
    assert_eq!(
        pointers,
        [
            "/body/0/children/0",
            "/body/0/children/1",
            "/body/0/children/2"
        ]
    );
    assert!(inside.defects[0].message.contains("/body/0"));

    let fits = text_fits_box(&geometry);
    assert_eq!(fits.examined, 7);
    assert!(fits.defects.is_empty(), "{:?}", fits.defects);
}

/// Body: a Row of Pcard, Pipe, Pcard, then a Fact; five body nodes.
/// Geometry nodes: root, /kicker, /title, /lede, /body and the five, so 9 relations.
/// Sibling pairs: root has 4 children (6), /body 2 (1), the Row 3 (3), so 10.
/// Text runs: kicker 2, title 1, lede 1, two fn, one tag label, one fact, so 8.
#[test]
fn small_document_has_hand_derived_counts() {
    let page = page_with_body(
        1280,
        json!([
            { "tag": "Row", "children": [
                { "tag": "Pcard", "fn": "Left" },
                { "tag": "Pipe", "dir": "h", "kind": "gray", "label": "call" },
                { "tag": "Pcard", "fn": "Right" }
            ]},
            { "tag": "Fact", "text": "A fact under the row" }
        ]),
    );
    let geometry = layout(&page);
    assert_eq!(geometry.nodes.len(), 10);
    let inside = child_inside_container(&geometry);
    let overlap = siblings_do_not_overlap(&geometry);
    let fits = text_fits_box(&geometry);
    assert_eq!(
        (inside.examined, overlap.examined, fits.examined),
        (9, 10, 8)
    );
    assert!(inside.passed() && overlap.passed() && fits.passed());
}

#[test]
fn g7_geometry_checks_have_the_section_9_4_counts_and_pass() {
    let geometry = layout(&common::g7_page());
    let inside = child_inside_container(&geometry);
    let overlap = siblings_do_not_overlap(&geometry);
    let fits = text_fits_box(&geometry);
    assert_eq!(inside.examined, 33);
    assert_eq!(overlap.examined, 32);
    assert_eq!(fits.examined, 40);
    assert!(inside.passed(), "{:?}", inside.defects);
    assert!(overlap.passed(), "{:?}", overlap.defects);
    assert!(fits.passed(), "{:?}", fits.defects);
}

/// The root stretches `/title` to the content width with no min-content floor, so a single
/// 200-character word keeps its box at 640 wide and the run overflows it: text-fits-box
/// reports it and child-inside-container does not.
#[test]
fn title_with_a_200_character_word_overflows_its_text_box() {
    let mut page = page_with_body(640, json!([{ "tag": "Fact", "text": "Short" }]));
    page.title = "w".repeat(200);
    let geometry = layout(&page);

    let fits = text_fits_box(&geometry);
    assert_eq!(fits.defects.len(), 1, "{:?}", fits.defects);
    assert_eq!(fits.defects[0].pointer.as_str(), "/title");
    assert!(
        fits.defects[0]
            .message
            .ends_with("measured 1920.00x24.00 in box 640.00x24.00"),
        "{}",
        fits.defects[0].message
    );

    let inside = child_inside_container(&geometry);
    assert!(inside.passed(), "{:?}", inside.defects);
}

#[test]
fn parent_index_outside_the_geometry_is_a_defect_in_both_structural_checks() {
    let geometry = page(vec![
        root(),
        geometry_node("/kicker", Some(0), rect(10.0, 10.0, 30.0, 20.0)),
        geometry_node("/title", Some(7), rect(10.0, 40.0, 30.0, 20.0)),
    ]);
    for report in [
        child_inside_container(&geometry),
        siblings_do_not_overlap(&geometry),
    ] {
        assert_eq!(report.defects.len(), 1, "{:?}", report.check);
        assert_eq!(report.defects[0].pointer.as_str(), "/title");
        assert_eq!(
            report.defects[0].message,
            "parent index 7 is not a geometry node"
        );
        assert!(report.examined >= 1, "{:?}", report.check);
        assert!(!report.passed(), "{:?}", report.check);
    }
}
