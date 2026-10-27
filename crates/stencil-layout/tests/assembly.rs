//! The design sheet's assembly surfaces as laid out and as the checks read them back
//! (section 14.4 rules 23 to 27, section 14.6 rule 4): every bend drawn, tags and valves off
//! bends, hops, spread line ends, rails, step badges and tabs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{layout, page_from};
use serde_json::{Value, json};
use stencil_layout::checks::{links_routed, marks_clear};
use stencil_layout::sheet_lines::OUTLINE_AIR_PX;
use stencil_layout::{
    BoxRect, DashRun, DrawnLink, EndCap, LinkRoute, NodeGeometry, PageGeometry, PartName,
    PathPiece, dash_runs, drawn_link, rail_bar, stroke_on_at, turn_positions,
};
use stencil_model::{Page, PagePoint};

const ONEPAGER: &str = include_str!("../../../examples/onepager.json");

/// The dash patterns of a sheet line: dashed, dotted, and a planned line's outer stroke.
const PATTERNS: [(f32, f32); 3] = [(6.0, 5.0), (2.0, 3.0), (7.0, 4.0)];

fn onepager() -> (Page, PageGeometry) {
    let page: Page = serde_json::from_str(ONEPAGER).unwrap();
    let geometry = layout(&page);
    (page, geometry)
}

fn route(geometry: &PageGeometry, index: usize) -> &LinkRoute {
    geometry
        .links
        .iter()
        .find(|route| route.index == index)
        .unwrap()
}

fn route_mut(geometry: &mut PageGeometry, index: usize) -> &mut LinkRoute {
    geometry
        .links
        .iter_mut()
        .find(|route| route.index == index)
        .unwrap()
}

/// The index of the link labelled `label`.
fn link_index(page: &Page, label: &str) -> usize {
    page.links
        .iter()
        .position(|link| link.label.as_deref() == Some(label))
        .unwrap()
}

fn node_with_id<'a>(page: &Page, geometry: &'a PageGeometry, id: &str) -> &'a NodeGeometry {
    let pointer = stencil_model::body_nodes(page)
        .into_iter()
        .find(|entry| entry.node.id() == Some(id))
        .unwrap()
        .pointer;
    geometry
        .nodes
        .iter()
        .find(|node| node.pointer == pointer)
        .unwrap()
}

/// (pointer, message) of every defect of a report.
fn messages(report: &stencil_model::checks::CheckReport) -> Vec<(String, String)> {
    report
        .defects
        .iter()
        .map(|defect| (defect.pointer.to_string(), defect.message.clone()))
        .collect()
}

fn centered_on(point: PagePoint, bounds: BoxRect) -> BoxRect {
    BoxRect {
        x: point.x - bounds.width / 2.0,
        y: point.y - bounds.height / 2.0,
        ..bounds
    }
}

/// The interior corners of a route.
fn bends(route: &LinkRoute) -> Vec<PagePoint> {
    let count = route.points.len();
    route
        .points
        .iter()
        .copied()
        .skip(1)
        .take(count.saturating_sub(2))
        .collect()
}

/// A sheet with two actors: `q` above `w` in one column and `s` and `u` either side of it,
/// so the read link from q to w crosses the send link from s to u.
fn crossing_document() -> Value {
    let wrap = |item: Value, justify: &str| json!({ "tag": "Row", "justify": justify, "grow": [0], "children": [item] });
    let item = |id: &str, title: &str, actor: &str| json!({ "tag": "Item", "id": id, "kind": "product", "title": title, "actor": actor });
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "sheet": {
            "problem": { "points": [{ "text": "A point.", "marks": [{ "attention": "first" }] }] },
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
            "title": "System",
            "children": [{
                "tag": "Col",
                "gap": 64,
                "children": [
                    wrap(item("q", "Queue reader", "crew"), "center"),
                    {
                        "tag": "Row",
                        "grow": [1, 1],
                        "children": [
                            wrap(item("s", "Sender", "ops"), "start"),
                            wrap(item("u", "Receiver", "ops"), "end")
                        ]
                    },
                    wrap(item("w", "Writer", "crew"), "center")
                ]
            }]
        }],
        "legend": [{ "line": "solid", "by_actor": true, "text": "work moves" }],
        "links": [
            { "from": "q", "to": "w", "line": "solid", "label": "read" },
            { "from": "s", "to": "u", "line": "solid", "label": "send" }
        ]
    })
}

/// The mockup drew a patterned line as one dashed path, so a corner 8 px along fell in the
/// gap of both the dashed and the planned pattern and the bend vanished. Drawn in runs, the
/// corner lies in a solid joint under every pattern.
#[test]
fn a_bend_falls_in_a_dash_gap_without_a_joint_and_on_a_joint_with_one() {
    let corner = PagePoint { x: 8.0, y: 0.0 };
    let drawn = DrawnLink {
        start: PagePoint { x: 0.0, y: 0.0 },
        legs: vec![
            vec![PathPiece::Line(corner)],
            vec![PathPiece::Line(PagePoint { x: 8.0, y: 30.0 })],
        ],
        start_arrow: None,
        end_arrow: None,
        cap: EndCap::Arrowhead,
    };
    let turns = turn_positions(&drawn);
    assert_eq!(turns, [8.0]);
    let mockup = [DashRun {
        start: drawn.start,
        pieces: drawn.legs.concat(),
        joint: false,
        from_px: 0.0,
        length_px: 38.0,
    }];
    assert!(!stroke_on_at(&mockup, PATTERNS[0], turns[0]));
    assert!(!stroke_on_at(&mockup, PATTERNS[2], turns[0]));
    let runs = dash_runs(&drawn);
    assert_eq!(
        runs.iter().map(|run| run.joint).collect::<Vec<_>>(),
        [false, true, false]
    );
    // The run after the joint starts in a gap, so the joint reads as one dash.
    assert!(!stroke_on_at(&runs, PATTERNS[0], runs[2].from_px + 0.5));
    let mut examined = 0;
    for pattern in PATTERNS {
        assert!(stroke_on_at(&runs, pattern, turns[0]), "{pattern:?}");
        examined += 1;
    }
    assert_eq!(examined, 3);
}

/// A rounded bend is a solid joint from before its arc to after it.
#[test]
fn a_rounded_bend_is_one_joint_and_every_onepager_bend_is_drawn() {
    let drawn = DrawnLink {
        start: PagePoint { x: 0.0, y: 0.0 },
        legs: vec![
            vec![
                PathPiece::Line(PagePoint { x: 2.0, y: 0.0 }),
                PathPiece::Arc {
                    radius: 6.0,
                    clockwise: true,
                    corner: PagePoint { x: 8.0, y: 0.0 },
                    to: PagePoint { x: 8.0, y: 6.0 },
                },
            ],
            vec![PathPiece::Line(PagePoint { x: 8.0, y: 30.0 })],
        ],
        start_arrow: None,
        end_arrow: None,
        cap: EndCap::Arrowhead,
    };
    let runs = dash_runs(&drawn);
    let joint = runs.iter().find(|run| run.joint).unwrap();
    assert!(
        joint
            .pieces
            .iter()
            .any(|piece| matches!(piece, PathPiece::Arc { .. }))
    );
    for position in turn_positions(&drawn) {
        assert!(stroke_on_at(&runs, PATTERNS[0], position));
    }

    let (_, geometry) = onepager();
    let mut turns = 0;
    for route in &geometry.links {
        let drawn = drawn_link(&geometry, route);
        let runs = dash_runs(&drawn);
        for position in turn_positions(&drawn) {
            for pattern in PATTERNS {
                assert!(
                    stroke_on_at(&runs, pattern, position),
                    "/links/{}",
                    route.index
                );
            }
            turns += 1;
        }
    }
    assert!(turns > 10, "examined {turns} turns");
}

/// The mockup set the planned line's tag on its corner. A tag or a valve within a dash
/// period and its air of a bend is a links-routed defect; as laid out neither is.
#[test]
fn a_tag_or_a_valve_on_a_bend_is_a_links_routed_defect() {
    // Leaving the Holds API to its right, as on the mockup, the planned line bends.
    let mut document: Value = serde_json::from_str(ONEPAGER).unwrap();
    let planned = link_index(&page_from(document.clone()), "schedule expiry");
    document["links"][planned]["from_side"] = json!("right");
    let page = page_from(document);
    let mut geometry = layout(&page);
    let valved = link_index(&page, "queue job");
    let before = links_routed(&page, &geometry);
    assert_eq!(before.examined, page.links.len() as u64);
    let of = |report: &[(String, String)], index: usize| -> Vec<String> {
        report
            .iter()
            .filter(|(pointer, _)| *pointer == format!("/links/{index}"))
            .map(|(_, message)| message.clone())
            .collect()
    };
    let before = messages(&before);
    assert_eq!(of(&before, planned), Vec::<String>::new());
    assert_eq!(of(&before, valved), Vec::<String>::new());

    let corner = bends(route(&geometry, planned))[0];
    let tagged = route_mut(&mut geometry, planned);
    tagged.tag = tagged.tag.map(|tag| centered_on(corner, tag));
    let valve_corner = bends(route(&geometry, valved))[0];
    let valve = route_mut(&mut geometry, valved);
    for part in &mut valve.parts {
        if matches!(part.name, PartName::ValveNear | PartName::ValveFar) {
            part.bounds = centered_on(valve_corner, part.bounds);
        }
    }
    let after = messages(&links_routed(&page, &geometry));
    let on_bend = |index: usize, what: &str| {
        after.iter().any(|(pointer, message)| {
            *pointer == format!("/links/{index}")
                && message.starts_with(what)
                && message.contains("of a bend")
        })
    };
    assert!(on_bend(planned, "tag "), "{after:?}");
    assert!(on_bend(valved, "valve "), "{after:?}");
}

/// A later link of another actor hops where it crosses; the hop is two quarter arcs on its
/// drawn line, clear of bends, ends and tags.
#[test]
fn a_later_link_of_another_actor_hops_where_it_crosses() {
    let page = page_from(crossing_document());
    let geometry = layout(&page);
    let hops: Vec<(usize, PagePoint)> = geometry
        .links
        .iter()
        .flat_map(|route| {
            route
                .parts
                .iter()
                .filter(|part| part.name == PartName::Hop)
                .map(|part| (route.index, stencil_layout::sheet_lines::hop_point(part)))
        })
        .collect();
    let read = route(&geometry, 0);
    let send = route(&geometry, 1);
    let crossing = PagePoint {
        x: read.points[0].x,
        y: send.points[0].y,
    };
    assert_eq!(hops, [(1, crossing)]);
    let arcs = drawn_link(&geometry, send)
        .legs
        .concat()
        .into_iter()
        .filter(|piece| matches!(piece, PathPiece::Arc { .. }))
        .count();
    assert_eq!(arcs, 2);
    let report = marks_clear(&page, &geometry);
    assert_eq!(messages(&report), Vec::<(String, String)>::new());

    let mut one_actor = crossing_document();
    one_actor["body"][0]["children"][0]["children"][1]["children"][0]["children"][0]["actor"] =
        json!("crew");
    one_actor["body"][0]["children"][0]["children"][1]["children"][1]["children"][0]["actor"] =
        json!("crew");
    one_actor["actors"] = json!([{ "id": "crew", "name": "Crew", "tint": 2 }]);
    let page = page_from(one_actor);
    let geometry = layout(&page);
    assert!(
        geometry
            .links
            .iter()
            .all(|route| route.parts.iter().all(|part| part.name != PartName::Hop))
    );
}

/// A hop on the approach to a line's end or under a tag is a marks-clear defect.
#[test]
fn a_hop_near_an_end_or_under_a_tag_is_a_marks_clear_defect() {
    let page = page_from(crossing_document());
    let mut geometry = layout(&page);
    let send_end = *route(&geometry, 1).points.last().unwrap();
    let read_tag = route(&geometry, 0).tag.unwrap();
    let mut check_at = |point: PagePoint| {
        let send = route_mut(&mut geometry, 1);
        let hop = send
            .parts
            .iter_mut()
            .find(|part| part.name == PartName::Hop)
            .unwrap();
        hop.bounds = BoxRect {
            x: point.x - hop.bounds.width / 2.0,
            y: point.y - hop.bounds.height,
            ..hop.bounds
        };
        messages(&marks_clear(&page, &geometry))
    };
    let near_end = check_at(PagePoint {
        x: send_end.x - 10.0,
        y: send_end.y,
    });
    assert!(
        near_end
            .iter()
            .any(|(_, message)| message.contains("lies on the approach at a line's end")),
        "{near_end:?}"
    );
    let under_tag = check_at(PagePoint {
        x: read_tag.x + read_tag.width / 2.0,
        y: read_tag.y + read_tag.height / 2.0,
    });
    assert!(
        under_tag
            .iter()
            .any(|(_, message)| message.contains("lies on a tag")),
        "{under_tag:?}"
    );
}

/// Two ends that meet one side of a node spread along it; two ends closer than 12 px on
/// one node are a links-routed defect.
#[test]
fn two_ends_on_one_side_spread_apart_and_ends_closer_than_12_px_are_a_defect() {
    let mut document = crossing_document();
    document["links"] = json!([
        { "from": "q", "to": "u", "line": "solid", "label": "read", "to_side": "left" },
        { "from": "s", "to": "u", "line": "solid", "label": "send", "to_side": "left" }
    ]);
    document["body"][0]["children"][0]["children"][1]["children"][0]["children"][0]["actor"] =
        json!("crew");
    let page = page_from(document);
    let mut geometry = layout(&page);
    let ends: Vec<PagePoint> = (0..2)
        .map(|index| *route(&geometry, index).points.last().unwrap())
        .collect();
    assert!((ends[0].x - ends[1].x).abs() < 0.01, "{ends:?}");
    assert!((ends[0].y - ends[1].y).abs() >= 12.0, "{ends:?}");
    assert_eq!(
        messages(&links_routed(&page, &geometry)),
        Vec::<(String, String)>::new()
    );

    let moved = route_mut(&mut geometry, 1);
    let count = moved.points.len();
    let shift = ends[0].y + 6.0 - ends[1].y;
    moved.points[count - 1].y += shift;
    moved.points[count - 2].y += shift;
    let after = messages(&links_routed(&page, &geometry));
    assert!(
        after.iter().any(|(pointer, message)| pointer == "/links/1"
            && message.contains("lies within 12.00 px of an end of /links/0 on one node")),
        "{after:?}"
    );
}

/// A mark glyph on an arrowhead hides where the line arrives: a marks-clear defect.
#[test]
fn a_glyph_on_an_arrival_is_a_marks_clear_defect() {
    let (page, mut geometry) = onepager();
    assert_eq!(
        messages(&marks_clear(&page, &geometry)),
        Vec::<(String, String)>::new()
    );
    let index = link_index(&page, "mark ready");
    let tip = *route(&geometry, index).points.last().unwrap();
    let carrier = geometry
        .nodes
        .iter_mut()
        .find(|node| node.parts.iter().any(|part| part.name == PartName::Balloon))
        .unwrap();
    let pointer = carrier.pointer.to_string();
    let balloon = carrier
        .parts
        .iter_mut()
        .find(|part| part.name == PartName::Balloon)
        .unwrap();
    balloon.bounds = centered_on(
        PagePoint {
            x: tip.x,
            y: tip.y - 4.0,
        },
        balloon.bounds,
    );
    let after = messages(&marks_clear(&page, &geometry));
    assert!(
        after
            .iter()
            .any(|(at, message)| *at == format!("/links/{index}")
                && message.contains(&format!("is covered by a glyph of {pointer}"))),
        "{after:?}"
    );
}

/// The mockup ran a line a few px inside an assembly's dash-dot outline, where it reads as
/// the outline. A leg within 12 px along a Box outline is a links-routed defect.
#[test]
fn a_leg_along_a_box_outline_is_a_links_routed_defect() {
    let (page, mut geometry) = onepager();
    let assembly = node_with_id(&page, &geometry, "evaluation-path").clone();
    let outline = assembly.part(PartName::TabBorder).unwrap().bounds;
    let index = link_index(&page, "send notice");
    let along = route_mut(&mut geometry, index);
    let x = outline.x + 5.0;
    along.points = vec![
        PagePoint {
            x,
            y: outline.y + 20.0,
        },
        PagePoint {
            x,
            y: outline.bottom() - 20.0,
        },
    ];
    let after = messages(&links_routed(&page, &geometry));
    assert!(
        after
            .iter()
            .any(|(pointer, message)| *pointer == format!("/links/{index}")
                && message.contains(&format!(
                    "runs within 12.00 px along the outline of {}",
                    assembly.pointer
                ))),
        "{after:?}"
    );
}

/// A link ends on a rail on a long side inside its ends, or at an end of the bar, and
/// draws a dot there; an end off the bar, or without its dot, is a links-routed defect.
#[test]
fn a_rail_end_off_its_bar_or_without_its_dot_is_a_defect() {
    let (page, mut geometry) = onepager();
    let queue = node_with_id(&page, &geometry, "queue");
    let bar = rail_bar(queue).unwrap();
    let index = link_index(&page, "deliver job");
    let start = route(&geometry, index).points[0];
    assert!(
        (start.x - bar.x).abs() < 0.01 || (start.x - bar.right()).abs() < 0.01,
        "{start:?} on {bar:?}"
    );
    let dots = route(&geometry, index)
        .parts
        .iter()
        .filter(|part| part.name == PartName::RailDot)
        .count();
    assert_eq!(dots, 1);

    let moved = route_mut(&mut geometry, index);
    moved.points[0].x += 30.0;
    moved.parts.retain(|part| part.name != PartName::RailDot);
    let after = messages(&links_routed(&page, &geometry));
    let pointer = format!("/links/{index}");
    for expected in [
        "is neither on a side of the rail bar",
        "on a rail draws no rail dot",
    ] {
        assert!(
            after
                .iter()
                .any(|(at, message)| *at == pointer && message.contains(expected)),
            "{expected}: {after:?}"
        );
    }
}

/// A step badge sits at the left of its tag, clear of the label, its numeral reading its
/// line's step; a badge moved out of its tag is a marks-clear defect.
#[test]
fn a_step_badge_sits_left_in_its_tag_and_reads_its_step() {
    let (page, mut geometry) = onepager();
    let mut examined = 0;
    for (index, link) in page.links.iter().enumerate() {
        let Some(step) = link.step else {
            continue;
        };
        let route = route(&geometry, index);
        let tag = route.tag.unwrap();
        let badge = route
            .parts
            .iter()
            .find(|part| part.name == PartName::StepBadge)
            .unwrap()
            .bounds;
        let label = route
            .parts
            .iter()
            .find(|part| part.name == PartName::TagLabel)
            .unwrap()
            .bounds;
        assert!(badge.x > tag.x && badge.right() < label.x, "/links/{index}");
        assert_eq!((badge.width, badge.height), (18.0, 18.0));
        let numeral = route
            .parts
            .iter()
            .find(|part| part.name == PartName::StepNumeral)
            .unwrap();
        assert_eq!(numeral.text.as_ref().unwrap().text, step.to_string());
        examined += 1;
    }
    assert_eq!(examined, 6);

    let index = link_index(&page, "deliver job");
    let moved = route_mut(&mut geometry, index);
    let tag = moved.tag.unwrap();
    for part in &mut moved.parts {
        if matches!(part.name, PartName::StepBadge | PartName::StepNumeral) {
            part.bounds.x -= tag.width;
        }
    }
    let after = messages(&marks_clear(&page, &geometry));
    assert!(
        after
            .iter()
            .any(|(at, message)| *at == format!("/links/{index}")
                && message.contains("extends outside its tag")),
        "{after:?}"
    );
}

/// An assembly's tab lies inside its Box and straddles the top of its dash-dot outline.
#[test]
fn a_tab_lies_inside_its_box_across_the_top_of_its_outline() {
    let (page, geometry) = onepager();
    let assembly = node_with_id(&page, &geometry, "evaluation-path");
    let tab = assembly.part(PartName::Tab).unwrap().bounds;
    let outline = assembly.part(PartName::TabBorder).unwrap().bounds;
    let bounds = assembly.bounds;
    assert!(tab.x >= bounds.x && tab.right() <= bounds.right());
    assert!(tab.y >= bounds.y && tab.bottom() <= bounds.bottom());
    assert!(
        tab.y < outline.y && tab.bottom() > outline.y,
        "{tab:?} {outline:?}"
    );
    assert!(tab.x > outline.x);
    let label = assembly.part(PartName::Label).unwrap().bounds;
    assert!(label.x >= tab.x && label.right() <= tab.right());
}

/// A leg that turns straight back on the one before it, as a valve approach longer than
/// the gap it ran into once drew, is a links-routed defect.
#[test]
fn a_leg_that_turns_back_is_a_links_routed_defect() {
    let (page, mut geometry) = onepager();
    let index = link_index(&page, "deliver job");
    let start = route(&geometry, index).points[0];
    route_mut(&mut geometry, index).points = vec![
        start,
        PagePoint {
            x: start.x + 60.0,
            y: start.y,
        },
        PagePoint {
            x: start.x + 30.0,
            y: start.y,
        },
    ];
    let after = messages(&links_routed(&page, &geometry));
    assert!(
        after
            .iter()
            .any(|(pointer, message)| *pointer == format!("/links/{index}")
                && message == "leg 1 turns back on leg 0"),
        "{after:?}"
    );
}

/// A store's top is its lid's curve, so an end on it lies on the curve wherever along the
/// side a leg shift moved it, and an arrowhead or stop plate there meets the drawn lid.
#[test]
fn every_end_on_a_store_lid_lies_on_the_curve_after_its_leg_moves() {
    let (_, geometry) = onepager();
    let mut examined = 0;
    for route in &geometry.links {
        for (end, node) in [
            (route.points.first(), route.from_node),
            (route.points.last(), route.to_node),
        ] {
            let (Some(end), Some(node)) = (end, geometry.nodes.get(node)) else {
                continue;
            };
            if node.outline != Some(stencil_model::Outline::Store) || end.y > node.bounds.y + 10.0 {
                continue;
            }
            let half = node.bounds.width / 2.0;
            let across = (end.x - (node.bounds.x + half)) / half;
            let lid = node.bounds.y
                + stencil_layout::STORE_LID_RY_PX * (1.0 - (1.0 - across * across).sqrt());
            assert!(
                (end.y - lid).abs() < 0.01,
                "/links/{} ends at ({:.2}, {:.2}), off the lid at y {lid:.2}",
                route.index,
                end.x,
                end.y
            );
            examined += 1;
        }
    }
    assert!(examined >= 2, "examined {examined} store ends");
}

/// The mark ready line leaves the pickup worker and turns above the notifier, an
/// Item: the turn keeps OUTLINE_AIR_PX above the card, and no onepager leg crowds an
/// outline.
#[test]
fn a_sheet_leg_keeps_its_air_from_an_item_outline() {
    let (page, geometry) = onepager();
    let evaluator = node_with_id(&page, &geometry, "evaluator").bounds;
    let index = link_index(&page, "mark ready");
    let mut examined = 0;
    for pair in route(&geometry, index).points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let overlap = a.x.max(b.x).min(evaluator.right()) - a.x.min(b.x).max(evaluator.x);
        if a.y == b.y && overlap > 0.01 {
            examined += 1;
            let air = (evaluator.y - a.y).abs();
            assert!(air >= OUTLINE_AIR_PX, "{air} px above {evaluator:?}");
        }
    }
    assert_eq!(examined, 1);
    let crowded: Vec<(String, String)> = messages(&links_routed(&page, &geometry))
        .into_iter()
        .filter(|(_, message)| message.contains("along the outline of"))
        .collect();
    assert!(crowded.is_empty(), "{crowded:?}");
}
