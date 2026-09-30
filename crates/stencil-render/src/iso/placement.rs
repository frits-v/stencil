//! Moves a billboard to the first clear spot on its surface (section 12.4, rule 3): a zone
//! label onto open floor of its zone's top face, a card's icon and label to where no
//! stroke runs through them.

use stencil_layout::BoxRect;

use super::shapes::{
    grown, point_in_convex, rectangle_corners, rectangle_overlaps_polygon, rectangles_overlap,
    segment_crosses_box,
};
use super::{ScreenPoint, ZERO_OFFSET, project_point};

/// Room kept between a placed zone label and every stroke, block and other label.
pub const ISO_LABEL_CLEARANCE_PX: f32 = 6.0;
/// Room kept between a card's marks and every stroke and other label.
pub const ISO_CARD_CLEARANCE_PX: f32 = 3.0;
/// Pitch of the flat grid of candidate billboard centers.
const CANDIDATE_STEP_PX: f32 = 4.0;
/// The candidate grid is coarsened until it has at most this many points.
const CANDIDATES_MAX: usize = 4096;

/// Everything a billboard must stay clear of, in screen px before the offset.
#[derive(Debug, Default)]
pub(crate) struct Obstacles<'a> {
    /// Block silhouettes and the silhouettes of slabs nested in a zone.
    pub shapes: Vec<&'a [ScreenPoint]>,
    /// Visible slab edges, link paths, wires and spines.
    pub strokes: Vec<(ScreenPoint, ScreenPoint)>,
    /// Billboards already placed.
    pub boxes: Vec<BoxRect>,
}

/// Where the candidates are tried first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Order {
    /// Nearest the region's back corner first, which is also top of the screen first.
    FromBackCorner,
    /// Nearest the region's center first.
    FromCenter,
}

/// One billboard to place.
#[derive(Debug)]
pub(crate) struct Request<'a> {
    /// Flat rectangle whose points are the candidate centers.
    pub region: BoxRect,
    /// Height the candidate centers are projected at.
    pub z: f32,
    /// Screen size of the billboard box.
    pub size: (f32, f32),
    /// What the billboard inks, relative to its box's top-left corner.
    pub marks: &'a [BoxRect],
    /// A convex screen polygon the grown box must lie in, if any.
    pub within: Option<&'a [ScreenPoint]>,
    pub clearance: f32,
    pub order: Order,
}

impl Request<'_> {
    fn fits(&self, candidate: BoxRect, obstacles: &Obstacles<'_>) -> bool {
        let padded = grown(candidate, self.clearance);
        if let Some(polygon) = self.within
            && !rectangle_corners(padded)
                .into_iter()
                .all(|corner| point_in_convex(corner, polygon))
        {
            return false;
        }
        let clear_of_boxes = obstacles
            .boxes
            .iter()
            .all(|placed| !rectangles_overlap(padded, *placed));
        let clear_of_shapes = obstacles
            .shapes
            .iter()
            .all(|shape| !rectangle_overlaps_polygon(padded, shape));
        clear_of_boxes
            && clear_of_shapes
            && self.marks.iter().all(|mark| {
                let placed = grown(
                    BoxRect {
                        x: candidate.x + mark.x,
                        y: candidate.y + mark.y,
                        ..*mark
                    },
                    self.clearance,
                );
                obstacles
                    .strokes
                    .iter()
                    .all(|(start, end)| !segment_crosses_box(*start, *end, placed))
            })
    }
}

/// The candidate centers of a region in the request's order, at most CANDIDATES_MAX.
fn candidates(region: BoxRect, order: Order) -> Vec<(f32, f32)> {
    let mut step = CANDIDATE_STEP_PX;
    let count = |step: f32| {
        (
            (region.width / step).floor() as usize + 1,
            (region.height / step).floor() as usize + 1,
        )
    };
    let (mut columns, mut rows) = count(step);
    for _ in 0..16 {
        if columns.saturating_mul(rows) <= CANDIDATES_MAX {
            break;
        }
        step *= 2.0;
        (columns, rows) = count(step);
    }
    let (center_x, center_y) = (
        region.x + region.width / 2.0,
        region.y + region.height / 2.0,
    );
    // A centered grid holds the center itself; a corner grid holds the back corner.
    let (origin_x, origin_y) = match order {
        Order::FromBackCorner => (region.x, region.y),
        Order::FromCenter => (
            center_x - (region.width / 2.0 / step).floor() * step,
            center_y - (region.height / 2.0 / step).floor() * step,
        ),
    };
    let mut points = Vec::with_capacity(columns.saturating_mul(rows).min(CANDIDATES_MAX));
    for row in 0..rows {
        for column in 0..columns {
            let point = (
                origin_x + column as f32 * step,
                origin_y + row as f32 * step,
            );
            if point.0 <= region.right() && point.1 <= region.bottom() {
                points.push(point);
            }
        }
    }
    let key = |point: &(f32, f32)| match order {
        Order::FromBackCorner => (point.0 - region.x) + (point.1 - region.y),
        Order::FromCenter => (point.0 - center_x).powi(2) + (point.1 - center_y).powi(2),
    };
    points.sort_by(|first, second| key(first).total_cmp(&key(second)));
    points
}

/// The first candidate screen box that fits, or None when none does.
pub(crate) fn place(request: &Request<'_>, obstacles: &Obstacles<'_>) -> Option<BoxRect> {
    let (width, height) = request.size;
    candidates(request.region, request.order)
        .into_iter()
        .map(|center| {
            let anchor = project_point(center.0, center.1, request.z, ZERO_OFFSET);
            BoxRect {
                x: anchor.x - width / 2.0,
                y: anchor.y - height / 2.0,
                width,
                height,
            }
        })
        .find(|candidate| request.fits(*candidate, obstacles))
}

/// The four projected corners of a flat rectangle at `z`, back corner first.
pub(crate) fn top_face(bounds: BoxRect, z: f32) -> [ScreenPoint; 4] {
    [
        project_point(bounds.x, bounds.y, z, ZERO_OFFSET),
        project_point(bounds.right(), bounds.y, z, ZERO_OFFSET),
        project_point(bounds.right(), bounds.bottom(), z, ZERO_OFFSET),
        project_point(bounds.x, bounds.bottom(), z, ZERO_OFFSET),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZONE: BoxRect = BoxRect {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 200.0,
    };

    fn zone_label(size: (f32, f32), face: &[ScreenPoint]) -> Request<'_> {
        Request {
            region: ZONE,
            z: 6.0,
            size,
            marks: &[],
            within: Some(face),
            clearance: ISO_LABEL_CLEARANCE_PX,
            order: Order::FromBackCorner,
        }
    }

    #[test]
    fn an_empty_zone_takes_its_label_near_the_back_corner_and_inside_the_face() {
        let face = top_face(ZONE, 6.0);
        let placed = place(&zone_label((60.0, 16.0), &face), &Obstacles::default()).unwrap();
        let back = project_point(0.0, 0.0, 6.0, ZERO_OFFSET);
        assert!(placed.y > back.y, "{placed:?}");
        assert!(placed.y - back.y < 80.0, "{placed:?}");
    }

    #[test]
    fn a_label_moves_off_a_stroke_and_gives_up_when_none_is_clear() {
        let face = top_face(ZONE, 6.0);
        let request = Request {
            marks: &[BoxRect {
                x: 0.0,
                y: 0.0,
                width: 60.0,
                height: 16.0,
            }],
            ..zone_label((60.0, 16.0), &face)
        };
        let free = place(&request, &Obstacles::default()).unwrap();
        let across = Obstacles {
            strokes: vec![(
                ScreenPoint {
                    x: free.x - 100.0,
                    y: free.y + 8.0,
                },
                ScreenPoint {
                    x: free.right() + 100.0,
                    y: free.y + 8.0,
                },
            )],
            ..Obstacles::default()
        };
        let moved = place(&request, &across).unwrap();
        assert!(moved.y > free.y + 8.0, "{moved:?}");
        assert!(place(&zone_label((600.0, 16.0), &face), &Obstacles::default()).is_none());
    }

    #[test]
    fn a_centered_request_tries_the_region_center_first() {
        let request = Request {
            region: ZONE,
            z: 0.0,
            size: (10.0, 10.0),
            marks: &[],
            within: None,
            clearance: 0.0,
            order: Order::FromCenter,
        };
        let placed = place(&request, &Obstacles::default()).unwrap();
        let center = project_point(150.0, 100.0, 0.0, ZERO_OFFSET);
        assert!((placed.x + 5.0 - center.x).abs() < 1e-3);
        assert!((placed.y + 5.0 - center.y).abs() < 1e-3);
    }
}
