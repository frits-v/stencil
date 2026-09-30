//! The body zoom of section 12.2, rule 7: a body whose projection is narrower than its canvas
//! is scaled about the body's top-left corner before it is projected, so a sparse cover
//! figure fills the slide. Text and icons keep their size.

use stencil_layout::{BoxRect, NodeGeometry, NodeTag, PageGeometry, Part};
use stencil_model::PagePoint;

use super::{ISO_COS_30, billboard_member};

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

fn translated(bounds: BoxRect, delta: (f32, f32)) -> BoxRect {
    BoxRect {
        x: bounds.x + delta.0,
        y: bounds.y + delta.1,
        ..bounds
    }
}

/// Where a node's billboard members move: with the node's top-left corner, except on a pipe
/// or tee, whose tag is centered on its box.
fn member_delta(node: &NodeGeometry, scaled: BoxRect) -> (f32, f32) {
    match node.tag {
        NodeTag::Pipe | NodeTag::Tee => (
            scaled.x + scaled.width / 2.0 - (node.bounds.x + node.bounds.width / 2.0),
            scaled.y + scaled.height / 2.0 - (node.bounds.y + node.bounds.height / 2.0),
        ),
        _ => (scaled.x - node.bounds.x, scaled.y - node.bounds.y),
    }
}

fn zoomed_node(node: &NodeGeometry, scale: &Scale) -> NodeGeometry {
    let bounds = scale.rectangle(node.bounds);
    let delta = member_delta(node, bounds);
    let parts = node
        .parts
        .iter()
        .map(|part| Part {
            bounds: if billboard_member(node.tag, part.name) {
                translated(part.bounds, delta)
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
/// billboard member keeps its size and its offset from its node, and a link tag keeps its
/// size and moves with its center. A zoom of 1 returns an equal geometry.
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
            let delta = route.tag.map_or((0.0, 0.0), |tag| {
                let center = (tag.x + tag.width / 2.0, tag.y + tag.height / 2.0);
                let moved = scale.point(center.0, center.1);
                (moved.0 - center.0, moved.1 - center.1)
            });
            stencil_layout::LinkRoute {
                points,
                tag: route.tag.map(|tag| translated(tag, delta)),
                parts: route
                    .parts
                    .iter()
                    .map(|part| Part {
                        bounds: translated(part.bounds, delta),
                        ..part.clone()
                    })
                    .collect(),
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
