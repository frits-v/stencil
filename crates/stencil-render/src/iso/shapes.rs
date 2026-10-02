//! Screen-space tests over boxes, convex polygons and segments, shared by the label
//! placement of section 12.4 and the check of section 12.7.

use stencil_layout::GEOMETRY_EPSILON_PX;

use super::ScreenPoint;

/// Unit direction from `from` to `to`, None when the two coincide.
pub(crate) fn unit_direction(from: (f32, f32), to: (f32, f32)) -> Option<(f32, f32)> {
    let delta_x = to.0 - from.0;
    let delta_y = to.1 - from.1;
    let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
    (length > f32::EPSILON).then(|| (delta_x / length, delta_y / length))
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

/// `count` points around an axis-aligned ellipse, clockwise on screen from the leftmost.
pub(crate) fn ellipse_points(
    center: ScreenPoint,
    radius_x: f32,
    radius_y: f32,
    count: usize,
) -> Vec<ScreenPoint> {
    (0..count)
        .map(|index| {
            let angle = std::f32::consts::PI + std::f32::consts::TAU * index as f32 / count as f32;
            ScreenPoint {
                x: center.x + radius_x * angle.cos(),
                y: center.y + radius_y * angle.sin(),
            }
        })
        .collect()
}

/// The convex hull of the points (Andrew's monotone chain), clockwise on screen, with
/// collinear boundary points dropped. Fewer than three distinct points come back as given.
pub(crate) fn convex_hull(points: &[ScreenPoint]) -> Vec<ScreenPoint> {
    let mut sorted: Vec<ScreenPoint> = points.to_vec();
    sorted.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    sorted.dedup_by(|a, b| (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6);
    if sorted.len() < 3 {
        return sorted;
    }
    let cross = |o: ScreenPoint, a: ScreenPoint, b: ScreenPoint| {
        (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
    };
    let mut lower: Vec<ScreenPoint> = Vec::new();
    for &point in &sorted {
        while let [.., second_last, last] = lower.as_slice()
            && cross(*second_last, *last, point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper: Vec<ScreenPoint> = Vec::new();
    for &point in sorted.iter().rev() {
        while let [.., second_last, last] = upper.as_slice()
            && cross(*second_last, *last, point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    // lower then upper runs counterclockwise in y-up terms, which is clockwise on screen.
    lower.extend(upper);
    lower
}

/// True when two convex polygons overlap by more than GEOMETRY_EPSILON_PX on the x and y
/// axes and on the unit normal of every edge of either (separating axes).
pub(crate) fn polygons_overlap(first: &[ScreenPoint], second: &[ScreenPoint]) -> bool {
    let mut axes = vec![(1.0, 0.0), (0.0, 1.0)];
    for polygon in [first, second] {
        for (index, start) in polygon.iter().enumerate() {
            let Some(end) = polygon.get((index + 1) % polygon.len()) else {
                continue;
            };
            if let Some((direction_x, direction_y)) =
                unit_direction((start.x, start.y), (end.x, end.y))
            {
                axes.push((-direction_y, direction_x));
            }
        }
    }
    axes.into_iter().all(|axis| {
        let (first_low, first_high) = axis_interval(first, axis);
        let (second_low, second_high) = axis_interval(second, axis);
        first_high.min(second_high) - first_low.max(second_low) > GEOMETRY_EPSILON_PX
    })
}

/// True when the segment passes through the inside of the convex polygon shrunk by the
/// epsilon: a segment that only touches or runs along an edge does not cross.
pub(crate) fn segment_crosses_convex(
    start: ScreenPoint,
    end: ScreenPoint,
    polygon: &[ScreenPoint],
) -> bool {
    let Some((enter, exit)) = segment_inside_interval(start, end, polygon) else {
        return false;
    };
    let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
    if (exit - enter) * length <= 2.0 * GEOMETRY_EPSILON_PX {
        return false;
    }
    let middle = (enter + exit) / 2.0;
    let point = ScreenPoint {
        x: start.x + middle * (end.x - start.x),
        y: start.y + middle * (end.y - start.y),
    };
    inward_edges(polygon).into_iter().all(|(on_edge, normal)| {
        let length = (normal.0 * normal.0 + normal.1 * normal.1).sqrt();
        length > 0.0
            && (normal.0 * (point.x - on_edge.x) + normal.1 * (point.y - on_edge.y)) / length
                > GEOMETRY_EPSILON_PX
    })
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

    fn square() -> [ScreenPoint; 4] {
        [
            point(0.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 10.0),
            point(0.0, 10.0),
        ]
    }

    #[test]
    fn a_segment_through_a_polygon_crosses_it_and_one_along_its_edge_does_not() {
        let square = square();
        assert!(segment_crosses_convex(
            point(-5.0, 5.0),
            point(15.0, 5.0),
            &square
        ));
        assert!(!segment_crosses_convex(
            point(-5.0, 0.0),
            point(15.0, 0.0),
            &square
        ));
        assert!(!segment_crosses_convex(
            point(-5.0, 20.0),
            point(15.0, 20.0),
            &square
        ));
        assert!(segment_crosses_convex(
            point(2.0, 2.0),
            point(3.0, 3.0),
            &square
        ));
    }

    #[test]
    fn polygons_overlap_when_they_share_area_and_not_when_they_touch() {
        let square = square();
        let touching = [
            point(10.0, 0.0),
            point(20.0, 0.0),
            point(20.0, 10.0),
            point(10.0, 10.0),
        ];
        let sheared = [
            point(5.0, 5.0),
            point(25.0, 15.0),
            point(15.0, 20.0),
            point(-5.0, 10.0),
        ];
        let apart = [
            point(30.0, 30.0),
            point(40.0, 30.0),
            point(40.0, 40.0),
            point(30.0, 40.0),
        ];
        assert!(!polygons_overlap(&square, &touching));
        assert!(polygons_overlap(&square, &sheared));
        assert!(!polygons_overlap(&square, &apart));
    }

    #[test]
    fn the_inside_interval_of_a_segment_entering_a_square_starts_at_the_edge() {
        let square = square();
        let (entry, exit) =
            segment_inside_interval(point(-10.0, 5.0), point(10.0, 5.0), &square).unwrap();
        assert!((entry - 0.5).abs() < 1e-5, "{entry}");
        assert!((exit - 1.0).abs() < 1e-5, "{exit}");
        assert!(segment_inside_interval(point(-10.0, 50.0), point(10.0, 50.0), &square).is_none());
    }

    #[test]
    fn a_segment_behind_a_square_shows_only_the_pieces_outside_it() {
        let square = square();
        let pieces = visible_pieces(point(-10.0, 5.0), point(20.0, 5.0), &[&square]);
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0].1.x - 0.0).abs() < 1e-4);
        assert!((pieces[1].0.x - 10.0).abs() < 1e-4);
        assert!(visible_pieces(point(2.0, 5.0), point(8.0, 5.0), &[&square]).is_empty());
    }

    #[test]
    fn point_in_convex_accepts_either_winding() {
        let clockwise = square();
        let mut counter = clockwise;
        counter.reverse();
        for polygon in [clockwise, counter] {
            assert!(point_in_convex(point(5.0, 5.0), &polygon));
            assert!(point_in_convex(point(10.0, 5.0), &polygon));
            assert!(!point_in_convex(point(11.0, 5.0), &polygon));
        }
    }
}
