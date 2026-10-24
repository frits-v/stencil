//! The detail ladder and its frame (section 14.4 rules 16 and 20) and what `marks-clear`
//! holds a detail frame and tag to (section 14.6).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{assert_close, layout, node, page_from, part};
use serde_json::{Value, json};
use stencil_layout::checks::{marks_clear, text_fits_box};
use stencil_layout::{
    BoxRect, DETAIL_FRAME_GROW_PX, DETAIL_TAG_RADIUS_PX, NodeTag, PageGeometry, PartName,
};
use stencil_model::Page;

const PHASES: &str = "/body/0/children/0";
const LADDER: &str = "/body/0/children/1";

/// Two lanes of one phase: `a`, `b` and `c` in columns 1 to 3 of the crew's lane and `d`
/// under `b` in the ops lane; `a` links to `b` and to `d`, and a ladder of three rungs
/// frames `around`.
fn ladder_document(around: &[&str]) -> Value {
    let step = |id: &str, lane: &str, col: u8| {
        json!({ "tag": "Step", "id": id, "lane": lane, "phase": "run", "col": col,
                "title": format!("Step {id}") })
    };
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "actors": [
            { "id": "crew", "name": "Crew", "tint": 2 },
            { "id": "ops", "name": "Ops", "tint": 4 }
        ],
        "grammar": "gcp",
        "body": [{
            "tag": "View",
            "title": "Workflow",
            "children": [
                {
                    "tag": "Phases",
                    "lanes": ["crew", "ops"],
                    "phases": [{ "id": "run", "title": "Run" }],
                    "children": [
                        step("a", "crew", 1),
                        step("b", "crew", 2),
                        step("c", "crew", 3),
                        step("d", "ops", 2)
                    ]
                },
                {
                    "tag": "Ladder",
                    "title": "When work fails",
                    "note": "nothing waits",
                    "around": around,
                    "rungs": [
                        { "title": "Retry", "who": "the crew", "actor": "crew",
                          "text": "The crew runs the work again." },
                        { "title": "Escalate", "who": "ops", "actor": "ops",
                          "text": "Ops takes the work over.",
                          "marks": [{ "attention": "first" }] },
                        { "title": "Page", "planned": true, "text": "Someone is paged." }
                    ]
                }
            ]
        }],
        "legend": [{ "line": "solid", "by_actor": true, "text": "work moves" }],
        "links": [
            { "from": "a", "to": "b", "line": "solid" },
            { "from": "a", "to": "d", "line": "solid" }
        ]
    })
}

fn laid_out(around: &[&str]) -> (Page, PageGeometry) {
    let page = page_from(ladder_document(around));
    let geometry = layout(&page);
    (page, geometry)
}

fn union(boxes: &[BoxRect]) -> BoxRect {
    let x = boxes
        .iter()
        .map(|bounds| bounds.x)
        .fold(f32::INFINITY, f32::min);
    let y = boxes
        .iter()
        .map(|bounds| bounds.y)
        .fold(f32::INFINITY, f32::min);
    let right = boxes
        .iter()
        .map(BoxRect::right)
        .fold(f32::NEG_INFINITY, f32::max);
    let bottom = boxes
        .iter()
        .map(BoxRect::bottom)
        .fold(f32::NEG_INFINITY, f32::max);
    BoxRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    }
}

fn step_box(geometry: &PageGeometry, index: usize) -> BoxRect {
    node(geometry, &format!("{PHASES}/children/{index}")).bounds
}

/// Sets the first part named `name` on the Phases node.
fn set_phases_part(geometry: &mut PageGeometry, name: PartName, bounds: BoxRect) {
    let phases = geometry
        .nodes
        .iter_mut()
        .find(|node| node.tag == NodeTag::Phases)
        .unwrap();
    phases
        .parts
        .iter_mut()
        .find(|part| part.name == name)
        .unwrap()
        .bounds = bounds;
}

fn frame_messages(page: &Page, geometry: &PageGeometry) -> Vec<String> {
    marks_clear(page, geometry)
        .defects
        .into_iter()
        .map(|defect| defect.message)
        .filter(|message| message.starts_with("detail"))
        .collect()
}

#[test]
fn a_ladder_draws_rung_1_at_the_bottom_beside_rails_that_span_every_rung() {
    let (page, geometry) = laid_out(&["b", "d"]);
    let ladder = node(&geometry, LADDER);
    let rungs: Vec<BoxRect> = (0..3)
        .map(|index| node(&geometry, &format!("{LADDER}/rungs/{index}")).bounds)
        .collect();
    assert!(
        rungs[0].y > rungs[1].y && rungs[1].y > rungs[2].y,
        "{rungs:?}"
    );
    let rail = part(ladder, PartName::RailLeft).bounds;
    assert_close(rail.y, rungs[2].y, "rail top");
    assert_close(rail.bottom(), rungs[0].bottom(), "rail bottom");
    let ticks: Vec<BoxRect> = ladder
        .parts
        .iter()
        .filter(|part| part.name == PartName::RungTick)
        .map(|part| part.bounds)
        .collect();
    assert_eq!(ticks.len(), 3);
    for (tick, rung) in ticks.iter().zip(&rungs) {
        assert_close(
            tick.y + tick.height / 2.0,
            rung.y + rung.height / 2.0,
            "tick at the rung's middle",
        );
    }
    let titles: Vec<(String, f32)> = (0..3)
        .map(|index| {
            let rung = node(&geometry, &format!("{LADDER}/rungs/{index}"));
            let title = part(rung, PartName::RungTitle);
            let text = part(rung, PartName::RungText);
            (title.text.as_ref().unwrap().text.clone(), text.bounds.x)
        })
        .collect();
    assert_eq!(titles[0].0, "1 \u{b7} RETRY");
    assert_eq!(titles[2].0, "3 \u{b7} PAGE");
    assert!(
        titles.iter().all(|(_, x)| (x - titles[0].1).abs() < 0.01),
        "every rung's text starts at one x: {titles:?}"
    );
    let letter = part(ladder, PartName::DetailLetter);
    assert_eq!(letter.text.as_ref().unwrap().text, "A");
    assert_eq!(text_fits_box(&geometry).defects, Vec::new());
    assert_eq!(marks_clear(&page, &geometry).defects, Vec::new());
}

#[test]
fn a_detail_frame_stands_its_grow_outside_its_steps_with_its_tag_on_its_corner() {
    let (page, geometry) = laid_out(&["b", "d"]);
    let phases = node(&geometry, PHASES);
    let frame = part(phases, PartName::DetailFrame).bounds;
    let steps = union(&[step_box(&geometry, 1), step_box(&geometry, 3)]);
    assert_close(frame.x, steps.x - DETAIL_FRAME_GROW_PX, "frame x");
    assert_close(frame.y, steps.y - DETAIL_FRAME_GROW_PX, "frame y");
    assert_close(
        frame.bottom(),
        steps.bottom() + DETAIL_FRAME_GROW_PX,
        "frame bottom",
    );
    let tag = part(phases, PartName::DetailTag).bounds;
    assert_close(tag.width, 2.0 * DETAIL_TAG_RADIUS_PX, "tag size");
    assert!(tag.bottom() <= phases.bounds.bottom() + 0.01, "{tag:?}");
    assert_close(
        tag.x + tag.width / 2.0,
        frame.x,
        "tag on the frame's left edge",
    );
    let report = marks_clear(&page, &geometry);
    assert_eq!(report.defects, Vec::new());
}

/// Section 14.6 rule 3: a frame around columns 1 and 3 of one lane swallows the Step in
/// column 2.
#[test]
fn a_detail_frame_that_holds_a_step_it_does_not_frame_is_a_defect() {
    let (page, geometry) = laid_out(&["a", "c"]);
    let messages = frame_messages(&page, &geometry);
    assert!(
        messages.iter().any(|message| message.contains(&format!(
            "crosses {PHASES}/children/1, a Step it does not frame"
        ))),
        "{messages:?}"
    );
}

/// A frame grown 5 past its Steps, the first draft's size, crosses the arrowhead of the
/// line that enters a framed Step.
#[test]
fn a_detail_frame_through_an_arrowhead_is_a_defect() {
    let (page, mut geometry) = laid_out(&["b", "d"]);
    let steps = union(&[step_box(&geometry, 1), step_box(&geometry, 3)]);
    let tight = BoxRect {
        x: steps.x - 5.0,
        y: steps.y - 5.0,
        width: steps.width + 10.0,
        height: steps.height + 10.0,
    };
    set_phases_part(&mut geometry, PartName::DetailFrame, tight);
    let messages = frame_messages(&page, &geometry);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("passes through the arrowhead")),
        "{messages:?}"
    );
}

/// A frame edge 2 px inside a lane's edge runs along its band edge and rule, which the
/// lane padding of the first draft gave.
#[test]
fn a_detail_frame_along_a_lane_edge_is_a_defect() {
    let (page, mut geometry) = laid_out(&["b", "d"]);
    let phases = node(&geometry, PHASES);
    let band = phases
        .parts
        .iter()
        .find(|part| part.name == PartName::LaneBand)
        .unwrap()
        .bounds;
    let mut frame = part(phases, PartName::DetailFrame).bounds;
    let bottom = frame.bottom();
    frame.y = band.y + 2.0;
    frame.height = bottom - frame.y;
    set_phases_part(&mut geometry, PartName::DetailFrame, frame);
    let messages = frame_messages(&page, &geometry);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("runs 2.00 px from lane_band")),
        "{messages:?}"
    );
}

#[test]
fn a_detail_frame_at_a_bend_or_along_a_leg_is_a_defect() {
    let (page, geometry) = laid_out(&["b", "d"]);
    let route = &geometry.links[1];
    assert!(route.points.len() > 2, "a to d bends: {:?}", route.points);
    let bend = route.points[1];
    let frame = part(node(&geometry, PHASES), PartName::DetailFrame).bounds;

    let mut at_bend = geometry.clone();
    let mut moved = frame;
    moved.width = frame.right() - bend.x;
    moved.x = bend.x;
    set_phases_part(&mut at_bend, PartName::DetailFrame, moved);
    let messages = frame_messages(&page, &at_bend);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("from a bend of /links/1")),
        "{messages:?}"
    );

    let leg = route
        .points
        .windows(2)
        .find(|leg| (leg[0].x - leg[1].x).abs() < 0.01)
        .unwrap();
    let mut along = geometry.clone();
    let mut moved = frame;
    moved.x = leg[0].x + 3.0;
    moved.width = frame.right() - moved.x;
    set_phases_part(&mut along, PartName::DetailFrame, moved);
    let messages = frame_messages(&page, &along);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("runs 3.00 px from /links/1")),
        "{messages:?}"
    );
}

#[test]
fn a_detail_tag_over_a_step_or_a_line_is_a_defect() {
    let (page, geometry) = laid_out(&["b", "d"]);
    let radius = DETAIL_TAG_RADIUS_PX;
    let centred = |x: f32, y: f32| BoxRect {
        x: x - radius,
        y: y - radius,
        width: 2.0 * radius,
        height: 2.0 * radius,
    };
    let mut on_step = geometry.clone();
    let step = step_box(&geometry, 2);
    set_phases_part(
        &mut on_step,
        PartName::DetailTag,
        centred(step.x + step.width / 2.0, step.y + step.height / 2.0),
    );
    let messages = frame_messages(&page, &on_step);
    assert!(
        messages
            .iter()
            .any(|message| message.contains(&format!("lies over {PHASES}/children/2"))),
        "{messages:?}"
    );

    let route = &geometry.links[0];
    let (start, end) = (route.points[0], route.points[route.points.len() - 1]);
    let mut on_line = geometry.clone();
    set_phases_part(
        &mut on_line,
        PartName::DetailTag,
        centred((start.x + end.x) / 2.0, (start.y + end.y) / 2.0),
    );
    let messages = frame_messages(&page, &on_line);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("lies over /links/0")),
        "{messages:?}"
    );
}

/// Section 14.4 rule 2: each Ladder is followed by its rungs in geometry order, rung 1
/// first, each a child of the Ladder.
#[test]
fn a_ladders_rungs_follow_it_in_geometry_order() {
    let (_, geometry) = laid_out(&["b", "d"]);
    let ladder = geometry
        .nodes
        .iter()
        .position(|node| node.pointer.as_str() == LADDER)
        .unwrap();
    for index in 0..3 {
        let rung = &geometry.nodes[ladder + 1 + index];
        assert_eq!(rung.pointer.as_str(), format!("{LADDER}/rungs/{index}"));
        assert_eq!((rung.tag, rung.parent), (NodeTag::Rung, Some(ladder)));
    }
}
