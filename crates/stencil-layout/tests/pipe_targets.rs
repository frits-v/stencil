#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Pipes aimed at named nodes (section 13.8): the slot translation and the pipes-land
//! target ends.

mod common;

use common::{assert_close, layout, node, page_from};
use serde_json::{Value, json};
use stencil_layout::PageGeometry;
use stencil_layout::checks::pipes_land;

const PRODUCERS: &str = "/body/0/children/0";
const TALL: &str = "/body/0/children/0/children/0";
const SHORT: &str = "/body/0/children/0/children/1";
const GUTTER: &str = "/body/0/children/1";
const FIRST_SLOT: &str = "/body/0/children/1/children/0";
const SECOND_SLOT: &str = "/body/0/children/1/children/1";
const FIRST_PIPE: &str = "/body/0/children/1/children/0/children/0";
const SECOND_PIPE: &str = "/body/0/children/1/children/1/children/0";
const TARGET: &str = "/body/0/children/2";

fn pipe(label: &str, targets: Value) -> Value {
    let mut pipe = json!({ "tag": "Pipe", "dir": "h", "line": "solid", "label": label });
    for (key, value) in targets.as_object().unwrap() {
        pipe[key] = value.clone();
    }
    pipe
}

/// Row [Col of a tall and a short Box, gutter Col of two slot Cols, target Box]. Neither
/// Col has a grow list, so without targets both slots sit at the top of the gutter.
fn gutter_page(first_targets: Value, second_targets: Value) -> Value {
    let facts: Vec<Value> = (0..6)
        .map(|index| json!({ "tag": "Fact", "text": format!("fact {index}") }))
        .collect();
    json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "canvas": "customer",
        "body": [{ "tag": "Row", "grow": [1, 0, 1], "children": [
            { "tag": "Col", "gap": 24, "children": [
                { "tag": "Box", "id": "tall", "kind": "onprem", "label": "Tall site", "children": facts },
                { "tag": "Box", "id": "short", "kind": "onprem", "label": "Short site", "children": [
                    { "tag": "Fact", "text": "one fact" }
                ] }
            ] },
            { "tag": "Col", "children": [
                { "tag": "Col", "children": [pipe("first", first_targets)] },
                { "tag": "Col", "children": [pipe("second", second_targets)] }
            ] },
            { "tag": "Box", "id": "cloud", "kind": "project", "label": "Cloud", "children": [
                { "tag": "Fact", "text": "service" }
            ] }
        ] }],
        "legend": [{ "line": "solid", "text": "path" }]
    })
}

fn center_y(geometry: &PageGeometry, pointer: &str) -> f32 {
    let bounds = node(geometry, pointer).bounds;
    bounds.y + bounds.height / 2.0
}

fn defects(geometry: &PageGeometry, page: &stencil_model::Page) -> Vec<(String, String)> {
    pipes_land(page, geometry)
        .defects
        .into_iter()
        .map(|defect| (defect.pointer.to_string(), defect.message))
        .collect()
}

#[test]
fn each_slot_centers_on_its_target_and_its_target_ends_land() {
    let page = page_from(gutter_page(
        json!({ "from": "tall", "to": "cloud" }),
        json!({ "from": "short", "to": "cloud" }),
    ));
    let geometry = layout(&page);
    assert!(
        (center_y(&geometry, TALL) - center_y(&geometry, SHORT)).abs() > 50.0,
        "the two Boxes differ in height and place"
    );
    assert_close(
        center_y(&geometry, FIRST_SLOT),
        center_y(&geometry, TALL),
        "first slot on the tall Box",
    );
    assert_close(
        center_y(&geometry, SECOND_SLOT),
        center_y(&geometry, SHORT),
        "second slot on the short Box",
    );
    let report = pipes_land(&page, &geometry);
    assert_eq!(report.examined, 4);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
}

#[test]
fn nothing_outside_the_slots_moves() {
    let untargeted = layout(&page_from(gutter_page(json!({}), json!({}))));
    let targeted = layout(&page_from(gutter_page(
        json!({ "from": "tall" }),
        json!({ "from": "short" }),
    )));
    assert_eq!(untargeted.nodes.len(), targeted.nodes.len());
    let mut unmoved = 0;
    for (before, after) in untargeted.nodes.iter().zip(&targeted.nodes) {
        assert_eq!(before.pointer, after.pointer);
        let in_a_slot = after.pointer.as_str().starts_with(FIRST_SLOT)
            || after.pointer.as_str().starts_with(SECOND_SLOT);
        if in_a_slot {
            assert_eq!(before.bounds.width, after.bounds.width, "{}", after.pointer);
            assert_eq!(
                before.bounds.height, after.bounds.height,
                "{}",
                after.pointer
            );
            assert_eq!(before.bounds.x, after.bounds.x, "{}", after.pointer);
            for (part_before, part_after) in before.parts.iter().zip(&after.parts) {
                assert_close(
                    part_after.bounds.y - part_before.bounds.y,
                    after.bounds.y - before.bounds.y,
                    "a part moves with its node",
                );
            }
        } else {
            assert_eq!(before.bounds, after.bounds, "{}", after.pointer);
            assert_eq!(before.parts, after.parts, "{}", after.pointer);
            unmoved += 1;
        }
    }
    assert!(unmoved > 10, "{unmoved}");
    assert!(node(&targeted, SECOND_SLOT).bounds.y > node(&untargeted, SECOND_SLOT).bounds.y);
}

#[test]
fn swapped_targets_are_reported_on_both_pipes() {
    let page = page_from(gutter_page(
        json!({ "from": "cloud", "to": "tall" }),
        json!({ "from": "cloud", "to": "short" }),
    ));
    let geometry = layout(&page);
    let report = pipes_land(&page, &geometry);
    assert_eq!(report.examined, 4);
    assert_eq!(
        defects(&geometry, &page),
        [
            (
                FIRST_PIPE.to_string(),
                format!("from target {TARGET} lies on the right of the pipe")
            ),
            (
                FIRST_PIPE.to_string(),
                format!("to target {TALL} lies on the left of the pipe")
            ),
            (
                SECOND_PIPE.to_string(),
                format!("from target {TARGET} lies on the right of the pipe")
            ),
            (
                SECOND_PIPE.to_string(),
                format!("to target {SHORT} lies on the left of the pipe")
            ),
        ]
    );
}

#[test]
fn a_pipe_with_only_to_keeps_the_neighbor_rule_on_its_left() {
    let page = page_from(gutter_page(
        json!({ "to": "cloud" }),
        json!({ "from": "short" }),
    ));
    let geometry = layout(&page);
    let report = pipes_land(&page, &geometry);
    // The first pipe: its left neighbor, the producer Col, then its to target. The second
    // pipe: its from target, then its right neighbor, the target Box.
    assert_eq!(report.examined, 4);
    assert!(report.defects.is_empty(), "{:?}", report.defects);

    let page = page_from(gutter_page(
        json!({ "to": "short" }),
        json!({ "from": "short" }),
    ));
    let geometry = layout(&page);
    let found = defects(&geometry, &page);
    assert_eq!(
        found,
        [(
            FIRST_PIPE.to_string(),
            format!("to target {SHORT} lies on the left of the pipe")
        )],
        "the left end still lands on the producer Col by the neighbor rule"
    );
    assert!(node(&geometry, PRODUCERS).bounds.x < node(&geometry, GUTTER).bounds.x);
}

#[test]
fn a_slot_aimed_past_its_parent_is_clamped_and_reported() {
    let page = page_from(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "canvas": "customer",
        "body": [
            { "tag": "Row", "grow": [1, 0, 1], "children": [
                { "tag": "Box", "id": "near", "kind": "onprem", "label": "Near", "children": [
                    { "tag": "Fact", "text": "router" }
                ] },
                { "tag": "Col", "children": [
                    { "tag": "Col", "children": [
                        { "tag": "Pipe", "dir": "h", "line": "solid", "label": "hop", "from": "near", "to": "far" }
                    ] }
                ] },
                { "tag": "Box", "id": "cloud", "kind": "project", "label": "Cloud", "children": [
                    { "tag": "Fact", "text": "service" }
                ] }
            ] },
            { "tag": "Box", "id": "far", "kind": "project", "label": "Far below", "children": [
                { "tag": "Fact", "text": "elsewhere" }
            ] }
        ],
        "legend": [{ "line": "solid", "text": "path" }]
    }));
    let geometry = layout(&page);
    let gutter = node(&geometry, "/body/0/children/1");
    let slot = node(&geometry, "/body/0/children/1/children/0");
    assert_close(
        slot.bounds.bottom(),
        gutter.content.bottom(),
        "slot clamped to the bottom of the gutter",
    );
    let found = defects(&geometry, &page);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .1
            .starts_with("to target /body/1 has no box across the pipe's center y"),
        "{found:?}"
    );
}

#[test]
fn a_v_pipe_reports_targets_on_the_wrong_side_as_above_and_below() {
    let page = page_from(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "canvas": "customer",
        "body": [{ "tag": "Col", "children": [
            { "tag": "Box", "id": "upper", "kind": "onprem", "label": "Upper", "children": [
                { "tag": "Fact", "text": "router" }
            ] },
            { "tag": "Row", "children": [
                { "tag": "Pipe", "dir": "v", "line": "solid", "label": "hop", "from": "lower", "to": "upper" }
            ] },
            { "tag": "Box", "id": "lower", "kind": "project", "label": "Lower", "children": [
                { "tag": "Fact", "text": "service" }
            ] }
        ] }],
        "legend": [{ "line": "solid", "text": "path" }]
    }));
    let geometry = layout(&page);
    let pipe = "/body/0/children/1/children/0";
    assert_eq!(
        defects(&geometry, &page),
        [
            (
                pipe.to_string(),
                "from target /body/0/children/2 lies below the pipe".to_string()
            ),
            (
                pipe.to_string(),
                "to target /body/0/children/0 lies above the pipe".to_string()
            ),
        ]
    );
}
