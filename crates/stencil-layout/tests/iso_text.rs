// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 12.4 rule 4: type and lines on a plane.

mod common;

use common::{layout, node, page_from, part};
use serde_json::{Value, json};
use stencil_layout::{ISO_LINE_MIN_PX, PartName, WRAP_EPSILON_PX};
use stencil_model::text::{FixedMetricsMeasurer, TextMeasurer};

/// A narrow customer page holding `body`, flat or under iso.
fn page(body: Value, iso: bool) -> stencil_model::Page {
    let mut document = json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": 640,
        "canvas": "customer",
        "body": body,
        "legend": []
    });
    if iso {
        document["projection"] = json!("iso");
    }
    page_from(document)
}

/// A Row of `count` copies of `child` at the page width, so each copy gets a fifth or a
/// quarter of 640 px and its text is squeezed.
fn narrow_row(child: &Value, count: usize) -> Value {
    json!([{ "tag": "Row", "children": vec![child.clone(); count] }])
}

/// A gcp frame holding a region holding `children`: the two labelled tiers over a name.
fn tiers(children: Value) -> Value {
    json!({ "tag": "Box", "kind": "gcp", "label": "Frame label", "children": [
        { "tag": "Box", "kind": "region", "label": "Zone label", "children": children }
    ] })
}

const FIRST_ITEM: &str = "/body/0/children/0";

#[test]
fn the_frame_label_is_a_tier_above_the_zone_label_under_iso() {
    let geometry = layout(&page(
        json!([tiers(
            json!([{ "tag": "Item", "kind": "product", "title": "Name" }])
        )]),
        true,
    ));
    let frame = part(node(&geometry, "/body/0"), PartName::Label)
        .text
        .as_ref()
        .unwrap();
    let zone = part(node(&geometry, "/body/0/children/0"), PartName::Label)
        .text
        .as_ref()
        .unwrap();
    assert_eq!(frame.style.size_px, 18.0 * 1.4);
    assert_eq!(zone.style.size_px, 12.0 * 1.25);
    assert!(frame.style.size_px > zone.style.size_px);
}

/// A name, a zone label, a frame label and a tag label never wrap under iso, however
/// narrow their box; flat, the same name wraps.
#[test]
fn a_name_a_zone_label_a_frame_label_and_a_tag_label_stay_on_one_line_under_iso() {
    let name = "acme prod analytics raw events eu west four";
    let mut frame = tiers(json!([
        { "tag": "Item", "kind": "product", "title": name },
        { "tag": "Col", "children": [
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "VLAN one two three four five" }
        ] }
    ]));
    frame["label"] = json!("Google Cloud production frame for the raw events");
    frame["children"][0]["label"] = json!("Region europe west four raw events zone");
    let body = narrow_row(&frame, 4);
    let iso = layout(&page(body.clone(), true));
    let frame_pointer = FIRST_ITEM;
    let zone_pointer = "/body/0/children/0/children/0";
    let name_pointer = "/body/0/children/0/children/0/children/0";
    let pipe_pointer = "/body/0/children/0/children/0/children/1/children/0";
    let runs = [
        (node(&iso, frame_pointer), PartName::Label, "frame label"),
        (node(&iso, zone_pointer), PartName::Label, "zone label"),
        (node(&iso, name_pointer), PartName::FunctionName, "name"),
        (node(&iso, pipe_pointer), PartName::TagLabel, "tag label"),
    ];
    for (owner, part_name, what) in runs {
        let run = part(owner, part_name).text.as_ref().unwrap();
        assert_eq!(run.metrics.line_count, 1, "{what}: {:?}", run.text);
        assert!(
            run.metrics.width_px > 120.0,
            "{what} is {} px wide, too short to prove anything",
            run.metrics.width_px
        );
    }
    let flat = layout(&page(body, false));
    let flat_name = part(node(&flat, name_pointer), PartName::FunctionName)
        .text
        .as_ref()
        .unwrap();
    assert!(flat_name.metrics.line_count >= 2, "{:?}", flat_name.metrics);
}

/// A subtitle squeezed by its row wraps as it would at ISO_LINE_MIN_PX, whatever its box
/// allows; flat, the same subtitle wraps at its box.
#[test]
fn a_subtitle_wraps_no_narrower_than_120_px_under_iso() {
    let subtitle = "managed relational database with automatic failover and backups";
    let item = json!({ "tag": "Item", "kind": "product", "title": "DB", "subtitle": subtitle });
    let body = narrow_row(&item, 5);
    let wrap_at_least = |run: &stencil_layout::TextRun| -> Vec<(usize, usize)> {
        FixedMetricsMeasurer::default()
            .measure(
                &run.text,
                &run.style,
                Some(ISO_LINE_MIN_PX + WRAP_EPSILON_PX),
            )
            .unwrap()
            .lines
            .iter()
            .map(|line| (line.byte_start, line.byte_end))
            .collect()
    };
    let ranges = |run: &stencil_layout::TextRun| -> Vec<(usize, usize)> {
        run.metrics
            .lines
            .iter()
            .map(|line| (line.byte_start, line.byte_end))
            .collect()
    };
    let iso = layout(&page(body.clone(), true));
    let iso_subtitle = part(node(&iso, FIRST_ITEM), PartName::ProductName);
    let run = iso_subtitle.text.as_ref().unwrap();
    assert!(
        iso_subtitle.bounds.width < ISO_LINE_MIN_PX,
        "the row squeezes the subtitle box to {} px",
        iso_subtitle.bounds.width
    );
    assert!(run.metrics.line_count >= 3, "{:?}", run.metrics);
    assert_eq!(ranges(run), wrap_at_least(run), "{:?}", run.metrics);
    let flat = layout(&page(body, false));
    let flat_subtitle = part(node(&flat, FIRST_ITEM), PartName::ProductName);
    let flat_run = flat_subtitle.text.as_ref().unwrap();
    assert!(flat_subtitle.bounds.width < ISO_LINE_MIN_PX);
    assert!(
        ranges(flat_run).len() > wrap_at_least(flat_run).len(),
        "{:?}",
        flat_run.metrics
    );
}

/// The spaces beside a middle dot become no-break spaces under iso, so a run never breaks
/// beside the dot; flat, the same run breaks there.
#[test]
fn a_run_never_breaks_beside_its_middle_dot_under_iso() {
    let subtitle = "Metro one two · Region A";
    let item = json!({ "tag": "Item", "kind": "product", "title": "Router", "subtitle": subtitle });
    let body = narrow_row(&item, 5);
    let iso = layout(&page(body.clone(), true));
    let run = part(node(&iso, FIRST_ITEM), PartName::ProductName)
        .text
        .as_ref()
        .unwrap();
    assert_eq!(run.text, "Metro one two\u{a0}·\u{a0}Region A");
    let dot = run.text.find('·').unwrap();
    let dot_line = run
        .metrics
        .lines
        .iter()
        .find(|line| line.byte_start <= dot && dot < line.byte_end)
        .unwrap();
    let text = &run.text[dot_line.byte_start..dot_line.byte_end];
    assert!(
        text.contains("two\u{a0}·\u{a0}R"),
        "{text:?} in {:?}",
        run.metrics
    );
    let flat = layout(&page(body, false));
    let flat_run = part(node(&flat, FIRST_ITEM), PartName::ProductName)
        .text
        .as_ref()
        .unwrap();
    assert_eq!(flat_run.text, subtitle);
    assert!(
        flat_run.metrics.lines.iter().any(|line| {
            let text = &flat_run.text[line.byte_start..line.byte_end];
            text.ends_with('·') || text.starts_with('·')
        }),
        "{:?}",
        flat_run.metrics
    );
}
