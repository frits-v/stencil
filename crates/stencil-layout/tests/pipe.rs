#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::{Value, json};
use stencil_layout::{NodeGeometry, PartName};

fn pipe(dir: &str, kind: &str, label: &str) -> Value {
    json!({ "tag": "Pipe", "dir": dir, "kind": kind, "label": label })
}

fn card(function_name: &str) -> Value {
    json!({ "tag": "Pcard", "fn": function_name })
}

fn wire_lengths(pipe_node: &NodeGeometry, horizontal: bool) -> (f32, f32) {
    let start = part(pipe_node, PartName::WireStart).bounds;
    let end = part(pipe_node, PartName::WireEnd).bounds;
    if horizontal {
        (start.width, end.width)
    } else {
        (start.height, end.height)
    }
}

#[test]
fn horizontal_pipe_in_a_col_spans_the_col_with_equal_wires() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [
            { "tag": "Col", "children": [card("A wide card with a long function name"), pipe("h", "blue", "VLAN 1")] },
            card("Right")
        ]}]),
    ));
    let column = node(&geometry, "/body/0/children/0");
    let pipe_node = node(&geometry, "/body/0/children/0/children/1");
    assert_close(pipe_node.bounds.x, column.bounds.x, "pipe x");
    assert_close(pipe_node.bounds.width, column.bounds.width, "pipe width");
    let (start, end) = wire_lengths(pipe_node, true);
    assert!((start - end).abs() <= 0.01, "wires {start} and {end}");
    assert!(start >= 14.0 && end >= 14.0);
    assert!(start > 14.0, "wires take up the free length");
    let tag = part(pipe_node, PartName::Tag);
    let center = pipe_node.bounds.x + pipe_node.bounds.width / 2.0;
    assert_close(
        tag.bounds.x + tag.bounds.width / 2.0,
        center,
        "tag centered in the gutter",
    );
}

#[test]
fn vertical_pipe_in_a_col_is_centered_and_at_least_36_tall() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Col", "children": [card("Upper"), pipe("v", "dash", "failover"), card("Lower")] }]),
    ));
    let column = node(&geometry, "/body/0");
    let pipe_node = node(&geometry, "/body/0/children/1");
    assert_close(
        pipe_node.bounds.x + pipe_node.bounds.width / 2.0,
        column.bounds.x + column.bounds.width / 2.0,
        "pipe center x",
    );
    assert!(pipe_node.bounds.height >= 36.0);
    assert_close(
        pipe_node.bounds.height,
        8.0 + 12.0 + 30.6 + 12.0 + 8.0,
        "max-content run length",
    );
    let tag = part(pipe_node, PartName::Tag);
    assert_close(
        tag.bounds.width,
        pipe_node.bounds.width,
        "the tag sets the pipe width",
    );
}

#[test]
fn vertical_wire_minimum_is_16_for_deny_and_12_otherwise() {
    for (kind, minimum) in [
        ("gray", 12.0),
        ("blue", 12.0),
        ("pink", 12.0),
        ("dash", 12.0),
        ("deny", 16.0),
    ] {
        let geometry = layout(&page_with_body(
            1280,
            json!([{ "tag": "Col", "children": [card("Upper"), pipe("v", kind, "hop"), card("Lower")] }]),
        ));
        let pipe_node = node(&geometry, "/body/0/children/1");
        let (start, end) = wire_lengths(pipe_node, false);
        assert_close(start, minimum, kind);
        assert_close(end, minimum, kind);
        let wire = part(pipe_node, PartName::WireStart);
        assert_eq!(wire.bounds.width, 2.0);
    }
}

#[test]
fn vertical_pipe_in_a_row_stretches_to_the_row_height() {
    let stack = json!({ "tag": "Col", "children": [card("One"), card("Two"), card("Three"), card("Four")] });
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [stack, pipe("v", "blue", "down"), card("Right")] }]),
    ));
    let row = node(&geometry, "/body/0");
    let pipe_node = node(&geometry, "/body/0/children/1");
    assert_close(pipe_node.bounds.y, row.bounds.y, "pipe top");
    assert_close(pipe_node.bounds.height, row.bounds.height, "pipe height");
    let (start, end) = wire_lengths(pipe_node, false);
    assert!((start - end).abs() <= 0.01);
    assert!(start > 12.0);
}

#[test]
fn horizontal_pipe_in_a_row_is_vertically_centered() {
    let stack = json!({ "tag": "Col", "children": [card("One"), card("Two"), card("Three")] });
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [stack, pipe("h", "gray", "call"), card("Right")] }]),
    ));
    let row = node(&geometry, "/body/0");
    let pipe_node = node(&geometry, "/body/0/children/1");
    assert_close(
        pipe_node.bounds.y + pipe_node.bounds.height / 2.0,
        row.bounds.y + row.bounds.height / 2.0,
        "pipe center y",
    );
    assert!(pipe_node.bounds.height < row.bounds.height);
}

#[test]
fn tag_in_a_narrow_gutter_wraps_and_wires_stay_at_14() {
    let label = "alpha beta gamma delta epsilon";
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Row", "grow": [1, 1, 1], "children": [
            card("Left"),
            { "tag": "Col", "children": [pipe("h", "blue", label)] },
            card("Right")
        ]}]),
    ));
    // flex_basis 0 on a border-box item keeps its padding and border, so the two cards are
    // 23 px wider than the gutter column rather than equal thirds.
    let column = node(&geometry, "/body/0/children/1");
    let gutter = (640.0 - 16.0 - 2.0 * 23.0) / 3.0;
    assert_close(column.bounds.width, gutter, "gutter width");
    let max_content_run = 8.0 + 14.0 + (30.0 * 6.0 + 19.0) + 14.0 + 8.0;
    assert!(gutter < max_content_run);
    let pipe_node = node(&geometry, "/body/0/children/1/children/0");
    assert_close(pipe_node.bounds.width, gutter, "pipe spans the gutter");
    let (start, end) = wire_lengths(pipe_node, true);
    assert_close(start, 14.0, "start wire");
    assert_close(end, 14.0, "end wire");
    let tag = part(pipe_node, PartName::Tag);
    assert_close(
        tag.bounds.width,
        gutter - 44.0,
        "tag shrinks into the gutter",
    );
    let label_part = part(pipe_node, PartName::TagLabel);
    let run = label_part.text.as_ref().unwrap();
    assert_eq!(run.metrics.line_count, 2);
    assert_close(label_part.bounds.height, 2.0 * 15.6, "two-line label box");
    assert!(run.metrics.width_px <= label_part.bounds.width + 0.01);
}

#[test]
fn sub_sits_2_px_under_the_label_and_the_tag_grows() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Col", "children": [
            { "tag": "Pipe", "dir": "h", "kind": "deny", "label": "VLAN 1", "sub": "EAD 1 · BGP" }
        ]}]),
    ));
    let pipe_node = node(&geometry, "/body/0/children/0");
    let label = part(pipe_node, PartName::TagLabel);
    let sub = part(pipe_node, PartName::TagSub);
    assert_close(sub.bounds.y - label.bounds.bottom(), 2.0, "sub margin");
    assert_close(
        part(pipe_node, PartName::Tag).bounds.height,
        6.0 + 15.6 + 2.0 + 15.6 + 6.0 + 3.0,
        "tag height",
    );
    assert_eq!(label.text.as_ref().unwrap().color, "#C5221F");
    assert_eq!(sub.text.as_ref().unwrap().color, "#5F6368");
}
