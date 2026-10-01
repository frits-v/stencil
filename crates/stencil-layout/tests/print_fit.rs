#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! print-fit (section 13.10): each text run printed at the print width, 8 pt the floor.

mod common;

use common::{layout, page_with_body};
use serde_json::json;
use stencil_layout::checks::{PrintWidth, print_fit, text_fits_box};
use stencil_model::checks::{CheckName, CheckOutcome};

/// The drawn canvas of a flat g7: width 1280 plus 20 px padding on each side.
const G7_CANVAS_PX: f32 = 1320.0;

#[test]
fn g7_at_14_inches_fails_once_at_the_badge() {
    let geometry = layout(&common::g7_page());
    let report = print_fit(
        &geometry,
        G7_CANVAS_PX,
        Some(PrintWidth::new(14.0).unwrap()),
    );
    assert_eq!(report.check, CheckName::PrintFit);
    assert_eq!(report.examined, 40);
    let found: Vec<(String, String)> = report
        .defects
        .iter()
        .map(|defect| (defect.pointer.to_string(), defect.message.clone()))
        .collect();
    assert_eq!(
        found,
        [(
            "/kicker".to_string(),
            "badge_text \"CUSTOMER\" prints at 7.64 pt, below 8 pt (10.00 px on a 1320.00 px canvas at 14.00 in)"
                .to_string()
        )]
    );
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn g7_at_16_inches_passes() {
    let geometry = layout(&common::g7_page());
    let report = print_fit(
        &geometry,
        G7_CANVAS_PX,
        Some(PrintWidth::new(16.0).unwrap()),
    );
    assert_eq!(report.examined, 40);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
    assert_eq!(report.outcome(), CheckOutcome::Passed);
}

#[test]
fn print_fit_counts_the_runs_text_fits_box_counts() {
    let geometry = layout(&common::g7_page());
    let printed = print_fit(
        &geometry,
        G7_CANVAS_PX,
        Some(PrintWidth::new(16.0).unwrap()),
    );
    assert_eq!(printed.examined, text_fits_box(&geometry).examined);
}

#[test]
fn without_a_print_width_print_fit_is_not_applicable() {
    let geometry = layout(&common::g7_page());
    let report = print_fit(&geometry, G7_CANVAS_PX, None);
    assert_eq!(report.not_applicable, Some("no print width"));
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
}

#[test]
fn a_wider_drawn_canvas_prints_smaller() {
    let geometry = layout(&common::g7_page());
    let width = Some(PrintWidth::new(16.0).unwrap());
    assert!(print_fit(&geometry, G7_CANVAS_PX, width).passed());
    let report = print_fit(&geometry, 1600.0, width);
    let badge = report
        .defects
        .iter()
        .find(|defect| defect.pointer.as_str() == "/kicker")
        .unwrap();
    assert!(
        badge
            .message
            .ends_with("(10.00 px on a 1600.00 px canvas at 16.00 in)"),
        "{}",
        badge.message
    );
}

#[test]
fn link_tag_runs_are_examined_at_the_link() {
    let mut page = page_with_body(
        1280,
        json!([{ "tag": "Row", "gap": 64, "children": [
            { "tag": "Item", "id": "api", "kind": "product", "title": "API" },
            { "tag": "Item", "id": "worker", "kind": "product", "title": "Worker" }
        ] }]),
    );
    page.links = vec![
        serde_json::from_value(json!({
            "from": "api", "to": "worker", "line": "solid", "label": "call", "sub": "async"
        }))
        .unwrap(),
    ];
    let geometry = layout(&page);
    let report = print_fit(&geometry, G7_CANVAS_PX, Some(PrintWidth::new(0.5).unwrap()));
    let link_defects: Vec<&str> = report
        .defects
        .iter()
        .filter(|defect| defect.pointer.as_str() == "/links/0")
        .map(|defect| defect.message.as_str())
        .collect();
    assert_eq!(link_defects.len(), 2, "{link_defects:?}");
    assert!(
        link_defects[0].starts_with("tag_label \"call\""),
        "{link_defects:?}"
    );
    assert!(
        link_defects[1].starts_with("tag_sub \"async\""),
        "{link_defects:?}"
    );
    assert_eq!(report.examined, text_fits_box(&geometry).examined);
}

#[test]
fn a_print_width_outside_half_an_inch_to_200_inches_is_rejected() {
    for inches in [0.5, 14.0, 200.0] {
        assert_eq!(PrintWidth::new(inches).unwrap().inches(), inches);
    }
    for inches in [0.4, 0.0, -1.0, 200.5, f32::NAN, f32::INFINITY] {
        assert!(PrintWidth::new(inches).is_err(), "{inches}");
    }
}
