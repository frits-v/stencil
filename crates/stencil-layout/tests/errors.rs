#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::page_with_body;
use serde_json::json;
use stencil_layout::{LayoutError, layout_page};
use stencil_model::VetRule;
use stencil_model::text::{FixedMetricsMeasurer, MeasureError};

#[test]
fn missing_glyph_is_a_measure_error_at_the_text_pointer() {
    let page = page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [
            { "tag": "Pcard", "fn": "Clean" },
            { "tag": "Pcard", "fn": "Router \u{4E00}" }
        ]}]),
    );
    let mut measurer = FixedMetricsMeasurer {
        advance_em: 0.5,
        missing_glyphs: vec!['\u{4E00}'],
    };
    let error = layout_page(&page, &mut measurer).unwrap_err();
    match &error {
        LayoutError::Measure { pointer, source } => {
            assert_eq!(pointer.as_str(), "/body/0/children/1/fn");
            assert!(matches!(
                source,
                MeasureError::MissingGlyph {
                    character: '\u{4E00}',
                    ..
                }
            ));
        }
        other => panic!("expected a measure error, got {other:?}"),
    }
    assert!(
        error
            .to_string()
            .starts_with("text at /body/0/children/1/fn could not be measured")
    );
}

#[test]
fn missing_glyph_in_page_text_names_the_page_field() {
    let page = page_with_body(1280, json!([{ "tag": "Fact", "text": "clean" }]));
    let mut measurer = FixedMetricsMeasurer {
        advance_em: 0.5,
        missing_glyphs: vec!['L'],
    };
    let error = layout_page(&page, &mut measurer).unwrap_err();
    let LayoutError::Measure { pointer, .. } = error else {
        panic!("expected a measure error");
    };
    assert_eq!(pointer.as_str(), "/lede");
}

#[test]
fn invalid_page_is_rejected_before_layout() {
    let page = page_with_body(
        1280,
        json!([{ "tag": "Row", "gap": 65, "children": [{ "tag": "Pcard", "fn": " padded" }] }]),
    );
    let error = layout_page(&page, &mut FixedMetricsMeasurer::default()).unwrap_err();
    let LayoutError::Invalid(violations) = &error else {
        panic!("expected Invalid, got {error:?}");
    };
    let rules: Vec<VetRule> = violations.iter().map(|violation| violation.rule).collect();
    assert!(rules.contains(&VetRule::GapOutOfRange));
    assert!(rules.contains(&VetRule::TextUntrimmed));
    assert_eq!(
        error.to_string(),
        format!("page fails vet: {} violation(s)", violations.len())
    );
}

#[test]
fn page_width_outside_the_range_is_rejected() {
    let page = page_with_body(639, json!([{ "tag": "Fact", "text": "narrow" }]));
    let error = layout_page(&page, &mut FixedMetricsMeasurer::default()).unwrap_err();
    assert!(matches!(error, LayoutError::Invalid(_)));
    let page = page_with_body(640, json!([{ "tag": "Fact", "text": "narrow" }]));
    assert!(layout_page(&page, &mut FixedMetricsMeasurer::default()).is_ok());
}
