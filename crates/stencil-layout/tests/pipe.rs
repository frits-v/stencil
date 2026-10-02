#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::{Value, json};
use stencil_layout::{NodeGeometry, PartName};

fn pipe(dir: &str, line: &str, label: &str) -> Value {
    json!({ "tag": "Pipe", "dir": dir, "line": line, "label": label })
}

fn card(function_name: &str) -> Value {
    json!({ "tag": "Item", "kind": "product", "title": function_name })
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
            { "tag": "Col", "children": [card("A wide card with a long function name"), pipe("h", "solid", "VLAN 1")] },
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
        ("solid", 12.0),
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
        json!([{ "tag": "Row", "children": [stack, pipe("v", "solid", "down"), card("Right")] }]),
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
            { "tag": "Col", "children": [pipe("h", "solid", label)] },
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
            { "tag": "Pipe", "dir": "h", "line": "deny", "label": "VLAN 1", "sub": "EAD 1 · BGP" }
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

fn arrow_pipe(dir: &str, arrow: &str) -> Value {
    json!({ "tag": "Pipe", "dir": dir, "line": "solid", "tint": 1, "label": "VLAN 1", "arrow": arrow })
}

#[test]
fn an_arrowhead_end_shortens_its_wire_to_the_arrowhead_base_and_keeps_the_dot() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Col", "children": [
            pipe("h", "solid", "VLAN 1"),
            arrow_pipe("h", "end"),
            arrow_pipe("h", "both")
        ]}]),
    ));
    let plain = node(&geometry, "/body/0/children/0");
    let ending = node(&geometry, "/body/0/children/1");
    let both = node(&geometry, "/body/0/children/2");
    for name in [PartName::DotStart, PartName::DotEnd, PartName::Tag] {
        let plain_box = part(plain, name).bounds;
        for arrowed in [ending, both] {
            let arrowed_box = part(arrowed, name).bounds;
            assert_close(arrowed_box.x, plain_box.x, "dot and tag keep x");
            assert_close(arrowed_box.width, plain_box.width, "dot and tag keep width");
        }
    }
    let plain_end = part(plain, PartName::WireEnd).bounds;
    let end_wire = part(ending, PartName::WireEnd).bounds;
    let dot_end = part(ending, PartName::DotEnd).bounds;
    assert_close(
        end_wire.width,
        plain_end.width - 2.0,
        "end wire shortened by 2",
    );
    assert_close(end_wire.x, plain_end.x, "end wire keeps its tag side");
    assert_close(
        end_wire.right(),
        dot_end.right() - 10.0,
        "wire stops at the base",
    );
    assert_close(
        part(ending, PartName::WireStart).bounds.width,
        part(plain, PartName::WireStart).bounds.width,
        "the start wire of an end arrow is untouched",
    );

    let both_start = part(both, PartName::WireStart).bounds;
    let dot_start = part(both, PartName::DotStart).bounds;
    assert_close(
        both_start.x,
        dot_start.x + 10.0,
        "start wire begins at the base",
    );
    assert_close(
        both_start.right(),
        part(plain, PartName::WireStart).bounds.right(),
        "start wire keeps its tag side",
    );
}

#[test]
fn a_vertical_start_arrowhead_shortens_the_start_wire_from_the_top() {
    let geometry = layout(&page_with_body(
        640,
        json!([{ "tag": "Col", "children": [
            card("Upper"),
            pipe("v", "solid", "failover"),
            card("Lower"),
            arrow_pipe("v", "start")
        ]}]),
    ));
    let plain = node(&geometry, "/body/0/children/1");
    let arrowed = node(&geometry, "/body/0/children/3");
    let wire = part(arrowed, PartName::WireStart).bounds;
    let dot = part(arrowed, PartName::DotStart).bounds;
    assert_close(wire.y, dot.y + 10.0, "start wire begins at the base");
    assert_close(
        wire.height,
        part(plain, PartName::WireStart).bounds.height - 2.0,
        "start wire shortened by 2",
    );
    assert_close(
        part(arrowed, PartName::WireEnd).bounds.height,
        part(plain, PartName::WireEnd).bounds.height,
        "end wire untouched",
    );
}

fn iso_gutter_page(from_to: Option<(&str, &str)>) -> stencil_model::Page {
    let mut pipe = pipe("h", "solid", "VLAN 1");
    if let (Some((from, to)), Some(fields)) = (from_to, pipe.as_object_mut()) {
        fields.insert("from".to_string(), json!(from));
        fields.insert("to".to_string(), json!(to));
    }
    let document = json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "width": 1100,
        "canvas": "customer", "projection": "iso", "legend": [],
        "body": [{ "tag": "Row", "gap": 32, "children": [
            { "tag": "Box", "kind": "onprem", "label": "Site", "children": [
                { "tag": "Item", "kind": "product", "id": "edge", "title": "Edge" } ] },
            { "tag": "Col", "children": [pipe] },
            { "tag": "Box", "kind": "gcp", "label": "Google Cloud", "children": [
                { "tag": "Item", "kind": "product", "id": "hub", "title": "Hub" } ] } ] }]
    });
    common::page_from(document)
}

fn dot_center_x(pipe_node: &NodeGeometry, name: PartName) -> f32 {
    let dot = part(pipe_node, name);
    dot.bounds.x + dot.bounds.width / 2.0
}

/// Section 12.3 rule 5: under iso a pipe end reaches the box it lands on.
#[test]
fn an_iso_pipe_end_reaches_the_neighbour_box_it_lands_on() {
    let geometry = layout(&iso_gutter_page(None));
    let pipe_node = node(&geometry, "/body/0/children/1/children/0");
    let site = node(&geometry, "/body/0/children/0");
    let cloud = node(&geometry, "/body/0/children/2");
    assert_close(
        dot_center_x(pipe_node, PartName::DotStart),
        site.bounds.right(),
        "start on the site edge",
    );
    assert_close(
        dot_center_x(pipe_node, PartName::DotEnd),
        cloud.bounds.x,
        "end on the cloud edge",
    );
    assert!(
        site.bounds.right() < pipe_node.bounds.x,
        "the site lies a gap left of the pipe"
    );
}

/// Section 12.3 rule 5: a pipe that names its targets reaches their solids.
#[test]
fn an_iso_pipe_end_reaches_a_named_target() {
    let geometry = layout(&iso_gutter_page(Some(("edge", "hub"))));
    let pipe_node = node(&geometry, "/body/0/children/1/children/0");
    let edge = node(&geometry, "/body/0/children/0/children/0");
    let hub = node(&geometry, "/body/0/children/2/children/0");
    assert_close(
        dot_center_x(pipe_node, PartName::DotStart),
        edge.bounds.right(),
        "start on the edge router",
    );
    assert_close(
        dot_center_x(pipe_node, PartName::DotEnd),
        hub.bounds.x,
        "end on the hub",
    );
}

/// Flat keeps its dots inside the pipe's own box.
#[test]
fn a_flat_pipe_end_stays_in_its_box() {
    let mut page = iso_gutter_page(None);
    page.projection = stencil_model::Projection::Flat;
    let geometry = layout(&page);
    let pipe_node = node(&geometry, "/body/0/children/1/children/0");
    assert!(dot_center_x(pipe_node, PartName::DotStart) >= pipe_node.bounds.x);
    assert!(dot_center_x(pipe_node, PartName::DotEnd) <= pipe_node.bounds.right());
}

/// Section 12.3 rule 6: under iso a Tee's spine stands on its left neighbour's edge and its
/// arms start there.
#[test]
fn an_iso_tee_spine_stands_on_the_left_neighbour_edge() {
    let geometry = layout(&common::page_from(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "width": 1100,
        "canvas": "customer", "projection": "iso", "legend": [],
        "body": [{ "tag": "Row", "gap": 32, "children": [
            { "tag": "Box", "kind": "onprem", "label": "Site", "children": [card("Edge")] },
            { "tag": "Col", "children": [
                { "tag": "Tee", "line": "solid", "hub": "Interconnect", "arms": [
                    pipe("h", "solid", "VLAN 1"), pipe("h", "solid", "VLAN 2") ] } ] },
            { "tag": "Box", "kind": "gcp", "label": "Google Cloud", "children": [card("Hub")] } ] }]
    })));
    let site = node(&geometry, "/body/0/children/0");
    let tee = node(&geometry, "/body/0/children/1/children/0");
    let spine = part(tee, PartName::Spine);
    assert_close(
        spine.bounds.x + spine.bounds.width / 2.0,
        site.bounds.right(),
        "spine on the site edge",
    );
    for arm in ["arms/0", "arms/1"] {
        let arm_node = node(&geometry, &format!("/body/0/children/1/children/0/{arm}"));
        assert_close(
            dot_center_x(arm_node, PartName::DotStart),
            site.bounds.right(),
            "arm starts on the spine",
        );
    }
}
