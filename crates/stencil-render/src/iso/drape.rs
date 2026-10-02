//! Link paths over the slabs (section 12.3, rule 7): a routed polyline lifted to the top of
//! the highest slab it runs over, so it lies on the highest terrain it crosses and floats
//! over lower floor by at most that difference, and each end cut back to where it enters
//! its endpoint block on screen.

use stencil_layout::{BoxRect, GEOMETRY_EPSILON_PX};
use stencil_model::PagePoint;

use super::shapes::{point_in_convex, segment_inside_interval};
use super::{IsoPoint, ScreenPoint, ZERO_OFFSET, project_point};

/// A cut closer than this to a segment end is dropped.
const CUT_EPSILON: f32 = 1e-4;

/// How far a link end that stops short of its block's outline reaches along its last
/// stretch to meet it, in zoomed flat px: more than the gap between a sprite's hull and the
/// footprint edge where its route ends.
const LAND_REACH_PX: f32 = 64.0;

/// A slab footprint and the z of its top face. Zones drawn as a ring (height 0) are not
/// terrain.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Terrain {
    pub bounds: BoxRect,
    pub top: f32,
}

fn contains(bounds: BoxRect, x: f32, y: f32) -> bool {
    x >= bounds.x - GEOMETRY_EPSILON_PX
        && x <= bounds.right() + GEOMETRY_EPSILON_PX
        && y >= bounds.y - GEOMETRY_EPSILON_PX
        && y <= bounds.bottom() + GEOMETRY_EPSILON_PX
}

/// The top of the highest slab whose footprint holds the flat point, edges included, or
/// the ground.
pub(crate) fn ground_z(terrain: &[Terrain], x: f32, y: f32) -> f32 {
    terrain
        .iter()
        .filter(|area| contains(area.bounds, x, y))
        .map(|area| area.top)
        .fold(0.0, f32::max)
}

/// Parameters in (0, 1) where the segment crosses an edge of a terrain footprint.
fn cuts(start: PagePoint, end: PagePoint, terrain: &[Terrain]) -> Vec<f32> {
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let mut cuts = vec![0.0, 1.0];
    for area in terrain {
        let bounds = area.bounds;
        if delta_x.abs() > f32::EPSILON {
            for edge_x in [bounds.x, bounds.right()] {
                let parameter = (edge_x - start.x) / delta_x;
                let crossing_y = start.y + parameter * delta_y;
                if parameter > CUT_EPSILON
                    && parameter < 1.0 - CUT_EPSILON
                    && crossing_y >= bounds.y - GEOMETRY_EPSILON_PX
                    && crossing_y <= bounds.bottom() + GEOMETRY_EPSILON_PX
                {
                    cuts.push(parameter);
                }
            }
        }
        if delta_y.abs() > f32::EPSILON {
            for edge_y in [bounds.y, bounds.bottom()] {
                let parameter = (edge_y - start.y) / delta_y;
                let crossing_x = start.x + parameter * delta_x;
                if parameter > CUT_EPSILON
                    && parameter < 1.0 - CUT_EPSILON
                    && crossing_x >= bounds.x - GEOMETRY_EPSILON_PX
                    && crossing_x <= bounds.right() + GEOMETRY_EPSILON_PX
                {
                    cuts.push(parameter);
                }
            }
        }
    }
    cuts.sort_by(f32::total_cmp);
    cuts.dedup_by(|later, earlier| (*later - *earlier).abs() < CUT_EPSILON);
    cuts
}

/// The top of the highest terrain under any stretch of the polyline: each segment is
/// sampled at the midpoint of every span between its edge crossings.
pub(crate) fn route_z(points: &[PagePoint], terrain: &[Terrain]) -> f32 {
    let mut highest: f32 = 0.0;
    for pair in points.windows(2) {
        let (Some(&start), Some(&end)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        for span in cuts(start, end, terrain).windows(2) {
            let (Some(&from), Some(&to)) = (span.first(), span.get(1)) else {
                continue;
            };
            let middle = (from + to) / 2.0;
            let x = start.x + middle * (end.x - start.x);
            let y = start.y + middle * (end.y - start.y);
            highest = highest.max(ground_z(terrain, x, y));
        }
    }
    if let Some(point) = points.first() {
        highest = highest.max(ground_z(terrain, point.x, point.y));
    }
    highest
}

/// Drops a point that lies on the straight run between its neighbours.
fn simplify(path: Vec<IsoPoint>) -> Vec<IsoPoint> {
    let mut kept: Vec<IsoPoint> = Vec::with_capacity(path.len());
    for point in path {
        if kept.len() >= 2
            && let (Some(&middle), Some(&first)) = (kept.last(), kept.get(kept.len() - 2))
        {
            let cross = (middle.x - first.x) * (point.y - first.y)
                - (middle.y - first.y) * (point.x - first.x);
            let forward = (middle.x - first.x) * (point.x - middle.x)
                + (middle.y - first.y) * (point.y - middle.y);
            if cross.abs() <= GEOMETRY_EPSILON_PX && forward >= 0.0 {
                kept.pop();
            }
        }
        kept.push(point);
    }
    kept
}

/// The routed polyline lifted to the highest terrain it runs over.
pub(crate) fn drape(points: &[PagePoint], terrain: &[Terrain]) -> Vec<IsoPoint> {
    let z = route_z(points, terrain);
    simplify(
        points
            .iter()
            .map(|point| IsoPoint {
                x: point.x,
                y: point.y,
                z,
            })
            .collect(),
    )
}

fn screen(point: IsoPoint) -> ScreenPoint {
    project_point(point.x, point.y, point.z, ZERO_OFFSET)
}

/// Cuts the end of the path back to where it enters `silhouette` on screen. A link that
/// reaches a block through a face the viewer cannot see would otherwise draw its last
/// stretch across the block's top face.
pub(crate) fn trim_end_at(path: &mut Vec<IsoPoint>, silhouette: &[ScreenPoint]) {
    for _ in 0..path.len() {
        let count = path.len();
        if count < 2 {
            return;
        }
        let (Some(&last), Some(&previous)) = (path.last(), path.get(count - 2)) else {
            return;
        };
        let Some((entry, _)) = segment_inside_interval(screen(previous), screen(last), silhouette)
        else {
            return;
        };
        if entry > CUT_EPSILON {
            if let Some(slot) = path.last_mut() {
                *slot = IsoPoint {
                    x: previous.x + entry * (last.x - previous.x),
                    y: previous.y + entry * (last.y - previous.y),
                    z: previous.z + entry * (last.z - previous.z),
                };
            }
            return;
        }
        if count == 2 {
            return;
        }
        path.pop();
    }
}

/// Extends the end of a path that stops outside `silhouette` along its last stretch until it
/// meets the silhouette, by at most LAND_REACH_PX. A route ends on its node's footprint
/// edge, and a sprite's outline is narrower than its footprint; left alone, the link would
/// stop in the air beside the figure. A path that would not meet the silhouette within
/// reach is left for `iso_link_ends` to report.
pub(crate) fn land_end_at(path: &mut [IsoPoint], silhouette: &[ScreenPoint]) {
    let Some((tip, direction)) = end_direction(path) else {
        return;
    };
    if point_in_convex(screen(tip), silhouette) {
        return;
    }
    let far = IsoPoint {
        x: tip.x + direction.0 * LAND_REACH_PX,
        y: tip.y + direction.1 * LAND_REACH_PX,
        z: tip.z,
    };
    let Some((entry, _)) = segment_inside_interval(screen(tip), screen(far), silhouette) else {
        return;
    };
    if let Some(slot) = path.last_mut() {
        *slot = IsoPoint {
            x: tip.x + entry * (far.x - tip.x),
            y: tip.y + entry * (far.y - tip.y),
            z: tip.z,
        };
    }
}

/// `land_end_at` for the first point of the path.
pub(crate) fn land_start_at(path: &mut [IsoPoint], silhouette: &[ScreenPoint]) {
    path.reverse();
    land_end_at(path, silhouette);
    path.reverse();
}

/// `trim_end_at` for the first point of the path.
pub(crate) fn trim_start_at(path: &mut Vec<IsoPoint>, silhouette: &[ScreenPoint]) {
    path.reverse();
    trim_end_at(path, silhouette);
    path.reverse();
}

/// The unit flat direction of the path's last horizontal stretch, pointing at its end, and
/// the end point; None when the path has no horizontal stretch.
pub(crate) fn end_direction(path: &[IsoPoint]) -> Option<(IsoPoint, (f32, f32))> {
    let tip = *path.last()?;
    path.iter().rev().skip(1).find_map(|point| {
        super::shapes::unit_direction((point.x, point.y), (tip.x, tip.y))
            .map(|direction| (tip, direction))
    })
}

/// `end_direction` for the first point of the path.
pub(crate) fn start_direction(path: &[IsoPoint]) -> Option<(IsoPoint, (f32, f32))> {
    let tip = *path.first()?;
    path.iter().skip(1).find_map(|point| {
        super::shapes::unit_direction((point.x, point.y), (tip.x, tip.y))
            .map(|direction| (tip, direction))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slab(x: f32, y: f32, width: f32, height: f32, top: f32) -> Terrain {
        Terrain {
            bounds: BoxRect {
                x,
                y,
                width,
                height,
            },
            top,
        }
    }

    fn page_point(x: f32, y: f32) -> PagePoint {
        PagePoint { x, y }
    }

    #[test]
    fn a_path_that_stops_short_of_a_silhouette_reaches_it_along_its_last_stretch() {
        // A square on screen, 100 wide, with its left edge at screen x 100.
        let square = [
            ScreenPoint { x: 100.0, y: 0.0 },
            ScreenPoint { x: 200.0, y: 0.0 },
            ScreenPoint { x: 200.0, y: 100.0 },
            ScreenPoint { x: 100.0, y: 100.0 },
        ];
        // Along flat x the screen x grows by cos 30 per px; the end at flat x 100 projects
        // to screen (86.6, 50) and must reach the edge at x 100.
        let mut path = vec![
            IsoPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            IsoPoint {
                x: 100.0,
                y: 0.0,
                z: 0.0,
            },
        ];
        land_end_at(&mut path, &square);
        let end = screen(path[1]);
        assert!((end.x - 100.0).abs() < 1e-3, "{end:?}");
        assert_eq!(path[1].y, 0.0);

        // A path pointing away from the silhouette, or already inside it, is unchanged.
        let mut away = vec![
            IsoPoint {
                x: 100.0,
                y: 0.0,
                z: 0.0,
            },
            IsoPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ];
        land_end_at(&mut away, &square);
        assert_eq!(away[1].x, 0.0);
        let mut inside = vec![
            IsoPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            IsoPoint {
                x: 150.0,
                y: 0.0,
                z: 0.0,
            },
        ];
        land_end_at(&mut inside, &square);
        assert_eq!(inside[1].x, 150.0);
    }

    #[test]
    fn a_path_over_two_slabs_lies_on_the_higher_one_throughout() {
        let terrain = [
            slab(0.0, 0.0, 100.0, 100.0, 6.0),
            slab(200.0, 0.0, 100.0, 100.0, 6.0),
            slab(210.0, 10.0, 80.0, 80.0, 12.0),
        ];
        let path = drape(&[page_point(50.0, 50.0), page_point(250.0, 50.0)], &terrain);
        assert_eq!(path.len(), 2);
        assert!(path.iter().all(|point| point.z == 12.0), "{path:?}");
        let low = drape(&[page_point(50.0, 50.0), page_point(150.0, 50.0)], &terrain);
        assert!(low.iter().all(|point| point.z == 6.0), "{low:?}");
        let ground = drape(
            &[page_point(120.0, 50.0), page_point(180.0, 50.0)],
            &terrain,
        );
        assert!(ground.iter().all(|point| point.z == 0.0), "{ground:?}");
    }

    #[test]
    fn a_path_on_one_slab_keeps_its_corners_and_nothing_else() {
        let terrain = [slab(0.0, 0.0, 100.0, 100.0, 6.0)];
        let path = drape(
            &[
                page_point(10.0, 10.0),
                page_point(50.0, 10.0),
                page_point(50.0, 80.0),
            ],
            &terrain,
        );
        assert_eq!(path.len(), 3);
        assert!(path.iter().all(|point| point.z == 6.0));
    }

    /// A flat point that projects to screen (x, y): along x = -y the map sends x to
    /// 2 * cos 30 * x across and -z down.
    fn at_screen(x: f32, y: f32) -> IsoPoint {
        let flat = x / (2.0 * super::super::ISO_COS_30);
        IsoPoint {
            x: flat,
            y: -flat,
            z: -y,
        }
    }

    #[test]
    fn trimming_stops_the_path_on_the_silhouette_edge() {
        let square = [
            ScreenPoint { x: 0.0, y: 0.0 },
            ScreenPoint { x: 10.0, y: 0.0 },
            ScreenPoint { x: 10.0, y: 5.0 },
            ScreenPoint { x: 10.0, y: 10.0 },
            ScreenPoint { x: 0.0, y: 10.0 },
            ScreenPoint { x: 0.0, y: 5.0 },
        ];
        let mut path = vec![at_screen(-20.0, 5.0), at_screen(5.0, 5.0)];
        trim_end_at(&mut path, &square);
        let end = screen(path[1]);
        assert!(end.x.abs() < 1e-3 && (end.y - 5.0).abs() < 1e-3, "{end:?}");

        let mut touching = vec![at_screen(-20.0, 5.0), at_screen(0.0, 5.0)];
        trim_end_at(&mut touching, &square);
        assert_eq!(touching.len(), 2);
        assert!(screen(touching[1]).x.abs() < 1e-3);

        let mut from_inside = vec![at_screen(5.0, 5.0), at_screen(-20.0, 5.0)];
        trim_start_at(&mut from_inside, &square);
        assert!(screen(from_inside[0]).x.abs() < 1e-3);
    }
}
