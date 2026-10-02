//! Iso adjustments of a routed link before it is laid over the slabs (section 12.3, rule 8),
//! and the flat measures `iso-links-clear` reads (section 12.7). A route that joins two
//! facing sides with a shared span is drawn as one straight leg, and an inner leg that runs
//! along a zone edge moves off it.

use stencil_layout::{BoxRect, GEOMETRY_EPSILON_PX};
use stencil_model::PagePoint;

/// Least distance between a link leg and a parallel zone edge beside it.
pub const ISO_LINK_CLEARANCE_PX: f32 = 24.0;
/// Least span two facing sides share before their link is drawn straight across it.
pub const ISO_STRAIGHT_SHARED_MIN_PX: f32 = 16.0;

/// An axis-aligned flat segment: a zone edge or a link leg.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FlatSegment {
    pub start: PagePoint,
    pub end: PagePoint,
}

impl FlatSegment {
    fn is_horizontal(&self) -> bool {
        (self.start.y - self.end.y).abs() <= GEOMETRY_EPSILON_PX
            && (self.start.x - self.end.x).abs() > GEOMETRY_EPSILON_PX
    }

    fn is_vertical(&self) -> bool {
        (self.start.x - self.end.x).abs() <= GEOMETRY_EPSILON_PX
            && (self.start.y - self.end.y).abs() > GEOMETRY_EPSILON_PX
    }

    pub fn length(&self) -> f32 {
        ((self.end.x - self.start.x).powi(2) + (self.end.y - self.start.y).powi(2)).sqrt()
    }
}

/// The four edges of a zone footprint.
pub(crate) fn footprint_edges(bounds: BoxRect) -> [FlatSegment; 4] {
    let corner = |x: f32, y: f32| PagePoint { x, y };
    let (left, top, right, bottom) = (bounds.x, bounds.y, bounds.right(), bounds.bottom());
    [
        FlatSegment {
            start: corner(left, top),
            end: corner(right, top),
        },
        FlatSegment {
            start: corner(right, top),
            end: corner(right, bottom),
        },
        FlatSegment {
            start: corner(left, bottom),
            end: corner(right, bottom),
        },
        FlatSegment {
            start: corner(left, top),
            end: corner(left, bottom),
        },
    ]
}

/// The distance between a leg and an edge when both are horizontal or both vertical and
/// their extents overlap by more than GEOMETRY_EPSILON_PX; None otherwise.
pub(crate) fn parallel_gap(leg: FlatSegment, edge: FlatSegment) -> Option<f32> {
    let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| {
        a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1)) > GEOMETRY_EPSILON_PX
    };
    if leg.is_horizontal() && edge.is_horizontal() {
        return overlap(leg.start.x, leg.end.x, edge.start.x, edge.end.x)
            .then(|| (leg.start.y - edge.start.y).abs());
    }
    if leg.is_vertical() && edge.is_vertical() {
        return overlap(leg.start.y, leg.end.y, edge.start.y, edge.end.y)
            .then(|| (leg.start.x - edge.start.x).abs());
    }
    None
}

/// The legs of a polyline, dropping repeated points.
pub(crate) fn legs(points: &[PagePoint]) -> Vec<FlatSegment> {
    points
        .windows(2)
        .filter_map(|pair| {
            let (Some(&start), Some(&end)) = (pair.first(), pair.get(1)) else {
                return None;
            };
            let leg = FlatSegment { start, end };
            (leg.length() > GEOMETRY_EPSILON_PX).then_some(leg)
        })
        .collect()
}

/// The polyline without repeated points and without points in the middle of a straight
/// run. A point where the line turns back is kept.
pub(crate) fn corners(points: &[PagePoint]) -> Vec<PagePoint> {
    let mut kept: Vec<PagePoint> = Vec::with_capacity(points.len());
    for &point in points {
        if let Some(last) = kept.last()
            && (last.x - point.x).abs() <= GEOMETRY_EPSILON_PX
            && (last.y - point.y).abs() <= GEOMETRY_EPSILON_PX
        {
            continue;
        }
        if kept.len() >= 2
            && let (Some(&middle), Some(&first)) = (kept.last(), kept.get(kept.len() - 2))
        {
            let cross = (middle.x - first.x) * (point.y - first.y)
                - (middle.y - first.y) * (point.x - first.x);
            let forward = (middle.x - first.x) * (point.x - middle.x)
                + (middle.y - first.y) * (point.y - middle.y);
            if cross.abs() <= GEOMETRY_EPSILON_PX && forward > 0.0 {
                kept.pop();
            }
        }
        kept.push(point);
    }
    kept
}

/// True when two consecutive legs run in opposite directions.
pub(crate) fn turns_back(first: FlatSegment, second: FlatSegment) -> bool {
    let a = (first.end.x - first.start.x, first.end.y - first.start.y);
    let b = (second.end.x - second.start.x, second.end.y - second.start.y);
    let cross = a.0 * b.1 - a.1 * b.0;
    let dot = a.0 * b.0 + a.1 * b.1;
    cross.abs() <= GEOMETRY_EPSILON_PX * a.0.hypot(a.1).max(1.0) && dot < 0.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

fn side_of(point: PagePoint, bounds: BoxRect) -> Option<Side> {
    let near = |a: f32, b: f32| (a - b).abs() <= GEOMETRY_EPSILON_PX;
    let within_x = point.x >= bounds.x - GEOMETRY_EPSILON_PX
        && point.x <= bounds.right() + GEOMETRY_EPSILON_PX;
    let within_y = point.y >= bounds.y - GEOMETRY_EPSILON_PX
        && point.y <= bounds.bottom() + GEOMETRY_EPSILON_PX;
    if near(point.x, bounds.right()) && within_y {
        Some(Side::Right)
    } else if near(point.x, bounds.x) && within_y {
        Some(Side::Left)
    } else if near(point.y, bounds.bottom()) && within_x {
        Some(Side::Bottom)
    } else if near(point.y, bounds.y) && within_x {
        Some(Side::Top)
    } else {
        None
    }
}

/// True when the segment passes through the inside of `bounds`.
fn enters(start: PagePoint, end: PagePoint, bounds: BoxRect) -> bool {
    let inner = BoxRect {
        x: bounds.x + GEOMETRY_EPSILON_PX,
        y: bounds.y + GEOMETRY_EPSILON_PX,
        width: bounds.width - 2.0 * GEOMETRY_EPSILON_PX,
        height: bounds.height - 2.0 * GEOMETRY_EPSILON_PX,
    };
    if inner.width <= 0.0 || inner.height <= 0.0 {
        return false;
    }
    let (low_x, high_x) = (start.x.min(end.x), start.x.max(end.x));
    let (low_y, high_y) = (start.y.min(end.y), start.y.max(end.y));
    low_x < inner.right() && high_x > inner.x && low_y < inner.bottom() && high_y > inner.y
}

/// The route as one leg when its ends sit on two facing sides whose spans overlap by at
/// least ISO_STRAIGHT_SHARED_MIN_PX across the gap, and the leg enters no block in
/// `blocks`; None otherwise. A layout route joins side midpoints, so two blocks that nearly
/// line up get a short jog that the projection turns into a kink. The leg runs through the
/// target's center, held inside the shared span by a corner margin at each end: the corner
/// clearance of `iso-link-ends`, plus the block's height where the end sits on a hidden
/// side, since the cut of section 12.3 rule 7 lands such an end that much nearer the front
/// corner. A span with no room between its margins is left to the router.
pub(crate) fn straightened(
    points: &[PagePoint],
    (from, from_height): (BoxRect, f32),
    (to, to_height): (BoxRect, f32),
    blocks: &[BoxRect],
) -> Option<Vec<PagePoint>> {
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return None;
    };
    if corners(points).len() <= 2 {
        return None;
    }
    let clearance = super::ISO_LINK_CORNER_CLEARANCE_PX;
    // The span shared by two sides, each shrunk by its margins, and the target's center.
    let shared = |(low_a, high_a, front_a): (f32, f32, f32),
                  (low_b, high_b, front_b): (f32, f32, f32),
                  center_b: f32| {
        if high_a.min(high_b) - low_a.max(low_b) < ISO_STRAIGHT_SHARED_MIN_PX {
            return None;
        }
        let low = (low_a + clearance).max(low_b + clearance);
        let high = (high_a - clearance - front_a).min(high_b - clearance - front_b);
        (high >= low).then(|| center_b.clamp(low, high))
    };
    let (from_hidden, to_hidden) = match (side_of(first, from), side_of(last, to)) {
        (Some(Side::Right), Some(Side::Left)) if to.x >= from.right() => (0.0, to_height),
        (Some(Side::Left), Some(Side::Right)) if from.x >= to.right() => (from_height, 0.0),
        (Some(Side::Bottom), Some(Side::Top)) if to.y >= from.bottom() => (0.0, to_height),
        (Some(Side::Top), Some(Side::Bottom)) if from.y >= to.bottom() => (from_height, 0.0),
        _ => return None,
    };
    let leg = match (side_of(first, from), side_of(last, to)) {
        (Some(Side::Right), Some(Side::Left)) => {
            let y = shared(
                (from.y, from.bottom(), from_hidden),
                (to.y, to.bottom(), to_hidden),
                to.y + to.height / 2.0,
            )?;
            (PagePoint { x: from.right(), y }, PagePoint { x: to.x, y })
        }
        (Some(Side::Left), Some(Side::Right)) => {
            let y = shared(
                (from.y, from.bottom(), from_hidden),
                (to.y, to.bottom(), to_hidden),
                to.y + to.height / 2.0,
            )?;
            (PagePoint { x: from.x, y }, PagePoint { x: to.right(), y })
        }
        (Some(Side::Bottom), Some(Side::Top)) => {
            let x = shared(
                (from.x, from.right(), from_hidden),
                (to.x, to.right(), to_hidden),
                to.x + to.width / 2.0,
            )?;
            (
                PagePoint {
                    x,
                    y: from.bottom(),
                },
                PagePoint { x, y: to.y },
            )
        }
        (Some(Side::Top), Some(Side::Bottom)) => {
            let x = shared(
                (from.x, from.right(), from_hidden),
                (to.x, to.right(), to_hidden),
                to.x + to.width / 2.0,
            )?;
            (PagePoint { x, y: from.y }, PagePoint { x, y: to.bottom() })
        }
        _ => return None,
    };
    if blocks.iter().any(|block| enters(leg.0, leg.1, *block)) {
        return None;
    }
    Some(vec![leg.0, leg.1])
}

/// True when any leg of the polyline passes through the inside of a block.
pub(crate) fn enters_any(points: &[PagePoint], blocks: &[BoxRect]) -> bool {
    points
        .windows(2)
        .any(|pair| match (pair.first(), pair.get(1)) {
            (Some(&start), Some(&end)) => blocks.iter().any(|block| enters(start, end, *block)),
            _ => false,
        })
}

/// A route of three legs whose first and last run the same way, a Z, with its middle leg
/// moved to `fraction` of the run between its ends, so the step can sit away from both
/// blocks instead of at the end of a stub, where the router leaves it. None for any other
/// shape and for a step already there.
pub(crate) fn step_at(points: &[PagePoint], fraction: f32) -> Option<Vec<PagePoint>> {
    let kept = corners(points);
    if kept.len() != 4 {
        return None;
    }
    let legs = legs(&kept);
    let (first, middle, last) = (legs.first()?, legs.get(1)?, legs.get(2)?);
    if !same_way(*first, *last) {
        return None;
    }
    let (start, end) = (kept.first()?, kept.last()?);
    let mut moved = kept.clone();
    let shift = if first.is_horizontal() && middle.is_vertical() {
        let at = start.x + (end.x - start.x) * fraction;
        for point in moved.iter_mut().skip(1).take(2) {
            point.x = at;
        }
        at - middle.start.x
    } else if first.is_vertical() && middle.is_horizontal() {
        let at = start.y + (end.y - start.y) * fraction;
        for point in moved.iter_mut().skip(1).take(2) {
            point.y = at;
        }
        at - middle.start.y
    } else {
        return None;
    };
    (shift.abs() > GEOMETRY_EPSILON_PX).then_some(moved)
}

/// True when two legs are parallel and run the same way.
pub(crate) fn same_way(a: FlatSegment, b: FlatSegment) -> bool {
    let (ax, ay) = (a.end.x - a.start.x, a.end.y - a.start.y);
    let (bx, by) = (b.end.x - b.start.x, b.end.y - b.start.y);
    let cross = ax * by - ay * bx;
    cross.abs() <= GEOMETRY_EPSILON_PX * ax.hypot(ay).max(1.0) && ax * bx + ay * by > 0.0
}

/// True when two legs run along one axis closer than `clearance` across it, with their
/// extents within `clearance` of each other along it: side by side, or end to end with a
/// gap a reader cannot see.
pub(crate) fn near_parallel(a: FlatSegment, b: FlatSegment, clearance: f32) -> bool {
    let within = |a0: f32, a1: f32, b0: f32, b1: f32| {
        a0.max(a1) + clearance > b0.min(b1) && b0.max(b1) + clearance > a0.min(a1)
    };
    if a.is_horizontal() && b.is_horizontal() {
        return (a.start.y - b.start.y).abs() < clearance
            && within(a.start.x, a.end.x, b.start.x, b.end.x);
    }
    if a.is_vertical() && b.is_vertical() {
        return (a.start.x - b.start.x).abs() < clearance
            && within(a.start.y, a.end.y, b.start.y, b.end.y);
    }
    false
}

/// The polyline shifted sideways by `offset` to the left of its travel: every leg moves
/// along its left normal and each corner is where the two moved legs meet. The ends move
/// along the sides they attach to, since a route leaves its side at a right angle.
pub(crate) fn offset_polyline(points: &[PagePoint], offset: f32) -> Vec<PagePoint> {
    let kept = corners(points);
    let legs = legs(&kept);
    let moved: Vec<(PagePoint, (f32, f32))> = legs
        .iter()
        .map(|leg| {
            let length = leg.length().max(GEOMETRY_EPSILON_PX);
            let direction = (
                (leg.end.x - leg.start.x) / length,
                (leg.end.y - leg.start.y) / length,
            );
            let normal = (direction.1, -direction.0);
            (
                PagePoint {
                    x: leg.start.x + normal.0 * offset,
                    y: leg.start.y + normal.1 * offset,
                },
                direction,
            )
        })
        .collect();
    let (Some(&(first_point, _)), Some(&(last_point, last_direction)), Some(last_leg)) =
        (moved.first(), moved.last(), legs.last())
    else {
        return kept;
    };
    let mut shifted = vec![first_point];
    for pair in moved.windows(2) {
        let (Some(&(a, da)), Some(&(b, db))) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let cross = da.0 * db.1 - da.1 * db.0;
        if cross.abs() <= 1e-6 {
            shifted.push(b);
            continue;
        }
        let t = ((b.x - a.x) * db.1 - (b.y - a.y) * db.0) / cross;
        shifted.push(PagePoint {
            x: a.x + da.0 * t,
            y: a.y + da.1 * t,
        });
    }
    shifted.push(PagePoint {
        x: last_point.x + last_direction.0 * last_leg.length(),
        y: last_point.y + last_direction.1 * last_leg.length(),
    });
    shifted
}

/// The polyline with its first (or last) leg moved sideways by `along`, a vector along the
/// side the end attaches to: the end and the corner after it move together, so the leg
/// beside the corner, which runs the other way, only changes length. A two-point route has
/// no such corner and is left alone.
pub(crate) fn shift_end_leg(
    points: &[PagePoint],
    along: (f32, f32),
    at_start: bool,
) -> Vec<PagePoint> {
    let mut kept = corners(points);
    if kept.len() < 3 {
        return kept;
    }
    let count = kept.len();
    let (end, corner) = if at_start {
        (0, 1)
    } else {
        (count - 1, count - 2)
    };
    for index in [end, corner] {
        if let Some(point) = kept.get_mut(index) {
            point.x += along.0;
            point.y += along.1;
        }
    }
    kept
}

fn inside(bounds: BoxRect, point: PagePoint) -> bool {
    point.x > bounds.x
        && point.x < bounds.right()
        && point.y > bounds.y
        && point.y < bounds.bottom()
}

/// Moves each inner leg (neither the first nor the last) that runs parallel to a zone edge
/// closer than ISO_LINK_CLEARANCE_PX to the nearest place clear of every parallel edge
/// beside it. The moved leg keeps the direction of the two legs around it, enters no block
/// and stays on the same zones, so a leg in a narrow gap is not pushed onto a floor it
/// did not cross. A leg with no such place stays, and `iso-links-clear` reports it.
pub(crate) fn kept_clear(
    points: &[PagePoint],
    zones: &[BoxRect],
    blocks: &[BoxRect],
    clearance: f32,
) -> Vec<PagePoint> {
    let edges: Vec<FlatSegment> = zones
        .iter()
        .flat_map(|zone| footprint_edges(*zone))
        .collect();
    let floors = |start: PagePoint, end: PagePoint| -> Vec<bool> {
        let middle = PagePoint {
            x: (start.x + end.x) / 2.0,
            y: (start.y + end.y) / 2.0,
        };
        zones.iter().map(|zone| inside(*zone, middle)).collect()
    };
    let mut points = corners(points);
    let count = points.len();
    for index in 1..count.saturating_sub(2) {
        let (Some(&before), Some(&start), Some(&end), Some(&after)) = (
            points.get(index - 1),
            points.get(index),
            points.get(index + 1),
            points.get(index + 2),
        ) else {
            continue;
        };
        let leg = FlatSegment { start, end };
        let beside: Vec<(FlatSegment, f32)> = edges
            .iter()
            .filter_map(|edge| parallel_gap(leg, *edge).map(|gap| (*edge, gap)))
            .collect();
        if beside
            .iter()
            .all(|(_, gap)| *gap >= clearance - GEOMETRY_EPSILON_PX)
        {
            continue;
        }
        let horizontal = leg.is_horizontal();
        let (current, outer_before, outer_after) = if horizontal {
            (start.y, before.y, after.y)
        } else {
            (start.x, before.x, after.x)
        };
        let mut candidates: Vec<f32> = beside
            .iter()
            .flat_map(|(edge, _)| {
                let line = if horizontal {
                    edge.start.y
                } else {
                    edge.start.x
                };
                [line - clearance, line + clearance]
            })
            .collect();
        candidates
            .sort_by(|first, second| (first - current).abs().total_cmp(&(second - current).abs()));
        // A lane too narrow for the wider clearance still takes the base one.
        if clearance > ISO_LINK_CLEARANCE_PX + GEOMETRY_EPSILON_PX {
            let mut fallback: Vec<f32> = beside
                .iter()
                .flat_map(|(edge, _)| {
                    let line = if horizontal {
                        edge.start.y
                    } else {
                        edge.start.x
                    };
                    [line - ISO_LINK_CLEARANCE_PX, line + ISO_LINK_CLEARANCE_PX]
                })
                .collect();
            fallback.sort_by(|first, second| {
                (first - current).abs().total_cmp(&(second - current).abs())
            });
            candidates.extend(fallback);
        }
        let moved = |line: f32| {
            let place = |point: PagePoint| {
                if horizontal {
                    PagePoint { y: line, ..point }
                } else {
                    PagePoint { x: line, ..point }
                }
            };
            (place(start), place(end))
        };
        let fits = |line: f32| {
            let keeps_direction = (line - outer_before).signum()
                == (current - outer_before).signum()
                && (outer_after - line).signum() == (outer_after - current).signum()
                && (line - outer_before).abs() > GEOMETRY_EPSILON_PX
                && (outer_after - line).abs() > GEOMETRY_EPSILON_PX;
            let (new_start, new_end) = moved(line);
            let clear_of_edges = beside.iter().all(|(edge, _)| {
                parallel_gap(
                    FlatSegment {
                        start: new_start,
                        end: new_end,
                    },
                    *edge,
                )
                .is_none_or(|gap| gap >= ISO_LINK_CLEARANCE_PX - GEOMETRY_EPSILON_PX)
            });
            let clear_of_blocks = blocks.iter().all(|block| {
                !enters(before, new_start, *block)
                    && !enters(new_start, new_end, *block)
                    && !enters(new_end, after, *block)
            });
            let same_floor = floors(new_start, new_end) == floors(start, end);
            keeps_direction && clear_of_edges && clear_of_blocks && same_floor
        };
        if let Some(line) = candidates.into_iter().find(|line| fits(*line)) {
            let (new_start, new_end) = moved(line);
            if let Some(slot) = points.get_mut(index) {
                *slot = new_start;
            }
            if let Some(slot) = points.get_mut(index + 1) {
                *slot = new_end;
            }
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> PagePoint {
        PagePoint { x, y }
    }

    fn rectangle(x: f32, y: f32, width: f32, height: f32) -> BoxRect {
        BoxRect {
            x,
            y,
            width,
            height,
        }
    }

    /// A Z's step moves to the asked fraction of the run; a U, a longer route and a step
    /// already there give nothing.
    #[test]
    fn a_step_moves_to_a_fraction_of_the_run_and_other_shapes_stay() {
        let z = [
            point(100.0, 50.0),
            point(240.0, 50.0),
            point(240.0, 90.0),
            point(300.0, 90.0),
        ];
        assert_eq!(
            step_at(&z, 0.5),
            Some(vec![
                point(100.0, 50.0),
                point(200.0, 50.0),
                point(200.0, 90.0),
                point(300.0, 90.0),
            ])
        );
        assert_eq!(
            step_at(&z, 0.25).map(|moved| (moved[1].x, moved[2].x)),
            Some((150.0, 150.0))
        );
        let down = [
            point(50.0, 100.0),
            point(50.0, 140.0),
            point(90.0, 140.0),
            point(90.0, 300.0),
        ];
        assert_eq!(
            step_at(&down, 0.75).map(|moved| (moved[1].y, moved[2].y)),
            Some((250.0, 250.0))
        );
        assert_eq!(step_at(&z, 0.7), None);
        let u = [
            point(100.0, 50.0),
            point(240.0, 50.0),
            point(240.0, 90.0),
            point(100.0, 90.0),
        ];
        assert_eq!(step_at(&u, 0.5), None);
        let longer = [
            point(100.0, 50.0),
            point(240.0, 50.0),
            point(240.0, 90.0),
            point(300.0, 90.0),
            point(300.0, 120.0),
        ];
        assert_eq!(step_at(&longer, 0.5), None);
    }

    /// Legs side by side within the clearance, or end to end across a gap under it, are
    /// near; a leg across the other's axis never is.
    #[test]
    fn near_parallel_legs_are_side_by_side_or_end_to_end_within_the_clearance() {
        let leg = |x0: f32, y0: f32, x1: f32, y1: f32| FlatSegment {
            start: point(x0, y0),
            end: point(x1, y1),
        };
        assert!(near_parallel(
            leg(100.0, 0.0, 100.0, 50.0),
            leg(110.0, 20.0, 110.0, 80.0),
            24.0
        ));
        assert!(near_parallel(
            leg(100.0, 0.0, 100.0, 50.0),
            leg(100.0, 60.0, 100.0, 90.0),
            24.0
        ));
        assert!(!near_parallel(
            leg(100.0, 0.0, 100.0, 50.0),
            leg(100.0, 80.0, 100.0, 90.0),
            24.0
        ));
        assert!(!near_parallel(
            leg(100.0, 0.0, 100.0, 50.0),
            leg(130.0, 0.0, 130.0, 50.0),
            24.0
        ));
        assert!(!near_parallel(
            leg(100.0, 0.0, 100.0, 50.0),
            leg(90.0, 20.0, 150.0, 20.0),
            24.0
        ));
    }

    #[test]
    fn a_jog_between_facing_sides_that_share_a_span_becomes_one_leg() {
        let from = rectangle(0.0, 20.0, 100.0, 44.0);
        let to = rectangle(200.0, 0.0, 100.0, 44.0);
        let route = [
            point(100.0, 42.0),
            point(150.0, 42.0),
            point(150.0, 22.0),
            point(200.0, 22.0),
        ];
        // The leg aims at the target's center (22) but keeps the corner clearance from
        // both spans: 20 + 8 from the from box's back corner.
        let straight = straightened(&route, (from, 0.0), (to, 0.0), &[]).unwrap();
        assert_eq!(straight, vec![point(100.0, 28.0), point(200.0, 28.0)]);
        let blocker = rectangle(140.0, 0.0, 20.0, 100.0);
        assert!(straightened(&route, (from, 0.0), (to, 0.0), &[blocker]).is_none());
        let apart = rectangle(200.0, -40.0, 100.0, 44.0);
        assert!(straightened(&route, (from, 0.0), (apart, 0.0), &[]).is_none());
        // A hidden-side setback of 18 on the target leaves no room in the span.
        assert!(straightened(&route, (from, 0.0), (to, 18.0), &[]).is_none());
    }

    #[test]
    fn an_inner_leg_along_a_zone_edge_moves_clear_of_it() {
        let zone = rectangle(100.0, 0.0, 200.0, 200.0);
        let edges = footprint_edges(zone);
        let route = [
            point(0.0, 50.0),
            point(90.0, 50.0),
            point(90.0, 150.0),
            point(150.0, 150.0),
        ];
        let moved = kept_clear(&route, &[zone], &[], ISO_LINK_CLEARANCE_PX);
        assert_eq!(moved[1].x, 76.0);
        assert_eq!(moved[2].x, 76.0);
        let legs = legs(&moved);
        for edge in &edges {
            assert!(parallel_gap(legs[1], *edge).is_none_or(|gap| gap >= 24.0));
        }
    }

    #[test]
    fn a_leg_in_a_gap_narrower_than_twice_the_clearance_stays() {
        let left = rectangle(0.0, 0.0, 100.0, 200.0);
        let right = rectangle(132.0, 0.0, 100.0, 200.0);
        let route = [
            point(90.0, 50.0),
            point(116.0, 50.0),
            point(116.0, 150.0),
            point(140.0, 150.0),
        ];
        assert_eq!(
            kept_clear(&route, &[left, right], &[], ISO_LINK_CLEARANCE_PX),
            route.to_vec()
        );
    }

    #[test]
    fn a_leg_that_turns_back_is_found() {
        let legs = legs(&[point(0.0, 0.0), point(50.0, 0.0), point(20.0, 0.0)]);
        assert!(turns_back(legs[0], legs[1]));
        let square = self::legs(&[point(0.0, 0.0), point(50.0, 0.0), point(50.0, 20.0)]);
        assert!(!turns_back(square[0], square[1]));
    }
}
