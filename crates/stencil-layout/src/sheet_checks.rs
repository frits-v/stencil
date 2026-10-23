//! The design sheet's additions to `links-routed` and `marks-clear` (section 14.6 rule 4):
//! what a sheet line draws is read back from the geometry, so a tag on a bend, a bend in a
//! dash gap, two links on one line, two ends at one point, a rail end off its bar, a step
//! badge, a valve or a hop out of place, and a glyph on an arrival are defects.

use stencil_model::checks::Defect;
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, Node, NodeRef, Page, PagePoint, body_nodes};

use crate::drawn::{dash_runs, drawn_link, stroke_on_at, turn_positions};
use crate::route::{RAIL_END_CLEARANCE_PX, rail_bar};
use crate::sheet_lines::{
    HOP_END_CLEARANCE_PX, HOP_END_STUB_PX, bend_keep_out, bends, covers_a_bend, hop_point,
    legs_shared, valve_box,
};
use crate::{BoxRect, GEOMETRY_EPSILON_PX, LinkRoute, NodeTag, PageGeometry, Part, PartName};

/// The dash patterns a sheet line may draw, as (dash, gap): a dash line, a dotted line and
/// the outer stroke of a planned line. A bend must be drawn under each of them.
pub const SHEET_PATTERNS: [(f32, f32); 3] = [(6.0, 5.0), (2.0, 3.0), (7.0, 4.0)];
/// Two link ends on one node closer than this read as one.
pub const ENDS_APART_PX: f32 = 12.0;

fn describe(bounds: &BoxRect) -> String {
    format!(
        "{:.2}x{:.2} at ({:.2}, {:.2})",
        bounds.width, bounds.height, bounds.x, bounds.y
    )
}

fn overlap(first: &BoxRect, second: &BoxRect) -> bool {
    let width = first.right().min(second.right()) - first.x.max(second.x);
    let height = first.bottom().min(second.bottom()) - first.y.max(second.y);
    width > GEOMETRY_EPSILON_PX && height > GEOMETRY_EPSILON_PX
}

fn segments(points: &[PagePoint]) -> Vec<(PagePoint, PagePoint)> {
    points
        .windows(2)
        .filter_map(|pair| Some((*pair.first()?, *pair.get(1)?)))
        .collect()
}

/// A point of a route, for messages.
fn at(point: PagePoint) -> String {
    format!("({:.2}, {:.2})", point.x, point.y)
}

/// The position along the drawn line where `dash` on and `gap` off leaves a bend unpainted,
/// for each bend and hop of the drawn line: the check of section 14.5 rule 1 of the bends.
pub fn bends_in_gaps(geometry: &PageGeometry, route: &LinkRoute, pattern: (f32, f32)) -> Vec<f32> {
    let drawn = drawn_link(geometry, route);
    let runs = dash_runs(&drawn);
    turn_positions(&drawn)
        .into_iter()
        .filter(|position| !stroke_on_at(&runs, pattern, *position))
        .collect()
}

/// The sheet defects of `links-routed` for one route, in the order a reader meets them.
pub(crate) fn sheet_route_defects(
    geometry: &PageGeometry,
    route: &LinkRoute,
    routes: &[LinkRoute],
) -> Vec<Defect> {
    let pointer = NodePointer::root().child("links").index(route.index);
    let mut defects = Vec::new();
    let mut push = |message: String| {
        defects.push(Defect {
            pointer: pointer.clone(),
            message,
        });
    };
    // A leg that turns straight back on the one before it.
    for (index, pair) in segments(&route.points).windows(2).enumerate() {
        let (Some(first), Some(second)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let dot = (first.1.x - first.0.x) * (second.1.x - second.0.x)
            + (first.1.y - first.0.y) * (second.1.y - second.0.y);
        if dot < -GEOMETRY_EPSILON_PX {
            push(format!("leg {} turns back on leg {index}", index + 1));
        }
    }
    let keep_out = bend_keep_out(route.corner);
    if let Some(tag) = route.tag
        && covers_a_bend(&tag, &route.points, keep_out)
    {
        push(format!(
            "tag {} lies within {keep_out:.2} px of a bend",
            describe(&tag)
        ));
    }
    if let Some(valve) = valve_box(&route.parts)
        && covers_a_bend(&valve, &route.points, keep_out)
    {
        push(format!(
            "valve {} lies within {keep_out:.2} px of a bend",
            describe(&valve)
        ));
    }
    for pattern in crate::sheet_checks::SHEET_PATTERNS {
        for position in bends_in_gaps(geometry, route, pattern) {
            push(format!(
                "the bend {position:.2} px along the line falls in a gap of the {} {} pattern",
                pattern.0, pattern.1
            ));
        }
    }
    for other in routes.iter().take(LINKS_MAX + 1) {
        if other.index == route.index {
            continue;
        }
        let other_pointer = NodePointer::root().child("links").index(other.index);
        if other.index < route.index {
            let shared = segments(&route.points)
                .iter()
                .enumerate()
                .find_map(|(leg, mine)| {
                    segments(&other.points)
                        .iter()
                        .any(|theirs| legs_shared(*mine, *theirs))
                        .then_some(leg)
                });
            if let Some(leg) = shared {
                push(format!("leg {leg} runs along a leg of {other_pointer}"));
            }
            if let (Some(tag), Some(theirs)) = (route.tag, other.tag)
                && overlap(&tag, &theirs)
            {
                push(format!(
                    "tag {} overlaps the tag of {other_pointer}",
                    describe(&tag)
                ));
            }
        }
        if let Some(tag) = route.tag
            && let Some((a, b)) = segments(&other.points).into_iter().find(|(a, b)| {
                tag.x < a.x.max(b.x)
                    && a.x.min(b.x) < tag.right()
                    && tag.y < a.y.max(b.y)
                    && a.y.min(b.y) < tag.bottom()
            })
        {
            push(format!(
                "tag {} covers the leg of {other_pointer} from {} to {}",
                describe(&tag),
                at(a),
                at(b)
            ));
        }
        for (mine, node) in [
            (route.points.first(), route.from_node),
            (route.points.last(), route.to_node),
        ] {
            for (theirs, other_node) in [
                (other.points.first(), other.from_node),
                (other.points.last(), other.to_node),
            ] {
                let (Some(mine), Some(theirs)) = (mine, theirs) else {
                    continue;
                };
                if node == other_node
                    && other.index < route.index
                    && (mine.x - theirs.x).hypot(mine.y - theirs.y) < ENDS_APART_PX
                {
                    push(format!(
                        "end {} lies within {ENDS_APART_PX:.2} px of an end of {other_pointer} on one node",
                        at(*mine)
                    ));
                }
            }
        }
    }
    defects.extend(rail_end_defects(geometry, route, &pointer));
    defects
}

/// Each end of a route on a rail lies on the bar's long side at least
/// `RAIL_END_CLEARANCE_PX` inside its ends, or at one end of the bar, with a rail dot drawn
/// there (section 14.6 rule 4).
fn rail_end_defects(
    geometry: &PageGeometry,
    route: &LinkRoute,
    pointer: &NodePointer,
) -> Vec<Defect> {
    let mut defects = Vec::new();
    let dots: Vec<&Part> = route
        .parts
        .iter()
        .filter(|part| part.name == PartName::RailDot)
        .collect();
    for (end, node) in [
        (route.points.first(), route.from_node),
        (route.points.last(), route.to_node),
    ] {
        let (Some(end), Some(bar)) = (end, geometry.nodes.get(node).and_then(rail_bar)) else {
            continue;
        };
        let near = |a: f32, b: f32| (a - b).abs() <= GEOMETRY_EPSILON_PX;
        let center_x = bar.x + bar.width / 2.0;
        let on_side = (near(end.x, bar.x) || near(end.x, bar.right()))
            && end.y >= bar.y + RAIL_END_CLEARANCE_PX - GEOMETRY_EPSILON_PX
            && end.y <= bar.bottom() - RAIL_END_CLEARANCE_PX + GEOMETRY_EPSILON_PX;
        let on_end = near(end.x, center_x) && (near(end.y, bar.y) || near(end.y, bar.bottom()));
        if !on_side && !on_end {
            defects.push(Defect {
                pointer: pointer.clone(),
                message: format!(
                    "end {} is neither on a side of the rail bar {} {RAIL_END_CLEARANCE_PX:.2} px inside its ends nor at an end of it",
                    at(*end),
                    describe(&bar)
                ),
            });
        }
        let dotted = dots.iter().any(|dot| {
            near(dot.bounds.x + dot.bounds.width / 2.0, center_x)
                && near(dot.bounds.y + dot.bounds.height / 2.0, end.y)
        });
        if !dotted {
            defects.push(Defect {
                pointer: pointer.clone(),
                message: format!("end {} on a rail draws no rail dot", at(*end)),
            });
        }
    }
    defects
}

/// The box a route's end feature covers: its arrowhead or stop plate, or its dot on a rail.
fn end_features(geometry: &PageGeometry, route: &LinkRoute) -> Vec<BoxRect> {
    let drawn = drawn_link(geometry, route);
    route
        .parts
        .iter()
        .filter(|part| part.name == PartName::RailDot)
        .map(|part| part.bounds)
        .chain(
            [drawn.start_arrow, drawn.end_arrow]
                .into_iter()
                .flatten()
                .map(|arrow| drawn.cap.bounds(arrow)),
        )
        .collect()
}

/// The sheet's additions to `marks-clear` (section 14.6): each step badge inside its tag,
/// clear of the tag's text, with its numeral inside its square reading its line's step;
/// each valve clear of every tag, text and box a link avoids; each hop clear of bends, of
/// the approach at either line's end and of every tag; and no mark glyph on an arrowhead
/// or a rail dot. Returns how many badges, valves, hops and line ends it examined.
pub(crate) fn line_mark_defects(page: &Page, geometry: &PageGeometry) -> (u64, Vec<Defect>) {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    let links_pointer = NodePointer::root().child("links");
    let glyphs: Vec<(String, BoxRect)> = geometry
        .nodes
        .iter()
        .flat_map(|node| {
            node.parts
                .iter()
                .filter(|part| part.name.is_mark_glyph())
                .map(|part| (node.pointer.to_string(), part.bounds))
        })
        .chain(geometry.links.iter().flat_map(|route| {
            route
                .parts
                .iter()
                .filter(|part| part.name.is_mark_glyph())
                .map(|part| (links_pointer.index(route.index).to_string(), part.bounds))
        }))
        .collect();
    let tags: Vec<(usize, BoxRect)> = geometry
        .links
        .iter()
        .filter_map(|route| route.tag.map(|tag| (route.index, tag)))
        .collect();
    for route in geometry.links.iter().take(LINKS_MAX + 1) {
        let pointer = links_pointer.index(route.index);
        let step = page.links.get(route.index).and_then(|link| link.step);
        badge_defects(
            (&pointer, route.tag, &route.parts, step),
            (&mut examined, &mut defects),
        );
        if let Some(valve) = valve_box(&route.parts) {
            examined += 1;
            let blocking =
                crate::sheet_lines::tag_obstacles(geometry, route.from_node, route.to_node);
            for (index, tag) in &tags {
                if overlap(&valve, tag) {
                    defects.push(Defect {
                        pointer: pointer.clone(),
                        message: format!(
                            "valve {} overlaps the tag of {}",
                            describe(&valve),
                            links_pointer.index(*index)
                        ),
                    });
                }
            }
            if let Some(hit) = blocking.iter().find(|bounds| overlap(&valve, bounds)) {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!("valve {} overlaps box {}", describe(&valve), describe(hit)),
                });
            }
        }
        for hop in route.parts.iter().filter(|part| part.name == PartName::Hop) {
            examined += 1;
            let point = hop_point(hop);
            let near_bend = geometry.links.iter().any(|other| {
                let crosses = other.index == route.index || other_holds(other, point);
                crosses
                    && bends(&other.points).iter().any(|corner| {
                        (corner.x - point.x).abs() + (corner.y - point.y).abs()
                            < HOP_END_CLEARANCE_PX + other.corner
                    })
            });
            let near_end = geometry.links.iter().any(|other| {
                (other.index == route.index || other_holds(other, point))
                    && [other.points.first(), other.points.last()]
                        .into_iter()
                        .flatten()
                        .any(|end| {
                            (end.x - point.x).abs() + (end.y - point.y).abs() < HOP_END_STUB_PX
                        })
            });
            let under_tag = tags.iter().any(|(_, tag)| {
                point.x > tag.x
                    && point.x < tag.right()
                    && point.y > tag.y
                    && point.y < tag.bottom()
            });
            for (fails, what) in [
                (near_bend, "a bend"),
                (near_end, "the approach at a line's end"),
                (under_tag, "a tag"),
            ] {
                if fails {
                    defects.push(Defect {
                        pointer: pointer.clone(),
                        message: format!("hop at {} lies on {what}", at(point)),
                    });
                }
            }
        }
        for feature in end_features(geometry, route) {
            examined += 1;
            for (owner, glyph) in &glyphs {
                if overlap(&feature, glyph) {
                    defects.push(Defect {
                        pointer: pointer.clone(),
                        message: format!(
                            "end {} is covered by a glyph of {owner} {}",
                            describe(&feature),
                            describe(glyph)
                        ),
                    });
                }
            }
        }
    }
    for entry in body_nodes(page) {
        let (NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe)) = entry.node else {
            continue;
        };
        let Some(node) = geometry
            .nodes
            .iter()
            .find(|node| node.pointer == entry.pointer)
        else {
            continue;
        };
        let tag = node.part(PartName::Tag).map(|part| part.bounds);
        badge_defects(
            (&entry.pointer, tag, &node.parts, pipe.step),
            (&mut examined, &mut defects),
        );
    }
    if let Some(legend) = geometry
        .nodes
        .iter()
        .find(|node| node.tag == NodeTag::Legend)
    {
        let pointer = legend.pointer.clone();
        badge_defects(
            (&pointer, Some(legend.bounds), &legend.parts, Some(1)),
            (&mut examined, &mut defects),
        );
    }
    (examined, defects)
}

/// True when a straight run of `route` passes through `point`.
fn other_holds(route: &LinkRoute, point: PagePoint) -> bool {
    segments(&route.points).iter().any(|(a, b)| {
        let within = |value: f32, low: f32, high: f32| {
            value >= low.min(high) - GEOMETRY_EPSILON_PX
                && value <= low.max(high) + GEOMETRY_EPSILON_PX
        };
        within(point.x, a.x, b.x) && within(point.y, a.y, b.y)
    })
}

/// The step badge of one tag: inside the tag, clear of its text, its numeral inside the
/// badge's square and reading `step`.
fn badge_defects(
    (pointer, tag, parts, step): (&NodePointer, Option<BoxRect>, &[Part], Option<u16>),
    (examined, defects): (&mut u64, &mut Vec<Defect>),
) {
    let Some(badge) = parts.iter().find(|part| part.name == PartName::StepBadge) else {
        return;
    };
    *examined += 1;
    let bounds = badge.bounds;
    if let Some(tag) = tag
        && (bounds.x < tag.x - GEOMETRY_EPSILON_PX
            || bounds.y < tag.y - GEOMETRY_EPSILON_PX
            || bounds.right() > tag.right() + GEOMETRY_EPSILON_PX
            || bounds.bottom() > tag.bottom() + GEOMETRY_EPSILON_PX)
    {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!(
                "step badge {} extends outside its tag {}",
                describe(&bounds),
                describe(&tag)
            ),
        });
    }
    for text in parts
        .iter()
        .filter(|part| matches!(part.name, PartName::TagLabel | PartName::TagSub))
    {
        if overlap(&bounds, &text.bounds) {
            defects.push(Defect {
                pointer: pointer.clone(),
                message: format!(
                    "step badge {} overlaps {} {}",
                    describe(&bounds),
                    text.name.as_str(),
                    describe(&text.bounds)
                ),
            });
        }
    }
    let Some(numeral) = parts.iter().find(|part| part.name == PartName::StepNumeral) else {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!("step badge {} draws no numeral", describe(&bounds)),
        });
        return;
    };
    let text = numeral
        .text
        .as_ref()
        .map(|run| run.text.as_str())
        .unwrap_or_default();
    let expected = step.map(|step| step.to_string()).unwrap_or_default();
    if text != expected {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!("step badge reads {text:?}, its line's step is {expected:?}"),
        });
    }
    let run = match &numeral.text {
        Some(run) => BoxRect {
            x: numeral.bounds.x + (numeral.bounds.width - run.metrics.width_px) / 2.0,
            y: numeral.bounds.y,
            width: run.metrics.width_px,
            height: run.metrics.height_px,
        },
        None => numeral.bounds,
    };
    let radius = crate::STEP_BADGE_RADIUS_PX;
    let inside = |(x, y): (f32, f32)| -> bool {
        let dx = (bounds.x + radius - x)
            .max(x - (bounds.right() - radius))
            .max(0.0);
        let dy = (bounds.y + radius - y)
            .max(y - (bounds.bottom() - radius))
            .max(0.0);
        x >= bounds.x - GEOMETRY_EPSILON_PX
            && x <= bounds.right() + GEOMETRY_EPSILON_PX
            && y >= bounds.y - GEOMETRY_EPSILON_PX
            && y <= bounds.bottom() + GEOMETRY_EPSILON_PX
            && dx.hypot(dy) <= radius + GEOMETRY_EPSILON_PX
    };
    let corners = [
        (run.x, run.y),
        (run.right(), run.y),
        (run.x, run.bottom()),
        (run.right(), run.bottom()),
    ];
    if !corners.into_iter().all(inside) {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!(
                "step badge numeral box {} leaves its square {}",
                describe(&run),
                describe(&bounds)
            ),
        });
    }
}
