//! What a design sheet adds to a routed link (section 14.4 rules 23 and 24): a valve on the
//! leg before its arrowhead, a tag kept clear of every bend, and a hop where it crosses a
//! link of another actor.

use stencil_model::{Arrow, Link, Page, PageActors, PagePoint};

use crate::drawn::{DrawnLink, PathPiece, drawn_link};
use crate::{BoxRect, GEOMETRY_EPSILON_PX, LinkRoute, NodeTag, PageGeometry, Part, PartName};

/// A valve's two triangles: each is this long along the leg and twice this wide across it.
pub const VALVE_LENGTH_PX: f32 = 8.0;
pub const VALVE_HALF_WIDTH_PX: f32 = 7.0;
/// The valve's centre stands this far back from where the line's end feature begins: the
/// arrowhead's base, or the outline of the dot on a rail.
pub const VALVE_BACK_PX: f32 = 26.0;
/// The straight run a link with a valve keeps before the end the valve guards: the end
/// feature, the valve's stand-back and its far triangle, and a bend's keep-out.
pub const VALVE_RUN_PX: f32 = crate::ARROWHEAD_LENGTH_PX
    + VALVE_BACK_PX
    + VALVE_LENGTH_PX
    + 6.0
    + DASH_PERIOD_MAX_PX
    + BEND_AIR_PX;
/// The longest dash period a sheet line draws (`7 4` of a planned line and `6 5` of a dash
/// line are both 11), and the air a tag or valve keeps past it.
pub const DASH_PERIOD_MAX_PX: f32 = 11.0;
pub const BEND_AIR_PX: f32 = 4.0;
/// A tag or valve keeps this much of its leg clear before a route end.
pub const ROUTE_END_CLEARANCE_PX: f32 = 12.0;
/// The air a tag keeps from a Box edge it is slid off, and from a valve.
const TAG_EDGE_CLEARANCE_PX: f32 = 6.0;
const VALVE_AIR_PX: f32 = 4.0;
/// A hop is a semicircle of this radius, centred on the crossing (section 14.4 rule 24).
pub const HOP_RADIUS_PX: f32 = 4.0;
/// A crossing nearer than this to a bend or an end of either leg draws no hop.
pub const HOP_END_CLEARANCE_PX: f32 = 6.0;
/// A crossing nearer than this to an end of either line draws no hop: the straight approach
/// stub and the hop's own reach.
pub const HOP_END_STUB_PX: f32 = crate::LINK_ARROW_STUB_PX + HOP_RADIUS_PX;

/// How far from a bend's corner a tag, badge or valve keeps to: the rounded bend, one dash
/// period so the bend's dash shows, and air (section 14.4 rule 23).
pub fn bend_keep_out(corner: f32) -> f32 {
    corner + DASH_PERIOD_MAX_PX + BEND_AIR_PX
}

/// The corners of a route: every point but the first and the last.
pub fn bends(points: &[PagePoint]) -> &[PagePoint] {
    match points.len() {
        0..=2 => &[],
        count => points.get(1..count - 1).unwrap_or(&[]),
    }
}

/// True when `bounds` grown by `keep_out` holds a corner of the route.
pub fn covers_a_bend(bounds: &BoxRect, points: &[PagePoint], keep_out: f32) -> bool {
    bends(points).iter().any(|corner| {
        corner.x > bounds.x - keep_out + GEOMETRY_EPSILON_PX
            && corner.x < bounds.right() + keep_out - GEOMETRY_EPSILON_PX
            && corner.y > bounds.y - keep_out + GEOMETRY_EPSILON_PX
            && corner.y < bounds.bottom() + keep_out - GEOMETRY_EPSILON_PX
    })
}

/// One straight leg of a route: its two points and whether each is a bend.
#[derive(Debug, Clone, Copy)]
struct Leg {
    start: PagePoint,
    end: PagePoint,
    start_bend: bool,
    end_bend: bool,
}

impl Leg {
    fn horizontal(&self) -> bool {
        (self.end.x - self.start.x).abs() >= (self.end.y - self.start.y).abs()
    }

    fn length(&self) -> f32 {
        (self.end.x - self.start.x).abs() + (self.end.y - self.start.y).abs()
    }

    /// The point `distance` along the leg from its start.
    fn at(&self, distance: f32) -> PagePoint {
        let length = self.length().max(GEOMETRY_EPSILON_PX);
        let fraction = distance / length;
        PagePoint {
            x: self.start.x + (self.end.x - self.start.x) * fraction,
            y: self.start.y + (self.end.y - self.start.y) * fraction,
        }
    }

    /// The distance along the leg from its start to the projection of `point`.
    fn offset_of(&self, point: PagePoint) -> f32 {
        if self.horizontal() {
            (point.x - self.start.x).abs()
        } else {
            (point.y - self.start.y).abs()
        }
    }
}

fn legs(points: &[PagePoint]) -> Vec<Leg> {
    let last = points.len().saturating_sub(1);
    points
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            Some(Leg {
                start: *pair.first()?,
                end: *pair.get(1)?,
                start_bend: index > 0,
                end_bend: index + 1 < last,
            })
        })
        .collect()
}

/// The clear run a leg keeps at each end: the bend keep-out at a bend, the route-end
/// clearance at an end.
fn leg_clearance(leg: &Leg, corner: f32) -> (f32, f32) {
    let clear = |bend: bool| {
        if bend {
            bend_keep_out(corner)
        } else {
            ROUTE_END_CLEARANCE_PX
        }
    };
    (clear(leg.start_bend), clear(leg.end_bend))
}

/// Where a valve stands (section 14.4 rule 23): on the leg before the arrowed end,
/// `VALVE_BACK_PX` back from where the end feature begins (`end_inset` from the end point),
/// moved toward the end when that would bring it within a bend's keep-out, and at the leg's
/// middle when the leg has no such room. Returns its centre and the leg's unit direction
/// toward the end. An authored `arrow` of start puts it on the first leg.
pub fn valve_center(
    points: &[PagePoint],
    arrow: Arrow,
    end_inset: f32,
    corner: f32,
) -> Option<(PagePoint, (f32, f32))> {
    let all = legs(points);
    let (leg, toward_end) = match arrow {
        Arrow::End | Arrow::Both => (*all.last()?, true),
        Arrow::Start => (*all.first()?, false),
        Arrow::None => return None,
    };
    let length = leg.length();
    if length <= GEOMETRY_EPSILON_PX {
        return None;
    }
    // Distances measured from the leg's end that faces the arrow.
    let (near_clear, far_clear) = {
        let (start_clear, end_clear) = leg_clearance(&leg, corner);
        if toward_end {
            (end_inset, if leg.start_bend { start_clear } else { 0.0 })
        } else {
            (end_inset, if leg.end_bend { end_clear } else { 0.0 })
        }
    };
    let nominal = near_clear + VALVE_BACK_PX;
    let farthest = length - far_clear - VALVE_LENGTH_PX;
    let nearest = near_clear + VALVE_LENGTH_PX;
    let back = if farthest >= nearest {
        nominal.clamp(nearest, farthest)
    } else {
        length / 2.0
    };
    let distance_from_start = if toward_end { length - back } else { back };
    let center = leg.at(distance_from_start);
    let unit = (
        (leg.end.x - leg.start.x) / length,
        (leg.end.y - leg.start.y) / length,
    );
    let direction = if toward_end { unit } else { (-unit.0, -unit.1) };
    Some((center, direction))
}

/// The two triangles of a valve centred at `center`, the nearer one toward the arrowed end
/// along `direction`.
pub fn valve_parts(center: PagePoint, direction: (f32, f32)) -> [Part; 2] {
    let horizontal = direction.0.abs() >= direction.1.abs();
    let half = |toward: f32| -> BoxRect {
        if horizontal {
            BoxRect {
                x: if toward >= 0.0 {
                    center.x
                } else {
                    center.x - VALVE_LENGTH_PX
                },
                y: center.y - VALVE_HALF_WIDTH_PX,
                width: VALVE_LENGTH_PX,
                height: 2.0 * VALVE_HALF_WIDTH_PX,
            }
        } else {
            BoxRect {
                x: center.x - VALVE_HALF_WIDTH_PX,
                y: if toward >= 0.0 {
                    center.y
                } else {
                    center.y - VALVE_LENGTH_PX
                },
                width: 2.0 * VALVE_HALF_WIDTH_PX,
                height: VALVE_LENGTH_PX,
            }
        }
    };
    let along = if horizontal { direction.0 } else { direction.1 };
    [
        Part {
            name: PartName::ValveNear,
            bounds: half(along),
            text: None,
        },
        Part {
            name: PartName::ValveFar,
            bounds: half(-along),
            text: None,
        },
    ]
}

/// The valve of a route's parts as one box, when it has one.
pub fn valve_box(parts: &[Part]) -> Option<BoxRect> {
    let near = parts.iter().find(|part| part.name == PartName::ValveNear)?;
    let far = parts.iter().find(|part| part.name == PartName::ValveFar)?;
    Some(union(&near.bounds, &far.bounds))
}

fn union(a: &BoxRect, b: &BoxRect) -> BoxRect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    BoxRect {
        x,
        y,
        width: a.right().max(b.right()) - x,
        height: a.bottom().max(b.bottom()) - y,
    }
}

/// True when the border of `zone` runs through the interior of `tag`, clear by the edge
/// clearance: the two overlap and the tag does not lie inside the zone.
fn near_edge(tag: &BoxRect, zone: &BoxRect) -> bool {
    let grown = BoxRect {
        x: tag.x - TAG_EDGE_CLEARANCE_PX,
        y: tag.y - TAG_EDGE_CLEARANCE_PX,
        width: tag.width + 2.0 * TAG_EDGE_CLEARANCE_PX,
        height: tag.height + 2.0 * TAG_EDGE_CLEARANCE_PX,
    };
    let overlaps = grown.x < zone.right()
        && zone.x < grown.right()
        && grown.y < zone.bottom()
        && zone.y < grown.bottom();
    let inside = grown.x >= zone.x
        && grown.right() <= zone.right()
        && grown.y >= zone.y
        && grown.bottom() <= zone.bottom();
    overlaps && !inside
}

/// The drawn outline of every Box: an assembly's outline under its tab, else its border box.
pub(crate) fn zone_outlines(geometry: &PageGeometry) -> Vec<BoxRect> {
    geometry
        .nodes
        .iter()
        .filter(|node| node.tag == NodeTag::Zone)
        .map(|node| {
            node.part(PartName::TabBorder)
                .map_or(node.bounds, |part| part.bounds)
        })
        .collect()
}

/// Places a sheet link's tag (section 14.4 rules 15 and 23): on the longest leg that holds
/// it with a clear run of `bend_keep_out` to each bend, `ROUTE_END_CLEARANCE_PX` to each
/// end and the valve's span plus air, at the point nearest the leg's middle where the tag
/// meets no box links-avoid-boxes reads (every node but the containers and the endpoints'
/// ancestors), no earlier link's tag or valve in `taken`, and no Box edge it lies inside;
/// legs are tried longest first. Every part of the tag moves with it. A tag no leg holds
/// stays where it is, and links-routed reports it.
pub(crate) fn place_sheet_tag(
    geometry: &PageGeometry,
    (points, from, to): (&[PagePoint], usize, usize),
    (corner, valve, taken): (f32, Option<BoxRect>, &[BoxRect]),
    parts: &mut [Part],
) -> bool {
    let Some(tag) = parts.first().map(|part| part.bounds) else {
        return true;
    };
    let zones = zone_outlines(geometry);
    let blocking = tag_obstacles(geometry, from, to);
    let mut order = legs(points);
    // Longest first; on a tie the earlier leg.
    order.sort_by(|first, second| second.length().total_cmp(&first.length()));
    let center = (tag.x + tag.width / 2.0, tag.y + tag.height / 2.0);
    for leg in order {
        let length = leg.length();
        let horizontal = leg.horizontal();
        let half = if horizontal {
            tag.width / 2.0
        } else {
            tag.height / 2.0
        };
        let (start_clear, end_clear) = leg_clearance(&leg, corner);
        let low = start_clear + half;
        let high = length - end_clear - half;
        if low > high + GEOMETRY_EPSILON_PX {
            continue;
        }
        let placed = |offset: f32| -> BoxRect {
            let at = leg.at(offset);
            BoxRect {
                x: at.x - tag.width / 2.0,
                y: at.y - tag.height / 2.0,
                ..tag
            }
        };
        let mut candidates = vec![length / 2.0, low, high];
        let mut edge_offsets = |bounds: &BoxRect, air: f32| {
            let edges = if horizontal {
                [bounds.x, bounds.right()]
            } else {
                [bounds.y, bounds.bottom()]
            };
            for edge in edges {
                let edge_offset = leg.offset_of(PagePoint { x: edge, y: edge });
                for side in [-1.0, 1.0] {
                    candidates.push(edge_offset + side * (half + air));
                }
            }
        };
        if let Some(valve) = valve {
            edge_offsets(&valve, VALVE_AIR_PX + GEOMETRY_EPSILON_PX);
        }
        for zone in &zones {
            edge_offsets(zone, 2.0 * TAG_EDGE_CLEARANCE_PX);
        }
        for bounds in blocking.iter().chain(taken) {
            edge_offsets(bounds, VALVE_AIR_PX + GEOMETRY_EPSILON_PX);
        }
        let clear = |offset: f32| -> bool {
            if offset < low - GEOMETRY_EPSILON_PX || offset > high + GEOMETRY_EPSILON_PX {
                return false;
            }
            let bounds = placed(offset);
            let aired = grow(&bounds, VALVE_AIR_PX);
            if valve.is_some_and(|valve| overlaps(&aired, &valve)) {
                return false;
            }
            if blocking
                .iter()
                .chain(taken)
                .any(|other| overlaps(&aired, other))
            {
                return false;
            }
            !zones.iter().any(|zone| near_edge(&bounds, zone))
        };
        let middle = length / 2.0;
        let best = candidates
            .into_iter()
            .filter(|offset| offset.is_finite() && clear(*offset))
            .min_by(|first, second| (first - middle).abs().total_cmp(&(second - middle).abs()));
        if let Some(offset) = best {
            let bounds = placed(offset);
            let delta = (
                bounds.x + bounds.width / 2.0 - center.0,
                bounds.y + bounds.height / 2.0 - center.1,
            );
            for part in parts.iter_mut() {
                part.bounds.x += delta.0;
                part.bounds.y += delta.1;
            }
            return true;
        }
    }
    false
}

/// The offsets a sheet tries, in order, to move a leg so a tag no leg holds fits beside it.
pub const TAG_ROOM_SHIFTS_PX: [f32; 32] = [
    -4.0, 4.0, -8.0, 8.0, -12.0, 12.0, -16.0, 16.0, -20.0, 20.0, -24.0, 24.0, -28.0, 28.0, -32.0,
    32.0, -36.0, 36.0, -40.0, 40.0, -44.0, 44.0, -48.0, 48.0, -52.0, 52.0, -56.0, 56.0, -60.0,
    60.0, -64.0, 64.0,
];

/// Every route of `points` with one leg moved by an offset of `TAG_ROOM_SHIFTS_PX` that
/// keeps it out of every box the link avoids and off every leg of `others`: longest legs
/// first, smaller offsets first.
pub(crate) fn shifted_routes(
    geometry: &PageGeometry,
    (from, to): (usize, usize),
    points: &[PagePoint],
    others: &[Vec<PagePoint>],
) -> Vec<Vec<PagePoint>> {
    let obstacles = crate::route::link_obstacles(geometry, from, to);
    let other_segments: Vec<(PagePoint, PagePoint)> =
        others.iter().flat_map(|other| segments(other)).collect();
    let mut order: Vec<(usize, f32)> = segments(points)
        .iter()
        .enumerate()
        .map(|(index, (a, b))| (index, (b.x - a.x).abs() + (b.y - a.y).abs()))
        .collect();
    order.sort_by(|first, second| second.1.total_cmp(&first.1));
    let mut routes = Vec::new();
    for (leg, _) in order {
        for shift in TAG_ROOM_SHIFTS_PX {
            let Some(candidate) = shifted(geometry, (from, to), points, leg, shift) else {
                continue;
            };
            let clear = segments(&candidate).iter().all(|segment| {
                !obstacles.iter().any(|obstacle| {
                    crate::route::segment_enters(segment.0, segment.1, &obstacle.bounds)
                }) && !other_segments
                    .iter()
                    .any(|other| legs_shared(*segment, *other))
            });
            if clear {
                routes.push(candidate);
            }
        }
    }
    routes
}

/// The boxes a tag of the link between `from` and `to` may not meet, as links-avoid-boxes
/// reads them: every node but a container and the endpoints' ancestors.
pub(crate) fn tag_obstacles(geometry: &PageGeometry, from: usize, to: usize) -> Vec<BoxRect> {
    let ancestors = crate::checks::endpoint_ancestors(geometry, from, to);
    geometry
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| !ancestors.get(*index).copied().unwrap_or(true))
        .filter(|(_, node)| !node.tag.is_container_for_tags())
        .map(|(_, node)| node.bounds)
        .collect()
}

/// A leg of one link shares a line with a leg of another when the two run along one line,
/// less than this apart, over a stretch longer than it.
pub const SHARED_LEG_PX: f32 = 4.0;
/// The offsets a sheet tries, in order, to move a leg off another link's.
const LEG_SHIFTS_PX: [f32; 6] = [10.0, -10.0, 20.0, -20.0, 30.0, -30.0];
/// A leg a shift shortens keeps at least this length.
const SHIFTED_LEG_MIN_PX: f32 = 16.0;

/// True when the segments `a` and `b` run along one line less than `SHARED_LEG_PX` apart
/// over a stretch longer than that.
pub fn legs_shared(a: (PagePoint, PagePoint), b: (PagePoint, PagePoint)) -> bool {
    let horizontal = |run: (PagePoint, PagePoint)| (run.0.y - run.1.y).abs() < GEOMETRY_EPSILON_PX;
    let vertical = |run: (PagePoint, PagePoint)| (run.0.x - run.1.x).abs() < GEOMETRY_EPSILON_PX;
    let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| {
        a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1))
    };
    if horizontal(a) && horizontal(b) {
        (a.0.y - b.0.y).abs() < SHARED_LEG_PX && overlap(a.0.x, a.1.x, b.0.x, b.1.x) > SHARED_LEG_PX
    } else if vertical(a) && vertical(b) {
        (a.0.x - b.0.x).abs() < SHARED_LEG_PX && overlap(a.0.y, a.1.y, b.0.y, b.1.y) > SHARED_LEG_PX
    } else {
        false
    }
}

fn segments(points: &[PagePoint]) -> Vec<(PagePoint, PagePoint)> {
    points
        .windows(2)
        .filter_map(|pair| Some((*pair.first()?, *pair.get(1)?)))
        .collect()
}

/// On a design sheet a leg that runs along another link's leg moves off it (section 14.4
/// rule 24 of the lines): the later link's leg shifts across its run by the first offset of
/// `LEG_SHIFTS_PX` that keeps it out of every box the link avoids, off every other link's
/// legs, its ends on their sides, and its neighbouring legs long enough. A leg no shift
/// frees stays, and links-routed reports it.
pub(crate) fn separate_shared_legs(
    geometry: &PageGeometry,
    paths: &mut [crate::route::RoutedPath],
) {
    let edges = zone_edges(geometry);
    for later in 0..paths.len() {
        let Some(path) = paths.get(later) else {
            continue;
        };
        let (from, to) = (path.from_node, path.to_node);
        let obstacles = crate::route::link_obstacles(geometry, from, to);
        let mut points = path.points.clone();
        let mut moved = false;
        for leg in 0..points.len().saturating_sub(1) {
            let others: Vec<(PagePoint, PagePoint)> = paths
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != later)
                .flat_map(|(_, other)| segments(&other.points))
                .collect();
            let (Some(&a), Some(&b)) = (points.get(leg), points.get(leg + 1)) else {
                continue;
            };
            // The first link has no earlier one to share with, only outlines to crowd.
            let shares = later > 0 && others.iter().any(|other| legs_shared((a, b), *other));
            if !shares && !edges.iter().any(|edge| crowds((a, b), *edge)) {
                continue;
            }
            for shift in LEG_SHIFTS_PX {
                let Some(candidate) = shifted(geometry, (from, to), &points, leg, shift) else {
                    continue;
                };
                let clear = segments(&candidate).iter().all(|segment| {
                    !obstacles.iter().any(|obstacle| {
                        crate::route::segment_enters(segment.0, segment.1, &obstacle.bounds)
                    }) && !others.iter().any(|other| legs_shared(*segment, *other))
                        && !edges.iter().any(|edge| crowds(*segment, *edge))
                });
                if clear {
                    points = candidate;
                    moved = true;
                    break;
                }
            }
        }
        if moved && let Some(path) = paths.get_mut(later) {
            path.points = points;
        }
    }
}

/// A leg keeps at least this far from a Box outline it runs along.
pub const OUTLINE_AIR_PX: f32 = 12.0;

/// The four edges of every Box's drawn outline on the page.
pub fn zone_edges(geometry: &PageGeometry) -> Vec<(PagePoint, PagePoint)> {
    zone_outlines(geometry)
        .into_iter()
        .flat_map(|outline| {
            let corner = |x: f32, y: f32| PagePoint { x, y };
            let (left, top, right, bottom) =
                (outline.x, outline.y, outline.right(), outline.bottom());
            [
                (corner(left, top), corner(right, top)),
                (corner(right, top), corner(right, bottom)),
                (corner(left, bottom), corner(right, bottom)),
                (corner(left, top), corner(left, bottom)),
            ]
        })
        .collect()
}

/// True when a leg runs along a Box outline's edge, parallel and nearer than
/// `OUTLINE_AIR_PX`, over a stretch longer than that: the line would read as the outline.
pub fn crowds(leg: (PagePoint, PagePoint), edge: (PagePoint, PagePoint)) -> bool {
    let horizontal = |run: (PagePoint, PagePoint)| (run.0.y - run.1.y).abs() < GEOMETRY_EPSILON_PX;
    let vertical = |run: (PagePoint, PagePoint)| (run.0.x - run.1.x).abs() < GEOMETRY_EPSILON_PX;
    let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| {
        a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1))
    };
    if horizontal(leg) && horizontal(edge) {
        (leg.0.y - edge.0.y).abs() < OUTLINE_AIR_PX
            && overlap(leg.0.x, leg.1.x, edge.0.x, edge.1.x) > OUTLINE_AIR_PX
    } else if vertical(leg) && vertical(edge) {
        (leg.0.x - edge.0.x).abs() < OUTLINE_AIR_PX
            && overlap(leg.0.y, leg.1.y, edge.0.y, edge.1.y) > OUTLINE_AIR_PX
    } else {
        false
    }
}

/// `points` with leg `leg` moved `shift` px across its run, its ends moved with it; None
/// when an end of the route would leave its side, or a neighbouring leg would turn back
/// or fall under `SHIFTED_LEG_MIN_PX`.
fn shifted(
    geometry: &PageGeometry,
    (from, to): (usize, usize),
    points: &[PagePoint],
    leg: usize,
    shift: f32,
) -> Option<Vec<PagePoint>> {
    let a = *points.get(leg)?;
    let b = *points.get(leg + 1)?;
    let horizontal = (a.y - b.y).abs() < GEOMETRY_EPSILON_PX;
    let delta = if horizontal {
        (0.0, shift)
    } else {
        (shift, 0.0)
    };
    let mut moved = points.to_vec();
    for index in [leg, leg + 1] {
        let point = moved.get_mut(index)?;
        point.x += delta.0;
        point.y += delta.1;
    }
    let last = moved.len() - 1;
    // An end of the route moves along the side it stands on, and stays on it.
    for (index, node) in [(0, from), (last, to)] {
        if index != leg && index != leg + 1 {
            continue;
        }
        let node = geometry.nodes.get(node)?;
        let bounds = crate::route::attach_box(node);
        if !horizontal && node.outline == Some(stencil_model::Outline::Store) {
            // A store's top and bottom are its lid's curve, so the end follows the curve.
            let end = moved.get_mut(index)?;
            let side = if end.y < bounds.y + bounds.height / 2.0 {
                stencil_model::Side::Top
            } else {
                stencil_model::Side::Bottom
            };
            *end = crate::route::on_store_outline(&bounds, side, *end);
        }
        let point = *moved.get(index)?;
        let on_side = if horizontal {
            point.y >= bounds.y + SHARED_LEG_PX && point.y <= bounds.bottom() - SHARED_LEG_PX
        } else {
            point.x >= bounds.x + SHARED_LEG_PX && point.x <= bounds.right() - SHARED_LEG_PX
        };
        if !on_side {
            return None;
        }
    }
    for (before, after) in segments(&moved).iter().zip(segments(points)) {
        let length = (before.1.x - before.0.x).abs() + (before.1.y - before.0.y).abs();
        let original = (after.1.x - after.0.x).abs() + (after.1.y - after.0.y).abs();
        let same_way = (before.1.x - before.0.x) * (after.1.x - after.0.x)
            + (before.1.y - before.0.y) * (after.1.y - after.0.y)
            >= 0.0;
        if !same_way || (length < original && length < SHIFTED_LEG_MIN_PX) {
            return None;
        }
    }
    Some(moved)
}

fn grow(bounds: &BoxRect, by: f32) -> BoxRect {
    BoxRect {
        x: bounds.x - by,
        y: bounds.y - by,
        width: bounds.width + 2.0 * by,
        height: bounds.height + 2.0 * by,
    }
}

/// True when two boxes share interior; a box of no width or height, a straight run of a
/// line, overlaps a box it passes through.
pub(crate) fn overlaps(a: &BoxRect, b: &BoxRect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// The straight runs of a drawn link, each from where it starts to where it ends.
pub fn straight_runs(drawn: &DrawnLink) -> Vec<(PagePoint, PagePoint)> {
    let mut runs = Vec::new();
    let mut at = drawn.start;
    for piece in drawn.legs.iter().flatten() {
        if let PathPiece::Line(to) = *piece {
            runs.push((at, to));
        }
        at = piece.to();
    }
    runs
}

/// Where an axis-aligned run of one link crosses a run of another at a right angle,
/// `HOP_END_CLEARANCE_PX` or more inside both runs, and which of the two is horizontal.
fn crossing(first: (PagePoint, PagePoint), second: (PagePoint, PagePoint)) -> Option<PagePoint> {
    let horizontal = |run: (PagePoint, PagePoint)| (run.0.y - run.1.y).abs() < GEOMETRY_EPSILON_PX;
    let vertical = |run: (PagePoint, PagePoint)| (run.0.x - run.1.x).abs() < GEOMETRY_EPSILON_PX;
    let (across, down) = if horizontal(first) && vertical(second) {
        (first, second)
    } else if vertical(first) && horizontal(second) {
        (second, first)
    } else {
        return None;
    };
    let x = down.0.x;
    let y = across.0.y;
    let inside = |value: f32, a: f32, b: f32| {
        value > a.min(b) + HOP_END_CLEARANCE_PX && value < a.max(b) - HOP_END_CLEARANCE_PX
    };
    (inside(x, across.0.x, across.1.x) && inside(y, down.0.y, down.1.y))
        .then_some(PagePoint { x, y })
}

/// The hop of a link over a crossing at `point` on a run that is horizontal or not: the box
/// of a semicircle of `HOP_RADIUS_PX` bulging toward -y on a horizontal run and toward -x
/// on a vertical one.
pub fn hop_part(point: PagePoint, horizontal: bool) -> Part {
    let bounds = if horizontal {
        BoxRect {
            x: point.x - HOP_RADIUS_PX,
            y: point.y - HOP_RADIUS_PX,
            width: 2.0 * HOP_RADIUS_PX,
            height: HOP_RADIUS_PX,
        }
    } else {
        BoxRect {
            x: point.x - HOP_RADIUS_PX,
            y: point.y - HOP_RADIUS_PX,
            width: HOP_RADIUS_PX,
            height: 2.0 * HOP_RADIUS_PX,
        }
    };
    Part {
        name: PartName::Hop,
        bounds,
        text: None,
    }
}

/// The crossing a hop part stands on.
pub fn hop_point(part: &Part) -> PagePoint {
    let bounds = part.bounds;
    if bounds.width >= bounds.height {
        PagePoint {
            x: bounds.x + bounds.width / 2.0,
            y: bounds.bottom(),
        }
    } else {
        PagePoint {
            x: bounds.right(),
            y: bounds.y + bounds.height / 2.0,
        }
    }
}

/// Section 14.4 rule 24: where a straight run of one link crosses one of another link whose
/// effective actor differs (no actor counts as its own value), the later link in `links`
/// order hops. Crossings under a tag draw no hop, since the tag covers them.
pub(crate) fn add_hops(page: &Page, geometry: &PageGeometry, routes: &mut [LinkRoute]) {
    let actors = PageActors::of(page);
    let actor_of = |index: usize| -> Option<&str> {
        page.links
            .get(index)
            .and_then(|link: &Link| actors.link_actor(link))
    };
    let drawn: Vec<Vec<(PagePoint, PagePoint)>> = routes
        .iter()
        .map(|route| straight_runs(&drawn_link(geometry, route)))
        .collect();
    let tags: Vec<Option<BoxRect>> = routes.iter().map(|route| route.tag).collect();
    let routes_points: Vec<Vec<PagePoint>> =
        routes.iter().map(|route| route.points.clone()).collect();
    for later in 0..routes.len() {
        let mut hops = Vec::new();
        for earlier in 0..later {
            let (Some(first), Some(second)) = (routes.get(earlier), routes.get(later)) else {
                continue;
            };
            if actor_of(first.index) == actor_of(second.index) {
                continue;
            }
            let (Some(earlier_runs), Some(later_runs)) = (drawn.get(earlier), drawn.get(later))
            else {
                continue;
            };
            for later_run in later_runs {
                for earlier_run in earlier_runs {
                    let Some(point) = crossing(*later_run, *earlier_run) else {
                        continue;
                    };
                    let under_tag = [earlier, later].iter().any(|index| {
                        tags.get(*index).copied().flatten().is_some_and(|tag| {
                            point.x > tag.x
                                && point.x < tag.right()
                                && point.y > tag.y
                                && point.y < tag.bottom()
                        })
                    });
                    // No hop stands in the straight approach at an end of either line.
                    let near_an_end = [earlier, later].iter().any(|index| {
                        routes_points.get(*index).is_some_and(|points| {
                            [points.first(), points.last()]
                                .into_iter()
                                .flatten()
                                .any(|end| {
                                    (end.x - point.x).abs() + (end.y - point.y).abs()
                                        < HOP_END_STUB_PX
                                })
                        })
                    });
                    if under_tag || near_an_end {
                        continue;
                    }
                    let horizontal = (later_run.0.y - later_run.1.y).abs() < GEOMETRY_EPSILON_PX;
                    hops.push(hop_part(point, horizontal));
                }
            }
        }
        if let Some(route) = routes.get_mut(later) {
            route.parts.extend(hops);
        }
    }
}
