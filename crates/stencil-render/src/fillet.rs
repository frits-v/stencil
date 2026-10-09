//! The rounded bends of a routed link (section 11.6). Both writers draw the bend at a corner
//! between two perpendicular legs from the same fillet, so flat and iso clamp alike.

use stencil_layout::GEOMETRY_EPSILON_PX;

/// The largest |cos| between two legs that still counts as a right angle.
const PERPENDICULAR_COS_MAX: f32 = 1e-3;

/// One rounded bend, in flat (page or floor) coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fillet {
    /// Where the bend leaves the incoming leg, `radius` short of the corner.
    pub start: (f32, f32),
    /// Where the bend joins the outgoing leg, `radius` past the corner.
    pub end: (f32, f32),
    pub radius: f32,
    /// The turn is clockwise with y pointing down: the SVG sweep flag 1.
    pub clockwise: bool,
}

/// The bend at `corner` between the legs from `previous` and to `next`, with its radius
/// clamped to half the shorter leg, so two bends on one leg never overlap. None when the
/// clamped radius is nothing or the legs are not perpendicular; that corner stays square.
pub(crate) fn fillet(
    previous: (f32, f32),
    corner: (f32, f32),
    next: (f32, f32),
    corner_px: f32,
) -> Option<Fillet> {
    let incoming = (corner.0 - previous.0, corner.1 - previous.1);
    let outgoing = (next.0 - corner.0, next.1 - corner.1);
    let incoming_length = incoming.0.hypot(incoming.1);
    let outgoing_length = outgoing.0.hypot(outgoing.1);
    let radius = corner_px.min(incoming_length.min(outgoing_length) / 2.0);
    if !radius.is_finite() || radius < GEOMETRY_EPSILON_PX {
        return None;
    }
    let incoming = (incoming.0 / incoming_length, incoming.1 / incoming_length);
    let outgoing = (outgoing.0 / outgoing_length, outgoing.1 / outgoing_length);
    let cos = incoming.0 * outgoing.0 + incoming.1 * outgoing.1;
    if cos.abs() > PERPENDICULAR_COS_MAX {
        return None;
    }
    Some(Fillet {
        start: (
            corner.0 - incoming.0 * radius,
            corner.1 - incoming.1 * radius,
        ),
        end: (
            corner.0 + outgoing.0 * radius,
            corner.1 + outgoing.1 * radius,
        ),
        radius,
        clockwise: incoming.0 * outgoing.1 - incoming.1 * outgoing.0 > 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_right_turn_is_clockwise_and_takes_the_full_radius_on_long_legs() {
        let bend = fillet((0.0, 0.0), (100.0, 0.0), (100.0, 100.0), 6.0).unwrap();
        assert_eq!(bend.start, (94.0, 0.0));
        assert_eq!(bend.end, (100.0, 6.0));
        assert_eq!(bend.radius, 6.0);
        assert!(bend.clockwise);
        let left = fillet((0.0, 0.0), (100.0, 0.0), (100.0, -100.0), 6.0).unwrap();
        assert!(!left.clockwise);
    }

    #[test]
    fn the_radius_clamps_to_half_the_shorter_leg() {
        let bend = fillet((0.0, 0.0), (100.0, 0.0), (100.0, 8.0), 6.0).unwrap();
        assert_eq!(bend.radius, 4.0);
        assert_eq!(bend.start, (96.0, 0.0));
        assert_eq!(bend.end, (100.0, 4.0));
    }

    #[test]
    fn a_zero_leg_a_zero_corner_or_a_straight_run_stays_square() {
        assert_eq!(fillet((0.0, 0.0), (100.0, 0.0), (100.0, 0.0), 6.0), None);
        assert_eq!(fillet((0.0, 0.0), (100.0, 0.0), (100.0, 50.0), 0.0), None);
        assert_eq!(fillet((0.0, 0.0), (50.0, 0.0), (100.0, 0.0), 6.0), None);
        assert_eq!(fillet((0.0, 0.0), (50.0, 0.0), (0.0, 0.0), 6.0), None);
    }
}
