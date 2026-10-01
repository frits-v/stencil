//! The body zoom of section 12.2, rule 7: a body whose projection is narrower than its canvas
//! is scaled about the body's top-left corner before it is projected, so a sparse cover
//! figure fills the slide. Text and icons keep their size.

use stencil_layout::{BoxRect, NodeGeometry, NodeTag, PageGeometry, Part};
use stencil_model::PagePoint;

use super::{ISO_COS_30, plane_member};

/// The zoom stops when the projected body is this fraction of the layout canvas width.
pub const ISO_FILL_FRACTION: f32 = 0.8;
/// The largest body zoom.
pub const ISO_ZOOM_MAX: f32 = 1.6;

/// The zoom that brings the projected width of the body nodes to ISO_FILL_FRACTION of
/// `canvas_width`, between 1 and ISO_ZOOM_MAX. The width is that of the footprints at
/// z = 0 of the nodes that draw a solid; a Row or Col draws nothing and can be as wide as
/// the page.
pub(crate) fn fit_zoom(geometry: &PageGeometry, in_body: &[bool], canvas_width: f32) -> f32 {
    let mut least = f32::INFINITY;
    let mut most = f32::NEG_INFINITY;
    for (node, _) in geometry
        .nodes
        .iter()
        .zip(in_body)
        .filter(|(node, inside)| **inside && !matches!(node.tag, NodeTag::Row | NodeTag::Col))
    {
        least = least.min(node.bounds.x - node.bounds.bottom());
        most = most.max(node.bounds.right() - node.bounds.y);
    }
    let width = (most - least) * ISO_COS_30;
    if !width.is_finite() || width <= f32::EPSILON {
        return 1.0;
    }
    (ISO_FILL_FRACTION * canvas_width / width).clamp(1.0, ISO_ZOOM_MAX)
}

struct Scale {
    origin: (f32, f32),
    zoom: f32,
}

impl Scale {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin.0 + (x - self.origin.0) * self.zoom,
            self.origin.1 + (y - self.origin.1) * self.zoom,
        )
    }

    fn rectangle(&self, bounds: BoxRect) -> BoxRect {
        let (x, y) = self.point(bounds.x, bounds.y);
        BoxRect {
            x,
            y,
            width: bounds.width * self.zoom,
            height: bounds.height * self.zoom,
        }
    }
}

/// A plane member keeps its layout box: the label's map zooms it with the plane.
fn zoomed_node(node: &NodeGeometry, scale: &Scale) -> NodeGeometry {
    let bounds = scale.rectangle(node.bounds);
    let parts = node
        .parts
        .iter()
        .map(|part| Part {
            bounds: if plane_member(node.tag, part.name) {
                part.bounds
            } else {
                scale.rectangle(part.bounds)
            },
            ..part.clone()
        })
        .collect();
    NodeGeometry {
        bounds,
        content: scale.rectangle(node.content),
        parts,
        ..node.clone()
    }
}

/// The geometry with every body node and every link scaled by `zoom` about `origin`. A
/// plane member and a link tag keep their layout boxes, which the label's map zooms with
/// the plane. A zoom of 1 returns an equal geometry.
pub(crate) fn zoomed(
    geometry: &PageGeometry,
    in_body: &[bool],
    origin: (f32, f32),
    zoom: f32,
) -> PageGeometry {
    let scale = Scale { origin, zoom };
    let nodes = geometry
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            if in_body.get(index).copied().unwrap_or(false) {
                zoomed_node(node, &scale)
            } else {
                node.clone()
            }
        })
        .collect();
    let links = geometry
        .links
        .iter()
        .map(|route| {
            let points = route
                .points
                .iter()
                .map(|point| {
                    let (x, y) = scale.point(point.x, point.y);
                    PagePoint { x, y }
                })
                .collect();
            stencil_layout::LinkRoute {
                points,
                ..route.clone()
            }
        })
        .collect();
    PageGeometry {
        canvas: geometry.canvas,
        nodes,
        links,
    }
}
