//! Moves a billboard to the first clear spot (section 12.4, rule 3): a card's icon and
//! label over its block where no stroke runs through them, a link tag along its link.

use stencil_layout::BoxRect;

use super::shapes::{grown, rectangle_overlaps_polygon, rectangles_overlap, segment_crosses_box};
use super::{ScreenPoint, ZERO_OFFSET, project_point};

/// Room kept between a card's marks or a link tag and every stroke, block and other label.
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

/// One billboard to place.
#[derive(Debug)]
pub(crate) struct Request<'a> {
    /// Flat rectangle whose points are the candidate centers.
    pub region: BoxRect,
    /// Height the candidate centers are projected at.
    pub z: f32,
    /// Screen size of the billboard box.
    pub size: (f32, f32),
    /// The point of the box that stands on a candidate, from the box's top-left corner.
    pub anchor: (f32, f32),
    /// What the billboard inks, relative to its box's top-left corner.
    pub marks: &'a [BoxRect],
    pub clearance: f32,
}

impl Request<'_> {
    fn fits(&self, candidate: BoxRect, obstacles: &Obstacles<'_>) -> bool {
        let padded = grown(candidate, self.clearance);
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

/// The candidate centers of a region, nearest its center first, at most CANDIDATES_MAX.
fn candidates(region: BoxRect) -> Vec<(f32, f32)> {
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
    // The grid holds the center itself.
    let origin_x = center_x - (region.width / 2.0 / step).floor() * step;
    let origin_y = center_y - (region.height / 2.0 / step).floor() * step;
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
    let key = |point: &(f32, f32)| (point.0 - center_x).powi(2) + (point.1 - center_y).powi(2);
    points.sort_by(|first, second| key(first).total_cmp(&key(second)));
    points
}

/// The first candidate screen box that fits, or None when none does.
pub(crate) fn place(request: &Request<'_>, obstacles: &Obstacles<'_>) -> Option<BoxRect> {
    let (width, height) = request.size;
    let (across, down) = request.anchor;
    candidates(request.region)
        .into_iter()
        .map(|center| {
            let anchor = project_point(center.0, center.1, request.z, ZERO_OFFSET);
            BoxRect {
                x: anchor.x - across,
                y: anchor.y - down,
                width,
                height,
            }
        })
        .find(|candidate| request.fits(*candidate, obstacles))
}

/// The first opaque box of `size` centered on one of `centers`, in order, that clears every
/// obstacle by ISO_CARD_CLEARANCE_PX, with the height stored beside that center; None when
/// none does. A link tag is placed this way along its link.
pub(crate) fn place_centered(
    size: (f32, f32),
    centers: &[(ScreenPoint, f32)],
    obstacles: &Obstacles<'_>,
) -> Option<(BoxRect, f32)> {
    let (width, height) = size;
    let whole = [BoxRect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    }];
    let request = Request {
        region: whole[0],
        z: 0.0,
        size,
        anchor: (width / 2.0, height / 2.0),
        marks: &whole,
        clearance: ISO_CARD_CLEARANCE_PX,
    };
    centers.iter().find_map(|(center, z)| {
        let candidate = BoxRect {
            x: center.x - width / 2.0,
            y: center.y - height / 2.0,
            width,
            height,
        };
        request
            .fits(candidate, obstacles)
            .then_some((candidate, *z))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGION: BoxRect = BoxRect {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 200.0,
    };

    fn card(size: (f32, f32), marks: &[BoxRect]) -> Request<'_> {
        Request {
            region: REGION,
            z: 0.0,
            size,
            anchor: (size.0 / 2.0, size.1 / 2.0),
            marks,
            clearance: ISO_CARD_CLEARANCE_PX,
        }
    }

    #[test]
    fn a_request_tries_the_region_center_first() {
        let placed = place(&card((10.0, 10.0), &[]), &Obstacles::default()).unwrap();
        let center = project_point(150.0, 100.0, 0.0, ZERO_OFFSET);
        assert!((placed.x + 5.0 - center.x).abs() < 1e-3);
        assert!((placed.y + 5.0 - center.y).abs() < 1e-3);
    }

    #[test]
    fn a_label_moves_off_a_stroke() {
        let marks = [BoxRect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 16.0,
        }];
        let request = card((60.0, 16.0), &marks);
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
        assert!(
            moved.y > free.y + 8.0 || moved.bottom() < free.y + 8.0,
            "{moved:?}"
        );
    }

    #[test]
    fn a_tag_takes_the_first_clear_center_along_its_link() {
        let centers = [
            (ScreenPoint { x: 0.0, y: 0.0 }, 0.0),
            (ScreenPoint { x: 100.0, y: 0.0 }, 6.0),
        ];
        let blocked = Obstacles {
            boxes: vec![BoxRect {
                x: -10.0,
                y: -10.0,
                width: 20.0,
                height: 20.0,
            }],
            ..Obstacles::default()
        };
        let (screen, z) = place_centered((40.0, 20.0), &centers, &blocked).unwrap();
        assert_eq!((screen.x, screen.y, z), (80.0, -10.0, 6.0));
        assert!(place_centered((40.0, 20.0), &centers[..1], &blocked).is_none());
    }
}
