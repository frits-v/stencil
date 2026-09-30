//! Screen-space tests over boxes, convex polygons and segments, shared by the label
//! placement of section 12.4 and the check of section 12.7.

use stencil_layout::{BoxRect, GEOMETRY_EPSILON_PX};

use super::ScreenPoint;

/// Unit direction from `from` to `to`, None when the two coincide.
pub(crate) fn unit_direction(from: (f32, f32), to: (f32, f32)) -> Option<(f32, f32)> {
    let delta_x = to.0 - from.0;
    let delta_y = to.1 - from.1;
    let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
    (length > f32::EPSILON).then(|| (delta_x / length, delta_y / length))
}

pub(crate) fn rectangle_corners(screen: BoxRect) -> [ScreenPoint; 4] {
    [
        ScreenPoint {
            x: screen.x,
            y: screen.y,
        },
        ScreenPoint {
            x: screen.right(),
            y: screen.y,
        },
        ScreenPoint {
            x: screen.right(),
            y: screen.bottom(),
        },
        ScreenPoint {
            x: screen.x,
            y: screen.bottom(),
        },
    ]
}

/// `screen` grown by `margin` on every side.
pub(crate) fn grown(screen: BoxRect, margin: f32) -> BoxRect {
    BoxRect {
        x: screen.x - margin,
        y: screen.y - margin,
        width: screen.width + 2.0 * margin,
        height: screen.height + 2.0 * margin,
    }
}

/// Minimum and maximum of the points projected on an axis.
fn axis_interval(points: &[ScreenPoint], axis: (f32, f32)) -> (f32, f32) {
    points
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), point| {
            let value = point.x * axis.0 + point.y * axis.1;
            (low.min(value), high.max(value))
        })
}

/// True when the rectangle and the convex polygon overlap by more than
/// GEOMETRY_EPSILON_PX on the x and y axes and on the unit normal of every polygon edge
/// (section 12.7, rule 2). Zero-length edges add no axis.
pub(crate) fn rectangle_overlaps_polygon(screen: BoxRect, polygon: &[ScreenPoint]) -> bool {
    let rectangle = rectangle_corners(screen);
    let mut axes = vec![(1.0, 0.0), (0.0, 1.0)];
    for (index, start) in polygon.iter().enumerate() {
        let Some(end) = polygon.get((index + 1) % polygon.len()) else {
            continue;
        };
        if let Some((direction_x, direction_y)) = unit_direction((start.x, start.y), (end.x, end.y))
        {
            axes.push((-direction_y, direction_x));
        }
    }
    axes.into_iter().all(|axis| {
        let (rectangle_low, rectangle_high) = axis_interval(&rectangle, axis);
        let (polygon_low, polygon_high) = axis_interval(polygon, axis);
        rectangle_high.min(polygon_high) - rectangle_low.max(polygon_low) > GEOMETRY_EPSILON_PX
    })
}

pub(crate) fn rectangles_overlap(first: BoxRect, second: BoxRect) -> bool {
    let overlap_width = first.right().min(second.right()) - first.x.max(second.x);
    let overlap_height = first.bottom().min(second.bottom()) - first.y.max(second.y);
    overlap_width > GEOMETRY_EPSILON_PX && overlap_height > GEOMETRY_EPSILON_PX
}

/// Twice the signed area of a polygon: positive when its vertices run clockwise on screen,
/// where y grows downward.
fn signed_area_twice(polygon: &[ScreenPoint]) -> f32 {
    let mut sum = 0.0;
    for (index, start) in polygon.iter().enumerate() {
        if let Some(end) = polygon.get((index + 1) % polygon.len()) {
            sum += start.x * end.y - end.x * start.y;
        }
    }
    sum
}

/// The inward normal of each non-degenerate edge of a convex polygon, with a point on the
/// edge. The normals are not unit length.
fn inward_edges(polygon: &[ScreenPoint]) -> Vec<(ScreenPoint, (f32, f32))> {
    let orientation = if signed_area_twice(polygon) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let mut edges = Vec::with_capacity(polygon.len());
    for (index, start) in polygon.iter().enumerate() {
        let Some(end) = polygon.get((index + 1) % polygon.len()) else {
            continue;
        };
        let delta = (end.x - start.x, end.y - start.y);
        if delta.0.abs() <= f32::EPSILON && delta.1.abs() <= f32::EPSILON {
            continue;
        }
        edges.push((*start, (-delta.1 * orientation, delta.0 * orientation)));
    }
    edges
}

/// True when `point` lies inside the convex polygon or on its boundary.
pub(crate) fn point_in_convex(point: ScreenPoint, polygon: &[ScreenPoint]) -> bool {
    inward_edges(polygon).into_iter().all(|(on_edge, normal)| {
        normal.0 * (point.x - on_edge.x) + normal.1 * (point.y - on_edge.y) >= -GEOMETRY_EPSILON_PX
    })
}

/// The parameter interval of the segment `start + t * (end - start)`, t in 0 to 1, that
/// lies inside the convex polygon (Cyrus-Beck), or None when the segment misses it.
pub(crate) fn segment_inside_interval(
    start: ScreenPoint,
    end: ScreenPoint,
    polygon: &[ScreenPoint],
) -> Option<(f32, f32)> {
    let direction = (end.x - start.x, end.y - start.y);
    let mut entry: f32 = 0.0;
    let mut exit: f32 = 1.0;
    for (on_edge, normal) in inward_edges(polygon) {
        let numerator = normal.0 * (start.x - on_edge.x) + normal.1 * (start.y - on_edge.y);
        let denominator = normal.0 * direction.0 + normal.1 * direction.1;
        if denominator.abs() <= f32::EPSILON {
            if numerator < 0.0 {
                return None;
            }
            continue;
        }
        let crossing = -numerator / denominator;
        if denominator > 0.0 {
            entry = entry.max(crossing);
        } else {
            exit = exit.min(crossing);
        }
    }
    (entry <= exit).then_some((entry, exit))
}

/// True when the segment passes through the inside of `screen` shrunk by
/// GEOMETRY_EPSILON_PX, so a segment that only touches the edge is clear.
pub(crate) fn segment_crosses_box(start: ScreenPoint, end: ScreenPoint, screen: BoxRect) -> bool {
    let inner = grown(screen, -GEOMETRY_EPSILON_PX);
    if inner.width <= 0.0 || inner.height <= 0.0 {
        return false;
    }
    let corners = rectangle_corners(inner);
    segment_inside_interval(start, end, &corners).is_some_and(|(entry, exit)| exit > entry)
}

/// The pieces of the segment that no occluding convex polygon covers, in order.
pub(crate) fn visible_pieces(
    start: ScreenPoint,
    end: ScreenPoint,
    occluders: &[&[ScreenPoint]],
) -> Vec<(ScreenPoint, ScreenPoint)> {
    let mut hidden: Vec<(f32, f32)> = occluders
        .iter()
        .filter_map(|polygon| segment_inside_interval(start, end, polygon))
        .filter(|(entry, exit)| exit > entry)
        .collect();
    hidden.sort_by(|first, second| first.0.total_cmp(&second.0));
    let at = |parameter: f32| ScreenPoint {
        x: start.x + (end.x - start.x) * parameter,
        y: start.y + (end.y - start.y) * parameter,
    };
    let mut pieces = Vec::new();
    let mut shown_from: f32 = 0.0;
    for (entry, exit) in hidden {
        if entry > shown_from {
            pieces.push((at(shown_from), at(entry)));
        }
        shown_from = shown_from.max(exit);
    }
    if shown_from < 1.0 {
        pieces.push((at(shown_from), at(1.0)));
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32) -> ScreenPoint {
        ScreenPoint { x, y }
    }

    const SQUARE: BoxRect = BoxRect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };

    #[test]
    fn a_segment_through_a_box_crosses_it_and_one_along_its_edge_does_not() {
        assert!(segment_crosses_box(
            point(-5.0, 5.0),
            point(15.0, 5.0),
            SQUARE
        ));
        assert!(!segment_crosses_box(
            point(-5.0, 0.0),
            point(15.0, 0.0),
            SQUARE
        ));
        assert!(!segment_crosses_box(
            point(-5.0, 20.0),
            point(15.0, 20.0),
            SQUARE
        ));
        assert!(segment_crosses_box(
            point(2.0, 2.0),
            point(3.0, 3.0),
            SQUARE
        ));
    }

    #[test]
    fn the_inside_interval_of_a_segment_entering_a_square_starts_at_the_edge() {
        let square = rectangle_corners(SQUARE);
        let (entry, exit) =
            segment_inside_interval(point(-10.0, 5.0), point(10.0, 5.0), &square).unwrap();
        assert!((entry - 0.5).abs() < 1e-5, "{entry}");
        assert!((exit - 1.0).abs() < 1e-5, "{exit}");
        assert!(segment_inside_interval(point(-10.0, 50.0), point(10.0, 50.0), &square).is_none());
    }

    #[test]
    fn a_segment_behind_a_square_shows_only_the_pieces_outside_it() {
        let square = rectangle_corners(SQUARE);
        let pieces = visible_pieces(point(-10.0, 5.0), point(20.0, 5.0), &[&square]);
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0].1.x - 0.0).abs() < 1e-4);
        assert!((pieces[1].0.x - 10.0).abs() < 1e-4);
        assert!(visible_pieces(point(2.0, 5.0), point(8.0, 5.0), &[&square]).is_empty());
    }

    #[test]
    fn point_in_convex_accepts_either_winding() {
        let clockwise = rectangle_corners(SQUARE);
        let mut counter = clockwise;
        counter.reverse();
        for polygon in [clockwise, counter] {
            assert!(point_in_convex(point(5.0, 5.0), &polygon));
            assert!(point_in_convex(point(10.0, 5.0), &polygon));
            assert!(!point_in_convex(point(11.0, 5.0), &polygon));
        }
    }
}
