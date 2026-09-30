//! The section 12 isometric projection: solids, billboards, link paths and the drawn canvas,
//! computed from `PageGeometry` alone, and the `iso-labels-clear` check over the result.

mod drape;
mod placement;
mod route;
mod shapes;
mod zoom;

use stencil_layout::{
    BoxRect, LinkRoute, NodeGeometry, NodeTag, PageGeometry, Part, PartName, Size, TextAlign,
};
use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, NODES_MAX, PagePoint, PipeKind};

use crate::RenderError;
use crate::svg::DOT_RADIUS_PX;

pub(crate) use drape::{end_direction, start_direction};
pub use placement::ISO_CARD_CLEARANCE_PX;
pub use route::{ISO_LINK_CLEARANCE_PX, ISO_STRAIGHT_SHARED_MIN_PX};
pub(crate) use shapes::unit_direction;
pub use zoom::{ISO_FILL_FRACTION, ISO_ZOOM_MAX};

use drape::Terrain;
use placement::Obstacles;
use shapes::{rectangle_overlaps_polygon, rectangles_overlap, segment_crosses_box};

/// Slab thickness of a filled Zone, and the rise of each filled nested Zone over its parent.
pub const ISO_SLAB_THICKNESS_PX: f32 = 6.0;
/// Height of a leaf block (Pcard, Fact, Note, Text, Callout, Frame).
pub const ISO_BLOCK_HEIGHT_PX: f32 = 18.0;
/// cos 30 degrees, written out so every build uses the same f32.
pub const ISO_COS_30: f32 = 0.866_025_4;
/// sin 30 degrees.
pub const ISO_SIN_30: f32 = 0.5;
/// Smallest side margin of the projected body.
pub const ISO_MARGIN_PX: f32 = 20.0;
/// Screen semi-axes of a 4 px dot: `4 * sqrt(1.5)` across and `4 * sqrt(0.5)` down
/// (section 12.2, rule 6).
pub const ISO_DOT_RADIUS_X_PX: f32 = 4.898_979_5;
pub const ISO_DOT_RADIUS_Y_PX: f32 = 2.828_427;
/// Arrowhead length of a pipe, shared with the flat writer (section 11.2).
const ARROWHEAD_LENGTH_PX: f32 = stencil_layout::ARROWHEAD_LENGTH_PX;
/// Half width over length of every arrowhead, as in flat.
const ARROWHEAD_WIDTH_RATIO: f32 =
    stencil_layout::ARROWHEAD_WIDTH_PX / 2.0 / stencil_layout::ARROWHEAD_LENGTH_PX;
/// A zone tab grows the Label run box by this padding across and down (section 12.4): the
/// gcp bar padding for gcp, a smaller one for every other zone.
const GCP_TAB_PADDING_PX: (f32, f32) = (16.0, 7.0);
const ZONE_TAB_PADDING_PX: (f32, f32) = (10.0, 5.0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
}

/// A flat point at a height: one vertex of a link path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsoPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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
    /// One per `PageGeometry.links` entry: the routed polyline adjusted by section 12.3
    /// rule 8, laid over the slabs and cut back at its endpoint blocks (rule 7), in zoomed
    /// flat px.
    pub link_paths: Vec<Vec<IsoPoint>>,
    /// The kind of each link, in `link_paths` order.
    pub link_kinds: Vec<PipeKind>,
    /// The body zoom of section 12.2, rule 7.
    pub zoom: f32,
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
    /// ISO_SLAB_THICKNESS_PX for a filled slab, 0 for a ring zone and a surface,
    /// ISO_BLOCK_HEIGHT_PX for a block.
    pub height: f32,
    /// The six section 12.3 silhouette vertices in canvas px, offset included.
    pub silhouette: [ScreenPoint; 6],
    /// True when the solid's faces are filled in every theme, so it hides what was drawn
    /// before it: a slab with height and every block except Note and Frame.
    pub opaque: bool,
    /// The node's border box in zoomed flat px.
    pub footprint: BoxRect,
}

impl Solid {
    /// The drawn edges of a slab (section 12.7): the four top-face edges, and for a slab
    /// with height the lower silhouette edges and the front vertical edge.
    pub fn slab_edges(&self) -> Vec<(ScreenPoint, ScreenPoint)> {
        let [back, right_top, right_base, front_base, left_base, left_top] = self.silhouette;
        let front_top = ScreenPoint {
            x: front_base.x,
            y: front_base.y - self.height,
        };
        let mut edges = vec![
            (back, right_top),
            (right_top, front_top),
            (front_top, left_top),
            (left_top, back),
        ];
        if self.height > 0.0 {
            edges.extend([
                (right_top, right_base),
                (right_base, front_base),
                (front_base, left_base),
                (left_base, left_top),
                (front_top, front_base),
            ]);
        }
        edges
    }
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
    /// True when the billboard is drawn on a box of its own that hides what lies under it:
    /// the gcp chip and every tag.
    pub opaque: bool,
    /// The screen ink boxes of the billboard's text runs, offset included. The check of
    /// section 12.7 tests these against slab edges.
    pub marks: Vec<BoxRect>,
    /// True when every mark must lie inside the owner block's silhouette: the text of a
    /// Fact, Note, Text, Callout or Frame, which is laid out to fit its block.
    pub contained: bool,
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

/// A zone drawn as a ring on its parent's top instead of a slab: the zone kinds that have
/// no fill in any theme.
pub fn is_ring_zone(node: &NodeGeometry) -> bool {
    node.tag == NodeTag::Zone && node.kind == Some("vpc")
}

fn is_gcp_zone(node: &NodeGeometry) -> bool {
    node.tag == NodeTag::Zone && node.kind == Some("gcp")
}

/// The padding of a zone's tab around its Label run.
fn tab_padding(node: &NodeGeometry) -> (f32, f32) {
    if is_gcp_zone(node) {
        GCP_TAB_PADDING_PX
    } else {
        ZONE_TAB_PADDING_PX
    }
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
pub(crate) fn member_box(part: &Part) -> BoxRect {
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

/// The union of a node's member boxes, grown to the tab for a zone; None when the node has
/// no member part.
pub(crate) fn billboard_flat_box(node: &NodeGeometry) -> Option<BoxRect> {
    let flat = node
        .parts
        .iter()
        .filter(|part| billboard_member(node.tag, part.name))
        .map(member_box)
        .reduce(union)?;
    if node.tag == NodeTag::Zone {
        let (across, down) = tab_padding(node);
        return Some(BoxRect {
            x: flat.x - across,
            y: flat.y - down,
            width: flat.width + 2.0 * across,
            height: flat.height + 2.0 * down,
        });
    }
    Some(flat)
}

/// The flat ink boxes of a node's billboard text runs. Icons sit on an opaque chip, which
/// hides a stroke under it the way a tag does, so they are not marks.
fn flat_marks(node: &NodeGeometry) -> Vec<BoxRect> {
    node.parts
        .iter()
        .filter(|part| billboard_member(node.tag, part.name) && part.text.is_some())
        .map(member_box)
        .collect()
}

/// `boxes` moved by `delta`.
fn moved(boxes: &[BoxRect], delta: (f32, f32)) -> Vec<BoxRect> {
    boxes
        .iter()
        .map(|bounds| BoxRect {
            x: bounds.x + delta.0,
            y: bounds.y + delta.1,
            ..*bounds
        })
        .collect()
}

/// A `width` by `height` screen box whose top-left corner is at `corner`.
fn box_at(corner: ScreenPoint, width: f32, height: f32) -> BoxRect {
    BoxRect {
        x: corner.x,
        y: corner.y,
        width,
        height,
    }
}

/// A `width` by `height` screen box centered on `center`.
fn box_around(center: ScreenPoint, width: f32, height: f32) -> BoxRect {
    BoxRect {
        x: center.x - width / 2.0,
        y: center.y - height / 2.0,
        width,
        height,
    }
}

fn center(bounds: BoxRect) -> (f32, f32) {
    (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// Per-node facts computed in one pass in geometry order, where a parent always precedes
/// its children.
struct NodeFacts {
    in_body: Vec<bool>,
    /// Top of the nearest enclosing Zone's solid, 0 without a Zone ancestor.
    slab_top: Vec<f32>,
}

/// True when a zone encloses node `index`. Parents precede children, so the walk ends
/// within `nodes.len()` steps.
pub(crate) fn has_zone_ancestor(geometry: &PageGeometry, index: usize) -> bool {
    let mut current = geometry.nodes.get(index).and_then(|node| node.parent);
    for _ in 0..geometry.nodes.len() {
        let Some(parent) = current.and_then(|parent| geometry.nodes.get(parent)) else {
            return false;
        };
        if parent.tag == NodeTag::Zone {
            return true;
        }
        current = parent.parent;
    }
    false
}

/// The height of a zone's solid.
fn zone_height(node: &NodeGeometry) -> f32 {
    if is_ring_zone(node) {
        0.0
    } else {
        ISO_SLAB_THICKNESS_PX
    }
}

fn node_facts(geometry: &PageGeometry, body: usize) -> Result<NodeFacts, RenderError> {
    let count = geometry.nodes.len();
    let mut facts = NodeFacts {
        in_body: vec![false; count],
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
        let parent_top = facts.slab_top.get(parent).copied().unwrap_or(0.0);
        let top = if parent_node.tag == NodeTag::Zone {
            parent_top + zone_height(parent_node)
        } else {
            parent_top
        };
        if let Some(slot) = facts.in_body.get_mut(index) {
            *slot = parent == body || parent_in_body;
        }
        if let Some(slot) = facts.slab_top.get_mut(index) {
            *slot = top;
        }
    }
    Ok(facts)
}

/// Shape, base and height of a body node's solid; None for Row and Col. `slab_top` is the
/// top of the nearest enclosing Zone's solid.
fn solid_shape(node: &NodeGeometry, slab_top: f32) -> Option<(SolidShape, f32, f32)> {
    match node.tag {
        NodeTag::Zone => Some((SolidShape::Slab, slab_top, zone_height(node))),
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

/// The point of a card's billboard that stands on the projected center of its top face:
/// the icon's center, or the box center for a card without an icon, relative to the flat
/// box's top-left corner (section 12.4, rule 2).
fn card_anchor(node: &NodeGeometry, flat: BoxRect) -> (f32, f32) {
    match node.part(PartName::Icon) {
        Some(icon) => {
            let (icon_x, icon_y) = center(icon.bounds);
            (icon_x - flat.x, icon_y - flat.y)
        }
        None => (flat.width / 2.0, flat.height / 2.0),
    }
}

/// The billboard of a body node before any offset, or None for a node without members.
fn node_billboard(
    index: usize,
    node: &NodeGeometry,
    shape: SolidShape,
    base_z: f32,
    height: f32,
) -> Option<Billboard> {
    let flat = billboard_flat_box(node)?;
    let top_z = base_z + height;
    let (z, screen, role) = match shape {
        SolidShape::Slab => {
            let corner = project_point(node.bounds.x, node.bounds.y, top_z, ZERO_OFFSET);
            let tab_corner = ScreenPoint {
                x: corner.x,
                y: corner.y - flat.height / 2.0,
            };
            (
                top_z,
                box_at(tab_corner, flat.width, flat.height),
                BillboardRole::Content,
            )
        }
        SolidShape::Block if node.tag == NodeTag::Pcard => {
            let (center_x, center_y) = center(node.bounds);
            let anchor = project_point(center_x, center_y, top_z, ZERO_OFFSET);
            let (across, down) = card_anchor(node, flat);
            let corner = ScreenPoint {
                x: anchor.x - across,
                y: anchor.y - down,
            };
            (
                top_z,
                box_at(corner, flat.width, flat.height),
                BillboardRole::Content,
            )
        }
        SolidShape::Block => {
            let corner = project_point(flat.x, flat.y, top_z, ZERO_OFFSET);
            (
                top_z,
                box_at(corner, flat.width, flat.height),
                BillboardRole::Content,
            )
        }
        SolidShape::Surface => {
            let (center_x, center_y) = center(flat);
            let anchor = project_point(center_x, center_y, base_z, ZERO_OFFSET);
            (
                base_z,
                box_around(anchor, flat.width, flat.height),
                BillboardRole::Tag,
            )
        }
    };
    let marks = moved(&flat_marks(node), (screen.x - flat.x, screen.y - flat.y));
    Some(Billboard {
        owner: node.pointer.clone(),
        node: Some(index),
        role,
        flat,
        z,
        screen,
        opaque: role == BillboardRole::Tag || node.tag == NodeTag::Zone,
        marks,
        contained: matches!(
            node.tag,
            NodeTag::Fact | NodeTag::Note | NodeTag::Text | NodeTag::Callout | NodeTag::Frame
        ),
    })
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
    fn empty() -> Self {
        Extent {
            min_x: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            min_y: f32::INFINITY,
            max_y: f32::NEG_INFINITY,
        }
    }

    fn is_empty(&self) -> bool {
        self.min_x > self.max_x
    }

    fn add(&mut self, point: ScreenPoint) {
        self.min_x = self.min_x.min(point.x);
        self.max_x = self.max_x.max(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_y = self.max_y.max(point.y);
    }

    fn add_box(&mut self, screen: BoxRect) {
        for corner in shapes::rectangle_corners(screen) {
            self.add(corner);
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

/// Flat length of a link arrowhead under iso, by kind (section 12.3, rule 7). Each is at
/// least four times the widest stroke the kind has in any theme, and it does not depend on
/// the theme, so the drawn canvas does not either.
pub fn iso_link_arrowhead_length(kind: PipeKind) -> f32 {
    match kind {
        PipeKind::Blue => 18.0,
        PipeKind::Dash => 15.0,
        PipeKind::Gray | PipeKind::Pink | PipeKind::Deny => 14.0,
    }
}

/// The three flat vertices of an arrowhead of `length` whose tip is `tip` and which points
/// along the unit vector (direction_x, direction_y): tip, then the two base corners.
pub(crate) fn arrowhead_vertices(
    tip: (f32, f32),
    direction: (f32, f32),
    length: f32,
) -> [(f32, f32); 3] {
    let (tip_x, tip_y) = tip;
    let (direction_x, direction_y) = direction;
    let half_width = length * ARROWHEAD_WIDTH_RATIO;
    let base_x = tip_x - direction_x * length;
    let base_y = tip_y - direction_y * length;
    let across_x = -direction_y * half_width;
    let across_y = direction_x * half_width;
    [
        (tip_x, tip_y),
        (base_x + across_x, base_y + across_y),
        (base_x - across_x, base_y - across_y),
    ]
}

/// The wire of a pipe from dot center to dot center, or a Tee's spine, at `z`, before any
/// offset; None for a node with neither.
fn surface_stroke(node: &NodeGeometry, z: f32) -> Option<(ScreenPoint, ScreenPoint)> {
    if let (Some(start), Some(end)) = (node.part(PartName::DotStart), node.part(PartName::DotEnd)) {
        let (start_x, start_y) = center(start.bounds);
        let (end_x, end_y) = center(end.bounds);
        return Some((
            project_point(start_x, start_y, z, ZERO_OFFSET),
            project_point(end_x, end_y, z, ZERO_OFFSET),
        ));
    }
    let spine = node.part(PartName::Spine)?;
    let (center_x, _) = center(spine.bounds);
    Some((
        project_point(center_x, spine.bounds.y, z, ZERO_OFFSET),
        project_point(center_x, spine.bounds.bottom(), z, ZERO_OFFSET),
    ))
}

/// Adds a pipe's or tee's surface primitives. The geometry does not record which ends carry
/// an arrowhead, so both the dot and the arrowhead extent of each end are added. The extra
/// vertices do not move the extent: an arrowhead on a pipe end lies inside the pipe's own
/// box, and that box lies inside a zone top face or the gap between two zones.
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
                for (x, y) in arrowhead_vertices(tip, outward, ARROWHEAD_LENGTH_PX) {
                    extent.add(project_point(x, y, z, ZERO_OFFSET));
                }
            }
        }
    }
    if let Some((start, end)) = surface_stroke(node, z) {
        extent.add(start);
        extent.add(end);
    }
}

/// The screen segments of a link path before any offset.
fn path_strokes(path: &[IsoPoint]) -> Vec<(ScreenPoint, ScreenPoint)> {
    path.windows(2)
        .filter_map(|pair| {
            let (Some(start), Some(end)) = (pair.first(), pair.get(1)) else {
                return None;
            };
            Some((
                project_point(start.x, start.y, start.z, ZERO_OFFSET),
                project_point(end.x, end.y, end.z, ZERO_OFFSET),
            ))
        })
        .collect()
}

/// A link's routed polyline with the adjustments of section 12.3 rule 8: one leg across two
/// facing sides that share a span, then every inner leg moved clear of the zone edges.
fn adjusted_route(route: &LinkRoute, geometry: &PageGeometry, solids: &[Solid]) -> Vec<PagePoint> {
    let blocks: Vec<BoxRect> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Block)
        .map(|solid| solid.footprint)
        .collect();
    let zones: Vec<BoxRect> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Slab)
        .map(|solid| solid.footprint)
        .collect();
    let ends = (
        geometry.nodes.get(route.from_node),
        geometry.nodes.get(route.to_node),
    );
    let straight = match ends {
        (Some(from), Some(to)) => {
            route::straightened(&route.points, from.bounds, to.bounds, &blocks)
        }
        _ => None,
    };
    let points = straight.unwrap_or_else(|| route.points.clone());
    route::kept_clear(&points, &zones, &blocks)
}

/// Every link: its adjusted flat route, and its path laid over the filled slabs and cut back
/// at its endpoint blocks.
fn link_paths(
    geometry: &PageGeometry,
    terrain: &[Terrain],
    solids: &[Solid],
) -> Vec<(Vec<PagePoint>, Vec<IsoPoint>)> {
    let block_silhouette = |node: usize| {
        solids
            .iter()
            .find(|solid| solid.node == node && solid.shape == SolidShape::Block)
            .map(|solid| solid.silhouette)
    };
    geometry
        .links
        .iter()
        .take(LINKS_MAX)
        .map(|route| {
            let points = adjusted_route(route, geometry, solids);
            let mut path = drape::drape(&points, terrain);
            if let Some(outline) = block_silhouette(route.from_node) {
                drape::trim_start_at(&mut path, &outline);
            }
            if let Some(outline) = block_silhouette(route.to_node) {
                drape::trim_end_at(&mut path, &outline);
            }
            (points, path)
        })
        .collect()
}

/// Pitch of the candidate tag centers along a link.
const TAG_CANDIDATE_STEP_PX: f32 = 4.0;
/// The candidate pitch along a link is coarsened until it gives at most this many.
const TAG_CANDIDATES_MAX: usize = 1024;

/// Candidate screen centers for a link tag, before any offset, with the terrain height under
/// each: points along the adjusted route, nearest the middle of its longest leg first
/// (section 12.4, rule 3).
fn tag_candidates(points: &[PagePoint], terrain: &[Terrain]) -> Vec<(ScreenPoint, f32)> {
    let legs = route::legs(points);
    let total: f32 = legs.iter().map(route::FlatSegment::length).sum();
    let mut preferred = 0.0;
    let mut longest = f32::NEG_INFINITY;
    let mut walked = 0.0;
    for leg in &legs {
        let length = leg.length();
        if length > longest + GEOMETRY_EPSILON {
            longest = length;
            preferred = walked + length / 2.0;
        }
        walked += length;
    }
    let mut step = TAG_CANDIDATE_STEP_PX;
    for _ in 0..16 {
        if ((total / step) as usize) < TAG_CANDIDATES_MAX {
            break;
        }
        step *= 2.0;
    }
    let count = ((total / step) as usize + 1).min(TAG_CANDIDATES_MAX);
    let mut distances: Vec<f32> = (0..count).map(|index| index as f32 * step).collect();
    distances.push(preferred);
    distances.sort_by(|first, second| {
        (first - preferred)
            .abs()
            .total_cmp(&(second - preferred).abs())
    });
    distances
        .into_iter()
        .filter_map(|distance| {
            let mut remaining = distance;
            for leg in &legs {
                let length = leg.length();
                if remaining <= length + GEOMETRY_EPSILON {
                    let fraction = if length > 0.0 {
                        (remaining / length).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let x = leg.start.x + fraction * (leg.end.x - leg.start.x);
                    let y = leg.start.y + fraction * (leg.end.y - leg.start.y);
                    let z = drape::ground_z(terrain, x, y);
                    return Some((project_point(x, y, z, ZERO_OFFSET), z));
                }
                remaining -= length;
            }
            None
        })
        .collect()
}

const GEOMETRY_EPSILON: f32 = stencil_layout::GEOMETRY_EPSILON_PX;

/// A link tag waiting for placement: its billboard index, its link index and the candidate
/// centers along the link.
struct PendingTag {
    billboard: usize,
    link: usize,
    candidates: Vec<(ScreenPoint, f32)>,
}

/// Everything the billboard placement of section 12.4 rule 3 reads.
struct PlacementInput<'a> {
    solids: &'a [Solid],
    /// Visible slab edges, every link path and every wire and spine.
    strokes: &'a [(ScreenPoint, ScreenPoint)],
    /// The strokes of each link's path, in link order.
    link_strokes: &'a [Vec<(ScreenPoint, ScreenPoint)>],
    /// Wires and spines.
    surface_strokes: &'a [(ScreenPoint, ScreenPoint)],
}

/// Moves each link tag along its link to the first spot clear of every block, every other
/// link and every billboard placed before it, then each card billboard to the first clear
/// spot over its block (section 12.4, rule 3). A billboard with no clear spot keeps its
/// first placement, which `iso-labels-clear` then reports.
fn place_billboards(
    input: &PlacementInput<'_>,
    billboards: &mut [Billboard],
    tags: &[PendingTag],
    cards: &[usize],
) {
    let blocks: Vec<&[ScreenPoint]> = input
        .solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Block)
        .map(|solid| solid.silhouette.as_slice())
        .collect();
    for (position, tag) in tags.iter().enumerate() {
        let unplaced: Vec<usize> = tags
            .iter()
            .skip(position)
            .map(|pending| pending.billboard)
            .chain(cards.iter().copied())
            .collect();
        let boxes: Vec<BoxRect> = billboards
            .iter()
            .enumerate()
            .filter(|(index, _)| !unplaced.contains(index))
            .map(|(_, placed)| placed.screen)
            .collect();
        let mut strokes: Vec<(ScreenPoint, ScreenPoint)> = input.surface_strokes.to_vec();
        for (other, other_strokes) in input.link_strokes.iter().enumerate() {
            if other != tag.link {
                strokes.extend_from_slice(other_strokes);
            }
        }
        let obstacles = Obstacles {
            shapes: blocks.clone(),
            strokes,
            boxes,
        };
        let Some(billboard) = billboards.get(tag.billboard) else {
            continue;
        };
        let size = (billboard.screen.width, billboard.screen.height);
        let found = placement::place_centered(size, &tag.candidates, &obstacles);
        if let (Some((screen, z)), Some(slot)) = (found, billboards.get_mut(tag.billboard)) {
            slot.marks = vec![screen];
            slot.screen = screen;
            slot.z = z;
        }
    }
    for (position, &billboard_index) in cards.iter().enumerate() {
        let Some(billboard) = billboards.get(billboard_index) else {
            continue;
        };
        let Some(owner) = billboard.node else {
            continue;
        };
        let Some(solid) = input.solids.iter().find(|solid| solid.node == owner) else {
            continue;
        };
        let unplaced: Vec<usize> = cards.iter().skip(position).copied().collect();
        let boxes: Vec<BoxRect> = billboards
            .iter()
            .enumerate()
            .filter(|(index, _)| !unplaced.contains(index))
            .map(|(_, placed)| placed.screen)
            .collect();
        let shapes: Vec<&[ScreenPoint]> = input
            .solids
            .iter()
            .filter(|other| other.shape == SolidShape::Block && other.node != owner)
            .map(|other| other.silhouette.as_slice())
            .collect();
        let obstacles = Obstacles {
            shapes,
            strokes: input.strokes.to_vec(),
            boxes,
        };
        let top_z = solid.base_z + solid.height;
        let origin = billboard.screen;
        let marks = moved(&billboard.marks, (-origin.x, -origin.y));
        let anchor = project_point(
            solid.footprint.x + solid.footprint.width / 2.0,
            solid.footprint.y + solid.footprint.height / 2.0,
            top_z,
            ZERO_OFFSET,
        );
        let request = placement::Request {
            region: solid.footprint,
            z: top_z,
            size: (origin.width, origin.height),
            anchor: (anchor.x - origin.x, anchor.y - origin.y),
            marks: &marks,
            clearance: placement::ISO_CARD_CLEARANCE_PX,
        };
        if let Some(screen) = placement::place(&request, &obstacles)
            && let Some(slot) = billboards.get_mut(billboard_index)
        {
            slot.marks = moved(&marks, (screen.x, screen.y));
            slot.screen = screen;
            slot.z = top_z;
        }
    }
}

/// The pieces of every slab edge that no opaque solid painted after it covers, plus the
/// given strokes, which are drawn over the faces.
fn visible_strokes(
    solids: &[Solid],
    drawn_over: &[(ScreenPoint, ScreenPoint)],
) -> Vec<(ScreenPoint, ScreenPoint)> {
    let all: Vec<&Solid> = solids.iter().collect();
    let mut strokes: Vec<(ScreenPoint, ScreenPoint)> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Slab)
        .flat_map(|slab| visible_slab_edges(slab, &all))
        .collect();
    strokes.extend_from_slice(drawn_over);
    strokes
}

/// The body index and per-node facts, after asserting nodes[0] is the root and /body is
/// present.
fn body_facts(geometry: &PageGeometry) -> Result<(usize, NodeFacts), RenderError> {
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
    let Some(body_index) = geometry
        .nodes
        .iter()
        .position(|node| node.pointer == body_pointer)
    else {
        return Err(RenderError::GeometryMismatch {
            expected: body_pointer,
            found: NodePointer::root().child("<absent>"),
        });
    };
    let facts = node_facts(geometry, body_index)?;
    Ok((body_index, facts))
}

/// The geometry the projection draws: the body zoomed by section 12.2 rule 7, and the zoom.
/// The SVG writer draws faces and billboards from it, so they agree with the scene.
pub fn zoomed_geometry(geometry: &PageGeometry) -> Result<(PageGeometry, f32), RenderError> {
    let (body_index, facts) = body_facts(geometry)?;
    let origin = geometry
        .nodes
        .get(body_index)
        .map_or((0.0, 0.0), |body| (body.bounds.x, body.bounds.y));
    let zoom = zoom::fit_zoom(geometry, &facts.in_body, geometry.canvas.width);
    let mut zoomed = zoom::zoomed(geometry, &facts.in_body, origin, zoom);
    for node in &mut zoomed.nodes {
        if node.tag == NodeTag::Pcard {
            stack_card(node);
        }
    }
    Ok((zoomed, zoom))
}

/// Gap between a card's icon chip and the text stacked under it.
const CARD_STACK_GAP_PX: f32 = 4.0;

/// Moves a card's billboard text under its icon, centered on it (section 12.4, rule 2). The
/// flat card puts the text beside the icon, which on a thin iso block reaches past the
/// block's back edge and into the zone edge behind it.
fn stack_card(node: &mut NodeGeometry) {
    let Some(icon) = node.part(PartName::Icon).map(|part| part.bounds) else {
        return;
    };
    let text = node
        .parts
        .iter()
        .filter(|part| part.name != PartName::Icon && billboard_member(node.tag, part.name))
        .map(member_box)
        .reduce(union);
    let Some(text) = text else {
        return;
    };
    let delta = (
        icon.x + icon.width / 2.0 - (text.x + text.width / 2.0),
        icon.y + icon.height / 2.0 + crate::svg::ICON_CHIP_SIZE_PX / 2.0 + CARD_STACK_GAP_PX
            - text.y,
    );
    for part in &mut node.parts {
        if part.name != PartName::Icon && billboard_member(node.tag, part.name) {
            part.bounds.x += delta.0;
            part.bounds.y += delta.1;
        }
    }
}

/// Asserts nodes[0] is the root and /body is present, then zooms and projects the body.
pub fn project_page(geometry: &PageGeometry) -> Result<IsoScene, RenderError> {
    let (zoomed, zoom) = zoomed_geometry(geometry)?;
    project_zoomed(&zoomed, zoom)
}

/// `project_page` over a geometry `zoomed_geometry` returned.
pub fn project_zoomed(geometry: &PageGeometry, zoom: f32) -> Result<IsoScene, RenderError> {
    let (body_index, facts) = body_facts(geometry)?;
    let Some(body) = geometry.nodes.get(body_index) else {
        return Err(RenderError::GeometryMismatch {
            expected: NodePointer::root().child("body"),
            found: NodePointer::root().child("<absent>"),
        });
    };

    let mut extent = Extent::empty();
    let mut solids = Vec::new();
    let mut billboards = Vec::new();
    let mut cards = Vec::new();
    let mut surface_strokes = Vec::new();
    for (index, node) in geometry.nodes.iter().enumerate() {
        if !facts.in_body.get(index).copied().unwrap_or(false) {
            continue;
        }
        let slab_top = facts.slab_top.get(index).copied().unwrap_or(0.0);
        let Some((shape, flat_base_z, flat_height)) = solid_shape(node, slab_top) else {
            continue;
        };
        // Heights scale with the zoom, so a zoomed scene keeps the proportions of section
        // 12.3.
        let (base_z, height) = (flat_base_z * zoom, flat_height * zoom);
        let outline = silhouette(node.bounds, base_z, base_z + height, ZERO_OFFSET);
        match shape {
            SolidShape::Slab | SolidShape::Block => {
                for point in outline {
                    extent.add(point);
                }
            }
            SolidShape::Surface => {
                add_surface_extent(&mut extent, node, base_z);
                surface_strokes.extend(surface_stroke(node, base_z));
            }
        }
        let opaque = match shape {
            SolidShape::Slab => height > 0.0,
            SolidShape::Block => !matches!(node.tag, NodeTag::Note | NodeTag::Frame),
            SolidShape::Surface => false,
        };
        solids.push(Solid {
            node: index,
            pointer: node.pointer.clone(),
            shape,
            base_z,
            height,
            silhouette: outline,
            opaque,
            footprint: node.bounds,
        });
        if let Some(billboard) = node_billboard(index, node, shape, base_z, height) {
            if node.tag == NodeTag::Pcard {
                cards.push(billboards.len());
            }
            billboards.push(billboard);
        }
    }

    let terrain: Vec<Terrain> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Slab && solid.height > 0.0)
        .map(|solid| Terrain {
            bounds: solid.footprint,
            top: solid.base_z + solid.height,
        })
        .collect();
    let routes = link_paths(geometry, &terrain, &solids);
    let mut link_strokes = Vec::with_capacity(routes.len());
    let mut tags = Vec::new();
    for (position, (route, (points, path))) in geometry.links.iter().zip(&routes).enumerate() {
        link_strokes.push(path_strokes(path));
        for point in path {
            extent.add(project_point(point.x, point.y, point.z, ZERO_OFFSET));
        }
        // LinkRoute does not carry the arrow value, so both ends get an arrowhead extent.
        let head = iso_link_arrowhead_length(route.kind);
        for (tip, direction) in [start_direction(path), end_direction(path)]
            .into_iter()
            .flatten()
        {
            for (x, y) in arrowhead_vertices((tip.x, tip.y), direction, head) {
                extent.add(project_point(x, y, tip.z, ZERO_OFFSET));
            }
        }
        if let Some(tag) = route.tag {
            let candidates = tag_candidates(points, &terrain);
            let (anchor, z) = candidates.first().copied().unwrap_or_else(|| {
                let (center_x, center_y) = center(tag);
                let z = drape::ground_z(&terrain, center_x, center_y);
                (project_point(center_x, center_y, z, ZERO_OFFSET), z)
            });
            let screen = box_around(anchor, tag.width, tag.height);
            tags.push(PendingTag {
                billboard: billboards.len(),
                link: position,
                candidates,
            });
            billboards.push(Billboard {
                owner: NodePointer::root().child("links").index(route.index),
                node: None,
                role: BillboardRole::Tag,
                flat: tag,
                z,
                screen,
                opaque: true,
                marks: vec![screen],
                contained: false,
            });
        }
    }
    let mut drawn_over = surface_strokes.clone();
    for strokes in &link_strokes {
        drawn_over.extend_from_slice(strokes);
    }
    let strokes = visible_strokes(&solids, &drawn_over);
    let input = PlacementInput {
        solids: &solids,
        strokes: &strokes,
        link_strokes: &link_strokes,
        surface_strokes: &surface_strokes,
    };
    place_billboards(&input, &mut billboards, &tags, &cards);
    for billboard in &billboards {
        extent.add_box(billboard.screen);
    }

    let body_bounds = body.bounds;
    if extent.is_empty() {
        for (x, y) in [
            (body_bounds.x, body_bounds.y),
            (body_bounds.right(), body_bounds.y),
            (body_bounds.x, body_bounds.bottom()),
            (body_bounds.right(), body_bounds.bottom()),
        ] {
            extent.add(project_point(x, y, 0.0, ZERO_OFFSET));
        }
    }
    let drawn_width = extent.max_x - extent.min_x;
    let canvas_width = geometry.canvas.width.max(drawn_width + 2.0 * ISO_MARGIN_PX);
    let offset = ScreenPoint {
        x: (canvas_width - drawn_width) / 2.0 - extent.min_x,
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
        billboard.marks = moved(&billboard.marks, (offset.x, offset.y));
    }
    let projected_height = extent.max_y - extent.min_y;
    let footer_shift = projected_height - body_bounds.height;
    let canvas = Size {
        width: canvas_width,
        height: geometry.canvas.height + footer_shift,
    };
    Ok(IsoScene {
        canvas,
        offset,
        footer_shift,
        solids,
        billboards,
        link_paths: routes.into_iter().map(|(_, path)| path).collect(),
        link_kinds: geometry.links.iter().map(|route| route.kind).collect(),
        zoom,
    })
}

/// The pieces of a slab's edges that no opaque solid painted after it covers. Painter
/// order is geometry order (section 12.5), so those are the solids of later nodes.
fn visible_slab_edges(slab: &Solid, solids: &[&Solid]) -> Vec<(ScreenPoint, ScreenPoint)> {
    let occluders: Vec<&[ScreenPoint]> = solids
        .iter()
        .filter(|solid| solid.opaque && solid.node > slab.node)
        .map(|solid| solid.silhouette.as_slice())
        .collect();
    slab.slab_edges()
        .into_iter()
        .flat_map(|(start, end)| shapes::visible_pieces(start, end, &occluders))
        .collect()
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
    debug_assert!(
        scene.billboards.len() <= billboard_limit,
        "iso-labels-clear would drop billboards past {billboard_limit}"
    );
    let billboards: Vec<&Billboard> = scene.billboards.iter().take(billboard_limit).collect();
    debug_assert!(
        scene.solids.len() <= NODES_MAX,
        "iso-labels-clear would drop solids past {NODES_MAX}"
    );
    let solids: Vec<&Solid> = scene.solids.iter().take(NODES_MAX).collect();
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
        for block in solids
            .iter()
            .filter(|solid| solid.shape == SolidShape::Block)
        {
            if billboard.node == Some(block.node) {
                continue;
            }
            examined += 1;
            if rectangle_overlaps_polygon(billboard.screen, &block.silhouette) {
                defects.push(Defect {
                    pointer: billboard.owner.clone(),
                    message: format!("{} covers block {}", describe(billboard), block.pointer),
                });
            }
        }
    }
    let slabs: Vec<(&Solid, Vec<(ScreenPoint, ScreenPoint)>)> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Slab)
        .map(|slab| (*slab, visible_slab_edges(slab, &solids)))
        .collect();
    for billboard in billboards.iter().filter(|billboard| !billboard.opaque) {
        for (slab, edges) in &slabs {
            examined += 1;
            let crossed = edges.iter().any(|(start, end)| {
                billboard
                    .marks
                    .iter()
                    .any(|mark| segment_crosses_box(*start, *end, *mark))
            });
            if crossed {
                defects.push(Defect {
                    pointer: billboard.owner.clone(),
                    message: format!(
                        "{} is crossed by an edge of slab {}",
                        describe(billboard),
                        slab.pointer
                    ),
                });
            }
        }
    }
    for billboard in billboards.iter().filter(|billboard| billboard.contained) {
        let Some(block) = solids
            .iter()
            .find(|solid| Some(solid.node) == billboard.node && solid.shape == SolidShape::Block)
        else {
            continue;
        };
        examined += 1;
        let leaves = billboard.marks.iter().any(|mark| {
            !shapes::rectangle_corners(*mark)
                .into_iter()
                .all(|corner| shapes::point_in_convex(corner, &block.silhouette))
        });
        if leaves {
            defects.push(Defect {
                pointer: billboard.owner.clone(),
                message: format!("{} leaves its block", describe(billboard)),
            });
        }
    }
    CheckReport {
        check: CheckName::IsoLabelsClear,
        examined,
        defects,
        not_applicable: None,
    }
}

/// NotApplicable with reason "projection is flat" for None and "page has no links" for a
/// scene without links; section 12.7 otherwise. Each leg of each drawn link path, taken in
/// flat px with its risers dropped, is one examined unit.
pub fn iso_links_clear(scene: Option<&IsoScene>) -> CheckReport {
    let Some(scene) = scene else {
        return CheckReport::not_applicable(CheckName::IsoLinksClear, "projection is flat");
    };
    if scene.link_paths.is_empty() {
        return CheckReport::not_applicable(CheckName::IsoLinksClear, "page has no links");
    }
    debug_assert!(
        scene.link_paths.len() <= LINKS_MAX,
        "iso-links-clear would drop links past {LINKS_MAX}"
    );
    let zones: Vec<(&Solid, [route::FlatSegment; 4])> = scene
        .solids
        .iter()
        .take(NODES_MAX)
        .filter(|solid| solid.shape == SolidShape::Slab)
        .map(|solid| (solid, route::footprint_edges(solid.footprint)))
        .collect();
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for (index, (path, kind)) in scene
        .link_paths
        .iter()
        .zip(&scene.link_kinds)
        .take(LINKS_MAX)
        .enumerate()
    {
        let pointer = NodePointer::root().child("links").index(index);
        let flat: Vec<PagePoint> = path
            .iter()
            .map(|point| PagePoint {
                x: point.x,
                y: point.y,
            })
            .collect();
        let legs = route::legs(&route::corners(&flat));
        let least_last = 2.0 * iso_link_arrowhead_length(*kind);
        for (leg_index, leg) in legs.iter().enumerate() {
            examined += 1;
            for (zone, edges) in &zones {
                let closest = edges
                    .iter()
                    .filter_map(|edge| route::parallel_gap(*leg, *edge))
                    .fold(f32::INFINITY, f32::min);
                if closest < ISO_LINK_CLEARANCE_PX - GEOMETRY_EPSILON {
                    defects.push(Defect {
                        pointer: pointer.clone(),
                        message: format!(
                            "leg {leg_index} runs {closest:.2} px beside an edge of zone {}, closer than {ISO_LINK_CLEARANCE_PX}",
                            zone.pointer
                        ),
                    });
                }
            }
            if let Some(previous) = leg_index.checked_sub(1).and_then(|before| legs.get(before))
                && route::turns_back(*previous, *leg)
            {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!("leg {leg_index} turns back on the leg before it"),
                });
            }
            if leg_index + 1 == legs.len() && leg.length() < least_last - GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!(
                        "last leg is {:.2} px, shorter than two arrowheads ({least_last:.2} px)",
                        leg.length()
                    ),
                });
            }
        }
    }
    CheckReport {
        check: CheckName::IsoLinksClear,
        examined,
        defects,
        not_applicable: None,
    }
}
