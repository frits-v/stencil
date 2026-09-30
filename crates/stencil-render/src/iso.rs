//! The section 12 isometric projection: solids, billboards and the drawn canvas, computed
//! from `PageGeometry` alone, and the `iso-labels-clear` check over the result.

use stencil_layout::{
    BoxRect, GEOMETRY_EPSILON_PX, NodeGeometry, NodeTag, PageGeometry, Part, PartName, Size,
    TextAlign,
};
use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, NODES_MAX};

use crate::RenderError;

/// Slab thickness of a Zone, and the rise of each nested Zone over its parent.
pub const ISO_SLAB_THICKNESS_PX: f32 = 6.0;
/// Height of a leaf block (Pcard, Fact, Note, Text, Callout, Frame).
pub const ISO_BLOCK_HEIGHT_PX: f32 = 18.0;
/// cos 30 degrees, written out so every build uses the same f32.
pub const ISO_COS_30: f32 = 0.866_025_4;
/// sin 30 degrees.
pub const ISO_SIN_30: f32 = 0.5;
/// Left margin of the projected body and the sum of both side margins.
pub const ISO_MARGIN_PX: f32 = 20.0;
/// Screen semi-axes of a 4 px dot: `4 * sqrt(1.5)` across and `4 * sqrt(0.5)` down
/// (section 12.2, rule 6).
pub const ISO_DOT_RADIUS_X_PX: f32 = 4.898_979_5;
pub const ISO_DOT_RADIUS_Y_PX: f32 = 2.828_427;
/// Flat radius of a pipe end dot and half the width of an arrowhead (sections 5.2, 11.2).
const DOT_RADIUS_PX: f32 = 4.0;
const ARROWHEAD_LENGTH_PX: f32 = stencil_layout::ARROWHEAD_LENGTH_PX;
const ARROWHEAD_HALF_WIDTH_PX: f32 = stencil_layout::ARROWHEAD_WIDTH_PX / 2.0;
/// The gcp label chip grows the Label run box by the bar padding (section 12.4).
const GCP_CHIP_PADDING_X_PX: f32 = 16.0;
const GCP_CHIP_PADDING_Y_PX: f32 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
}

/// The projected page. The pipeline builds it once for `measured_json` and
/// `iso_labels_clear`; `render_svg` builds an equal one itself, because the projection is a
/// pure function of the geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoScene {
    /// The drawn canvas (section 12.2). `PageGeometry.canvas` stays the layout canvas.
    pub canvas: Size,
    /// Added to every projected point.
    pub offset: ScreenPoint,
    /// Added to the y of /legend, its entries and /foot.
    pub footer_shift: f32,
    /// One per body node that draws a solid, in geometry order.
    pub solids: Vec<Solid>,
    /// Painter order (section 12.5): body nodes in geometry order, then link tags in link
    /// order.
    pub billboards: Vec<Billboard>,
    /// One per `PageGeometry.links` entry: the z of the plane the link lies on.
    pub link_planes: Vec<f32>,
}

/// A body node's solid. Beyond the section 12.2 fields it carries the node's pointer and
/// its screen silhouette, so `iso_labels_clear` needs the scene alone.
#[derive(Debug, Clone, PartialEq)]
pub struct Solid {
    /// Geometry index of the node.
    pub node: usize,
    pub pointer: NodePointer,
    pub shape: SolidShape,
    pub base_z: f32,
    /// ISO_SLAB_THICKNESS_PX for a slab, ISO_BLOCK_HEIGHT_PX for a block, 0 for a surface.
    pub height: f32,
    /// The six section 12.3 silhouette vertices in canvas px, offset included.
    pub silhouette: [ScreenPoint; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidShape {
    Slab,
    Block,
    Surface,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Billboard {
    /// The owning node's pointer, or `/links/<i>` for a link tag.
    pub owner: NodePointer,
    /// Geometry index of the owner; None for a link tag.
    pub node: Option<usize>,
    pub role: BillboardRole,
    /// The flat box the billboard redraws, in layout px.
    pub flat: BoxRect,
    pub z: f32,
    /// The drawn box, in canvas px, offset included.
    pub screen: BoxRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillboardRole {
    Content,
    Tag,
}

impl BillboardRole {
    /// The `data-role` and measured JSON name.
    pub fn as_str(self) -> &'static str {
        match self {
            BillboardRole::Content => "content",
            BillboardRole::Tag => "tag",
        }
    }
}

/// Which point of the flat box lands on the projected anchor (section 12.4, rule 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Anchor {
    TopLeft,
    Center,
}

/// A flat point (x, y) at height z on screen (section 12.2, rule 1).
pub fn project_point(x: f32, y: f32, z: f32, offset: ScreenPoint) -> ScreenPoint {
    ScreenPoint {
        x: (x - y) * ISO_COS_30 + offset.x,
        y: (x + y) * ISO_SIN_30 - z + offset.y,
    }
}

/// The six silhouette vertices of a box from `base_z` to `top_z` (section 12.3, rule 3).
pub fn silhouette(
    bounds: BoxRect,
    base_z: f32,
    top_z: f32,
    offset: ScreenPoint,
) -> [ScreenPoint; 6] {
    let (left, top, right, bottom) = (bounds.x, bounds.y, bounds.right(), bounds.bottom());
    [
        project_point(left, top, top_z, offset),
        project_point(right, top, top_z, offset),
        project_point(right, top, base_z, offset),
        project_point(right, bottom, base_z, offset),
        project_point(left, bottom, base_z, offset),
        project_point(left, bottom, top_z, offset),
    ]
}

/// The parts a node's billboard redraws (section 12.4), by the node's tag. The Zone label
/// and the gcp chip are handled by `billboard_flat_box`.
pub(crate) fn billboard_member(tag: NodeTag, part: PartName) -> bool {
    match tag {
        NodeTag::Zone => part == PartName::Label,
        NodeTag::Pcard => matches!(
            part,
            PartName::Icon
                | PartName::FactBox
                | PartName::AskBox
                | PartName::FunctionName
                | PartName::ProductName
                | PartName::Fact
                | PartName::Ask
        ),
        NodeTag::Fact | NodeTag::Note => part == PartName::Text,
        NodeTag::Text => matches!(
            part,
            PartName::Heading | PartName::Marker | PartName::BodyLine
        ),
        NodeTag::Callout => matches!(part, PartName::Heading | PartName::Text),
        NodeTag::Frame => matches!(part, PartName::LabelChip | PartName::Label),
        NodeTag::Pipe => matches!(part, PartName::Tag | PartName::TagLabel | PartName::TagSub),
        NodeTag::Tee => matches!(part, PartName::Hub | PartName::HubText),
        NodeTag::Page
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::LegendEntry
        | NodeTag::Foot
        | NodeTag::Row
        | NodeTag::Col => false,
    }
}

/// A run's ink box (section 12.4, rule 1), or the part box for a part without a run.
fn member_box(part: &Part) -> BoxRect {
    let Some(run) = &part.text else {
        return part.bounds;
    };
    let align_offset = match run.align {
        TextAlign::Start => 0.0,
        TextAlign::Center => (part.bounds.width - run.metrics.width_px) / 2.0,
    };
    BoxRect {
        x: part.bounds.x + align_offset,
        y: part.bounds.y,
        width: run.metrics.width_px,
        height: run.metrics.height_px,
    }
}

fn union(first: BoxRect, second: BoxRect) -> BoxRect {
    let left = first.x.min(second.x);
    let top = first.y.min(second.y);
    let right = first.right().max(second.right());
    let bottom = first.bottom().max(second.bottom());
    BoxRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

/// The union of a node's member boxes, grown to the chip for a gcp zone; None when the node
/// has no member part.
pub(crate) fn billboard_flat_box(node: &NodeGeometry) -> Option<BoxRect> {
    let flat = node
        .parts
        .iter()
        .filter(|part| billboard_member(node.tag, part.name))
        .map(member_box)
        .reduce(union)?;
    if node.tag == NodeTag::Zone && node.kind == Some("gcp") {
        return Some(BoxRect {
            x: flat.x - GCP_CHIP_PADDING_X_PX,
            y: flat.y - GCP_CHIP_PADDING_Y_PX,
            width: flat.width + 2.0 * GCP_CHIP_PADDING_X_PX,
            height: flat.height + 2.0 * GCP_CHIP_PADDING_Y_PX,
        });
    }
    Some(flat)
}

fn screen_box(flat: BoxRect, z: f32, anchor: Anchor, offset: ScreenPoint) -> BoxRect {
    let (x, y) = match anchor {
        Anchor::TopLeft => {
            let corner = project_point(flat.x, flat.y, z, offset);
            (corner.x, corner.y)
        }
        Anchor::Center => {
            let center = project_point(
                flat.x + flat.width / 2.0,
                flat.y + flat.height / 2.0,
                z,
                offset,
            );
            (center.x - flat.width / 2.0, center.y - flat.height / 2.0)
        }
    };
    BoxRect {
        x,
        y,
        width: flat.width,
        height: flat.height,
    }
}

/// Per-node facts computed in one pass in geometry order, where a parent always precedes
/// its children.
struct NodeFacts {
    in_body: Vec<bool>,
    /// Number of Zone ancestors.
    zone_depth: Vec<usize>,
    /// Top of the nearest enclosing slab, 0 without a Zone ancestor.
    slab_top: Vec<f32>,
}

fn node_facts(geometry: &PageGeometry, body: usize) -> Result<NodeFacts, RenderError> {
    let count = geometry.nodes.len();
    let mut facts = NodeFacts {
        in_body: vec![false; count],
        zone_depth: vec![0; count],
        slab_top: vec![0.0; count],
    };
    for (index, node) in geometry.nodes.iter().enumerate() {
        let Some(parent) = node.parent else {
            continue;
        };
        let parent_node = geometry
            .nodes
            .get(parent)
            .filter(|_| parent < index)
            .ok_or_else(|| RenderError::GeometryMismatch {
                expected: NodePointer::root().child("<parent before child>"),
                found: node.pointer.clone(),
            })?;
        let parent_in_body = facts.in_body.get(parent).copied().unwrap_or(false);
        let parent_depth = facts.zone_depth.get(parent).copied().unwrap_or(0);
        let parent_top = facts.slab_top.get(parent).copied().unwrap_or(0.0);
        let parent_is_zone = parent_node.tag == NodeTag::Zone;
        let (depth, top) = if parent_is_zone {
            (
                parent_depth + 1,
                ISO_SLAB_THICKNESS_PX * (parent_depth + 1) as f32,
            )
        } else {
            (parent_depth, parent_top)
        };
        if let Some(slot) = facts.in_body.get_mut(index) {
            *slot = parent == body || parent_in_body;
        }
        if let Some(slot) = facts.zone_depth.get_mut(index) {
            *slot = depth;
        }
        if let Some(slot) = facts.slab_top.get_mut(index) {
            *slot = top;
        }
    }
    Ok(facts)
}

/// Shape, base and height of a body node's solid; None for Row and Col.
fn solid_shape(tag: NodeTag, zone_depth: usize, slab_top: f32) -> Option<(SolidShape, f32, f32)> {
    match tag {
        NodeTag::Zone => Some((
            SolidShape::Slab,
            ISO_SLAB_THICKNESS_PX * zone_depth as f32,
            ISO_SLAB_THICKNESS_PX,
        )),
        NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame => Some((SolidShape::Block, slab_top, ISO_BLOCK_HEIGHT_PX)),
        NodeTag::Pipe | NodeTag::Tee => Some((SolidShape::Surface, slab_top, 0.0)),
        NodeTag::Page
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::LegendEntry
        | NodeTag::Foot
        | NodeTag::Row
        | NodeTag::Col => None,
    }
}

/// z of a node's billboard: a zone's sits ISO_BLOCK_HEIGHT_PX above its slab top (rule 3),
/// a block's on its top, a pipe or tee tag on its surface.
fn billboard_placement(
    shape: SolidShape,
    base_z: f32,
    height: f32,
) -> (f32, Anchor, BillboardRole) {
    match shape {
        SolidShape::Slab => (
            base_z + height + ISO_BLOCK_HEIGHT_PX,
            Anchor::TopLeft,
            BillboardRole::Content,
        ),
        SolidShape::Block => (base_z + height, Anchor::TopLeft, BillboardRole::Content),
        SolidShape::Surface => (base_z, Anchor::Center, BillboardRole::Tag),
    }
}

/// The plane of a link: the slab top of the innermost Zone that strictly contains both
/// endpoints, or the ground.
fn link_plane(geometry: &PageGeometry, facts: &NodeFacts, from: usize, to: usize) -> f32 {
    let from_zones = zone_ancestors(geometry, from);
    zone_ancestors(geometry, to)
        .into_iter()
        .find(|zone| from_zones.contains(zone))
        .and_then(|zone| facts.zone_depth.get(zone))
        .map_or(0.0, |depth| ISO_SLAB_THICKNESS_PX * (*depth + 1) as f32)
}

/// Strict Zone ancestors of a node, innermost first. Parents precede children, so the walk
/// ends within `nodes.len()` steps.
fn zone_ancestors(geometry: &PageGeometry, index: usize) -> Vec<usize> {
    let mut zones = Vec::new();
    let mut current = geometry.nodes.get(index).and_then(|node| node.parent);
    for _ in 0..geometry.nodes.len() {
        let Some(ancestor) = current else {
            break;
        };
        let Some(node) = geometry.nodes.get(ancestor) else {
            break;
        };
        if node.tag == NodeTag::Zone {
            zones.push(ancestor);
        }
        current = node.parent;
    }
    zones
}

/// Screen extremes of every drawn primitive, before any offset (section 12.2, rule 3).
#[derive(Debug, Clone, Copy)]
struct Extent {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl Extent {
    fn new(first: ScreenPoint) -> Self {
        Extent {
            min_x: first.x,
            max_x: first.x,
            min_y: first.y,
            max_y: first.y,
        }
    }

    fn add(&mut self, point: ScreenPoint) {
        self.min_x = self.min_x.min(point.x);
        self.max_x = self.max_x.max(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_y = self.max_y.max(point.y);
    }

    fn add_box(&mut self, screen: BoxRect) {
        for (x, y) in [
            (screen.x, screen.y),
            (screen.right(), screen.y),
            (screen.x, screen.bottom()),
            (screen.right(), screen.bottom()),
        ] {
            self.add(ScreenPoint { x, y });
        }
    }

    /// A dot at a flat center: the four extreme points of its screen ellipse.
    fn add_dot(&mut self, center: ScreenPoint) {
        for (x, y) in [
            (center.x - ISO_DOT_RADIUS_X_PX, center.y),
            (center.x + ISO_DOT_RADIUS_X_PX, center.y),
            (center.x, center.y - ISO_DOT_RADIUS_Y_PX),
            (center.x, center.y + ISO_DOT_RADIUS_Y_PX),
        ] {
            self.add(ScreenPoint { x, y });
        }
    }
}

const ZERO_OFFSET: ScreenPoint = ScreenPoint { x: 0.0, y: 0.0 };

fn center(bounds: BoxRect) -> (f32, f32) {
    (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// The three flat vertices of an arrowhead whose tip is `tip` and which points along the
/// unit vector (direction_x, direction_y): tip, then the two base corners.
pub(crate) fn arrowhead_vertices(tip: (f32, f32), direction: (f32, f32)) -> [(f32, f32); 3] {
    let (tip_x, tip_y) = tip;
    let (direction_x, direction_y) = direction;
    let base_x = tip_x - direction_x * ARROWHEAD_LENGTH_PX;
    let base_y = tip_y - direction_y * ARROWHEAD_LENGTH_PX;
    let across_x = -direction_y * ARROWHEAD_HALF_WIDTH_PX;
    let across_y = direction_x * ARROWHEAD_HALF_WIDTH_PX;
    [
        (tip_x, tip_y),
        (base_x + across_x, base_y + across_y),
        (base_x - across_x, base_y - across_y),
    ]
}

/// Unit direction from `from` to `to`, None when the two coincide.
pub(crate) fn unit_direction(from: (f32, f32), to: (f32, f32)) -> Option<(f32, f32)> {
    let delta_x = to.0 - from.0;
    let delta_y = to.1 - from.1;
    let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
    (length > f32::EPSILON).then(|| (delta_x / length, delta_y / length))
}

/// Adds a pipe's or tee's surface primitives. The geometry does not record which ends carry
/// an arrowhead, so both the dot and the arrowhead extent of each end are added; an
/// arrowhead lies within 10 px of a pipe end, which the wire and slab already bound in
/// every figure where a pipe sits between two boxes.
fn add_surface_extent(extent: &mut Extent, node: &NodeGeometry, z: f32) {
    let dots = (node.part(PartName::DotStart), node.part(PartName::DotEnd));
    if let (Some(start), Some(end)) = dots {
        let start_center = center(start.bounds);
        let end_center = center(end.bounds);
        for (dot_center, other) in [(start_center, end_center), (end_center, start_center)] {
            extent.add_dot(project_point(dot_center.0, dot_center.1, z, ZERO_OFFSET));
            if let Some(outward) = unit_direction(other, dot_center) {
                let tip = (
                    dot_center.0 + outward.0 * DOT_RADIUS_PX,
                    dot_center.1 + outward.1 * DOT_RADIUS_PX,
                );
                for (x, y) in arrowhead_vertices(tip, outward) {
                    extent.add(project_point(x, y, z, ZERO_OFFSET));
                }
            }
        }
    }
    if let Some(spine) = node.part(PartName::Spine) {
        let (center_x, _) = center(spine.bounds);
        extent.add(project_point(center_x, spine.bounds.y, z, ZERO_OFFSET));
        extent.add(project_point(
            center_x,
            spine.bounds.bottom(),
            z,
            ZERO_OFFSET,
        ));
    }
}

/// Asserts nodes[0] is the root and /body is present, then projects the body.
pub fn project_page(geometry: &PageGeometry) -> Result<IsoScene, RenderError> {
    let root = NodePointer::root();
    match geometry.nodes.first() {
        Some(first) if first.pointer == root => {}
        other => {
            return Err(RenderError::GeometryMismatch {
                expected: root,
                found: other.map_or_else(
                    || NodePointer::root().child("<absent>"),
                    |node| node.pointer.clone(),
                ),
            });
        }
    }
    let body_pointer = root.child("body");
    let Some((body_index, body)) = geometry
        .nodes
        .iter()
        .enumerate()
        .find(|(_, node)| node.pointer == body_pointer)
    else {
        return Err(RenderError::GeometryMismatch {
            expected: body_pointer,
            found: NodePointer::root().child("<absent>"),
        });
    };
    let facts = node_facts(geometry, body_index)?;

    let body_bounds = body.bounds;
    let mut extent = Extent::new(project_point(
        body_bounds.x,
        body_bounds.y,
        0.0,
        ZERO_OFFSET,
    ));
    for (x, y) in [
        (body_bounds.right(), body_bounds.y),
        (body_bounds.x, body_bounds.bottom()),
        (body_bounds.right(), body_bounds.bottom()),
    ] {
        extent.add(project_point(x, y, 0.0, ZERO_OFFSET));
    }

    let mut solids = Vec::new();
    let mut billboards = Vec::new();
    for (index, node) in geometry.nodes.iter().enumerate() {
        if !facts.in_body.get(index).copied().unwrap_or(false) {
            continue;
        }
        let zone_depth = facts.zone_depth.get(index).copied().unwrap_or(0);
        let slab_top = facts.slab_top.get(index).copied().unwrap_or(0.0);
        let Some((shape, base_z, height)) = solid_shape(node.tag, zone_depth, slab_top) else {
            continue;
        };
        let outline = silhouette(node.bounds, base_z, base_z + height, ZERO_OFFSET);
        match shape {
            SolidShape::Slab | SolidShape::Block => {
                for point in outline {
                    extent.add(point);
                }
            }
            SolidShape::Surface => add_surface_extent(&mut extent, node, base_z),
        }
        solids.push(Solid {
            node: index,
            pointer: node.pointer.clone(),
            shape,
            base_z,
            height,
            silhouette: outline,
        });
        if let Some(flat) = billboard_flat_box(node) {
            let (z, anchor, role) = billboard_placement(shape, base_z, height);
            let screen = screen_box(flat, z, anchor, ZERO_OFFSET);
            extent.add_box(screen);
            billboards.push(Billboard {
                owner: node.pointer.clone(),
                node: Some(index),
                role,
                flat,
                z,
                screen,
            });
        }
    }

    let mut link_planes = Vec::with_capacity(geometry.links.len().min(LINKS_MAX));
    for route in geometry.links.iter().take(LINKS_MAX) {
        let plane = link_plane(geometry, &facts, route.from_node, route.to_node);
        link_planes.push(plane);
        for point in &route.points {
            extent.add(project_point(point.x, point.y, plane, ZERO_OFFSET));
        }
        // LinkRoute does not carry the arrow value, so both ends get an arrowhead extent.
        let first_two = (route.points.first(), route.points.get(1));
        let last_two = (
            route.points.last(),
            route
                .points
                .len()
                .checked_sub(2)
                .and_then(|index| route.points.get(index)),
        );
        for (tip, previous) in [first_two, last_two] {
            if let (Some(tip), Some(previous)) = (tip, previous)
                && let Some(direction) = unit_direction((previous.x, previous.y), (tip.x, tip.y))
            {
                for (x, y) in arrowhead_vertices((tip.x, tip.y), direction) {
                    extent.add(project_point(x, y, plane, ZERO_OFFSET));
                }
            }
        }
        if let Some(tag) = route.tag {
            let screen = screen_box(tag, plane, Anchor::Center, ZERO_OFFSET);
            extent.add_box(screen);
            billboards.push(Billboard {
                owner: NodePointer::root().child("links").index(route.index),
                node: None,
                role: BillboardRole::Tag,
                flat: tag,
                z: plane,
                screen,
            });
        }
    }

    let offset = ScreenPoint {
        x: ISO_MARGIN_PX - extent.min_x,
        y: body_bounds.y - extent.min_y,
    };
    for solid in &mut solids {
        solid.silhouette = solid.silhouette.map(|point| ScreenPoint {
            x: point.x + offset.x,
            y: point.y + offset.y,
        });
    }
    for billboard in &mut billboards {
        billboard.screen.x += offset.x;
        billboard.screen.y += offset.y;
    }
    let projected_height = extent.max_y - extent.min_y;
    let footer_shift = projected_height - body_bounds.height;
    let canvas = Size {
        width: geometry
            .canvas
            .width
            .max((extent.max_x - extent.min_x) + 2.0 * ISO_MARGIN_PX),
        height: geometry.canvas.height + footer_shift,
    };
    Ok(IsoScene {
        canvas,
        offset,
        footer_shift,
        solids,
        billboards,
        link_planes,
    })
}

/// A convex shape for the separating-axis test.
fn rectangle_corners(screen: BoxRect) -> [ScreenPoint; 4] {
    [
        ScreenPoint {
            x: screen.x,
            y: screen.y,
        },
        ScreenPoint {
            x: screen.right(),
            y: screen.y,
        },
        ScreenPoint {
            x: screen.right(),
            y: screen.bottom(),
        },
        ScreenPoint {
            x: screen.x,
            y: screen.bottom(),
        },
    ]
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

/// True when the two convex shapes overlap by more than GEOMETRY_EPSILON_PX on the x and
/// y axes and on the unit normal of every edge of `hexagon` (section 12.7, rule 2).
fn rectangle_overlaps_hexagon(screen: BoxRect, hexagon: &[ScreenPoint; 6]) -> bool {
    let rectangle = rectangle_corners(screen);
    let mut axes = vec![(1.0, 0.0), (0.0, 1.0)];
    for (index, start) in hexagon.iter().enumerate() {
        let Some(end) = hexagon.get((index + 1) % hexagon.len()) else {
            continue;
        };
        if let Some((direction_x, direction_y)) = unit_direction((start.x, start.y), (end.x, end.y))
        {
            axes.push((-direction_y, direction_x));
        }
    }
    axes.into_iter().all(|axis| {
        let (rectangle_low, rectangle_high) = axis_interval(&rectangle, axis);
        let (hexagon_low, hexagon_high) = axis_interval(hexagon, axis);
        rectangle_high.min(hexagon_high) - rectangle_low.max(hexagon_low) > GEOMETRY_EPSILON_PX
    })
}

fn rectangles_overlap(first: BoxRect, second: BoxRect) -> bool {
    let overlap_width = first.right().min(second.right()) - first.x.max(second.x);
    let overlap_height = first.bottom().min(second.bottom()) - first.y.max(second.y);
    overlap_width > GEOMETRY_EPSILON_PX && overlap_height > GEOMETRY_EPSILON_PX
}

fn describe(billboard: &Billboard) -> String {
    let screen = billboard.screen;
    format!(
        "{} billboard {:.2},{:.2} {:.2}x{:.2}",
        billboard.role.as_str(),
        screen.x,
        screen.y,
        screen.width,
        screen.height
    )
}

/// NotApplicable with reason "projection is flat" for None; section 12.7 otherwise.
pub fn iso_labels_clear(scene: Option<&IsoScene>) -> CheckReport {
    let Some(scene) = scene else {
        return CheckReport::not_applicable(CheckName::IsoLabelsClear, "projection is flat");
    };
    let billboard_limit = NODES_MAX + LINKS_MAX;
    let billboards: Vec<&Billboard> = scene.billboards.iter().take(billboard_limit).collect();
    let blocks: Vec<&Solid> = scene
        .solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Block)
        .take(NODES_MAX)
        .collect();
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for (later_index, later) in billboards.iter().enumerate() {
        for earlier in billboards.iter().take(later_index) {
            examined += 1;
            if rectangles_overlap(earlier.screen, later.screen) {
                defects.push(Defect {
                    pointer: later.owner.clone(),
                    message: format!(
                        "{} overlaps {} billboard {}",
                        describe(later),
                        earlier.role.as_str(),
                        earlier.owner
                    ),
                });
            }
        }
    }
    for billboard in &billboards {
        for block in &blocks {
            if billboard.node == Some(block.node) {
                continue;
            }
            examined += 1;
            if rectangle_overlaps_hexagon(billboard.screen, &block.silhouette) {
                defects.push(Defect {
                    pointer: billboard.owner.clone(),
                    message: format!("{} covers block {}", describe(billboard), block.pointer),
                });
            }
        }
    }
    CheckReport {
        check: CheckName::IsoLabelsClear,
        examined,
        defects,
        not_applicable: None,
    }
}
