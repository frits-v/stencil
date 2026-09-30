#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::json;
use stencil_layout::checks::{child_inside_container, siblings_do_not_overlap, text_fits_box};
use stencil_layout::{NodeGeometry, Part, PartName};

/// Fixed-metrics sizes: block_body is 12 px at 0.5 em (6 px per character) on a 17.4 px
/// line, card_function 13 px (6.5 px per character) on a 15.6 px line.
const BODY_LINE_PX: f32 = 17.4;
const HEADING_LINE_PX: f32 = 15.6;
const BLOCK_INSET_PX: f32 = 1.25 + 12.0;

fn parts_named(block: &NodeGeometry, name: PartName) -> Vec<&Part> {
    block
        .parts
        .iter()
        .filter(|part| part.name == name)
        .collect()
}

#[test]
fn numbered_text_measures_one_run_per_line_with_its_number_in_the_indent() {
    let geometry = layout(&page_with_body(
        640,
        json!([{
            "tag": "Text", "list": "numbered",
            "body": ["First", "Second", "Third"]
        }]),
    ));
    let block = node(&geometry, "/body/0");
    let lines = parts_named(block, PartName::BodyLine);
    let markers = parts_named(block, PartName::Marker);
    assert_eq!(lines.len(), 3);
    assert_eq!(markers.len(), 3);
    for (index, (line, marker)) in lines.iter().zip(&markers).enumerate() {
        let run = line.text.as_ref().expect("a body line carries a run");
        assert_eq!(run.metrics.line_count, 1);
        assert_eq!(run.text, ["First", "Second", "Third"][index]);
        let number = marker
            .text
            .as_ref()
            .expect("a numbered marker carries a run");
        assert_eq!(number.text, format!("{}.", index + 1));
        assert_close(marker.bounds.width, 22.0, "marker width");
        assert_close(marker.bounds.x, block.content.x, "marker x");
        assert_close(line.bounds.x, block.content.x + 22.0, "hanging indent");
        assert_close(line.bounds.y, marker.bounds.y, "marker on the first line");
    }
    assert_close(
        lines[1].bounds.y,
        lines[0].bounds.y + BODY_LINE_PX + 4.0,
        "line gap",
    );
    assert_close(
        lines[2].bounds.y,
        lines[1].bounds.y + BODY_LINE_PX + 4.0,
        "line gap",
    );
}

#[test]
fn a_wrapped_list_line_keeps_its_hanging_indent() {
    let long_line = "word ".repeat(40).trim_end().to_string();
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Text", "list": "numbered", "body": [long_line] }]),
    ));
    let block = node(&geometry, "/body/0");
    let line = part(block, PartName::BodyLine);
    let marker = part(block, PartName::Marker);
    let run = line.text.as_ref().unwrap();
    assert!(run.metrics.line_count > 1, "{:?}", run.metrics);
    assert_close(line.bounds.x, block.content.x + 22.0, "indent");
    assert_close(line.bounds.width, block.content.width - 22.0, "wrap width");
    assert_close(
        line.bounds.height,
        run.metrics.line_count as f32 * BODY_LINE_PX,
        "line height",
    );
    assert_close(
        marker.bounds.height,
        BODY_LINE_PX,
        "marker is one line tall",
    );
}

#[test]
fn bulleted_markers_are_indent_cells_without_runs() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Text", "list": "bulleted", "body": ["One", "Two"] }]),
    ));
    let block = node(&geometry, "/body/0");
    let markers = parts_named(block, PartName::Marker);
    assert_eq!(markers.len(), 2);
    for marker in markers {
        assert!(marker.text.is_none());
        assert_close(marker.bounds.width, 22.0, "marker width");
        assert_close(marker.bounds.height, BODY_LINE_PX, "marker height");
    }
    for line in parts_named(block, PartName::BodyLine) {
        assert_close(line.bounds.x, block.content.x + 22.0, "indent");
    }
}

#[test]
fn plain_text_stacks_heading_and_lines_inside_twelve_px_padding() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Text", "heading": "Problem", "body": ["One line", "Two line"] }]),
    ));
    let block = node(&geometry, "/body/0");
    let body = node(&geometry, "/body");
    assert_close(
        block.bounds.width,
        body.content.width,
        "width from the container",
    );
    assert!(parts_named(block, PartName::Marker).is_empty());
    let heading = part(block, PartName::Heading);
    assert_close(
        heading.bounds.x,
        block.bounds.x + BLOCK_INSET_PX,
        "heading x",
    );
    assert_close(
        heading.bounds.y,
        block.bounds.y + BLOCK_INSET_PX,
        "heading y",
    );
    assert_close(heading.bounds.height, HEADING_LINE_PX, "heading height");
    let lines = parts_named(block, PartName::BodyLine);
    assert_close(lines[0].bounds.x, block.content.x, "plain line x");
    assert_close(
        lines[0].bounds.y,
        heading.bounds.bottom() + 6.0,
        "6 px under the heading",
    );
    let expected_height = 2.0 * BLOCK_INSET_PX + HEADING_LINE_PX + 6.0 + 2.0 * BODY_LINE_PX + 4.0;
    assert_close(block.bounds.height, expected_height, "text height");
}

#[test]
fn blocks_in_a_row_share_its_width_like_facts() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Row", "gap": 0, "children": [
            { "tag": "Text", "body": ["Left"] },
            { "tag": "Callout", "kind": "note", "text": "Middle" },
            { "tag": "Frame", "label": "Right" }
        ]}]),
    ));
    let text_block = node(&geometry, "/body/0/children/0");
    let callout = node(&geometry, "/body/0/children/1");
    let frame = node(&geometry, "/body/0/children/2");
    let row = node(&geometry, "/body/0");
    assert_close(
        text_block.bounds.width + callout.bounds.width + frame.bounds.width,
        row.bounds.width,
        "the three blocks fill the row",
    );
    // Each grows from a basis of its own padding and border (section 2.3 weight 1).
    let share = (640.0 - 26.5 - 30.5 - 18.5) / 3.0;
    assert_close(text_block.bounds.width, share + 26.5, "text share");
    assert_close(callout.bounds.width, share + 30.5, "callout share");
    assert_close(frame.bounds.width, share + 18.5, "frame share");
}

#[test]
fn callout_accent_fills_the_left_padding_inside_the_border() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Callout", "kind": "decision", "title": "Queue", "text": "Runs later." }]),
    ));
    let callout = node(&geometry, "/body/0");
    let accent = part(callout, PartName::Accent);
    assert!(accent.text.is_none());
    assert_close(accent.bounds.x, callout.bounds.x + 1.25, "accent x");
    assert_close(accent.bounds.y, callout.bounds.y + 1.25, "accent y");
    assert_close(accent.bounds.width, 4.0, "accent width");
    assert_close(
        accent.bounds.height,
        callout.bounds.height - 2.5,
        "accent height",
    );
    let heading = part(callout, PartName::Heading);
    assert_close(heading.bounds.x, callout.bounds.x + 1.25 + 16.0, "title x");
    assert_close(
        callout.content.x,
        callout.bounds.x + 1.25 + 16.0,
        "content x",
    );
    let text = part(callout, PartName::Text);
    assert_close(
        text.bounds.y,
        heading.bounds.bottom() + 6.0,
        "text under title",
    );
    assert_close(
        callout.bounds.height,
        2.0 * BLOCK_INSET_PX + HEADING_LINE_PX + 6.0 + BODY_LINE_PX,
        "callout height",
    );
}

#[test]
fn frame_height_is_honored_beside_a_taller_sibling_and_under_a_grow_weight() {
    let tall_lines: Vec<String> = (1..=12).map(|index| format!("Line {index}")).collect();
    let geometry = layout(&page_with_body(
        640,
        json!([
            { "tag": "Row", "children": [
                { "tag": "Frame", "label": "Screen", "height": 60 },
                { "tag": "Text", "body": tall_lines }
            ]},
            { "tag": "Row", "children": [
                { "tag": "Col", "grow": [1, 0], "children": [
                    { "tag": "Frame", "label": "Grown", "height": 80 },
                    { "tag": "Fact", "text": "under" }
                ]},
                { "tag": "Text", "body": tall_lines }
            ]}
        ]),
    ));
    let row = node(&geometry, "/body/0");
    assert!(row.bounds.height > 100.0, "the Text makes the row tall");
    assert_close(
        node(&geometry, "/body/0/children/0").bounds.height,
        60.0,
        "frame",
    );
    assert_close(
        node(&geometry, "/body/1/children/0/children/0")
            .bounds
            .height,
        80.0,
        "frame with grow",
    );
}

#[test]
fn frame_without_height_is_two_hundred_tall() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Frame", "label": "Screen" }]),
    ));
    assert_close(
        node(&geometry, "/body/0").bounds.height,
        200.0,
        "default height",
    );
}

#[test]
fn frame_label_chip_is_centered_and_wraps_inside_the_frame() {
    let long_label = "screen ".repeat(30).trim_end().to_string();
    let geometry = layout(&page_with_body(
        640,
        json!([
            { "tag": "Frame", "label": "Editor", "height": 100 },
            { "tag": "Frame", "label": long_label, "height": 200 }
        ]),
    ));
    let short = node(&geometry, "/body/0");
    let chip = part(short, PartName::LabelChip);
    assert_close(
        chip.bounds.x + chip.bounds.width / 2.0,
        short.bounds.x + short.bounds.width / 2.0,
        "chip centered across",
    );
    assert_close(
        chip.bounds.y + chip.bounds.height / 2.0,
        short.bounds.y + short.bounds.height / 2.0,
        "chip centered down",
    );
    assert_close(chip.bounds.width, 6.0 * 6.0 + 16.0, "chip hugs the label");

    let long = node(&geometry, "/body/1");
    let long_chip = part(long, PartName::LabelChip);
    let label = part(long, PartName::Label);
    assert!(label.text.as_ref().unwrap().metrics.line_count > 1);
    assert!(long_chip.bounds.width <= long.content.width + 0.01);
    assert!(long_chip.bounds.x >= long.content.x - 0.01);
}

#[test]
fn a_page_of_text_callout_and_frame_blocks_has_no_geometry_defects() {
    let geometry = layout(&page_with_body(
        960,
        json!([
            { "tag": "Row", "gap": 16, "children": [
                { "tag": "Col", "children": [
                    { "tag": "Text", "heading": "Goals", "list": "numbered",
                      "body": ["Accept a trigger in plain language.", "Evaluate every clip against each active trigger and store the verdict."] },
                    { "tag": "Text", "heading": "Non-goals", "list": "bulleted",
                      "body": ["Live streams.", "Identifying people."] },
                    { "tag": "Callout", "kind": "risk", "title": "Plan drift",
                      "text": "A model update can change how a description is read." }
                ]},
                { "tag": "Col", "children": [
                    { "tag": "Text", "body": ["A plain paragraph that is long enough to wrap onto a second line in this column."] },
                    { "tag": "Callout", "kind": "open", "text": "No title on this one." }
                ]}
            ]},
            { "tag": "Row", "gap": 16, "children": [
                { "tag": "Frame", "label": "Trigger editor", "height": 160 },
                { "tag": "Frame", "label": "Verdict review", "height": 160 }
            ]}
        ]),
    ));
    for report in [
        child_inside_container(&geometry),
        siblings_do_not_overlap(&geometry),
        text_fits_box(&geometry),
    ] {
        assert!(report.passed(), "{report:?}");
    }
}
