//! Drawn item sprites (section 12.3 rule 8): the figure, laptop and phone forms, each
//! composed from boxes and round solids inside the item's footprint. The parts here are
//! flat boxes and heights the writer paints and the projection outlines.

use stencil_layout::BoxRect;

use super::shapes::{convex_hull, ellipse_points};
use super::{ScreenPoint, ZERO_OFFSET, ellipse_radii, project_point, silhouette};

/// A figure's torso is a cylinder of this share of the footprint side.
const TORSO_SHARE: f32 = 0.62;
/// A figure's torso takes this share of its height; the head sphere takes the rest.
const TORSO_HEIGHT_SHARE: f32 = 0.56;
/// Gap between the torso top and the head.
const NECK_PX: f32 = 2.0;
/// A laptop's base plate height.
pub const LAPTOP_BASE_PX: f32 = 6.0;
/// Depth of a laptop screen, along y.
const SLAB_DEPTH_PX: f32 = 4.0;
/// Depth of a phone, along y.
const PHONE_DEPTH_PX: f32 = 6.0;
/// A phone's width as a share of its footprint side.
const PHONE_WIDTH_SHARE: f32 = 0.6;
/// Inset of a screen panel from the edges of the face it lies on.
pub const SCREEN_INSET_PX: f32 = 3.0;
/// Points sampled around each ellipse and circle of an outline.
const OUTLINE_POINTS: usize = 16;

/// A figure: its torso footprint and top, and its head center height and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Figure {
    pub torso: BoxRect,
    pub torso_top_z: f32,
    pub head_z: f32,
    pub head_radius: f32,
}

pub(crate) fn figure(footprint: BoxRect, base_z: f32, top_z: f32) -> Figure {
    let side = footprint.width.min(footprint.height);
    let torso_side = side * TORSO_SHARE;
    let torso = BoxRect {
        x: footprint.x + (footprint.width - torso_side) / 2.0,
        y: footprint.y + (footprint.height - torso_side) / 2.0,
        width: torso_side,
        height: torso_side,
    };
    let height = top_z - base_z;
    let torso_top_z = base_z + height * TORSO_HEIGHT_SHARE;
    let head_radius = ((height - height * TORSO_HEIGHT_SHARE - NECK_PX) / 2.0).max(1.0);
    Figure {
        torso,
        torso_top_z,
        head_z: torso_top_z + NECK_PX + head_radius,
        head_radius,
    }
}

/// A laptop: the base plate over the footprint and the screen standing on its back edge.
pub(crate) fn laptop(footprint: BoxRect) -> (BoxRect, BoxRect) {
    let screen = BoxRect {
        x: footprint.x,
        y: footprint.y,
        width: footprint.width,
        height: SLAB_DEPTH_PX,
    };
    (footprint, screen)
}

/// A phone: a thin slab standing across the middle of the footprint, facing +y.
pub(crate) fn phone(footprint: BoxRect) -> BoxRect {
    let width = footprint.width * PHONE_WIDTH_SHARE;
    BoxRect {
        x: footprint.x + (footprint.width - width) / 2.0,
        y: footprint.y + (footprint.height - PHONE_DEPTH_PX) / 2.0,
        width,
        height: PHONE_DEPTH_PX,
    }
}

/// The screen panel on the +y face of `slab` between two heights, inset from the face's
/// edges, as four screen points.
pub(crate) fn front_panel(
    slab: BoxRect,
    (bottom_z, top_z): (f32, f32),
    offset: ScreenPoint,
) -> [ScreenPoint; 4] {
    let (left, right) = (slab.x + SCREEN_INSET_PX, slab.right() - SCREEN_INSET_PX);
    let (low, high) = (bottom_z + SCREEN_INSET_PX, top_z - SCREEN_INSET_PX);
    let y = slab.bottom();
    [
        project_point(left, y, high, offset),
        project_point(right, y, high, offset),
        project_point(right, y, low, offset),
        project_point(left, y, low, offset),
    ]
}

/// The head of a figure on screen: its center and radius.
pub(crate) fn head(figure: &Figure, offset: ScreenPoint) -> (ScreenPoint, f32) {
    let center_x = figure.torso.x + figure.torso.width / 2.0;
    let center_y = figure.torso.y + figure.torso.height / 2.0;
    (
        project_point(center_x, center_y, figure.head_z, offset),
        figure.head_radius,
    )
}

/// The convex outline of a figure on screen: the hull of its torso ellipses and its head.
pub(crate) fn figure_outline(footprint: BoxRect, base_z: f32, top_z: f32) -> Vec<ScreenPoint> {
    let figure = figure(footprint, base_z, top_z);
    let center_x = figure.torso.x + figure.torso.width / 2.0;
    let center_y = figure.torso.y + figure.torso.height / 2.0;
    let (radius_x, radius_y) = ellipse_radii(figure.torso.width / 2.0);
    let mut points = ellipse_points(
        project_point(center_x, center_y, base_z, ZERO_OFFSET),
        radius_x,
        radius_y,
        OUTLINE_POINTS,
    );
    points.extend(ellipse_points(
        project_point(center_x, center_y, figure.torso_top_z, ZERO_OFFSET),
        radius_x,
        radius_y,
        OUTLINE_POINTS,
    ));
    let (head_center, head_radius) = head(&figure, ZERO_OFFSET);
    points.extend(ellipse_points(
        head_center,
        head_radius,
        head_radius,
        OUTLINE_POINTS,
    ));
    convex_hull(&points)
}

/// The outline of a laptop on screen: its base plate's silhouette, where a link lands. The
/// hull of plate and screen would bridge the plate's front to the screen's top with an edge
/// in the air, and a link landing on that edge touched nothing.
pub(crate) fn laptop_outline(footprint: BoxRect, base_z: f32, _top_z: f32) -> Vec<ScreenPoint> {
    let (base, _) = laptop(footprint);
    silhouette(base, base_z, base_z + LAPTOP_BASE_PX, ZERO_OFFSET).to_vec()
}

/// The outline of a phone on screen: its slab's silhouette.
pub(crate) fn phone_outline(footprint: BoxRect, base_z: f32, top_z: f32) -> Vec<ScreenPoint> {
    silhouette(phone(footprint), base_z, top_z, ZERO_OFFSET).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn footprint(side: f32) -> BoxRect {
        BoxRect {
            x: 100.0,
            y: 200.0,
            width: side,
            height: side,
        }
    }

    #[test]
    fn a_figure_stands_its_torso_centered_with_its_head_inside_the_height() {
        let figure = figure(footprint(36.0), 6.0, 50.0);
        assert!((figure.torso.width - 36.0 * TORSO_SHARE).abs() < 1e-3);
        assert!((figure.torso.x + figure.torso.width / 2.0 - 118.0).abs() < 1e-3);
        assert!((figure.torso.y + figure.torso.height / 2.0 - 218.0).abs() < 1e-3);
        assert!(figure.torso_top_z > 6.0 && figure.torso_top_z < 50.0);
        assert!((figure.head_z + figure.head_radius - 50.0).abs() < 1e-3);
    }

    #[test]
    fn a_laptop_screen_stands_on_the_back_edge_and_a_phone_in_the_middle() {
        let (base, screen) = laptop(footprint(56.0));
        assert_eq!(base, footprint(56.0));
        assert_eq!(
            (screen.x, screen.y, screen.width, screen.height),
            (100.0, 200.0, 56.0, SLAB_DEPTH_PX)
        );
        let slab = phone(footprint(36.0));
        assert!((slab.width - 21.6).abs() < 1e-3);
        assert!((slab.height - PHONE_DEPTH_PX).abs() < 1e-3);
        assert!((slab.x + slab.width / 2.0 - 118.0).abs() < 1e-3);
        assert!((slab.y + slab.height / 2.0 - 218.0).abs() < 1e-3);
    }

    #[test]
    fn a_front_panel_lies_on_the_front_face_inset_from_its_edges() {
        let slab = phone(footprint(36.0));
        let panel = front_panel(slab, (0.0, 44.0), ZERO_OFFSET);
        let expected = project_point(
            slab.x + SCREEN_INSET_PX,
            slab.bottom(),
            44.0 - SCREEN_INSET_PX,
            ZERO_OFFSET,
        );
        assert_eq!(panel[0], expected);
        assert!(panel[2].y > panel[1].y);
    }

    /// A laptop's outline is its base plate alone: a point on the plate's front edge lies
    /// inside it and a point on the screen's face above it outside.
    #[test]
    fn a_laptop_outline_is_its_base_plate_without_the_screen() {
        let footprint = footprint(56.0);
        let outline = laptop_outline(footprint, 0.0, 40.0);
        let (base, screen) = laptop(footprint);
        let plate_front = project_point(
            base.x + base.width / 2.0,
            base.bottom(),
            LAPTOP_BASE_PX,
            ZERO_OFFSET,
        );
        let screen_face =
            project_point(screen.x, screen.y + screen.height / 2.0, 20.0, ZERO_OFFSET);
        let inside = |point: ScreenPoint| {
            let centroid = ScreenPoint {
                x: outline.iter().map(|p| p.x).sum::<f32>() / outline.len() as f32,
                y: outline.iter().map(|p| p.y).sum::<f32>() / outline.len() as f32,
            };
            (0..outline.len()).all(|index| {
                let (a, b) = (outline[index], outline[(index + 1) % outline.len()]);
                let side = |p: ScreenPoint| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                side(point) * side(centroid) >= -1e-3
            })
        };
        assert!(inside(plate_front), "{plate_front:?} in {outline:?}");
        assert!(!inside(screen_face), "{screen_face:?} in {outline:?}");
    }

    #[test]
    fn every_outline_is_convex_and_holds_its_parts() {
        for outline in [
            figure_outline(footprint(44.0), 0.0, 56.0),
            laptop_outline(footprint(56.0), 0.0, 40.0),
            phone_outline(footprint(36.0), 0.0, 44.0),
        ] {
            assert!(outline.len() >= 4, "{outline:?}");
            let hull = convex_hull(&outline);
            assert_eq!(hull.len(), outline.len(), "{outline:?}");
        }
    }
}
