//! Link paths over the slabs (section 12.3, rule 7): a routed polyline split where it
//! crosses a slab edge, each piece at the top of the highest slab under it, a vertical
//! riser at every change of height, and each end cut back to where it enters its endpoint
//! block on screen.

use stencil_layout::{BoxRect, GEOMETRY_EPSILON_PX};
use stencil_model::PagePoint;

use super::shapes::segment_inside_interval;
use super::{IsoPoint, ScreenPoint, ZERO_OFFSET, project_point};

/// A cut closer than this to a segment end is dropped.
const CUT_EPSILON: f32 = 1e-4;

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

fn same_place(first: IsoPoint, x: f32, y: f32) -> bool {
    (first.x - x).abs() <= GEOMETRY_EPSILON_PX && (first.y - y).abs() <= GEOMETRY_EPSILON_PX
}

/// Appends a point, skipping a repeat; a point at the last point's place but another
/// height is a riser.
fn push_point(path: &mut Vec<IsoPoint>, x: f32, y: f32, z: f32) {
    if let Some(last) = path.last()
        && same_place(*last, x, y)
        && (last.z - z).abs() <= GEOMETRY_EPSILON_PX
    {
        return;
    }
    path.push(IsoPoint { x, y, z });
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

/// Drops a point that lies on the straight run between its neighbours at their height.
fn simplify(path: Vec<IsoPoint>) -> Vec<IsoPoint> {
    let mut kept: Vec<IsoPoint> = Vec::with_capacity(path.len());
    for point in path {
        if kept.len() >= 2
            && let (Some(&middle), Some(&first)) = (kept.last(), kept.get(kept.len() - 2))
        {
            let flat = (first.z - middle.z).abs() <= GEOMETRY_EPSILON_PX
                && (middle.z - point.z).abs() <= GEOMETRY_EPSILON_PX;
            let cross = (middle.x - first.x) * (point.y - first.y)
                - (middle.y - first.y) * (point.x - first.x);
            let forward = (middle.x - first.x) * (point.x - middle.x)
                + (middle.y - first.y) * (point.y - middle.y);
            if flat && cross.abs() <= GEOMETRY_EPSILON_PX && forward >= 0.0 {
                kept.pop();
            }
        }
        kept.push(point);
    }
    kept
}

/// The routed polyline laid over the terrain.
pub(crate) fn drape(points: &[PagePoint], terrain: &[Terrain]) -> Vec<IsoPoint> {
    let mut path: Vec<IsoPoint> = Vec::with_capacity(points.len() * 2);
    for pair in points.windows(2) {
        let (Some(&start), Some(&end)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let at = |parameter: f32| {
            (
                start.x + parameter * (end.x - start.x),
                start.y + parameter * (end.y - start.y),
            )
        };
        for span in cuts(start, end, terrain).windows(2) {
            let (Some(&from), Some(&to)) = (span.first(), span.get(1)) else {
                continue;
            };
            let middle = at((from + to) / 2.0);
            let z = ground_z(terrain, middle.0, middle.1);
            let (from_x, from_y) = at(from);
            let (to_x, to_y) = at(to);
            push_point(&mut path, from_x, from_y, z);
            push_point(&mut path, to_x, to_y, z);
        }
    }
    let mut path = simplify(path);
    move_hidden_risers(&mut path);
    path.dedup_by(|later, earlier| {
        same_place(*earlier, later.x, later.y) && (earlier.z - later.z).abs() <= GEOMETRY_EPSILON_PX
    });
    path
}

/// A riser at a slab's back or left edge stands behind a face the viewer cannot see: on
/// screen the lower piece would run across the slab's top for one step of height before
/// climbing. The lower piece is cut back by that step, to where it passes under the slab's
/// top edge on screen, and the two pieces no longer meet: the writer lifts the pen between
/// them. The lower piece lies on the +x or +y side of such an edge.
fn move_hidden_risers(path: &mut [IsoPoint]) {
    for index in 0..path.len().saturating_sub(1) {
        let (Some(&first), Some(&second)) = (path.get(index), path.get(index + 1)) else {
            continue;
        };
        if !same_place(first, second.x, second.y)
            || (first.z - second.z).abs() <= GEOMETRY_EPSILON_PX
        {
            continue;
        }
        let neighbour = if first.z < second.z {
            index.checked_sub(1).and_then(|at| path.get(at))
        } else {
            path.get(index + 2)
        };
        let Some(&neighbour) = neighbour else {
            continue;
        };
        let Some((dx, dy)) =
            super::shapes::unit_direction((neighbour.x, neighbour.y), (first.x, first.y))
        else {
            continue;
        };
        if dx <= GEOMETRY_EPSILON_PX && dy <= GEOMETRY_EPSILON_PX {
            continue;
        }
        let reach = (first.x - neighbour.x).hypot(first.y - neighbour.y);
        let step = (first.z - second.z).abs().min(reach);
        let lower = if first.z < second.z { index } else { index + 1 };
        if let Some(point) = path.get_mut(lower) {
            point.x -= dx * step;
            point.y -= dy * step;
        }
    }
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
    fn a_path_from_one_slab_to_another_steps_down_and_up_at_the_edges() {
        let terrain = [
            slab(0.0, 0.0, 100.0, 100.0, 6.0),
            slab(200.0, 0.0, 100.0, 100.0, 6.0),
            slab(210.0, 10.0, 80.0, 80.0, 12.0),
        ];
        let path = drape(&[page_point(50.0, 50.0), page_point(250.0, 50.0)], &terrain);
        let heights: Vec<(f32, f32)> = path.iter().map(|point| (point.x, point.z)).collect();
        assert_eq!(
            heights,
            vec![
                (50.0, 6.0),
                (100.0, 6.0),
                (100.0, 0.0),
                (194.0, 0.0),
                (200.0, 6.0),
                (204.0, 6.0),
                (210.0, 12.0),
                (250.0, 12.0),
            ]
        );
    }

    #[test]
    fn a_path_passes_under_a_hidden_edge_and_climbs_a_visible_one() {
        let terrain = [slab(100.0, 0.0, 100.0, 100.0, 12.0)];
        let onto = drape(&[page_point(50.0, 50.0), page_point(150.0, 50.0)], &terrain);
        let places: Vec<(f32, f32)> = onto.iter().map(|point| (point.x, point.z)).collect();
        assert_eq!(
            places,
            vec![(50.0, 0.0), (88.0, 0.0), (100.0, 12.0), (150.0, 12.0)]
        );
        let off = drape(&[page_point(150.0, 50.0), page_point(50.0, 50.0)], &terrain);
        let places: Vec<(f32, f32)> = off.iter().map(|point| (point.x, point.z)).collect();
        assert_eq!(
            places,
            vec![(150.0, 12.0), (100.0, 12.0), (88.0, 0.0), (50.0, 0.0)]
        );
        let back = drape(
            &[page_point(150.0, -50.0), page_point(150.0, 50.0)],
            &terrain,
        );
        let places: Vec<(f32, f32)> = back.iter().map(|point| (point.y, point.z)).collect();
        assert_eq!(
            places,
            vec![(-50.0, 0.0), (-12.0, 0.0), (0.0, 12.0), (50.0, 12.0)]
        );
        let front = drape(
            &[page_point(250.0, 50.0), page_point(150.0, 50.0)],
            &terrain,
        );
        let places: Vec<(f32, f32)> = front.iter().map(|point| (point.x, point.z)).collect();
        assert_eq!(
            places,
            vec![(250.0, 0.0), (200.0, 0.0), (200.0, 12.0), (150.0, 12.0)]
        );
        let short = drape(&[page_point(95.0, 50.0), page_point(150.0, 50.0)], &terrain);
        let places: Vec<(f32, f32)> = short.iter().map(|point| (point.x, point.z)).collect();
        assert_eq!(places, vec![(95.0, 0.0), (100.0, 12.0), (150.0, 12.0)]);
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
