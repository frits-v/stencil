#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_layout::{LeadRun, measure_lead_run};
use stencil_model::text::{
    FixedMetricsMeasurer, MeasureError, TextMetrics, TextStyle, TextStyleName,
};

/// A bold lead and a regular body of one size and line height: 13 px on 15.6, which the
/// fake advances 6.5 px per scalar.
fn lead_style() -> TextStyle {
    TextStyleName::CardFunction.text_style().style
}

fn body_style() -> TextStyle {
    TextStyleName::Lede.text_style().style
}

fn lead_run(lead: &str, body: &str, max_width_px: Option<f32>) -> LeadRun {
    measure_lead_run(
        &mut FixedMetricsMeasurer::default(),
        lead,
        &lead_style(),
        body,
        &body_style(),
        max_width_px,
    )
    .unwrap()
}

fn line_texts<'a>(text: &'a str, metrics: &TextMetrics) -> Vec<&'a str> {
    metrics
        .lines
        .iter()
        .map(|line| &text[line.byte_start..line.byte_end])
        .collect()
}

#[test]
fn a_body_that_fits_after_its_lead_continues_the_lead_line() {
    let run = lead_run("Ask", "the operator", Some(200.0));
    assert_eq!(run.body_indent_px, 19.5 + 6.5);
    assert_eq!(line_texts("the operator", &run.body), ["the operator"]);
    assert!(run.body_continues_lead_line());
    assert_eq!(run.body_line_x_px(0), 26.0);
    assert_eq!(run.body_top_px, 0.0);
    assert_eq!(run.body.width_px, 26.0 + 78.0);
    assert_eq!(run.width_px, 104.0);
    assert_eq!(run.height_px, 15.6);
}

#[test]
fn a_body_wraps_at_the_full_width_after_a_lead_that_wraps() {
    // The lead wraps to "Page the" and "owner" (32.5 px), so the body starts 39 px into the
    // second line with 21 px of room: "on" fits there, and later lines wrap at 60.
    let body = "on call within an hour";
    let run = lead_run("Page the owner", body, Some(60.0));
    assert_eq!(
        line_texts("Page the owner", &run.lead),
        ["Page the", "owner"]
    );
    assert_eq!(run.body_indent_px, 32.5 + 6.5);
    assert_eq!(
        line_texts(body, &run.body),
        ["on", "call", "within an", "hour"]
    );
    assert!(run.body_continues_lead_line());
    assert_eq!(run.body_top_px, 15.6);
    assert_eq!(run.body_line_x_px(0), 39.0);
    assert_eq!(run.body_line_x_px(1), 0.0);
    assert_eq!(run.body.width_px, 58.5);
    assert_eq!(run.width_px, 58.5);
    assert_eq!(run.height_px, 15.6 + run.body.height_px);
    assert_eq!(run.body.line_count, 4);
}

#[test]
fn a_body_whose_first_word_does_not_fit_after_its_lead_starts_on_the_next_line() {
    // The lead is 104 px and the indent 110.5, past the 110 px line.
    let body = "Escalate within";
    let run = lead_run("Ask the operator", body, Some(110.0));
    assert_eq!(run.body_indent_px, 110.5);
    assert_eq!(line_texts(body, &run.body), ["", "Escalate within"]);
    assert!(!run.body_continues_lead_line());
    assert_eq!(run.body_top_px, 0.0);
    assert_eq!(run.body.width_px, 97.5);
    assert_eq!(run.width_px, 104.0);
    assert_eq!(run.height_px, 2.0 * 15.6);

    let fits = lead_run("Ask the operator", body, Some(110.5 + 52.0));
    assert!(fits.body_continues_lead_line());
    assert_eq!(line_texts(body, &fits.body), ["Escalate", "within"]);
}

#[test]
fn a_lead_wider_than_the_line_puts_its_body_on_the_next_line() {
    let run = lead_run("Interconnect", "now", Some(50.0));
    assert_eq!(line_texts("Interconnect", &run.lead), ["Interconnect"]);
    assert_eq!(run.lead.width_px, 78.0);
    assert_eq!(line_texts("now", &run.body), ["", "now"]);
    assert_eq!(run.width_px, 78.0);
    assert_eq!(run.height_px, 2.0 * 15.6);
}

#[test]
fn a_lead_and_a_body_of_different_line_heights_are_rejected() {
    let taller_body = TextStyleName::BlockBody.text_style().style;
    let error = measure_lead_run(
        &mut FixedMetricsMeasurer::default(),
        "Ask",
        &lead_style(),
        "the operator",
        &taller_body,
        Some(200.0),
    )
    .unwrap_err();
    assert!(
        matches!(error, MeasureError::InvalidStyle { .. }),
        "{error:?}"
    );
}
