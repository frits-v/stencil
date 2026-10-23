//! Link routing (section 11.2): attach points on the endpoint boxes, an orthogonal grid built
//! from the obstacle edges, A* over the grid intersections, and the tag box on the longest
//! segment. Routing reads finished geometry and never moves a node.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use stencil_model::Projection;
use stencil_model::pointer::NodePointer;
use stencil_model::text::{TextMeasurer, TextMetrics, TextStyleName};
use stencil_model::{
    Arrow, Bend, Grammar, LINK_CORNER_DEFAULT_PX, LINK_SEGMENTS_MAX, LINKS_MAX, Line, Link, Page,
    PagePoint, ROUTER_GRID_LINES_MAX, Side, VetRule, Violation, body_nodes, line_tint,
};

use crate::lanes::{LanesPlan, MessageRow};
use crate::sheet_lines::{add_hops, place_sheet_tag, valve_box, valve_center, valve_parts};
use crate::styles::{text_color, text_style_for};
use crate::{Axis, ISO_APPROACH_PX, ISO_BLOCK_HEIGHT_PX, shape_height_px, turned_box};
use crate::{
    BoxRect, GEOMETRY_EPSILON_PX, LINK_ARROW_STUB_PX, LayoutError, LinkRoute, NodeGeometry,
    NodeTag, PageGeometry, Part, PartName, RouteStatus, TextAlign, TextRun,
};

/// Distance kept between a route and an obstacle: grid lines sit this far outside each edge.
pub const OBSTACLE_CLEARANCE_PX: f32 = 8.0;
/// Cost of one turn, in px of route length.
const TURN_COST_PX: f64 = 40.0;
/// Two grid coordinates closer than this are one line.
const GRID_MERGE_PX: f32 = 0.001;

/// Tag box insets of a pipe tag (section 2.7): border 1.5, padding 6 top and bottom, 8 left
/// and right, and 2 px between label and sub.
const TAG_BORDER_PX: f32 = 1.5;
const TAG_PADDING_X_PX: f32 = 8.0;
const TAG_PADDING_Y_PX: f32 = 6.0;
const TAG_SUB_GAP_PX: f32 = 2.0;

/// Tie order for attach sides.
const SIDE_ORDER: [Side; 4] = [Side::Right, Side::Bottom, Side::Left, Side::Top];

/// A box a link must not enter, and the geometry node it belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Obstacle {
    pub node: usize,
    pub bounds: BoxRect,
}

/// Routes every link of a vetted page against finished geometry, in link order. A message
/// of a Lanes node is not searched for: it runs straight across its row (section 13.6).
pub(crate) fn route_links(
    page: &Page,
    grammar: &Grammar,
    geometry: &PageGeometry,
    lanes_plan: &LanesPlan,
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<LinkRoute>, LayoutError> {
    let nodes_by_id = nodes_by_id(page, geometry);
    let actors = stencil_model::PageActors::of(page);
    let page_marks = stencil_model::PageMarks::of(page);
    let links_pointer = NodePointer::root().child("links");
    let spread = if page.sheet.is_some() {
        spread_ends(page, geometry, &nodes_by_id, lanes_plan)
    } else {
        Vec::new()
    };
    // First every route, so a sheet can move a leg off another link's before any tag is
    // placed.
    let mut paths: Vec<RoutedPath> = Vec::with_capacity(page.links.len().min(LINKS_MAX));
    for (index, link) in page.links.iter().enumerate().take(LINKS_MAX) {
        let link_pointer = links_pointer.index(index);
        let from_node = resolve(&nodes_by_id, &link.from, link_pointer.child("from"))?;
        let to_node = resolve(&nodes_by_id, &link.to, link_pointer.child("to"))?;
        let (Some(from), Some(to)) = (geometry.nodes.get(from_node), geometry.nodes.get(to_node))
        else {
            return Err(LayoutError::Taffy {
                pointer: link_pointer,
                message: "link endpoint is not a geometry node".to_string(),
            });
        };
        // A link ends on a rail with a dot, not an arrowhead (section 14.4 rule 27).
        let arrow = drawn_arrow(
            link.arrow,
            (rail_bar(from).is_some(), rail_bar(to).is_some()),
        );
        let (points, status) = match lanes_plan.message(index) {
            Some(row) => (
                message_points(geometry, row, &from.bounds, &to.bounds, &link_pointer)?,
                RouteStatus::Routed,
            ),
            None => searched_route(
                geometry,
                link,
                (arrow, spread.get(index).copied().unwrap_or((None, None))),
                (from_node, from),
                (to_node, to),
                page.projection == Projection::Iso,
            ),
        };
        paths.push(RoutedPath {
            from_node,
            to_node,
            arrow,
            points,
            status,
        });
    }
    if page.sheet.is_some() {
        crate::sheet_lines::separate_shared_legs(geometry, &mut paths);
    }
    let all_points: Vec<Vec<PagePoint>> = paths.iter().map(|path| path.points.clone()).collect();
    let mut routes: Vec<LinkRoute> = Vec::with_capacity(paths.len());
    for (index, (link, path)) in page.links.iter().zip(paths).enumerate() {
        let link_pointer = links_pointer.index(index);
        let RoutedPath {
            from_node,
            to_node,
            arrow,
            mut points,
            status,
        } = path;
        let (Some(from), Some(to)) = (geometry.nodes.get(from_node), geometry.nodes.get(to_node))
        else {
            continue;
        };
        let (corner, bend) = link_bend(page, grammar, link.line);
        // A valve stands back from the arrowhead, or from the dot on a rail end.
        let end_on_rail = match link.arrow {
            Arrow::Start => rail_bar(from).is_some(),
            Arrow::End | Arrow::Both | Arrow::None => rail_bar(to).is_some(),
        };
        let end_inset = if end_on_rail {
            RAIL_DOT_RADIUS_PX - RAIL_BAR_PX / 2.0
        } else {
            crate::ARROWHEAD_LENGTH_PX
        };
        let valve_of = |points: &[PagePoint]| {
            if link.valve && page.sheet.is_some() {
                valve_center(points, link.arrow, end_inset, corner)
                    .map(|(center, direction)| valve_parts(center, direction))
            } else {
                None
            }
        };
        let mut valve = valve_of(&points);
        let mut parts = tag_parts(page, link, &link_pointer, &points, measurer)?;
        add_tag_marks(&page_marks, (link, index), &mut parts, measurer)?;
        if page.sheet.is_some() {
            // A tag keeps off the tags and valves placed before it and off every other
            // link's line, which it would hide.
            let mut taken: Vec<BoxRect> = routes
                .iter()
                .flat_map(|route: &LinkRoute| route.tag.into_iter().chain(valve_box(&route.parts)))
                .collect();
            for (other, other_points) in all_points.iter().enumerate() {
                if other != index {
                    taken.extend(other_points.windows(2).filter_map(|pair| {
                        let (a, b) = (pair.first()?, pair.get(1)?);
                        Some(BoxRect {
                            x: a.x.min(b.x),
                            y: a.y.min(b.y),
                            width: (a.x - b.x).abs(),
                            height: (a.y - b.y).abs(),
                        })
                    }));
                }
            }
            let valve_bounds = valve.as_ref().and_then(|valve| valve_box(valve));
            let placed = place_sheet_tag(
                geometry,
                (&points, from_node, to_node),
                (corner, valve_bounds, &taken),
                &mut parts,
            );
            // A tag no leg holds gets room by moving one leg a few px across its run.
            if !placed {
                let others: Vec<Vec<PagePoint>> = all_points
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .map(|(_, points)| points.clone())
                    .collect();
                for candidate in crate::sheet_lines::shifted_routes(
                    geometry,
                    (from_node, to_node),
                    &points,
                    &others,
                ) {
                    let candidate_valve = valve_of(&candidate);
                    let candidate_bounds =
                        candidate_valve.as_ref().and_then(|valve| valve_box(valve));
                    let mut trial = parts.clone();
                    if place_sheet_tag(
                        geometry,
                        (&candidate, from_node, to_node),
                        (corner, candidate_bounds, &taken),
                        &mut trial,
                    ) {
                        points = candidate;
                        valve = candidate_valve;
                        parts = trial;
                        break;
                    }
                }
            }
        }
        let tag = parts.first().map(|part| part.bounds);
        if let Some(valve) = valve {
            let valve_bounds = valve_box(&valve);
            parts.extend(valve);
            if let (None, Some(valve_bounds)) = (tag, valve_bounds) {
                add_valve_marks(
                    &page_marks,
                    (link, index),
                    valve_bounds,
                    &mut parts,
                    measurer,
                )?;
            }
        }
        for (end, node) in [(points.first(), from), (points.last(), to)] {
            if let (Some(point), Some(bar)) = (end, rail_bar(node)) {
                parts.push(rail_dot(*point, &bar));
            }
        }
        routes.push(LinkRoute {
            index,
            line: link.line,
            tint: line_tint(
                link.line,
                actors.slot(actors.link_actor(link)).or(link.tint),
            ),
            from_node,
            to_node,
            points,
            tag,
            parts,
            status,
            corner,
            bend,
            arrow,
        });
    }
    if page.sheet.is_some() {
        add_hops(page, geometry, &mut routes);
    }
    Ok(routes)
}

/// A link's route before its tag and marks are placed.
pub(crate) struct RoutedPath {
    pub from_node: usize,
    pub to_node: usize,
    pub arrow: Arrow,
    pub points: Vec<PagePoint>,
    pub status: RouteStatus,
}

/// The arrowheads a link draws: the authored ones, less any end on a rail, which draws a
/// dot instead.
fn drawn_arrow(arrow: Arrow, (from_rail, to_rail): (bool, bool)) -> Arrow {
    let start = matches!(arrow, Arrow::Start | Arrow::Both) && !from_rail;
    let end = matches!(arrow, Arrow::End | Arrow::Both) && !to_rail;
    match (start, end) {
        (false, false) => Arrow::None,
        (false, true) => Arrow::End,
        (true, false) => Arrow::Start,
        (true, true) => Arrow::Both,
    }
}

/// The drawn width of a rail's bar, centred in its strip.
pub const RAIL_BAR_PX: f32 = 5.0;
/// A link attaches to a rail at least this far inside the bar's ends.
pub const RAIL_END_CLEARANCE_PX: f32 = 12.0;
/// The radius of the dot a link draws where it meets a rail.
pub const RAIL_DOT_RADIUS_PX: f32 = 4.5;

/// The bar of a rail: the middle of its `Rail` strip, as wide as the drawn bar. None for
/// every node that is not a rail.
pub fn rail_bar(node: &NodeGeometry) -> Option<BoxRect> {
    if node.tag != NodeTag::Pipe || node.pipe_form != Some(stencil_model::PipeForm::Rail) {
        return None;
    }
    let strip = node.part(PartName::Rail)?.bounds;
    Some(BoxRect {
        x: strip.x + (strip.width - RAIL_BAR_PX) / 2.0,
        y: strip.y,
        width: RAIL_BAR_PX,
        height: strip.height,
    })
}

/// The dot centred on the rail's bar where a link ends on it.
fn rail_dot(point: PagePoint, bar: &BoxRect) -> Part {
    let center_x = bar.x + bar.width / 2.0;
    Part {
        name: PartName::RailDot,
        bounds: BoxRect {
            x: center_x - RAIL_DOT_RADIUS_PX,
            y: point.y - RAIL_DOT_RADIUS_PX,
            width: 2.0 * RAIL_DOT_RADIUS_PX,
            height: 2.0 * RAIL_DOT_RADIUS_PX,
        },
        text: None,
    }
}

/// The corner radius and bend of a link's line: the grammar's line kind first, then the
/// page, then the defaults.
fn link_bend(page: &Page, grammar: &Grammar, line: Line) -> (f32, Bend) {
    let kind = grammar.line(line);
    let corner = kind
        .and_then(|kind| kind.corner)
        .or(page.corner)
        .unwrap_or(LINK_CORNER_DEFAULT_PX);
    let bend = kind
        .and_then(|kind| kind.bend)
        .or(page.bend)
        .unwrap_or_default();
    (corner, bend)
}

/// The A* route of section 11.2, or the fallback L when there is none. An arrowed end of a
/// flat link is reached through the arrow stub of section 11.2, and under iso an end on a
/// block through an approach stub (section 12.4).
fn searched_route(
    geometry: &PageGeometry,
    link: &Link,
    (arrow, fractions): (Arrow, (Option<f32>, Option<f32>)),
    from: (usize, &NodeGeometry),
    to: (usize, &NodeGeometry),
    iso: bool,
) -> (Vec<PagePoint>, RouteStatus) {
    // A valve needs a straight run before the end it guards: the valve, its stand-back
    // and a bend's keep-out (section 14.4 rule 23).
    let (mut from_stub, mut to_stub) = stubs(arrow, (from.1, to.1), iso);
    let mut stub_px = (LINK_ARROW_STUB_PX, LINK_ARROW_STUB_PX);
    if link.valve && !iso {
        let run = crate::sheet_lines::VALVE_RUN_PX;
        match link.arrow {
            Arrow::Start => {
                from_stub = true;
                stub_px.0 = run;
            }
            Arrow::End | Arrow::Both | Arrow::None => {
                to_stub = true;
                stub_px.1 = run;
            }
        }
    }
    let sides = choose_sides(
        link,
        (&attach_box(from.1), allowed_sides(from.1)),
        (&attach_box(to.1), allowed_sides(to.1)),
    );
    route_between(
        geometry,
        from,
        to,
        sides,
        &link.via,
        RouteEnds {
            stubs: (from_stub, to_stub),
            fractions,
            stub_px,
        },
        iso,
    )
}

/// How a route meets its two ends: whether each is reached through a straight stub, how
/// long that stub is, and where along its side an end spread among others sits.
#[derive(Debug, Clone, Copy)]
struct RouteEnds {
    stubs: (bool, bool),
    fractions: (Option<f32>, Option<f32>),
    stub_px: (f32, f32),
}

/// Which ends of a link are reached through a stub: the arrowed ends of a flat link, the
/// block ends under iso.
fn stubs(arrow: Arrow, (from, to): (&NodeGeometry, &NodeGeometry), iso: bool) -> (bool, bool) {
    if iso {
        (is_block(from), is_block(to))
    } else {
        (
            matches!(arrow, Arrow::Start | Arrow::Both),
            matches!(arrow, Arrow::End | Arrow::Both),
        )
    }
}

/// The box a link attaches to: an item's Footprint part under iso when its shape is not a
/// card, where its solid stands (section 12.3), else the node's border box.
pub(crate) fn attach_box(node: &NodeGeometry) -> BoxRect {
    if let Some(bar) = rail_bar(node) {
        return bar;
    }
    // A stacked Item's links meet its front card, not the cards behind it.
    if node.tag == NodeTag::Pcard
        && let Some(front) = node.part(PartName::Front)
    {
        return front.bounds;
    }
    node.part(PartName::Footprint)
        .map_or(node.bounds, |part| part.bounds)
}

/// The sides a link may attach to when the author chose none: every side, except that an
/// item standing on a footprint keeps its bottom side, where its text lies on the floor.
fn allowed_sides(node: &NodeGeometry) -> &'static [Side] {
    if rail_bar(node).is_some() {
        return &RAIL_SIDES;
    }
    if node.part(PartName::Footprint).is_some() {
        &FOOTPRINT_SIDES
    } else {
        &SIDE_ORDER
    }
}

/// The attach sides of a footprint item, in SIDE_ORDER without the bottom side.
const FOOTPRINT_SIDES: [Side; 3] = [Side::Right, Side::Left, Side::Top];
/// A link meets a rail on its long sides or at either end of its bar.
const RAIL_SIDES: [Side; 4] = [Side::Right, Side::Left, Side::Bottom, Side::Top];

/// Where a link attaches on `side` of `node`: the side's midpoint, or on a rail the point of
/// the side level with `toward`, the middle of the other end, clamped
/// `RAIL_END_CLEARANCE_PX` inside the bar's ends.
fn attach_point(
    node: &NodeGeometry,
    side: Side,
    toward: &BoxRect,
    fraction: Option<f32>,
) -> PagePoint {
    let bounds = attach_box(node);
    let midpoint = match fraction {
        Some(fraction) => side_point(&bounds, side, fraction),
        None => side_midpoint(&bounds, side),
    };
    if node.outline == Some(stencil_model::Outline::Store) {
        return on_store_outline(&bounds, side, midpoint);
    }
    if rail_bar(node).is_none() || !matches!(side, Side::Left | Side::Right) {
        return midpoint;
    }
    let low = bounds.y + RAIL_END_CLEARANCE_PX;
    // The rail's tag stands left of its bar's foot: a link from the left stays above it.
    let tag_top = match side {
        Side::Left => node.part(PartName::Tag).map(|tag| tag.bounds.y),
        Side::Top | Side::Right | Side::Bottom => None,
    };
    let high = (bounds.bottom() - RAIL_END_CLEARANCE_PX)
        .min(tag_top.map_or(f32::INFINITY, |top| top - RAIL_END_CLEARANCE_PX))
        .max(low);
    PagePoint {
        x: midpoint.x,
        y: (toward.y + toward.height / 2.0).clamp(low, high),
    }
}

/// The height of a block under iso (section 12.3): an item's shape height, the block
/// height for every other leaf.
fn block_height(node: &NodeGeometry) -> f32 {
    match node.tag {
        NodeTag::Pcard => shape_height_px(node.shape.unwrap_or_default()),
        _ => ISO_BLOCK_HEIGHT_PX,
    }
}

/// A node drawn as a raised block under iso (section 12.3): a leaf other than a pipe or
/// tee.
fn is_block(node: &NodeGeometry) -> bool {
    matches!(
        node.tag,
        NodeTag::Pcard
            | NodeTag::Fact
            | NodeTag::Note
            | NodeTag::Text
            | NodeTag::Callout
            | NodeTag::Frame
    )
}

/// The section 11.2 route from the `from_side` midpoint of `from` through `via` to the
/// `to_side` midpoint of `to`, or the fallback L when the grid has none or a flat arrow
/// stub is blocked.
fn route_between(
    geometry: &PageGeometry,
    (from_node, from): (usize, &NodeGeometry),
    (to_node, to): (usize, &NodeGeometry),
    (from_side, to_side): (Side, Side),
    via: &[PagePoint],
    ends: RouteEnds,
    iso: bool,
) -> (Vec<PagePoint>, RouteStatus) {
    let RouteEnds {
        stubs: (from_stub, to_stub),
        fractions,
        stub_px,
    } = ends;
    let (from_box, to_box) = (attach_box(from), attach_box(to));
    let mut start = attach_point(from, from_side, &to_box, fractions.0);
    let mut end = attach_point(to, to_side, &from_box, fractions.1);
    // Facing ends that share a span line up, so the link runs straight rather than jogging
    // by the difference of their midpoints; an end spread along its side keeps its place.
    if to_side == facing_side(from_side) && !iso {
        match (fractions.0, fractions.1) {
            (Some(_), None) => {
                if let Some(aligned) = aligned_on_side(to, &to_box, to_side, start) {
                    end = aligned;
                }
            }
            (None, Some(_)) => {
                if let Some(aligned) = aligned_on_side(from, &from_box, from_side, end) {
                    start = aligned;
                }
            }
            (None, None) | (Some(_), Some(_)) => {}
        }
    }
    let mut obstacles = link_obstacles(geometry, from_node, to_node);
    if iso {
        obstacles.extend(block_shadows(geometry, &obstacles));
    }
    // Under iso a block end is reached through a straight stub outward from its side
    // (section 12.4): the search runs between the stub ends.
    // A stub into a hidden side (top or left) is cut back by the block's height on
    // screen, so it grows by that much. A stub never reaches past the nearest obstacle
    // ahead of it, so the search starts outside every box, and never past half the free
    // run to the other end's box, so two facing stubs never overrun each other.
    let outward = |point: PagePoint, side: Side, (node, other): (&NodeGeometry, BoxRect)| {
        let (dx, dy) = Direction::outward(side).unit();
        let hidden = matches!(side, Side::Top | Side::Left);
        let nominal = ISO_APPROACH_PX + if hidden { block_height(node) } else { 0.0 };
        let facing = [Obstacle {
            node: usize::MAX,
            bounds: other,
        }];
        let length = nominal
            .min(free_run(point, (dx, dy), &obstacles))
            .min(free_run(point, (dx, dy), &facing) / 2.0);
        PagePoint {
            x: point.x + dx * length,
            y: point.y + dy * length,
        }
    };
    // A flat arrow stub has its full length or no route at all: a shorter one is the leg
    // that ran into the arrowhead's flank.
    let arrow_stub = |point: PagePoint, side: Side, length: f32| {
        let (dx, dy) = Direction::outward(side).unit();
        PagePoint {
            x: point.x + dx * length,
            y: point.y + dy * length,
        }
    };
    let search_start = match (from_stub, iso) {
        (false, _) => start,
        (true, true) => outward(start, from_side, (from, to_box)),
        (true, false) => arrow_stub(start, from_side, stub_px.0),
    };
    let search_end = match (to_stub, iso) {
        (false, _) => end,
        (true, true) => outward(end, to_side, (to, from_box)),
        (true, false) => arrow_stub(end, to_side, stub_px.1),
    };
    if !iso {
        let blocked = |stub: bool, attach: PagePoint, stub_point: PagePoint, other: &BoxRect| {
            stub && (segment_enters(attach, stub_point, other)
                || obstacles
                    .iter()
                    .any(|obstacle| segment_enters(attach, stub_point, &obstacle.bounds)))
        };
        let (normal_x, normal_y) = Direction::outward(from_side).unit();
        let passing = from_stub
            && to_stub
            && to_side == facing_side(from_side)
            && (search_end.x - search_start.x) * normal_x
                + (search_end.y - search_start.y) * normal_y
                < -GEOMETRY_EPSILON_PX;
        if passing
            || blocked(from_stub, start, search_start, &to_box)
            || blocked(to_stub, end, search_end, &from_box)
        {
            return (fallback_route(start, end), RouteStatus::Fallback);
        }
    }
    // The search runs between the stub ends, outside the blocks, so a stubbed block is an
    // obstacle like any other: the route may not cut through it to reach its stub.
    for (stub, node, bounds) in [(from_stub, from_node, from_box), (to_stub, to_node, to_box)] {
        if stub {
            obstacles.push(Obstacle { node, bounds });
        }
    }
    let endpoints = Endpoints {
        from: from_box,
        to: to_box,
        start: search_start,
        end: search_end,
        from_side,
        arrival: match (to_stub, iso) {
            (false, _) => Arrival::Moving(Direction::outward(to_side).opposite()),
            (true, true) => Arrival::Any,
            (true, false) => Arrival::NotMoving(Direction::outward(to_side)),
        },
    };
    let with_stubs = |mut points: Vec<PagePoint>| {
        if from_stub {
            points.insert(0, start);
        }
        if to_stub {
            points.push(end);
        }
        corner_points(points)
    };
    match grid_route(&obstacles, &endpoints, via) {
        Some(points)
            if points.len() + usize::from(from_stub) + usize::from(to_stub)
                <= LINK_SEGMENTS_MAX + 1 =>
        {
            (with_stubs(points), RouteStatus::Routed)
        }
        Some(_) | None if iso => (
            with_stubs(fallback_route(search_start, search_end)),
            RouteStatus::Fallback,
        ),
        Some(_) | None => (fallback_route(start, end), RouteStatus::Fallback),
    }
}

/// The distance from `point` along the unit axis direction `direction` to the near edge of
/// the first obstacle whose span across that direction holds the point; unbounded when
/// none lies ahead.
fn free_run(point: PagePoint, direction: (f32, f32), obstacles: &[Obstacle]) -> f32 {
    obstacles
        .iter()
        .filter_map(|obstacle| {
            let bounds = obstacle.bounds;
            let (ahead, spans) = if direction.0 != 0.0 {
                let edge = if direction.0 > 0.0 {
                    bounds.x
                } else {
                    bounds.right()
                };
                (
                    (edge - point.x) * direction.0,
                    point.y >= bounds.y && point.y <= bounds.bottom(),
                )
            } else {
                let edge = if direction.1 > 0.0 {
                    bounds.y
                } else {
                    bounds.bottom()
                };
                (
                    (edge - point.y) * direction.1,
                    point.x >= bounds.x && point.x <= bounds.right(),
                )
            };
            (spans && ahead >= 0.0).then_some(ahead)
        })
        .fold(f32::INFINITY, f32::min)
}

/// Section 13.11 rule 3: the route of the link at `index` in `geometry.links`, searched again
/// with `from_side` and `via` in place of the authored ones; an end keeps its arrow stub, or
/// under iso its approach stub. The to end keeps the side its
/// routed polyline ends on, and the tag keeps its size and moves with the midpoint of the
/// longest segment. Pure: the geometry is read, never changed. None when no routed link has
/// that index or an endpoint is not a geometry node.
pub fn reroute_link(
    geometry: &PageGeometry,
    index: usize,
    from_side: Side,
    via: &[PagePoint],
    iso: bool,
) -> Option<LinkRoute> {
    let route = geometry.links.iter().find(|route| route.index == index)?;
    let from = geometry.nodes.get(route.from_node)?;
    let to = geometry.nodes.get(route.to_node)?;
    let to_side = route
        .points
        .last()
        .and_then(|end| side_holding(&attach_box(to), *end))
        .unwrap_or_else(|| facing_side(from_side));
    let (points, status) = route_between(
        geometry,
        (route.from_node, from),
        (route.to_node, to),
        (from_side, to_side),
        via,
        RouteEnds {
            stubs: stubs(route.arrow, (from, to), iso),
            fractions: (None, None),
            stub_px: (LINK_ARROW_STUB_PX, LINK_ARROW_STUB_PX),
        },
        iso,
    );
    let (old_center, _) = longest_segment_midpoint(&route.points);
    let (new_center, _) = longest_segment_midpoint(&points);
    let (delta_x, delta_y) = (new_center.x - old_center.x, new_center.y - old_center.y);
    let moved = |bounds: BoxRect| BoxRect {
        x: bounds.x + delta_x,
        y: bounds.y + delta_y,
        ..bounds
    };
    Some(LinkRoute {
        points,
        tag: route.tag.map(moved),
        parts: route
            .parts
            .iter()
            .map(|part| Part {
                bounds: moved(part.bounds),
                ..part.clone()
            })
            .collect(),
        status,
        ..route.clone()
    })
}

/// The first side, in SIDE_ORDER, whose edge line holds `point` within the geometry epsilon
/// and whose span covers it.
fn side_holding(bounds: &BoxRect, point: PagePoint) -> Option<Side> {
    let near = |a: f32, b: f32| (a - b).abs() <= GEOMETRY_EPSILON_PX;
    let within_x = point.x >= bounds.x - GEOMETRY_EPSILON_PX
        && point.x <= bounds.right() + GEOMETRY_EPSILON_PX;
    let within_y = point.y >= bounds.y - GEOMETRY_EPSILON_PX
        && point.y <= bounds.bottom() + GEOMETRY_EPSILON_PX;
    SIDE_ORDER.into_iter().find(|side| match side {
        Side::Right => near(point.x, bounds.right()) && within_y,
        Side::Bottom => near(point.y, bounds.bottom()) && within_x,
        Side::Left => near(point.x, bounds.x) && within_y,
        Side::Top => near(point.y, bounds.y) && within_x,
    })
}

/// A message: two points, from the center x of its `from` head to the center x of its `to`
/// head, at the middle of its row in the band of its Lanes node.
fn message_points(
    geometry: &PageGeometry,
    row: &MessageRow,
    from: &BoxRect,
    to: &BoxRect,
    link_pointer: &NodePointer,
) -> Result<Vec<PagePoint>, LayoutError> {
    let band = geometry
        .node(&row.lanes)
        .and_then(|lanes| lanes.part(PartName::Band))
        .ok_or_else(|| LayoutError::Taffy {
            pointer: link_pointer.clone(),
            message: format!("message names Lanes {} that has no band", row.lanes),
        })?;
    let y = row.line_y(band.bounds.y);
    Ok(vec![
        PagePoint {
            x: from.x + from.width / 2.0,
            y,
        },
        PagePoint {
            x: to.x + to.width / 2.0,
            y,
        },
    ])
}

/// Geometry index of every body node that carries an id.
fn nodes_by_id<'a>(page: &'a Page, geometry: &PageGeometry) -> HashMap<&'a str, usize> {
    let index_by_pointer: HashMap<&str, usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.pointer.as_str(), index))
        .collect();
    let mut by_id = HashMap::new();
    for entry in body_nodes(page) {
        let Some(id) = entry.node.id() else {
            continue;
        };
        if let Some(&index) = index_by_pointer.get(entry.pointer.as_str()) {
            by_id.insert(id, index);
        }
    }
    by_id
}

/// Vet guarantees every `from` and `to` names a node, so a miss here is reported as the
/// vet violation it would have been.
fn resolve(
    nodes_by_id: &HashMap<&str, usize>,
    id: &str,
    pointer: NodePointer,
) -> Result<usize, LayoutError> {
    nodes_by_id.get(id).copied().ok_or_else(|| {
        LayoutError::Invalid(vec![Violation {
            message: format!("id {id:?} names no node"),
            pointer,
            rule: VetRule::LinkUnknownId,
        }])
    })
}

/// The boxes a link between `from` and `to` must avoid: every leaf block (Pcard, Fact, Note,
/// Text, Callout, Frame), every Tee, the tag of every Pipe, the label of every Zone and the
/// page-level text (kicker, title, lede, legend entries, foot). The endpoints and every
/// node that contains one are left out. Order is geometry order.
pub(crate) fn link_obstacles(geometry: &PageGeometry, from: usize, to: usize) -> Vec<Obstacle> {
    let mut excluded = vec![false; geometry.nodes.len()];
    for endpoint in [from, to] {
        let mut current = Some(endpoint);
        for _ in 0..geometry.nodes.len() {
            let Some(index) = current else {
                break;
            };
            if let Some(flag) = excluded.get_mut(index) {
                *flag = true;
            }
            current = geometry.nodes.get(index).and_then(|node| node.parent);
        }
    }
    let mut obstacles: Vec<Obstacle> = geometry
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| !excluded.get(*index).copied().unwrap_or(true))
        .filter_map(|(index, node)| {
            obstacle_box(node).map(|bounds| Obstacle {
                node: index,
                bounds,
            })
        })
        .collect();
    // On a design sheet a Box that holds neither end is drawn work a line has no business
    // crossing: the whole outline is an obstacle (section 14.4 rule 15).
    let on_sheet = geometry
        .nodes
        .iter()
        .any(|node| node.tag == NodeTag::TitleBlock);
    if on_sheet {
        for (index, node) in geometry.nodes.iter().enumerate() {
            if node.tag == NodeTag::Zone && !excluded.get(index).copied().unwrap_or(true) {
                obstacles.push(Obstacle {
                    node: index,
                    bounds: node.bounds,
                });
            }
        }
    }
    // A View contains every link inside it, so its heading is never excluded with it.
    for (index, node) in geometry.nodes.iter().enumerate() {
        obstacles.extend(label_boxes(node).into_iter().map(|bounds| Obstacle {
            node: index,
            bounds,
        }));
    }
    // An endpoint standing on a footprint keeps its own floor text clear of its links.
    for index in [from, to] {
        let Some(node) = geometry.nodes.get(index) else {
            continue;
        };
        if node.part(PartName::Footprint).is_some()
            && let Some(text) = node.part(PartName::Text)
        {
            obstacles.push(Obstacle {
                node: index,
                bounds: text.bounds,
            });
        }
    }
    obstacles
}

/// Under iso a raised block covers the floor behind and left of its footprint by its
/// height on screen: the top face at z lies over the floor at (x - z, y - z). A route that
/// clears the footprint in flat px would run under that face, so each block obstacle casts
/// a second obstacle, its raised box grown by its height toward -x and -y (section 12.4).
pub(crate) fn block_shadows(geometry: &PageGeometry, obstacles: &[Obstacle]) -> Vec<Obstacle> {
    obstacles
        .iter()
        .filter_map(|obstacle| {
            let node = geometry.nodes.get(obstacle.node)?;
            if !is_block(node) {
                return None;
            }
            let raised = node
                .part(PartName::Footprint)
                .map_or(node.bounds, |part| part.bounds);
            if raised != obstacle.bounds && obstacle.bounds != node.bounds {
                // The endpoint's own floor text: flat, it casts nothing.
                return None;
            }
            let height = block_height(node);
            Some(Obstacle {
                node: obstacle.node,
                bounds: BoxRect {
                    x: raised.x - height,
                    y: raised.y - height,
                    width: raised.width + height,
                    height: raised.height + height,
                },
            })
        })
        .collect()
}

/// The heading parts of a node that contains the links it labels (section 14.4 rule 15):
/// for a View its number, title, subtitle and note; for a Phases node its lane names and
/// captions, its phase heads and titles and its detail tags; nothing for every other tag.
pub(crate) fn label_boxes(node: &NodeGeometry) -> Vec<BoxRect> {
    match node.tag {
        NodeTag::Phases => node
            .parts
            .iter()
            .filter(|part| {
                matches!(
                    part.name,
                    PartName::LaneName
                        | PartName::LaneText
                        | PartName::PhaseHead
                        | PartName::PhaseTitle
                        | PartName::DetailTag
                )
            })
            .map(|part| part.bounds)
            .collect(),
        NodeTag::View => node
            .parts
            .iter()
            .filter(|part| {
                matches!(
                    part.name,
                    PartName::ViewNumber
                        | PartName::ViewTitle
                        | PartName::ViewSubtitle
                        | PartName::ViewNote
                )
            })
            .map(|part| part.bounds)
            .collect(),
        // An assembly's tab names the links inside it, so it is an obstacle to them too.
        NodeTag::Zone => node
            .part(PartName::Tab)
            .map(|part| vec![part.bounds])
            .unwrap_or_default(),
        NodeTag::Page
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::LegendEntry
        | NodeTag::Foot
        | NodeTag::Row
        | NodeTag::Col
        | NodeTag::Lanes
        | NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Pipe
        | NodeTag::Tee
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame
        | NodeTag::TitleBlock
        | NodeTag::TitleCell
        | NodeTag::ProblemPoint
        | NodeTag::ActorChip
        | NodeTag::Step
        | NodeTag::PartsList
        | NodeTag::GeneralNotes
        | NodeTag::Ladder
        | NodeTag::Rung
        | NodeTag::HowTo
        | NodeTag::HowToItem => Vec::new(),
    }
}

fn obstacle_box(node: &NodeGeometry) -> Option<BoxRect> {
    match node.tag {
        NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame
        | NodeTag::Tee
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::LegendEntry
        | NodeTag::Foot
        | NodeTag::TitleCell
        | NodeTag::ActorChip
        | NodeTag::Step
        | NodeTag::PartsList
        | NodeTag::GeneralNotes
        | NodeTag::Ladder
        | NodeTag::HowTo => Some(node.bounds),
        // No link crosses a rail it does not attach to (section 14.4 rule 15).
        NodeTag::Pipe if rail_bar(node).is_some() => Some(node.bounds),
        NodeTag::Pipe => node.part(PartName::Tag).map(|part| part.bounds),
        NodeTag::Zone => node
            .part(PartName::Tab)
            .or_else(|| node.part(PartName::Label))
            .map(|part| part.bounds),
        // An enclosing node already blocks a title block's points and the block itself
        // holds nothing a link reaches; a View's heading is a label obstacle.
        NodeTag::Page
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::Row
        | NodeTag::Col
        | NodeTag::Lanes
        | NodeTag::TitleBlock
        | NodeTag::ProblemPoint
        | NodeTag::View
        | NodeTag::Phases
        | NodeTag::Rung
        | NodeTag::HowToItem => None,
    }
}

/// True when the axis-aligned segment from `a` to `b` reaches into the open interior of
/// `bounds` by more than the geometry epsilon. A segment along an edge does not enter.
pub(crate) fn segment_enters(a: PagePoint, b: PagePoint, bounds: &BoxRect) -> bool {
    let (x_low, x_high) = (a.x.min(b.x), a.x.max(b.x));
    let (y_low, y_high) = (a.y.min(b.y), a.y.max(b.y));
    x_high > bounds.x + GEOMETRY_EPSILON_PX
        && x_low < bounds.right() - GEOMETRY_EPSILON_PX
        && y_high > bounds.y + GEOMETRY_EPSILON_PX
        && y_low < bounds.bottom() - GEOMETRY_EPSILON_PX
}

fn side_midpoint(bounds: &BoxRect, side: Side) -> PagePoint {
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;
    match side {
        Side::Top => PagePoint {
            x: center_x,
            y: bounds.y,
        },
        Side::Right => PagePoint {
            x: bounds.right(),
            y: center_y,
        },
        Side::Bottom => PagePoint {
            x: center_x,
            y: bounds.bottom(),
        },
        Side::Left => PagePoint {
            x: bounds.x,
            y: center_y,
        },
    }
}

/// The point of `side` of `node` level with `other` across the side, when that point lies
/// inside the side's span by more than the default corner; None otherwise.
fn aligned_on_side(
    node: &NodeGeometry,
    bounds: &BoxRect,
    side: Side,
    other: PagePoint,
) -> Option<PagePoint> {
    if rail_bar(node).is_some() {
        return None;
    }
    let margin = LINK_CORNER_DEFAULT_PX + GEOMETRY_EPSILON_PX;
    let point = match side {
        Side::Top | Side::Bottom => {
            if other.x < bounds.x + margin || other.x > bounds.right() - margin {
                return None;
            }
            side_point(bounds, side, (other.x - bounds.x) / bounds.width)
        }
        Side::Left | Side::Right => {
            if other.y < bounds.y + margin || other.y > bounds.bottom() - margin {
                return None;
            }
            side_point(bounds, side, (other.y - bounds.y) / bounds.height)
        }
    };
    Some(if node.outline == Some(stencil_model::Outline::Store) {
        on_store_outline(bounds, side, point)
    } else {
        point
    })
}

/// The point `fraction` of the way along `side` of `bounds`, left to right or top to bottom.
fn side_point(bounds: &BoxRect, side: Side, fraction: f32) -> PagePoint {
    match side {
        Side::Top => PagePoint {
            x: bounds.x + bounds.width * fraction,
            y: bounds.y,
        },
        Side::Bottom => PagePoint {
            x: bounds.x + bounds.width * fraction,
            y: bounds.bottom(),
        },
        Side::Left => PagePoint {
            x: bounds.x,
            y: bounds.y + bounds.height * fraction,
        },
        Side::Right => PagePoint {
            x: bounds.right(),
            y: bounds.y + bounds.height * fraction,
        },
    }
}

/// A point on the top or bottom of a store moved onto its outline: the lid's ellipse on
/// top, the bottom arc below, so a line off the middle still lands on what is drawn.
fn on_store_outline(bounds: &BoxRect, side: Side, point: PagePoint) -> PagePoint {
    let ry = crate::STORE_LID_RY_PX;
    let half_width = bounds.width / 2.0;
    if half_width <= GEOMETRY_EPSILON_PX {
        return point;
    }
    let across = ((point.x - (bounds.x + half_width)) / half_width).clamp(-1.0, 1.0);
    let rise = ry * (1.0 - (1.0 - across * across).sqrt());
    match side {
        Side::Top => PagePoint {
            x: point.x,
            y: bounds.y + rise,
        },
        Side::Bottom => PagePoint {
            x: point.x,
            y: bounds.bottom() - rise,
        },
        Side::Left | Side::Right => point,
    }
}

/// On a design sheet, the ends that share a side of one node spread along it, in the order
/// of what they face, so no two meet at one point (section 14.4 rule 23 of the ends). Rail
/// ends attach level with what they face and are not spread. Returns, per link, the
/// fraction along its side of its from and to ends.
fn spread_ends(
    page: &Page,
    geometry: &PageGeometry,
    nodes_by_id: &HashMap<&str, usize>,
    lanes_plan: &LanesPlan,
) -> Vec<(Option<f32>, Option<f32>)> {
    let count = page.links.len().min(LINKS_MAX);
    let mut fractions = vec![(None, None); count];
    // (link, end is `to`, node, side, the other end's centre along the side).
    let mut ends: Vec<(usize, bool, usize, Side, f32)> = Vec::new();
    for (index, link) in page.links.iter().enumerate().take(count) {
        if lanes_plan.message(index).is_some() {
            continue;
        }
        let (Some(&from_node), Some(&to_node)) = (
            nodes_by_id.get(link.from.as_str()),
            nodes_by_id.get(link.to.as_str()),
        ) else {
            continue;
        };
        let (Some(from), Some(to)) = (geometry.nodes.get(from_node), geometry.nodes.get(to_node))
        else {
            continue;
        };
        let (from_box, to_box) = (attach_box(from), attach_box(to));
        let (from_side, to_side) = choose_sides(
            link,
            (&from_box, allowed_sides(from)),
            (&to_box, allowed_sides(to)),
        );
        let along = |side: Side, other: &BoxRect| match side {
            Side::Top | Side::Bottom => other.x + other.width / 2.0,
            Side::Left | Side::Right => other.y + other.height / 2.0,
        };
        if rail_bar(from).is_none() {
            ends.push((
                index,
                false,
                from_node,
                from_side,
                along(from_side, &to_box),
            ));
        }
        if rail_bar(to).is_none() {
            ends.push((index, true, to_node, to_side, along(to_side, &from_box)));
        }
    }
    // Each (node, side) with the (link index, whether the end is the link's to end, place
    // along the side) of every end that meets it.
    type SideEnds = ((usize, Side), Vec<(usize, bool, f32)>);
    let mut groups: Vec<SideEnds> = Vec::new();
    for (index, is_to, node, side, along) in ends {
        match groups.iter_mut().find(|(key, _)| *key == (node, side)) {
            Some((_, members)) => members.push((index, is_to, along)),
            None => groups.push(((node, side), vec![(index, is_to, along)])),
        }
    }
    for (_, mut members) in groups {
        if members.len() < 2 {
            continue;
        }
        members.sort_by(|first, second| first.2.total_cmp(&second.2).then(first.0.cmp(&second.0)));
        let slots = members.len() as f32 + 1.0;
        for (position, (index, is_to, _)) in members.into_iter().enumerate() {
            let fraction = (position as f32 + 1.0) / slots;
            if let Some(entry) = fractions.get_mut(index) {
                if is_to {
                    entry.1 = Some(fraction);
                } else {
                    entry.0 = Some(fraction);
                }
            }
        }
    }
    fractions
}

fn facing_side(side: Side) -> Side {
    match side {
        Side::Top => Side::Bottom,
        Side::Right => Side::Left,
        Side::Bottom => Side::Top,
        Side::Left => Side::Right,
    }
}

fn distance_squared(a: PagePoint, b: PagePoint) -> f32 {
    (a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)
}

/// The side of `bounds` among `allowed` whose midpoint is closest to `target`, ties in
/// `allowed` order.
fn closest_side(bounds: &BoxRect, target: PagePoint, allowed: &[Side]) -> Side {
    let mut best = allowed.first().copied().unwrap_or(Side::Right);
    let mut best_distance = f32::INFINITY;
    for &side in allowed {
        let distance = distance_squared(side_midpoint(bounds, side), target);
        if distance < best_distance {
            best = side;
            best_distance = distance;
        }
    }
    best
}

/// Section 11.2 attach sides. Both sides given: used as is. Neither given and no `via`: the
/// pair of facing sides whose midpoints are closest. Otherwise each open end takes the side
/// closest to what it faces: the first or last via point, or the other end's attach point.
fn choose_sides(
    link: &Link,
    (from, from_allowed): (&BoxRect, &[Side]),
    (to, to_allowed): (&BoxRect, &[Side]),
) -> (Side, Side) {
    match (link.from_side, link.to_side) {
        (Some(from_side), Some(to_side)) => (from_side, to_side),
        (Some(from_side), None) => {
            let target = link
                .via
                .last()
                .copied()
                .unwrap_or_else(|| side_midpoint(from, from_side));
            (from_side, closest_side(to, target, to_allowed))
        }
        (None, Some(to_side)) => {
            let target = link
                .via
                .first()
                .copied()
                .unwrap_or_else(|| side_midpoint(to, to_side));
            (closest_side(from, target, from_allowed), to_side)
        }
        (None, None) => match (link.via.first(), link.via.last()) {
            (Some(&first), Some(&last)) => (
                closest_side(from, first, from_allowed),
                closest_side(to, last, to_allowed),
            ),
            _ => facing_pair((from, from_allowed), (to, to_allowed)),
        },
    }
}

/// The pair of facing sides whose midpoints are closest, among the allowed sides of each
/// end; when no facing pair is allowed, the closest allowed pair.
fn facing_pair(
    (from, from_allowed): (&BoxRect, &[Side]),
    (to, to_allowed): (&BoxRect, &[Side]),
) -> (Side, Side) {
    let mut best: Option<(Side, Side)> = None;
    let mut best_distance = f32::INFINITY;
    for &side in from_allowed {
        let facing = facing_side(side);
        if !to_allowed.contains(&facing) {
            continue;
        }
        let distance = distance_squared(side_midpoint(from, side), side_midpoint(to, facing));
        if distance < best_distance {
            best = Some((side, facing));
            best_distance = distance;
        }
    }
    if let Some(pair) = best {
        return pair;
    }
    for &from_side in from_allowed {
        for &to_side in to_allowed {
            let distance =
                distance_squared(side_midpoint(from, from_side), side_midpoint(to, to_side));
            if distance < best_distance {
                best = Some((from_side, to_side));
                best_distance = distance;
            }
        }
    }
    best.unwrap_or((Side::Right, Side::Left))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Right,
    Down,
    Left,
    Up,
}

impl Direction {
    const ALL: [Direction; 4] = [
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Up,
    ];

    fn index(self) -> usize {
        match self {
            Direction::Right => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Up => 3,
        }
    }

    fn opposite(self) -> Direction {
        match self {
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
        }
    }

    /// The direction that leaves a box through `side`.
    fn outward(side: Side) -> Direction {
        match side {
            Side::Top => Direction::Up,
            Side::Right => Direction::Right,
            Side::Bottom => Direction::Down,
            Side::Left => Direction::Left,
        }
    }

    /// The unit step in page px along the direction.
    fn unit(self) -> (f32, f32) {
        match self {
            Direction::Right => (1.0, 0.0),
            Direction::Down => (0.0, 1.0),
            Direction::Left => (-1.0, 0.0),
            Direction::Up => (0.0, -1.0),
        }
    }
}

struct Endpoints {
    from: BoxRect,
    to: BoxRect,
    start: PagePoint,
    end: PagePoint,
    from_side: Side,
    /// How the last leg meets `end`: through the to side at a plain end, from any direction
    /// at an iso approach stub (section 12.4), which is the straight approach itself, and
    /// anyhow but back over the stub at a flat arrow stub (section 11.2).
    arrival: Arrival,
}

/// The moves a search may reach its goal with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrival {
    Any,
    Moving(Direction),
    NotMoving(Direction),
}

impl Arrival {
    fn admits(self, direction: Direction) -> bool {
        match self {
            Arrival::Any => true,
            Arrival::Moving(required) => direction == required,
            Arrival::NotMoving(refused) => direction != refused,
        }
    }
}

/// The routing grid: sorted line coordinates and which intersections and unit moves are
/// blocked by an obstacle interior. Every accessor returns blocked for an index outside the
/// grid.
struct Grid {
    xs: Vec<f32>,
    ys: Vec<f32>,
    node_blocked: Vec<bool>,
    /// Move from column i to i + 1 on row j, at `j * (xs.len() - 1) + i`.
    horizontal_blocked: Vec<bool>,
    /// Move from row j to j + 1 on column i, at `j * xs.len() + i`.
    vertical_blocked: Vec<bool>,
}

type GridPoint = (usize, usize);

impl Grid {
    /// None when either line set exceeds ROUTER_GRID_LINES_MAX.
    fn build(obstacles: &[Obstacle], endpoints: &Endpoints, via: &[PagePoint]) -> Option<Grid> {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let clearance_boxes = obstacles
            .iter()
            .map(|obstacle| obstacle.bounds)
            .chain([endpoints.from, endpoints.to]);
        for bounds in clearance_boxes {
            xs.push(bounds.x - OBSTACLE_CLEARANCE_PX);
            xs.push(bounds.right() + OBSTACLE_CLEARANCE_PX);
            ys.push(bounds.y - OBSTACLE_CLEARANCE_PX);
            ys.push(bounds.bottom() + OBSTACLE_CLEARANCE_PX);
        }
        for point in [endpoints.start, endpoints.end].iter().chain(via) {
            xs.push(point.x);
            ys.push(point.y);
        }
        let xs = sorted_lines(xs);
        let ys = sorted_lines(ys);
        if xs.len() > ROUTER_GRID_LINES_MAX || ys.len() > ROUTER_GRID_LINES_MAX {
            return None;
        }
        let columns = xs.len();
        let rows = ys.len();
        let mut grid = Grid {
            node_blocked: vec![false; columns * rows],
            horizontal_blocked: vec![false; columns.saturating_sub(1) * rows],
            vertical_blocked: vec![false; columns * rows.saturating_sub(1)],
            xs,
            ys,
        };
        for obstacle in obstacles {
            grid.block(&obstacle.bounds);
        }
        Some(grid)
    }

    /// Marks the intersections strictly inside `bounds` and every unit move that enters it.
    fn block(&mut self, bounds: &BoxRect) {
        let columns = self.xs.len();
        let rows = self.ys.len();
        let inside_low = |lines: &[f32], low: f32| {
            lines.partition_point(|&line| line <= low + GEOMETRY_EPSILON_PX)
        };
        let inside_high = |lines: &[f32], high: f32| {
            lines.partition_point(|&line| line < high - GEOMETRY_EPSILON_PX)
        };
        let column_low = inside_low(&self.xs, bounds.x);
        let column_high = inside_high(&self.xs, bounds.right());
        let row_low = inside_low(&self.ys, bounds.y);
        let row_high = inside_high(&self.ys, bounds.bottom());
        for row in row_low..row_high {
            for column in column_low..column_high {
                set_flag(&mut self.node_blocked, row * columns + column);
            }
            for column in column_low.saturating_sub(1)..column_high.min(columns.saturating_sub(1)) {
                set_flag(
                    &mut self.horizontal_blocked,
                    row * columns.saturating_sub(1) + column,
                );
            }
        }
        for column in column_low..column_high {
            for row in row_low.saturating_sub(1)..row_high.min(rows.saturating_sub(1)) {
                set_flag(&mut self.vertical_blocked, row * columns + column);
            }
        }
    }

    fn locate(&self, point: PagePoint) -> Option<GridPoint> {
        let column = self
            .xs
            .iter()
            .position(|&x| (x - point.x).abs() <= GRID_MERGE_PX)?;
        let row = self
            .ys
            .iter()
            .position(|&y| (y - point.y).abs() <= GRID_MERGE_PX)?;
        Some((column, row))
    }

    fn point(&self, (column, row): GridPoint) -> Option<PagePoint> {
        Some(PagePoint {
            x: *self.xs.get(column)?,
            y: *self.ys.get(row)?,
        })
    }

    fn node_open(&self, (column, row): GridPoint) -> bool {
        column < self.xs.len()
            && row < self.ys.len()
            && !self
                .node_blocked
                .get(row * self.xs.len() + column)
                .copied()
                .unwrap_or(true)
    }

    /// The neighbor one line away in `direction` and the move's length, when the move stays
    /// out of every obstacle.
    fn step(&self, (column, row): GridPoint, direction: Direction) -> Option<(GridPoint, f64)> {
        let columns = self.xs.len();
        let (target, move_open) = match direction {
            Direction::Right => (
                (column + 1, row),
                self.horizontal_blocked
                    .get(row * columns.saturating_sub(1) + column),
            ),
            Direction::Left => {
                let left = column.checked_sub(1)?;
                (
                    (left, row),
                    self.horizontal_blocked
                        .get(row * columns.saturating_sub(1) + left),
                )
            }
            Direction::Down => (
                (column, row + 1),
                self.vertical_blocked.get(row * columns + column),
            ),
            Direction::Up => {
                let up = row.checked_sub(1)?;
                (
                    (column, up),
                    self.vertical_blocked.get(up * columns + column),
                )
            }
        };
        if move_open.copied().unwrap_or(true) || !self.node_open(target) {
            return None;
        }
        let from = self.point((column, row))?;
        let to = self.point(target)?;
        let length = f64::from((to.x - from.x).abs() + (to.y - from.y).abs());
        Some((target, length))
    }
}

fn set_flag(flags: &mut [bool], index: usize) {
    if let Some(flag) = flags.get_mut(index) {
        *flag = true;
    }
}

fn sorted_lines(mut lines: Vec<f32>) -> Vec<f32> {
    lines.retain(|line| line.is_finite());
    lines.sort_by(f32::total_cmp);
    lines.dedup_by(|later, earlier| (*later - *earlier).abs() <= GRID_MERGE_PX);
    lines
}

/// One A* frontier entry. The heap pops the lowest estimate, then the lower x line, then the
/// lower y line, then the direction order Right, Down, Left, Up.
#[derive(Debug, Clone, Copy)]
struct Frontier {
    estimate: f64,
    cost: f64,
    point: GridPoint,
    direction: Direction,
}

impl PartialEq for Frontier {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Frontier {}

impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimate
            .total_cmp(&self.estimate)
            .then_with(|| other.point.0.cmp(&self.point.0))
            .then_with(|| other.point.1.cmp(&self.point.1))
            .then_with(|| other.direction.index().cmp(&self.direction.index()))
    }
}

/// Marks a state reached directly from the leg's start.
const FROM_START: usize = usize::MAX;

/// A* from `start` to `goal` with Manhattan moves, cost 1 per px plus TURN_COST_PX per
/// turn. `leave` forces the first move's direction and `arrive` admits the last one's.
/// Returns the grid points of the cheapest path, or None when no path exists.
fn search_leg(
    grid: &Grid,
    start: GridPoint,
    goal: GridPoint,
    leave: Option<Direction>,
    arrive: Arrival,
) -> Option<Vec<GridPoint>> {
    if !grid.node_open(start) || !grid.node_open(goal) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }
    let columns = grid.xs.len();
    let goal_point = grid.point(goal)?;
    let state_count = columns * grid.ys.len() * Direction::ALL.len();
    let state_of = |(column, row): GridPoint, direction: Direction| {
        (row * columns + column) * Direction::ALL.len() + direction.index()
    };
    let remaining = |point: GridPoint| -> f64 {
        grid.point(point).map_or(f64::INFINITY, |at| {
            f64::from((at.x - goal_point.x).abs() + (at.y - goal_point.y).abs())
        })
    };
    let mut best_cost = vec![f64::INFINITY; state_count];
    let mut parent = vec![FROM_START; state_count];
    let mut frontier = BinaryHeap::new();

    let first_moves: Vec<Direction> = match leave {
        Some(direction) => vec![direction],
        None => Direction::ALL.to_vec(),
    };
    for direction in first_moves {
        let Some((next, length)) = grid.step(start, direction) else {
            continue;
        };
        let state = state_of(next, direction);
        if let Some(slot) = best_cost.get_mut(state)
            && length < *slot
        {
            *slot = length;
            frontier.push(Frontier {
                estimate: length + remaining(next),
                cost: length,
                point: next,
                direction,
            });
        }
    }

    // Each state is expanded at most once and pushes at most three entries.
    let pops_max = state_count * 3 + Direction::ALL.len() + 1;
    for _ in 0..pops_max {
        let current = frontier.pop()?;
        let state = state_of(current.point, current.direction);
        if current.cost > best_cost.get(state).copied().unwrap_or(f64::NEG_INFINITY) {
            continue;
        }
        if current.point == goal && arrive.admits(current.direction) {
            return Some(reconstruct(
                start,
                current.point,
                state,
                &parent,
                columns,
                state_count,
            ));
        }
        for direction in Direction::ALL {
            if direction == current.direction.opposite() {
                continue;
            }
            let Some((next, length)) = grid.step(current.point, direction) else {
                continue;
            };
            let turn = if direction == current.direction {
                0.0
            } else {
                TURN_COST_PX
            };
            let cost = current.cost + length + turn;
            let next_state = state_of(next, direction);
            let Some(slot) = best_cost.get_mut(next_state) else {
                continue;
            };
            if cost < *slot {
                *slot = cost;
                if let Some(parent_slot) = parent.get_mut(next_state) {
                    *parent_slot = state;
                }
                frontier.push(Frontier {
                    estimate: cost + remaining(next),
                    cost,
                    point: next,
                    direction,
                });
            }
        }
    }
    None
}

fn reconstruct(
    start: GridPoint,
    goal: GridPoint,
    goal_state: usize,
    parent: &[usize],
    columns: usize,
    state_count: usize,
) -> Vec<GridPoint> {
    let point_of = |state: usize| {
        let node = state / Direction::ALL.len();
        (node % columns, node / columns)
    };
    let mut path = vec![goal];
    let mut state = goal_state;
    for _ in 0..state_count {
        let previous = parent.get(state).copied().unwrap_or(FROM_START);
        if previous == FROM_START {
            break;
        }
        path.push(point_of(previous));
        state = previous;
    }
    path.push(start);
    path.reverse();
    path
}

/// The routed polyline through every via point, simplified to its corners, or None when a
/// leg has no path or the grid is over the cap.
fn grid_route(
    obstacles: &[Obstacle],
    endpoints: &Endpoints,
    via: &[PagePoint],
) -> Option<Vec<PagePoint>> {
    let grid = Grid::build(obstacles, endpoints, via)?;
    let mut waypoints = vec![endpoints.start];
    waypoints.extend_from_slice(via);
    waypoints.push(endpoints.end);
    let leg_count = waypoints.len() - 1;
    let mut path: Vec<GridPoint> = Vec::new();
    for (leg_index, leg) in waypoints.windows(2).enumerate() {
        let (Some(&leg_start), Some(&leg_end)) = (leg.first(), leg.get(1)) else {
            return None;
        };
        // A from stub keeps the first leg leaving outward, so the route never doubles back
        // over its stub.
        let leave = (leg_index == 0).then(|| Direction::outward(endpoints.from_side));
        let arrive = if leg_index + 1 == leg_count {
            endpoints.arrival
        } else {
            Arrival::Any
        };
        let leg_path = search_leg(
            &grid,
            grid.locate(leg_start)?,
            grid.locate(leg_end)?,
            leave,
            arrive,
        )?;
        let skip = usize::from(!path.is_empty());
        path.extend(leg_path.into_iter().skip(skip));
    }
    let points: Option<Vec<PagePoint>> = path.into_iter().map(|point| grid.point(point)).collect();
    Some(corner_points(points?))
}

/// Drops repeated points and every point that lies straight between its neighbors, so only
/// the corners remain. A point where the route reverses along one line is a corner.
fn corner_points(points: Vec<PagePoint>) -> Vec<PagePoint> {
    let mut corners: Vec<PagePoint> = Vec::with_capacity(points.len());
    for point in points {
        if corners.last() == Some(&point) {
            continue;
        }
        let length = corners.len();
        if let (Some(&before), Some(&middle)) = (
            corners.get(length.wrapping_sub(2)),
            corners.get(length.wrapping_sub(1)),
        ) {
            let vertical_run = before.x == middle.x
                && middle.x == point.x
                && (middle.y - before.y) * (point.y - middle.y) >= 0.0;
            let horizontal_run = before.y == middle.y
                && middle.y == point.y
                && (middle.x - before.x) * (point.x - middle.x) >= 0.0;
            if vertical_run || horizontal_run {
                corners.pop();
            }
        }
        corners.push(point);
    }
    if let [only] = corners.as_slice() {
        let only = *only;
        corners.push(only);
    }
    corners
}

/// Section 11.2 fallback: an L from the from attach point to the to attach point,
/// horizontal leg first.
fn fallback_route(start: PagePoint, end: PagePoint) -> Vec<PagePoint> {
    corner_points(vec![
        start,
        PagePoint {
            x: end.x,
            y: start.y,
        },
        end,
    ])
}

/// The label and sub runs of a labeled link's tag, measured at max-content, and the tag
/// size they give (section 11.2): the content plus a pipe tag's padding and border.
pub(crate) struct MeasuredTag {
    label: Option<TextRun>,
    sub: Option<TextRun>,
    /// The step badge's numeral, at the left of the tag.
    step: Option<TextRun>,
    content_width: f32,
    pub width: f32,
    pub height: f32,
}

/// The measured tag of `link`, or None when it has neither a label nor a step badge and so
/// no tag. A step badge widens the tag by the badge and its gap (section 14.4 rule 23).
pub(crate) fn measure_tag(
    page: &Page,
    link: &Link,
    link_pointer: &NodePointer,
    measurer: &mut dyn TextMeasurer,
) -> Result<Option<MeasuredTag>, LayoutError> {
    let step_run = match link.step.filter(|_| page.sheet.is_some()) {
        Some(step) => Some(step_numeral_run(step, link_pointer, measurer)?),
        None => None,
    };
    let label = match (&link.label, &step_run) {
        (Some(label), _) => label.as_str(),
        (None, Some(_)) => "",
        (None, None) => return Ok(None),
    };
    let label_run = tag_run(
        page,
        link.line,
        label,
        TextStyleName::TagLabel,
        link_pointer.child("label"),
        measurer,
    )?;
    let sub_run = match &link.sub {
        Some(sub) => Some(tag_run(
            page,
            link.line,
            sub,
            TextStyleName::TagSub,
            link_pointer.child("sub"),
            measurer,
        )?),
        None => None,
    };
    let label_size = &label_run.metrics;
    let content_width = sub_run.as_ref().map_or(label_size.width_px, |sub| {
        label_size.width_px.max(sub.metrics.width_px)
    });
    // A tag of a badge alone keeps the height a one-line label gives it.
    let line_height = label_run.style.line_height_px;
    let content_height = label_size.height_px.max(line_height)
        + sub_run
            .as_ref()
            .map_or(0.0, |sub| TAG_SUB_GAP_PX + sub.metrics.height_px);
    let badge_width = match (&step_run, link.label.is_some()) {
        (Some(_), true) => crate::STEP_BADGE_PX + STEP_BADGE_GAP_PX,
        (Some(_), false) => crate::STEP_BADGE_PX,
        (None, _) => 0.0,
    };
    Ok(Some(MeasuredTag {
        label: link.label.is_some().then_some(label_run),
        sub: sub_run,
        step: step_run,
        content_width,
        width: content_width + badge_width + 2.0 * (TAG_PADDING_X_PX + TAG_BORDER_PX),
        height: content_height + 2.0 * (TAG_PADDING_Y_PX + TAG_BORDER_PX),
    }))
}

/// The gap between a step badge and the label after it.
const STEP_BADGE_GAP_PX: f32 = 6.0;

/// The numeral of a step badge, in `mark_numeral`.
fn step_numeral_run(
    step: u16,
    link_pointer: &NodePointer,
    measurer: &mut dyn TextMeasurer,
) -> Result<TextRun, LayoutError> {
    let style_name = crate::marks::numeral_style(stencil_model::MarkKind::Part);
    let style = text_style_for(style_name, false);
    let text = step.to_string();
    let metrics = measurer
        .measure(&text, &style, None)
        .map_err(|source_error| LayoutError::Measure {
            pointer: link_pointer.child("step"),
            source: source_error,
        })?;
    Ok(TextRun {
        text,
        style,
        color: text_color(style_name, None),
        align: TextAlign::Center,
        metrics,
        first_line_indent_px: 0.0,
    })
}

/// The tag of a labeled link: sized like a pipe tag from the label and sub measured at
/// max-content, centered on the midpoint of the longest segment (the first one on a tie).
/// A link's marks at the right end of its tag (section 14.4 rule 22): the tag widens by
/// the gap and the glyphs, to the right and left in equal halves so it stays on its leg,
/// and the glyphs stand in the widened end, centred on the tag's height.
fn add_tag_marks(
    page_marks: &stencil_model::PageMarks<'_>,
    (link, index): (&Link, usize),
    parts: &mut Vec<Part>,
    measurer: &mut dyn TextMeasurer,
) -> Result<(), LayoutError> {
    let marks = stencil_model::PageMarks::link_marks(link, index);
    let Some(tag) = parts.first().map(|part| part.bounds) else {
        return Ok(());
    };
    if marks.is_empty() {
        return Ok(());
    }
    let selected: Vec<&stencil_model::CarrierMark> = marks.iter().collect();
    let glyphs = crate::marks::numbered(page_marks, &selected);
    let extra = crate::marks::MARK_GAP_PX + crate::marks::glyphs_width(&glyphs);
    for part in parts.iter_mut() {
        part.bounds.x -= extra / 2.0;
    }
    if let Some(tag_part) = parts.first_mut() {
        tag_part.bounds.width += extra;
    }
    let mut x = tag.x - extra / 2.0 + tag.width - TAG_BORDER_PX - TAG_PADDING_X_PX
        + crate::marks::MARK_GAP_PX;
    let source = NodePointer::root()
        .child("links")
        .index(index)
        .child("marks");
    for glyph in glyphs {
        let (width, height) = crate::marks::glyph_size(glyph.0, glyph.1);
        let y = tag.y + (tag.height - height) / 2.0;
        let placed = crate::marks::placed_glyph(
            (crate::marks::glyph_part(glyph.0), PartName::MarkNumeral),
            glyph,
            (x, y),
            measurer,
            &source,
        )?;
        parts.extend(placed);
        x += width + crate::marks::MARK_GAP_PX;
    }
    Ok(())
}

/// A link's marks when it has no tag: centred under its valve, `MARK_GAP_PX` below it
/// (section 14.4 rule 22).
fn add_valve_marks(
    page_marks: &stencil_model::PageMarks<'_>,
    (link, index): (&Link, usize),
    valve: BoxRect,
    parts: &mut Vec<Part>,
    measurer: &mut dyn TextMeasurer,
) -> Result<(), LayoutError> {
    let marks = stencil_model::PageMarks::link_marks(link, index);
    if marks.is_empty() {
        return Ok(());
    }
    let selected: Vec<&stencil_model::CarrierMark> = marks.iter().collect();
    let glyphs = crate::marks::numbered(page_marks, &selected);
    let width = crate::marks::glyphs_width(&glyphs);
    let mut x = valve.x + valve.width / 2.0 - width / 2.0;
    let top = valve.bottom() + crate::marks::MARK_GAP_PX;
    let source = NodePointer::root()
        .child("links")
        .index(index)
        .child("marks");
    for glyph in glyphs {
        let (glyph_width, _) = crate::marks::glyph_size(glyph.0, glyph.1);
        let placed = crate::marks::placed_glyph(
            (crate::marks::glyph_part(glyph.0), PartName::MarkNumeral),
            glyph,
            (x, top),
            measurer,
            &source,
        )?;
        parts.extend(placed);
        x += glyph_width + crate::marks::MARK_GAP_PX;
    }
    Ok(())
}

fn tag_parts(
    page: &Page,
    link: &Link,
    link_pointer: &NodePointer,
    points: &[PagePoint],
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<Part>, LayoutError> {
    let Some(measured) = measure_tag(page, link, link_pointer, measurer)? else {
        return Ok(Vec::new());
    };
    let MeasuredTag {
        label: label_run,
        sub: sub_run,
        step: step_run,
        content_width,
        width: tag_width,
        height: tag_height,
    } = measured;
    let (center, along_y) = longest_segment_midpoint(points);
    let tag = BoxRect {
        x: center.x - tag_width / 2.0,
        y: center.y - tag_height / 2.0,
        width: tag_width,
        height: tag_height,
    };
    let mut content_x = tag.x + TAG_BORDER_PX + TAG_PADDING_X_PX;
    let content_y = tag.y + TAG_BORDER_PX + TAG_PADDING_Y_PX;
    let mut parts = vec![Part {
        name: PartName::Tag,
        bounds: tag,
        text: None,
    }];
    if let Some(step_run) = step_run {
        let badge = BoxRect {
            x: content_x,
            y: tag.y + (tag.height - crate::STEP_BADGE_PX) / 2.0,
            width: crate::STEP_BADGE_PX,
            height: crate::STEP_BADGE_PX,
        };
        let metrics = &step_run.metrics;
        let numeral = BoxRect {
            x: badge.x + (badge.width - metrics.width_px) / 2.0,
            y: badge.y + (badge.height - metrics.height_px) / 2.0,
            width: metrics.width_px,
            height: metrics.height_px,
        };
        parts.push(Part {
            name: PartName::StepBadge,
            bounds: badge,
            text: None,
        });
        parts.push(Part {
            name: PartName::StepNumeral,
            bounds: numeral,
            text: Some(step_run),
        });
        content_x += crate::STEP_BADGE_PX + STEP_BADGE_GAP_PX;
    }
    let centered = |metrics: &TextMetrics, y: f32| BoxRect {
        x: content_x + (content_width - metrics.width_px) / 2.0,
        y,
        width: metrics.width_px,
        height: metrics.height_px,
    };
    let Some(label_run) = label_run else {
        return Ok(parts);
    };
    let sub_y = content_y + label_run.metrics.height_px + TAG_SUB_GAP_PX;
    parts.push(Part {
        name: PartName::TagLabel,
        bounds: centered(&label_run.metrics, content_y),
        text: Some(label_run),
    });
    if let Some(sub_run) = sub_run {
        parts.push(Part {
            name: PartName::TagSub,
            bounds: centered(&sub_run.metrics, sub_y),
            text: Some(sub_run),
        });
    }
    // Under iso a tag on a leg along flat y reads along y (section 12.4): every part box is
    // the strip its turned drawing covers.
    let along_y = match link.axis {
        Some(Axis::Y) => true,
        Some(Axis::X) => false,
        None => along_y,
    };
    if page.projection == Projection::Iso && along_y {
        let pivot = (center.x, center.y);
        for part in &mut parts {
            part.bounds = turned_box(part.bounds, pivot);
        }
    }
    Ok(parts)
}

fn tag_run(
    page: &Page,
    line: Line,
    text: &str,
    style_name: TextStyleName,
    source: NodePointer,
    measurer: &mut dyn TextMeasurer,
) -> Result<TextRun, LayoutError> {
    let style = text_style_for(style_name, page.projection == Projection::Iso);
    let metrics = measurer
        .measure(text, &style, None)
        .map_err(|source_error| LayoutError::Measure {
            pointer: source,
            source: source_error,
        })?;
    Ok(TextRun {
        text: text.to_string(),
        style,
        color: text_color(style_name, Some(line)),
        align: TextAlign::Center,
        metrics,
        first_line_indent_px: 0.0,
    })
}

/// The midpoint of the longest segment (the first on a tie) and whether that segment runs
/// along flat y.
pub fn longest_segment_midpoint(points: &[PagePoint]) -> (PagePoint, bool) {
    longest_segment_point(points, 0.5)
}

/// The point `fraction` of the way along the longest segment (the first on a tie), from
/// its start, and whether that segment runs along flat y. Links in one bundle take their
/// tags at different fractions so the pills do not stack (section 12.3 rule 8).
pub fn longest_segment_point(points: &[PagePoint], fraction: f32) -> (PagePoint, bool) {
    debug_assert!(
        (0.0..=1.0).contains(&fraction),
        "fraction {fraction} is not in 0 to 1"
    );
    let mut best: Option<(f32, PagePoint, bool)> = None;
    for segment in points.windows(2) {
        let (Some(a), Some(b)) = (segment.first(), segment.get(1)) else {
            continue;
        };
        let length = (b.x - a.x).abs() + (b.y - a.y).abs();
        let midpoint = PagePoint {
            x: a.x + (b.x - a.x) * fraction,
            y: a.y + (b.y - a.y) * fraction,
        };
        let along_y = (b.y - a.y).abs() > (b.x - a.x).abs();
        if best.is_none_or(|(best_length, _, _)| length > best_length) {
            best = Some((length, midpoint, along_y));
        }
    }
    best.map_or_else(
        || {
            (
                points
                    .first()
                    .copied()
                    .unwrap_or(PagePoint { x: 0.0, y: 0.0 }),
                false,
            )
        },
        |(_, midpoint, along_y)| (midpoint, along_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> PagePoint {
        PagePoint { x, y }
    }

    #[test]
    fn corner_points_keep_only_turns() {
        let simplified = corner_points(vec![
            point(0.0, 0.0),
            point(5.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 5.0),
            point(10.0, 10.0),
        ]);
        assert_eq!(
            simplified,
            vec![point(0.0, 0.0), point(10.0, 0.0), point(10.0, 10.0)]
        );
    }

    #[test]
    fn corner_points_keep_a_reversal() {
        let simplified = corner_points(vec![point(0.0, 0.0), point(10.0, 0.0), point(4.0, 0.0)]);
        assert_eq!(simplified.len(), 3);
    }

    #[test]
    fn fallback_is_an_l_with_the_horizontal_leg_first() {
        assert_eq!(
            fallback_route(point(0.0, 0.0), point(30.0, 20.0)),
            vec![point(0.0, 0.0), point(30.0, 0.0), point(30.0, 20.0)]
        );
        assert_eq!(
            fallback_route(point(0.0, 5.0), point(30.0, 5.0)),
            vec![point(0.0, 5.0), point(30.0, 5.0)]
        );
    }

    #[test]
    fn a_segment_along_an_edge_does_not_enter() {
        let bounds = BoxRect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        assert!(!segment_enters(
            point(0.0, 10.0),
            point(40.0, 10.0),
            &bounds
        ));
        assert!(segment_enters(
            point(0.0, 10.02),
            point(40.0, 10.02),
            &bounds
        ));
        assert!(segment_enters(point(0.0, 20.0), point(40.0, 20.0), &bounds));
        assert!(!segment_enters(
            point(0.0, 20.0),
            point(10.0, 20.0),
            &bounds
        ));
    }

    #[test]
    fn facing_sides_prefer_right_on_a_tie() {
        let square = |x: f32, y: f32| BoxRect {
            x,
            y,
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(
            facing_pair(
                (&square(0.0, 0.0), &SIDE_ORDER),
                (&square(40.0, 0.0), &SIDE_ORDER)
            ),
            (Side::Right, Side::Left)
        );
        assert_eq!(
            facing_pair(
                (&square(0.0, 0.0), &SIDE_ORDER),
                (&square(0.0, 40.0), &SIDE_ORDER)
            ),
            (Side::Bottom, Side::Top)
        );
        assert_eq!(
            facing_pair(
                (&square(40.0, 40.0), &SIDE_ORDER),
                (&square(0.0, 0.0), &SIDE_ORDER)
            ),
            (Side::Left, Side::Right)
        );
    }
}
