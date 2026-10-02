#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{G7_JSON, layout, page_from, page_with_body};
use serde_json::{Value, json};
use stencil_layout::checks::pipes_land;
use stencil_model::checks::{CheckName, CheckOutcome};

const METRO_COL: &str = "/body/0/children/0";

/// g7 as it stood before its on-prem Col took `grow [1, 1]`: the metro zones keep their
/// content height while the gutter halves split the Row height.
fn g7_with_metro_zones_at_content_height() -> Value {
    let mut document: Value = serde_json::from_str(G7_JSON).unwrap();
    let removed = document
        .pointer_mut(METRO_COL)
        .and_then(Value::as_object_mut)
        .unwrap()
        .remove("grow");
    assert_eq!(removed, Some(json!([1, 1])));
    document
}

#[test]
fn g7_with_metro_zones_at_content_height_fails_for_vlan_3_and_4() {
    let page = page_from(g7_with_metro_zones_at_content_height());
    let report = pipes_land(&page, &layout(&page));

    assert_eq!(report.check, CheckName::PipesLand);
    assert_eq!(report.examined, 8);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
    let pointers: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.pointer.as_str())
        .collect();
    assert_eq!(
        pointers,
        vec![
            "/body/0/children/1/children/1/children/0",
            "/body/0/children/1/children/1/children/1",
        ]
    );
    for defect in &report.defects {
        assert!(
            defect
                .message
                .starts_with(&format!("left neighbor {METRO_COL} has no box across")),
            "{}",
            defect.message
        );
    }
}

#[test]
fn g7_passes_with_every_vlan_pipe_beside_its_metro() {
    let page = common::g7_page();
    let report = pipes_land(&page, &layout(&page));
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.examined, 8);
}

#[test]
fn a_gutter_with_no_neighbor_is_not_applicable() {
    let page = page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [{
                "tag": "Col",
                "children": [
                    { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "VLAN 1" },
                    { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "VLAN 2" }
                ]
            }]
        }]),
    );
    let report = pipes_land(&page, &layout(&page));
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
    assert_eq!(report.not_applicable, Some("no pipe has a neighbor"));
}

#[test]
fn a_page_without_pipes_is_not_applicable() {
    let page = page_with_body(
        1280,
        json!([{ "tag": "Item", "kind": "product", "title": "Cloud Run", "subtitle": "service" }]),
    );
    let report = pipes_land(&page, &layout(&page));
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
    assert_eq!(report.not_applicable, Some("page has no pipes"));
}

#[test]
fn a_pipe_missing_from_the_geometry_fails() {
    let with_pipe = page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [
                { "tag": "Item", "kind": "product", "title": "Client", "subtitle": "browser" },
                { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "request" },
                { "tag": "Item", "kind": "product", "title": "Server", "subtitle": "Cloud Run" }
            ]
        }]),
    );
    let without_pipe = page_with_body(
        1280,
        json!([{ "tag": "Item", "kind": "product", "title": "Cloud Run", "subtitle": "service" }]),
    );
    let report = pipes_land(&with_pipe, &layout(&without_pipe));
    assert_eq!(report.outcome(), CheckOutcome::Failed);
    let [defect] = report.defects.as_slice() else {
        panic!("expected one defect: {report:?}");
    };
    assert_eq!(defect.pointer.as_str(), "/body/0/children/1");
}

/// A Tee in a Row between two cards: each arm is a Pipe h with a card on both sides, and the
/// cards stretch to the Row height, so all four ends land.
#[test]
fn tee_arms_are_examined_against_the_tee_neighbors() {
    let page = page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [
                { "tag": "Item", "kind": "product", "title": "Client", "subtitle": "browser" },
                {
                    "tag": "Tee",
                    "line": "gray",
                    "hub": "split",
                    "arms": [
                        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "read" },
                        { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": "write" }
                    ]
                },
                { "tag": "Item", "kind": "product", "title": "Server", "subtitle": "Cloud Run" }
            ]
        }]),
    );
    let report = pipes_land(&page, &layout(&page));
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.examined, 4);
}

/// A Pipe v between a Row of two cards and one card. Its center x falls in the 8 px gap
/// between the two cards above, and the Row itself does not count, so the upper end misses.
/// The card below spans the full width, so the lower end lands.
#[test]
fn a_vertical_pipe_is_examined_against_the_col_neighbors_above_and_below() {
    let page = page_with_body(
        1280,
        json!([{
            "tag": "Col",
            "children": [
                {
                    "tag": "Row",
                    "children": [
                        { "tag": "Item", "kind": "product", "title": "Left", "subtitle": "service" },
                        { "tag": "Item", "kind": "product", "title": "Right", "subtitle": "service" }
                    ]
                },
                { "tag": "Pipe", "dir": "v", "line": "dash", "label": "failover" },
                { "tag": "Item", "kind": "product", "title": "Below", "subtitle": "service" }
            ]
        }]),
    );
    let report = pipes_land(&page, &layout(&page));
    assert_eq!(report.examined, 2);
    let [defect] = report.defects.as_slice() else {
        panic!("expected one defect: {report:?}");
    };
    assert_eq!(defect.pointer.as_str(), "/body/0/children/1");
    assert!(
        defect
            .message
            .starts_with("above neighbor /body/0/children/0 has no box across the pipe's center x"),
        "{}",
        defect.message
    );
}

/// Section 12.3 rule 5 under iso: an untargeted end reaches the nearest node across the
/// pipe's center in the neighbour's subtree, so a Col holding one item with floor text
/// wider than its footprint still gets the tube on its solid; pipes-land reads the run axis.
#[test]
fn an_iso_pipe_end_reaches_the_solid_inside_a_col_and_a_short_end_is_a_defect() {
    let page = page_from(json!({
        "title": "Title", "kicker": "Kicker", "lede": "Lede", "width": 1100,
        "canvas": "customer", "projection": "iso", "legend": [],
        "body": [{ "tag": "Row", "gap": 32, "children": [
            { "tag": "Box", "kind": "onprem", "label": "Site", "children": [
                { "tag": "Item", "kind": "product", "id": "edge", "title": "Edge" } ] },
            { "tag": "Col", "justify": "center", "children": [ { "tag": "Col", "children": [
                { "tag": "Pipe", "dir": "h", "line": "solid", "label": "VLAN 1", "from": "edge", "to": "fw" } ] } ] },
            { "tag": "Col", "justify": "center", "children": [
                { "tag": "Item", "kind": "product", "id": "fw", "title": "Edge firewall",
                  "subtitle": "stateful, zone policy across both sites" } ] } ] }]
    }));
    let mut geometry = layout(&page);
    let report = pipes_land(&page, &geometry);
    assert_eq!(report.outcome(), CheckOutcome::Passed, "{report:?}");
    assert_eq!(report.examined, 2);
    let firewall = geometry
        .nodes
        .iter()
        .find(|node| node.pointer.as_str() == "/body/0/children/2/children/0")
        .unwrap();
    let footprint = firewall
        .part(stencil_layout::PartName::Footprint)
        .unwrap()
        .bounds;
    let pipe = geometry
        .nodes
        .iter_mut()
        .find(|node| node.pointer.as_str() == "/body/0/children/1/children/0/children/0")
        .unwrap();
    let dot = pipe
        .parts
        .iter_mut()
        .find(|part| part.name == stencil_layout::PartName::DotEnd)
        .unwrap();
    assert!((dot.bounds.x + dot.bounds.width / 2.0 - footprint.x).abs() < 0.01);
    dot.bounds.x -= 10.0;
    let short = pipes_land(&page, &geometry);
    assert_eq!(short.outcome(), CheckOutcome::Failed, "{short:?}");
    assert!(
        short.defects[0]
            .message
            .starts_with("right end stops 10.00 px short of /body/0/children/2/children/0"),
        "{:?}",
        short.defects
    );
}

/// Section 12.3 rule 5: under iso an end that names no target and lands on a zone holding
/// devices reads as a line to nowhere; naming the zone is the author's choice and passes.
#[test]
fn an_iso_pipe_end_that_lands_on_a_zone_of_devices_without_a_target_is_a_defect() {
    let document = |to: Option<&str>| {
        let mut pipe =
            json!({ "tag": "Pipe", "dir": "h", "line": "solid", "label": "VLAN 1", "from": "e" });
        if let Some(to) = to {
            pipe["to"] = json!(to);
        }
        json!({
            "title": "Title", "kicker": "Kicker", "lede": "Lede", "width": 1100,
            "canvas": "customer", "projection": "iso", "legend": [],
            "body": [{ "tag": "Row", "gap": 32, "children": [
                { "tag": "Box", "kind": "onprem", "label": "Site", "children": [
                    { "tag": "Item", "kind": "product", "id": "e", "title": "Edge" } ] },
                { "tag": "Col", "justify": "center", "children": [ { "tag": "Col", "children": [pipe] } ] },
                { "tag": "Box", "kind": "region", "id": "r", "label": "Region", "children": [
                    { "tag": "Item", "kind": "product", "title": "Hub" } ] } ] }]
        })
    };
    let page = page_from(document(None));
    let geometry = layout(&page);
    let report = pipes_land(&page, &geometry);
    assert_eq!(report.examined, 2);
    assert_eq!(report.outcome(), CheckOutcome::Failed, "{report:?}");
    assert_eq!(
        report.defects[0].message,
        "right end lands on the edge of zone /body/0/children/2 and names no target; name the device it reaches"
    );
    let named = page_from(document(Some("r")));
    let geometry = layout(&named);
    let report = pipes_land(&named, &geometry);
    assert_eq!(report.outcome(), CheckOutcome::Passed, "{report:?}");
}
