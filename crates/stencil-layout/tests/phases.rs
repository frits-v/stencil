//! A Phases node (section 14.4 rules 11 to 15): its columns, lanes, Steps, label obstacles
//! and the placement relations of child-inside-container.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{node, page_from, part};
use serde_json::{Value, json};
use stencil_layout::checks::child_inside_container_on;
use stencil_layout::{BoxRect, PageGeometry, PartName};
use stencil_model::Page;

const PHASES: &str = "/body/0/children/0";

fn step(lane: &str, phase: &str, col: u8, title: &str) -> Value {
    json!({ "tag": "Step", "lane": lane, "phase": phase, "col": col, "title": title })
}

/// A sheet whose one View holds a Phases node of two lanes and two phases over `steps`,
/// with a solid by-actor line for its legend. A sheet draws a line, so without `links` the
/// first two Steps are joined.
fn phases_page(mut steps: Value, mut links: Value) -> Page {
    if links.as_array().unwrap().is_empty() {
        for (index, step) in steps.as_array_mut().unwrap().iter_mut().enumerate() {
            if step.get("id").is_none() {
                step["id"] = json!(format!("step-{index}"));
            }
        }
        let ids: Vec<Value> = steps.as_array().unwrap()[..2]
            .iter()
            .map(|step| step["id"].clone())
            .collect();
        links = json!([{ "from": ids[0], "to": ids[1], "line": "solid" }]);
    }
    page_from(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "badge": "Internal",
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "actors": [
            { "id": "crew", "name": "Crew", "text": "writes the plan", "tint": 4 },
            { "id": "ops", "name": "Ops", "text": "runs the jobs", "tint": 1 }
        ],
        "body": [{
            "tag": "View",
            "title": "Workflow",
            "children": [{
                "tag": "Phases",
                "lanes": ["crew", "ops"],
                "phases": [{ "id": "plan", "title": "Plan" }, { "id": "run", "title": "Run" }],
                "children": steps
            }]
        }],
        "legend": [{ "line": "solid", "by_actor": true, "text": "work moves" }],
        "links": links
    }))
}

fn two_step_page() -> Page {
    phases_page(
        json!([
            { "tag": "Step", "id": "draft", "lane": "crew", "phase": "plan", "title": "Draft" },
            { "tag": "Step", "id": "run", "lane": "ops", "phase": "run", "col": 2, "title": "Run it" }
        ]),
        json!([{ "from": "draft", "to": "run", "line": "solid" }]),
    )
}

fn parts(geometry: &PageGeometry, pointer: &str, name: PartName) -> Vec<BoxRect> {
    node(geometry, pointer)
        .parts
        .iter()
        .filter(|part| part.name == name)
        .map(|part| part.bounds)
        .collect()
}

/// Rule 11: a Phases node is as wide as the sheet's content, its lane bands stack under the
/// row of phase heads with no gap, and each band spans the node.
#[test]
fn the_lanes_stack_under_the_phase_heads_across_the_node() {
    let page = two_step_page();
    let geometry = common::layout(&page);
    let phases = node(&geometry, PHASES);
    assert_eq!(phases.bounds.width, page.width as f32);
    let bands = parts(&geometry, PHASES, PartName::LaneBand);
    assert_eq!(bands.len(), 2);
    common::assert_close(bands[0].y, phases.bounds.y + 30.0, "first lane top");
    common::assert_close(bands[1].y, bands[0].bottom(), "second lane top");
    common::assert_close(
        bands[1].bottom(),
        phases.bounds.bottom(),
        "last lane bottom",
    );
    for band in &bands {
        assert_eq!(band.x, phases.bounds.x);
        assert_eq!(band.width, phases.bounds.width);
    }
    let heads = parts(&geometry, PHASES, PartName::PhaseHead);
    assert_eq!(heads.len(), 2);
    assert!(heads.iter().all(|head| head.bottom() <= bands[0].y));
}

/// Rule 12: Steps in one column of one phase share their x and width across lanes, and a
/// Step spanning two columns covers both and the gap between them.
#[test]
fn steps_of_one_column_share_their_extent_across_lanes() {
    let page = phases_page(
        json!([
            step("crew", "run", 1, "Plan the run"),
            step("ops", "run", 1, "Start the run"),
            step("crew", "run", 2, "Approve"),
            { "tag": "Step", "lane": "ops", "phase": "run", "col": 1, "span": 2, "title": "Run across both" }
        ]),
        json!([]),
    );
    let geometry = common::layout(&page);
    let children = format!("{PHASES}/children");
    let first = node(&geometry, &format!("{children}/0")).bounds;
    let below = node(&geometry, &format!("{children}/1")).bounds;
    let second = node(&geometry, &format!("{children}/2")).bounds;
    let spanning = node(&geometry, &format!("{children}/3")).bounds;
    assert_eq!((first.x, first.width), (below.x, below.width));
    assert!(second.x >= first.right() + 20.0);
    assert_eq!(spanning.x, first.x);
    common::assert_close(spanning.right(), second.right(), "span end");
}

/// Rule 13: Steps of one lane whose columns overlap stack in document order, 20 px apart.
#[test]
fn overlapping_steps_of_one_lane_stack_in_document_order() {
    let page = phases_page(
        json!([
            step("crew", "plan", 1, "First"),
            step("crew", "plan", 1, "Second"),
            step("ops", "run", 1, "Elsewhere")
        ]),
        json!([]),
    );
    let geometry = common::layout(&page);
    let first = node(&geometry, &format!("{PHASES}/children/0")).bounds;
    let second = node(&geometry, &format!("{PHASES}/children/1")).bounds;
    common::assert_close(second.y, first.bottom() + 20.0, "stacked Step");
    let band = parts(&geometry, PHASES, PartName::LaneBand)[0];
    assert!(second.bottom() <= band.bottom());
}

/// Rule 14: a stack look draws two boxes behind its front inside its bounds, and a text
/// look draws no box.
#[test]
fn a_stack_draws_its_back_boxes_inside_its_bounds_and_a_text_look_no_box() {
    let page = phases_page(
        json!([
            { "tag": "Step", "lane": "crew", "phase": "plan", "look": "stack", "title": "Many" },
            { "tag": "Step", "lane": "ops", "phase": "plan", "look": "text", "title": "nothing yet" }
        ]),
        json!([]),
    );
    let geometry = common::layout(&page);
    let stack = node(&geometry, &format!("{PHASES}/children/0"));
    let backs: Vec<&BoxRect> = stack
        .parts
        .iter()
        .filter(|part| part.name == PartName::StackBack)
        .map(|part| &part.bounds)
        .collect();
    assert_eq!(backs.len(), 2);
    let front = part(stack, PartName::Front).bounds;
    for back in backs.iter().copied().chain([&front]) {
        assert!(back.x >= stack.bounds.x && back.right() <= stack.bounds.right() + 0.01);
        assert!(back.y >= stack.bounds.y && back.bottom() <= stack.bounds.bottom() + 0.01);
    }
    assert!(backs[0].x > front.x && backs[0].y < front.y);
    let text = node(&geometry, &format!("{PHASES}/children/1"));
    assert!(text.part(PartName::Front).is_none());
}

/// Section 14.6: child-inside-container holds each Step inside its lane's band and its
/// phase, and reports a Step moved out of either.
#[test]
fn a_step_outside_its_lane_or_phase_is_a_defect() {
    let page = two_step_page();
    let geometry = common::layout(&page);
    let report = child_inside_container_on(&page, &geometry);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
    let plain = stencil_layout::checks::child_inside_container(&geometry);
    assert_eq!(report.examined, plain.examined + 4);

    let mut moved = geometry.clone();
    let index = moved
        .nodes
        .iter()
        .position(|candidate| candidate.pointer.as_str() == format!("{PHASES}/children/0"))
        .unwrap();
    let band = parts(&geometry, PHASES, PartName::LaneBand)[1];
    let target = &mut moved.nodes[index].bounds;
    target.y = band.y + 4.0;
    let report = child_inside_container_on(&page, &moved);
    assert!(
        report
            .defects
            .iter()
            .any(|defect| defect.message.ends_with("leaves lane crew")),
        "{:?}",
        report.defects
    );

    let mut shifted = geometry.clone();
    shifted.nodes[index].bounds.x += 400.0;
    let report = child_inside_container_on(&page, &shifted);
    assert!(
        report
            .defects
            .iter()
            .any(|defect| defect.message.ends_with("leaves phase Plan")),
        "{:?}",
        report.defects
    );
}

/// Rule 15: a lane's name and text are obstacles of every link, so a link between two
/// Steps that leaves and enters through their left sides routes around the lane label
/// column, and links-avoid-boxes finds nothing.
#[test]
fn a_link_between_two_steps_routes_around_a_lane_name() {
    let page = phases_page(
        json!([
            { "tag": "Step", "id": "draft", "lane": "crew", "phase": "plan", "title": "Draft" },
            { "tag": "Step", "id": "check", "lane": "ops", "phase": "plan", "title": "Check" }
        ]),
        json!([{ "from": "draft", "to": "check", "line": "solid", "from_side": "left", "to_side": "left" }]),
    );
    let geometry = common::layout(&page);
    let labels: Vec<BoxRect> = [PartName::LaneName, PartName::LaneText]
        .into_iter()
        .flat_map(|name| parts(&geometry, PHASES, name))
        .collect();
    assert_eq!(labels.len(), 4);
    let route = &geometry.links[0];
    for leg in route.points.windows(2) {
        let (a, b) = (leg[0], leg[1]);
        for label in &labels {
            let enters = a.x.max(b.x) > label.x
                && a.x.min(b.x) < label.right()
                && a.y.max(b.y) > label.y
                && a.y.min(b.y) < label.bottom();
            assert!(!enters, "leg {a:?} to {b:?} crosses a lane label {label:?}");
        }
    }
    let report = stencil_layout::checks::links_avoid_boxes(&geometry);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
}

/// On a sheet a link's tag never straddles a Box border (section 14.4 rule 15): onepager's
/// console link runs into the Google Cloud box, and its tag lies wholly inside it.
#[test]
fn no_tag_on_onepager_straddles_a_box_edge() {
    let page: Page = serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    let geometry = stencil_layout::layout_page(
        &page,
        &common::gcp(),
        &mut stencil_model::text::FixedMetricsMeasurer::default(),
    )
    .unwrap();
    let zones: Vec<BoxRect> = geometry
        .nodes
        .iter()
        .filter(|candidate| candidate.tag == stencil_layout::NodeTag::Zone)
        .map(|candidate| candidate.bounds)
        .collect();
    assert!(!zones.is_empty());
    let mut examined = 0;
    for route in &geometry.links {
        let Some(tag) = route.tag else {
            continue;
        };
        for zone in &zones {
            let overlaps = tag.x < zone.right()
                && zone.x < tag.right()
                && tag.y < zone.bottom()
                && zone.y < tag.bottom();
            let inside = tag.x >= zone.x
                && tag.right() <= zone.right()
                && tag.y >= zone.y
                && tag.bottom() <= zone.bottom();
            assert!(
                !overlaps || inside,
                "/links/{} tag {tag:?} straddles {zone:?}",
                route.index
            );
        }
        examined += 1;
    }
    assert!(examined >= 8, "examined {examined}");
}
