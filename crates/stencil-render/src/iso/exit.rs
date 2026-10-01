//! Section 13.11 rule 2: a link whose from node stands on a slab leaves that slab through
//! the edge nearest its target. When the adjusted route crosses the slab's footprint
//! boundary more than once, or first on another edge, the link is routed again from the
//! side facing that edge through a point just beyond it.

use stencil_layout::{BoxRect, GEOMETRY_EPSILON_PX, LinkRoute, PageGeometry, RouteStatus};
use stencil_model::{PagePoint, Side};

use super::route::ISO_LINK_CLEARANCE_PX;
use super::{Solid, SolidShape};

/// Parametric slack below which a crossing at a leg's end belongs to the next leg.
const PARAMETER_EPSILON: f32 = 1e-6;

/// The route that replaces `adjusted`, or None when it stays: the from node stands on no
/// slab, the slab also holds the to node, the route already leaves through the exit edge
/// once, or the router finds no route.
pub(crate) fn slab_exit_route(
    route: &LinkRoute,
    geometry: &PageGeometry,
    solids: &[Solid],
    adjusted: &[PagePoint],
) -> Option<Vec<PagePoint>> {
    let slab = nearest_slab(geometry, solids, route.from_node)?;
    if is_within(geometry, route.to_node, slab.node) {
        return None;
    }
    let target = geometry.nodes.get(route.to_node)?.bounds;
    let exit = exit_edge(slab.footprint, center(target))?;
    let crossings = boundary_crossings(adjusted, slab.footprint);
    match crossings.as_slice() {
        [] => return None,
        [only] if *only == exit => return None,
        _ => {}
    }
    let from = geometry.nodes.get(route.from_node)?.bounds;
    let via = exit_via(slab.footprint, from, exit);
    let rerouted = stencil_layout::reroute_link(geometry, route.index, exit, &[via], true)?;
    (rerouted.status == RouteStatus::Routed).then_some(rerouted.points)
}

/// The slab with height nearest `node` among its ancestors. Parents precede children, so
/// the walk ends within `nodes.len()` steps.
fn nearest_slab<'a>(
    geometry: &PageGeometry,
    solids: &'a [Solid],
    node: usize,
) -> Option<&'a Solid> {
    let mut current = geometry.nodes.get(node).and_then(|node| node.parent);
    for _ in 0..geometry.nodes.len() {
        let index = current?;
        let slab = solids.iter().find(|solid| {
            solid.node == index && solid.shape == SolidShape::Slab && solid.height > 0.0
        });
        if slab.is_some() {
            return slab;
        }
        current = geometry.nodes.get(index).and_then(|node| node.parent);
    }
    None
}

/// True when `node` is `ancestor` or lies inside it.
fn is_within(geometry: &PageGeometry, node: usize, ancestor: usize) -> bool {
    let mut current = Some(node);
    for _ in 0..=geometry.nodes.len() {
        let Some(index) = current else {
            return false;
        };
        if index == ancestor {
            return true;
        }
        current = geometry.nodes.get(index).and_then(|node| node.parent);
    }
    false
}

fn center(bounds: BoxRect) -> PagePoint {
    PagePoint {
        x: bounds.x + bounds.width / 2.0,
        y: bounds.y + bounds.height / 2.0,
    }
}

/// The side of `slab` whose outer line is nearest `target`, among the sides `target` lies
/// beyond; ties in the order right, bottom, left, top. None when `target` lies over the
/// footprint.
pub(crate) fn exit_edge(slab: BoxRect, target: PagePoint) -> Option<Side> {
    let beyond = [
        (Side::Right, target.x - slab.right()),
        (Side::Bottom, target.y - slab.bottom()),
        (Side::Left, slab.x - target.x),
        (Side::Top, slab.y - target.y),
    ];
    let mut best: Option<(Side, f32)> = None;
    for (side, distance) in beyond {
        if distance > GEOMETRY_EPSILON_PX && best.is_none_or(|(_, nearest)| distance < nearest) {
            best = Some((side, distance));
        }
    }
    best.map(|(side, _)| side)
}

/// The via point of a re-route: on the line through the from attach point perpendicular to
/// the exit edge, ISO_LINK_CLEARANCE_PX beyond that edge.
fn exit_via(slab: BoxRect, from: BoxRect, exit: Side) -> PagePoint {
    let attach = center(from);
    match exit {
        Side::Right => PagePoint {
            x: slab.right() + ISO_LINK_CLEARANCE_PX,
            y: attach.y,
        },
        Side::Bottom => PagePoint {
            x: attach.x,
            y: slab.bottom() + ISO_LINK_CLEARANCE_PX,
        },
        Side::Left => PagePoint {
            x: slab.x - ISO_LINK_CLEARANCE_PX,
            y: attach.y,
        },
        Side::Top => PagePoint {
            x: attach.x,
            y: slab.y - ISO_LINK_CLEARANCE_PX,
        },
    }
}

/// Where the polyline passes into or out of the open interior of `bounds`, in order, by the
/// side it passes through. A stretch along an edge stays outside.
pub(crate) fn boundary_crossings(points: &[PagePoint], bounds: BoxRect) -> Vec<Side> {
    let mut crossings = Vec::new();
    for leg in points.windows(2) {
        let (Some(&start), Some(&end)) = (leg.first(), leg.get(1)) else {
            continue;
        };
        let Some(((enter, enter_side), (leave, leave_side))) = clip(start, end, bounds) else {
            continue;
        };
        if enter > PARAMETER_EPSILON {
            crossings.push(enter_side);
        }
        if leave < 1.0 - PARAMETER_EPSILON {
            crossings.push(leave_side);
        }
    }
    crossings
}

/// Liang-Barsky clipping of the segment against `bounds` shrunk by the geometry epsilon:
/// the parameters where it enters and leaves, each with the side that bounds it, or None
/// when it never reaches the interior.
fn clip(start: PagePoint, end: PagePoint, bounds: BoxRect) -> Option<((f32, Side), (f32, Side))> {
    let (delta_x, delta_y) = (end.x - start.x, end.y - start.y);
    let inner = (
        bounds.x + GEOMETRY_EPSILON_PX,
        bounds.right() - GEOMETRY_EPSILON_PX,
        bounds.y + GEOMETRY_EPSILON_PX,
        bounds.bottom() - GEOMETRY_EPSILON_PX,
    );
    let constraints = [
        (-delta_x, start.x - inner.0, Side::Left),
        (delta_x, inner.1 - start.x, Side::Right),
        (-delta_y, start.y - inner.2, Side::Top),
        (delta_y, inner.3 - start.y, Side::Bottom),
    ];
    let mut enter = (0.0_f32, Side::Left);
    let mut leave = (1.0_f32, Side::Right);
    for (direction, room, side) in constraints {
        if direction == 0.0 {
            if room <= 0.0 {
                return None;
            }
            continue;
        }
        let parameter = room / direction;
        if direction < 0.0 {
            if parameter > enter.0 {
                enter = (parameter, side);
            }
        } else if parameter < leave.0 {
            leave = (parameter, side);
        }
    }
    (enter.0 < leave.0).then_some((enter, leave))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> PagePoint {
        PagePoint { x, y }
    }

    const SLAB: BoxRect = BoxRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
    };

    #[test]
    fn the_exit_edge_is_the_nearest_side_the_target_lies_beyond() {
        assert_eq!(exit_edge(SLAB, point(300.0, 25.0)), Some(Side::Right));
        assert_eq!(exit_edge(SLAB, point(130.0, 400.0)), Some(Side::Right));
        assert_eq!(exit_edge(SLAB, point(400.0, 60.0)), Some(Side::Bottom));
        assert_eq!(exit_edge(SLAB, point(-20.0, -20.0)), Some(Side::Left));
        assert_eq!(exit_edge(SLAB, point(110.0, 60.0)), Some(Side::Right));
        assert_eq!(exit_edge(SLAB, point(50.0, -5.0)), Some(Side::Top));
        assert_eq!(exit_edge(SLAB, point(50.0, 25.0)), None);
    }

    #[test]
    fn crossings_list_every_pass_through_the_boundary_in_order() {
        let out_right = [point(50.0, 25.0), point(200.0, 25.0)];
        assert_eq!(boundary_crossings(&out_right, SLAB), [Side::Right]);
        let out_bottom_and_back = [
            point(50.0, 25.0),
            point(50.0, 80.0),
            point(80.0, 80.0),
            point(80.0, 20.0),
            point(200.0, 20.0),
        ];
        assert_eq!(
            boundary_crossings(&out_bottom_and_back, SLAB),
            [Side::Bottom, Side::Bottom, Side::Right]
        );
        let along_the_edge = [point(50.0, 25.0), point(50.0, 50.0), point(200.0, 50.0)];
        assert_eq!(boundary_crossings(&along_the_edge, SLAB), [Side::Bottom]);
        let outside = [point(150.0, 25.0), point(200.0, 25.0)];
        assert_eq!(boundary_crossings(&outside, SLAB), Vec::<Side>::new());
    }
}
