//! Pipe and Tee tubes (section 12.3 rule 5): a wire or spine is a round tube lying on its
//! plane, with a flange ring at a dot end and a cone at an arrowed end. Everything here is
//! screen geometry from flat endpoints; the writer paints it.

use std::f32::consts::TAU;

use super::shapes::unit_direction;
use super::{ScreenPoint, project_point};

/// Radius of every tube, so a pipe's tag plane lies on its top at twice this over the floor.
pub const ISO_TUBE_RADIUS_PX: f32 = 5.0;
/// Radius of the ring at a dot end.
pub const FLANGE_RADIUS_PX: f32 = 7.0;
/// Length of the ring along the run.
pub const FLANGE_LENGTH_PX: f32 = 4.0;
/// Radius of a cone's base at an arrowed end.
pub const CONE_RADIUS_PX: f32 = 8.5;
/// Length of a cone from base to tip.
pub const CONE_LENGTH_PX: f32 = 12.0;
const CROSS_SECTION_POINTS: usize = 24;
/// Half the width of a band, a flat wide arrow on the plane.
pub const BAND_HALF_WIDTH_PX: f32 = 10.0;
/// Length of a band's arrowhead from base to tip.
pub const BAND_HEAD_LENGTH_PX: f32 = 24.0;
/// Half the width of a band's arrowhead base.
pub const BAND_HEAD_HALF_WIDTH_PX: f32 = 18.0;

/// A straight tube between two flat points, lying on the plane at `floor_z`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tube {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub floor_z: f32,
    pub radius: f32,
}

/// The body's screen corners: the lit half above the axis, the shaded half below it, and
/// the whole outline, each wound start to end and back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TubeBody {
    pub lit: [ScreenPoint; 4],
    pub shaded: [ScreenPoint; 4],
    pub outline: [ScreenPoint; 4],
}

impl Tube {
    pub fn axis_z(&self) -> f32 {
        self.floor_z + self.radius
    }

    /// The unit flat direction from start to end; None for a tube of no length.
    pub fn run(&self) -> Option<(f32, f32)> {
        unit_direction(self.start, self.end)
    }
}

fn add(point: ScreenPoint, offset: (f32, f32)) -> ScreenPoint {
    ScreenPoint {
        x: point.x + offset.0,
        y: point.y + offset.1,
    }
}

/// The circle of `radius` about the flat point `at`, at `axis_z`, in the vertical plane
/// across `run`, on screen.
pub(crate) fn cross_section(
    at: (f32, f32),
    run: (f32, f32),
    axis_z: f32,
    radius: f32,
    offset: ScreenPoint,
) -> Vec<ScreenPoint> {
    let across = (-run.1, run.0);
    (0..CROSS_SECTION_POINTS)
        .map(|index| {
            let angle = index as f32 * TAU / CROSS_SECTION_POINTS as f32;
            let (sin, cos) = angle.sin_cos();
            project_point(
                at.0 + across.0 * radius * cos,
                at.1 + across.1 * radius * cos,
                axis_z + radius * sin,
                offset,
            )
        })
        .collect()
}

/// The two silhouette offsets of a cross-section from its axis point on screen: the lit
/// one, on the tube's top side, then the shaded one.
fn silhouette_offsets(
    at: (f32, f32),
    run: (f32, f32),
    axis_z: f32,
    radius: f32,
) -> Option<((f32, f32), (f32, f32))> {
    let axis = project_point(at.0, at.1, axis_z, super::ZERO_OFFSET);
    let ahead = project_point(at.0 + run.0, at.1 + run.1, axis_z, super::ZERO_OFFSET);
    let screen_run = unit_direction((axis.x, axis.y), (ahead.x, ahead.y))?;
    let normal = (-screen_run.1, screen_run.0);
    let mut lowest = (0.0_f32, (0.0_f32, 0.0_f32));
    let mut highest = (0.0_f32, (0.0_f32, 0.0_f32));
    for point in cross_section(at, run, axis_z, radius, super::ZERO_OFFSET) {
        let offset = (point.x - axis.x, point.y - axis.y);
        let along = offset.0 * normal.0 + offset.1 * normal.1;
        if along < lowest.0 {
            lowest = (along, offset);
        }
        if along > highest.0 {
            highest = (along, offset);
        }
    }
    // The tube's top projects straight up on screen, so the lit side is the higher one.
    if lowest.1.1 <= highest.1.1 {
        Some((lowest.1, highest.1))
    } else {
        Some((highest.1, lowest.1))
    }
}

/// The body of a tube on screen; None for a tube of no length.
pub(crate) fn body(tube: &Tube, offset: ScreenPoint) -> Option<TubeBody> {
    let run = tube.run()?;
    let axis_z = tube.axis_z();
    let (lit, shaded) = silhouette_offsets(tube.start, run, axis_z, tube.radius)?;
    let start = project_point(tube.start.0, tube.start.1, axis_z, offset);
    let end = project_point(tube.end.0, tube.end.1, axis_z, offset);
    Some(TubeBody {
        lit: [add(start, lit), add(end, lit), end, start],
        shaded: [start, end, add(end, shaded), add(start, shaded)],
        outline: [
            add(start, lit),
            add(end, lit),
            add(end, shaded),
            add(start, shaded),
        ],
    })
}

/// True when the face across `run` at the far end of a run faces the viewer: the +x and +y
/// faces do.
pub(crate) fn faces_viewer(run: (f32, f32)) -> bool {
    run.0 + run.1 > 0.0
}

/// A cone from its flat base center to the tip `length` along `outward`: the lit and shaded
/// halves of its silhouette and the base circle.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cone {
    pub lit: [ScreenPoint; 3],
    pub shaded: [ScreenPoint; 3],
    pub base: Vec<ScreenPoint>,
    /// True when the base faces the viewer and paints over the cone.
    pub base_in_front: bool,
}

pub(crate) fn cone(
    tip: (f32, f32),
    outward: (f32, f32),
    (length, radius): (f32, f32),
    axis_z: f32,
    offset: ScreenPoint,
) -> Option<Cone> {
    let base_center = (tip.0 - outward.0 * length, tip.1 - outward.1 * length);
    let (lit, shaded) = silhouette_offsets(base_center, outward, axis_z, radius)?;
    let base_axis = project_point(base_center.0, base_center.1, axis_z, offset);
    let tip = project_point(tip.0, tip.1, axis_z, offset);
    Some(Cone {
        lit: [tip, add(base_axis, lit), base_axis],
        shaded: [tip, base_axis, add(base_axis, shaded)],
        base: cross_section(base_center, outward, axis_z, radius, offset),
        base_in_front: faces_viewer((-outward.0, -outward.1)),
    })
}

/// The ring at a dot end, centered on the flat point `center` along `run`.
pub(crate) fn flange(center: (f32, f32), run: (f32, f32), floor_z: f32) -> Tube {
    let half = FLANGE_LENGTH_PX / 2.0;
    Tube {
        start: (center.0 - run.0 * half, center.1 - run.1 * half),
        end: (center.0 + run.0 * half, center.1 + run.1 * half),
        // A ring sits on the same axis as its tube.
        floor_z: floor_z + ISO_TUBE_RADIUS_PX - FLANGE_RADIUS_PX,
        radius: FLANGE_RADIUS_PX,
    }
}

/// The body of a band on the plane at `z`: the flat rectangle between its ends, as four
/// screen points wound start to end and back.
pub(crate) fn band_body(
    (start, end): ((f32, f32), (f32, f32)),
    run: (f32, f32),
    z: f32,
    offset: ScreenPoint,
) -> [ScreenPoint; 4] {
    let across = (-run.1 * BAND_HALF_WIDTH_PX, run.0 * BAND_HALF_WIDTH_PX);
    [
        project_point(start.0 - across.0, start.1 - across.1, z, offset),
        project_point(end.0 - across.0, end.1 - across.1, z, offset),
        project_point(end.0 + across.0, end.1 + across.1, z, offset),
        project_point(start.0 + across.0, start.1 + across.1, z, offset),
    ]
}

/// A band's arrowhead on the plane at `z`: the flat triangle with its tip at `tip`
/// pointing along `outward`.
pub(crate) fn band_head(
    tip: (f32, f32),
    outward: (f32, f32),
    z: f32,
    offset: ScreenPoint,
) -> [ScreenPoint; 3] {
    let base = (
        tip.0 - outward.0 * BAND_HEAD_LENGTH_PX,
        tip.1 - outward.1 * BAND_HEAD_LENGTH_PX,
    );
    let across = (
        -outward.1 * BAND_HEAD_HALF_WIDTH_PX,
        outward.0 * BAND_HEAD_HALF_WIDTH_PX,
    );
    [
        project_point(tip.0, tip.1, z, offset),
        project_point(base.0 - across.0, base.1 - across.1, z, offset),
        project_point(base.0 + across.0, base.1 + across.1, z, offset),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso::{ISO_COS_30, ISO_SIN_30, ZERO_OFFSET};

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-3
    }

    #[test]
    fn a_cross_section_lies_in_the_vertical_plane_across_the_run() {
        let points = cross_section((10.0, 20.0), (1.0, 0.0), 5.0, 5.0, ZERO_OFFSET);
        assert_eq!(points.len(), CROSS_SECTION_POINTS);
        // Every point projects a flat point with x = 10, so x - y on screen spans y only.
        let axis = project_point(10.0, 20.0, 5.0, ZERO_OFFSET);
        for point in &points {
            let flat_y_offset = (axis.x - point.x) / ISO_COS_30;
            assert!(flat_y_offset.abs() <= 5.0 + 1e-3, "{point:?}");
            let z_offset = axis.y + flat_y_offset * ISO_SIN_30 - point.y;
            assert!(
                close(flat_y_offset.powi(2) + z_offset.powi(2), 25.0),
                "{point:?}"
            );
        }
    }

    #[test]
    fn a_body_along_x_has_its_lit_half_above_the_axis_and_a_known_width() {
        let tube = Tube {
            start: (0.0, 0.0),
            end: (100.0, 0.0),
            floor_z: 0.0,
            radius: 5.0,
        };
        let along_x = body(&tube, ZERO_OFFSET).unwrap();
        let axis_start = project_point(0.0, 0.0, 5.0, ZERO_OFFSET);
        assert_eq!(along_x.lit[3], axis_start);
        assert_eq!(along_x.shaded[0], axis_start);
        assert!(along_x.lit[0].y < axis_start.y);
        assert!(along_x.shaded[3].y > axis_start.y);
        // The silhouette half-width across the run is r * sqrt(1.5) on screen.
        let lit = along_x.lit[0];
        let half_width = ((lit.x - axis_start.x).powi(2) + (lit.y - axis_start.y).powi(2)).sqrt();
        assert!(close(half_width, 5.0 * 1.5_f32.sqrt()), "{half_width}");
        assert!(
            body(
                &Tube {
                    end: (0.0, 0.0),
                    ..tube
                },
                ZERO_OFFSET
            )
            .is_none()
        );
    }

    #[test]
    fn the_far_end_of_a_run_toward_x_or_y_faces_the_viewer() {
        assert!(faces_viewer((1.0, 0.0)));
        assert!(faces_viewer((0.0, 1.0)));
        assert!(!faces_viewer((-1.0, 0.0)));
        assert!(!faces_viewer((0.0, -1.0)));
    }

    #[test]
    fn a_cone_toward_x_hides_its_base_and_one_toward_minus_x_shows_it() {
        let toward = cone((50.0, 0.0), (1.0, 0.0), (12.0, 8.5), 5.0, ZERO_OFFSET).unwrap();
        assert!(!toward.base_in_front);
        assert_eq!(toward.lit[0], project_point(50.0, 0.0, 5.0, ZERO_OFFSET));
        assert_eq!(toward.lit[2], project_point(38.0, 0.0, 5.0, ZERO_OFFSET));
        assert_eq!(toward.base.len(), CROSS_SECTION_POINTS);
        let away = cone((50.0, 0.0), (-1.0, 0.0), (12.0, 8.5), 5.0, ZERO_OFFSET).unwrap();
        assert!(away.base_in_front);
    }

    #[test]
    fn a_band_lies_flat_on_its_plane_with_a_wide_head() {
        let body = band_body(((0.0, 0.0), (100.0, 0.0)), (1.0, 0.0), 6.0, ZERO_OFFSET);
        assert_eq!(body[0], project_point(0.0, -10.0, 6.0, ZERO_OFFSET));
        assert_eq!(body[2], project_point(100.0, 10.0, 6.0, ZERO_OFFSET));
        let head = band_head((130.0, 0.0), (1.0, 0.0), 6.0, ZERO_OFFSET);
        assert_eq!(head[0], project_point(130.0, 0.0, 6.0, ZERO_OFFSET));
        assert_eq!(head[1], project_point(106.0, -18.0, 6.0, ZERO_OFFSET));
        assert_eq!(head[2], project_point(106.0, 18.0, 6.0, ZERO_OFFSET));
    }

    #[test]
    fn a_flange_straddles_its_center_on_the_tube_axis() {
        let ring = flange((30.0, 10.0), (0.0, 1.0), 6.0);
        assert!(close(ring.start.1, 8.0) && close(ring.end.1, 12.0));
        assert!(close(ring.start.0, 30.0));
        assert!(close(ring.axis_z(), 6.0 + ISO_TUBE_RADIUS_PX));
    }
}
