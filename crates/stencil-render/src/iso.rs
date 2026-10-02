//! The section 12 isometric projection: solids, labels on their planes, link paths and the drawn canvas,
//! computed from `PageGeometry` alone, and the `iso-labels-clear` check over the result.

mod drape;
mod exit;
mod route;
mod shapes;
pub(crate) mod sprites;
pub(crate) mod tube;
pub use tube::ISO_TUBE_RADIUS_PX;
mod zoom;

use stencil_layout::{
    BoxRect, LinkRoute, NodeGeometry, NodeTag, PageGeometry, Part, PartName, Size, TextAlign,
};
use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, Line, Link, NODES_MAX, PagePoint, PipeForm, Side};

use crate::RenderError;

pub(crate) use drape::{end_direction, start_direction};
pub use route::{ISO_LINK_CLEARANCE_PX, ISO_STRAIGHT_SHARED_MIN_PX};
pub(crate) use shapes::unit_direction;
pub use zoom::{ISO_FILL_FRACTION, ISO_ZOOM_MAX};

use drape::Terrain;
use shapes::{convex_hull, ellipse_points, polygons_overlap, segment_crosses_convex};

pub use stencil_layout::ISO_BLOCK_HEIGHT_PX;
use stencil_layout::{ISO_LABEL_CLEARANCE_PX, shape_height_px, unturned_box};
use stencil_model::Shape;
/// cos 30 degrees, written out so every build uses the same f32.
pub const ISO_COS_30: f32 = 0.866_025_4;
/// sin 30 degrees.
pub const ISO_SIN_30: f32 = 0.5;
/// Smallest side margin of the projected body.
pub const ISO_MARGIN_PX: f32 = 20.0;
/// Half width over length of every arrowhead, as in flat.
const ARROWHEAD_WIDTH_RATIO: f32 =
    stencil_layout::ARROWHEAD_WIDTH_PX / 2.0 / stencil_layout::ARROWHEAD_LENGTH_PX;

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
///
/// Two px frames meet here. Solids, link paths and the dots and wires of a pipe are in
/// scene px: the zoomed flat geometry, projected with `project_point`. The parts a label
/// draws (`plane_member`) stay in layout px and reach the screen through the label's
/// `map`, which zooms and projects in one affine step, so text and icons scale with the
/// body like everything else on the plane.
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
    /// Every label on its plane (section 12.4): one per body node that draws parts on its
    /// solid, in geometry order, then one per link with a tag, in link order. Each is
    /// painted right after its solid or link, so later solids cover it.
    pub labels: Vec<Label>,
    /// One per `PageGeometry.links` entry: the routed polyline adjusted by section 12.3
    /// rule 8, laid over the slabs and cut back at its endpoint blocks (rule 7), in zoomed
    /// flat px.
    pub link_paths: Vec<Vec<IsoPoint>>,
    /// The line and effective tint of each link, in `link_paths` order.
    pub link_kinds: Vec<(Line, Option<u8>)>,
    /// The body zoom of section 12.2, rule 7, about `origin`.
    pub zoom: f32,
    /// The flat top-left corner of `/body`, about which the zoom scales.
    pub origin: (f32, f32),
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
    /// The theme's slab thickness for a filled slab, 0 for a ring zone and a surface,
    /// ISO_BLOCK_HEIGHT_PX for a block.
    pub height: f32,
    /// The six section 12.3 silhouette vertices of the footprint box in canvas px, offset
    /// included.
    pub silhouette: [ScreenPoint; 6],
    /// The convex outline the solid covers on screen, offset included: the six silhouette
    /// vertices of a box form, the hull of the top and base ellipses of a round one. The
    /// checks and the link cut-back read this.
    pub outline: Vec<ScreenPoint>,
    /// The form of an item's block (section 12.3); card for every other solid.
    pub form: Shape,
    /// True when the solid's faces are filled in every theme, so it hides what was drawn
    /// before it: a slab with height and every block except Note and Frame.
    pub opaque: bool,
    /// The box the solid rises from in zoomed flat px: the node's border box, or an item's
    /// Footprint part when its shape is not a card.
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

pub use stencil_layout::Axis;

/// An affine map from layout px to screen px: `(a x + c y + e, b x + d y + f)`, the form of
/// an SVG `matrix(a b c d e f)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneMap {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl PlaneMap {
    /// The plane at height `z`: zoom about `origin`, then the projection of section 12.2.
    pub fn plane(z: f32, zoom: f32, origin: (f32, f32), offset: ScreenPoint) -> PlaneMap {
        let (origin_x, origin_y) = origin;
        // project(origin + zoom (p - origin), z) + offset, expanded in p.
        let shift_x = (1.0 - zoom) * origin_x;
        let shift_y = (1.0 - zoom) * origin_y;
        PlaneMap {
            a: zoom * ISO_COS_30,
            b: zoom * ISO_SIN_30,
            c: -zoom * ISO_COS_30,
            d: zoom * ISO_SIN_30,
            e: (shift_x - shift_y) * ISO_COS_30 + offset.x,
            f: (shift_x + shift_y) * ISO_SIN_30 - z + offset.y,
        }
    }

    /// `self` after a quarter turn of the layout about `pivot`, so a label laid out along
    /// x reads along y: layout right becomes flat -y and layout down becomes flat +x.
    pub fn turned_about(self, pivot: (f32, f32)) -> PlaneMap {
        // turned(x, y) = (px + (y - py), py - (x - px)); then self.
        let (px, py) = pivot;
        let e = self.a * (px - py) + self.c * (py + px) + self.e;
        let f = self.b * (px - py) + self.d * (py + px) + self.f;
        PlaneMap {
            a: -self.c,
            b: -self.d,
            c: self.a,
            d: self.b,
            e,
            f,
        }
    }

    pub fn apply(&self, x: f32, y: f32) -> ScreenPoint {
        ScreenPoint {
            x: self.a * x + self.c * y + self.e,
            y: self.b * x + self.d * y + self.f,
        }
    }

    /// The four corners of a layout box on screen, top-left first then clockwise in layout
    /// terms.
    pub fn corners(&self, bounds: BoxRect) -> [ScreenPoint; 4] {
        [
            self.apply(bounds.x, bounds.y),
            self.apply(bounds.right(), bounds.y),
            self.apply(bounds.right(), bounds.bottom()),
            self.apply(bounds.x, bounds.bottom()),
        ]
    }

    /// `self` applied to layout points moved by `delta` first: `M'(p) = M(p + delta)`.
    pub fn shifted_layout(self, delta: (f32, f32)) -> PlaneMap {
        PlaneMap {
            e: self.e + self.a * delta.0 + self.c * delta.1,
            f: self.f + self.b * delta.0 + self.d * delta.1,
            ..self
        }
    }

    pub fn shifted(self, delta: ScreenPoint) -> PlaneMap {
        PlaneMap {
            e: self.e + delta.x,
            f: self.f + delta.y,
            ..self
        }
    }
}

/// The parts of one node or link tag, lying on the node's plane (section 12.4). The parts
/// themselves stay in `NodeGeometry` or `LinkRoute` in layout px; `map` puts them on
/// screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// The owning node's pointer, or `/links/<i>` for a link tag.
    pub owner: NodePointer,
    /// Geometry index of the owner; None for a link tag.
    pub node: Option<usize>,
    /// Height of the plane: the slab or block top, a pipe's wire, the terrain under a link
    /// tag.
    pub z: f32,
    pub axis: Axis,
    /// Layout px to screen px, offset included.
    pub map: PlaneMap,
    /// The union of the member part boxes, in layout px.
    pub flat: BoxRect,
    /// `flat` through `map`.
    pub corners: [ScreenPoint; 4],
    /// Each text run's ink box through `map`. The checks of section 12.7 test these.
    pub marks: Vec<[ScreenPoint; 4]>,
    /// True when the parts sit on a box of their own that hides what lies under them: a
    /// pipe, tee or link tag.
    pub opaque: bool,
    /// True when every mark must lie on the owner block's top face: the parts of a Pcard,
    /// Fact, Note, Text, Callout or Frame, which are laid out to fit the block.
    pub contained: bool,
    /// Which of the owner's plane members the label draws.
    pub parts: LabelParts,
}

/// The members of a label: every plane member of the owner, or for an item with a
/// footprint shape the icon alone on the solid's top or the text alone on the floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelParts {
    All,
    Icon,
    Text,
}

impl LabelParts {
    /// True when a plane member of the owner belongs to this label.
    pub fn holds(self, part: PartName) -> bool {
        match self {
            LabelParts::All => true,
            LabelParts::Icon => part == PartName::Icon,
            LabelParts::Text => part != PartName::Icon,
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

/// A Box drawn as a ring on its parent's top instead of a slab: the strong tone untinted,
/// which has no fill in any theme (section 13.4 rule 12).
pub fn is_ring_zone(node: &NodeGeometry) -> bool {
    node.tag == NodeTag::Zone && node.container.is_some_and(|look| look.is_ring(node.tint))
}

/// The parts a node draws on its plane (section 12.4), by the node's tag. The rest of a
/// node's parts belong to its solid: a pipe's dots and wire, a frame's bar and body.
pub(crate) fn plane_member(tag: NodeTag, part: PartName) -> bool {
    match tag {
        NodeTag::Zone => part == PartName::Label,
        NodeTag::Pcard => matches!(
            part,
            PartName::Icon
                | PartName::FactBox
                | PartName::BuiltBox
                | PartName::AskBox
                | PartName::FunctionName
                | PartName::ProductName
                | PartName::Fact
                | PartName::Built
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
        | NodeTag::Col
        | NodeTag::Lanes => false,
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
}

/// What the projection needs beyond the geometry (section 13.4 rule 12): the theme's slab
/// thickness, and per geometry node whether it is a ring. A Box is a ring when its tone is
/// strong and it has no effective tint, so the rings depend on the document and the
/// grammar, never on the theme.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidInputs {
    /// Slab thickness of a filled Box, and the rise of each filled nested Box over its
    /// parent.
    pub slab_thickness_px: f32,
    /// Indexed like `PageGeometry.nodes`.
    pub rings: Vec<bool>,
    /// Indexed like `Page.links`: true when the link has no authored `via` and no
    /// `from_side`, so it may be routed again to leave its slab (section 13.11 rule 2).
    pub slab_exits: Vec<bool>,
}

impl SolidInputs {
    /// `links` are the page's links, which `geometry.links` routes.
    pub fn new(geometry: &PageGeometry, links: &[Link], slab_thickness_px: f32) -> Self {
        SolidInputs {
            slab_thickness_px,
            rings: geometry.nodes.iter().map(is_ring_zone).collect(),
            slab_exits: links
                .iter()
                .take(LINKS_MAX)
                .map(|link| link.via.is_empty() && link.from_side.is_none())
                .collect(),
        }
    }
}

/// The height of the solid of the zone at geometry index `index`.
fn zone_height(index: usize, inputs: &SolidInputs) -> f32 {
    if inputs.rings.get(index).copied().unwrap_or(false) {
        0.0
    } else {
        inputs.slab_thickness_px
    }
}

/// Top of the nearest enclosing Zone's solid per node, 0 without a Zone ancestor. Parents
/// precede children, so one pass in geometry order sees every parent's top first.
fn slab_tops(geometry: &PageGeometry, inputs: &SolidInputs) -> Vec<f32> {
    let mut tops = vec![0.0; geometry.nodes.len()];
    for (index, node) in geometry.nodes.iter().enumerate() {
        let Some(parent) = node.parent.filter(|parent| *parent < index) else {
            continue;
        };
        let parent_top = tops.get(parent).copied().unwrap_or(0.0);
        let top = match geometry.nodes.get(parent) {
            Some(parent_node) if parent_node.tag == NodeTag::Zone => {
                parent_top + zone_height(parent, inputs)
            }
            _ => parent_top,
        };
        if let Some(slot) = tops.get_mut(index) {
            *slot = top;
        }
    }
    tops
}

fn node_facts(geometry: &PageGeometry, body: usize) -> Result<NodeFacts, RenderError> {
    let count = geometry.nodes.len();
    let mut facts = NodeFacts {
        in_body: vec![false; count],
    };
    for (index, node) in geometry.nodes.iter().enumerate() {
        let Some(parent) = node.parent else {
            continue;
        };
        if geometry
            .nodes
            .get(parent)
            .filter(|_| parent < index)
            .is_none()
        {
            return Err(RenderError::GeometryMismatch {
                expected: NodePointer::root().child("<parent before child>"),
                found: node.pointer.clone(),
            });
        }
        let parent_in_body = facts.in_body.get(parent).copied().unwrap_or(false);
        if let Some(slot) = facts.in_body.get_mut(index) {
            *slot = parent == body || parent_in_body;
        }
    }
    Ok(facts)
}

/// Shape, base and height of the solid of the body node at `index`; None for Row and Col.
/// `slab_top` is the top of the nearest enclosing Zone's solid.
fn solid_shape(
    node: &NodeGeometry,
    index: usize,
    slab_top: f32,
    inputs: &SolidInputs,
) -> Option<(SolidShape, f32, f32)> {
    match node.tag {
        NodeTag::Zone => Some((SolidShape::Slab, slab_top, zone_height(index, inputs))),
        NodeTag::Pcard => Some((
            SolidShape::Block,
            slab_top,
            shape_height_px(node.shape.unwrap_or_default()),
        )),
        NodeTag::Fact | NodeTag::Note | NodeTag::Text | NodeTag::Callout | NodeTag::Frame => {
            Some((SolidShape::Block, slab_top, ISO_BLOCK_HEIGHT_PX))
        }
        // A band lies flat on its plane, so its tag does too.
        NodeTag::Pipe if node.pipe_form == Some(PipeForm::Band) => {
            Some((SolidShape::Surface, slab_top, 0.0))
        }
        NodeTag::Pipe | NodeTag::Tee => Some((
            SolidShape::Surface,
            slab_top,
            2.0 * tube::ISO_TUBE_RADIUS_PX,
        )),
        NodeTag::Page
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::LegendEntry
        | NodeTag::Foot
        | NodeTag::Row
        | NodeTag::Col
        | NodeTag::Lanes => None,
    }
}

/// The labels a body node draws on its planes (section 12.4): a zone's name at its slab
/// top, a block's content at its top, a pipe's or tee's tag at its wire. An item whose
/// shape is not a card has two: its icon on the solid's top and its text on the floor it
/// stands on. Empty for a node with no plane member.
fn node_labels(
    index: usize,
    node: &NodeGeometry,
    shape: SolidShape,
    (base_z, top_z): (f32, f32),
    plane: &dyn Fn(f32) -> PlaneMap,
) -> Vec<Label> {
    let members: Vec<&Part> = node
        .parts
        .iter()
        .filter(|part| plane_member(node.tag, part.name))
        .collect();
    let footprint_shape = node.tag == NodeTag::Pcard
        && node.shape.is_some_and(|form| form != Shape::Card)
        && node.part(PartName::Footprint).is_some();
    if footprint_shape {
        let (decal, floor): (Vec<&Part>, Vec<&Part>) =
            members.iter().partition(|part| part.name == PartName::Icon);
        return [
            parts_label(
                index,
                node,
                &decal,
                top_z,
                plane,
                (false, true),
                LabelParts::Icon,
            ),
            parts_label(
                index,
                node,
                &floor,
                base_z,
                plane,
                (false, false),
                LabelParts::Text,
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
    }
    let contained = matches!(
        node.tag,
        NodeTag::Pcard
            | NodeTag::Fact
            | NodeTag::Note
            | NodeTag::Text
            | NodeTag::Callout
            | NodeTag::Frame
    );
    parts_label(
        index,
        node,
        &members,
        top_z,
        plane,
        (shape == SolidShape::Surface, contained),
        LabelParts::All,
    )
    .into_iter()
    .collect()
}

/// One label over `members` at height `z`; None when they are empty.
fn parts_label(
    index: usize,
    node: &NodeGeometry,
    members: &[&Part],
    z: f32,
    plane: &dyn Fn(f32) -> PlaneMap,
    (opaque, contained): (bool, bool),
    parts: LabelParts,
) -> Option<Label> {
    let (axis, pivot) = label_axis(members);
    let local: Vec<Part> = members
        .iter()
        .map(|part| local_part(part, axis, pivot))
        .collect();
    let flat = local.iter().map(member_box).reduce(union)?;
    let map = match axis {
        Axis::X => plane(z),
        Axis::Y => plane(z).turned_about(pivot),
    };
    Some(Label {
        owner: node.pointer.clone(),
        node: Some(index),
        z,
        axis,
        map,
        flat,
        corners: map.corners(flat),
        marks: local
            .iter()
            .filter(|part| part.text.is_some())
            .map(|part| map.corners(member_box(part)))
            .collect(),
        opaque,
        contained,
        parts,
    })
}

/// The convex outline a solid covers on screen before the offset: the six silhouette
/// vertices of a box form; for a round form the hull of the base and top ellipses, each a
/// circle inscribed in the footprint mapped by rule 6 of section 12.2.
fn solid_outline(
    form: Shape,
    footprint: BoxRect,
    base_z: f32,
    top_z: f32,
    vertices: &[ScreenPoint; 6],
) -> Vec<ScreenPoint> {
    match form {
        Shape::Cylinder | Shape::Stack => {
            let (center_x, center_y) = center(footprint);
            let radius = footprint.width.min(footprint.height) / 2.0;
            let (radius_x, radius_y) = ellipse_radii(radius);
            let mut points = ellipse_points(
                project_point(center_x, center_y, base_z, ZERO_OFFSET),
                radius_x,
                radius_y,
                ROUND_OUTLINE_POINTS,
            );
            points.extend(ellipse_points(
                project_point(center_x, center_y, top_z, ZERO_OFFSET),
                radius_x,
                radius_y,
                ROUND_OUTLINE_POINTS,
            ));
            convex_hull(&points)
        }
        Shape::Card | Shape::Tile | Shape::Tower | Shape::Block => vertices.to_vec(),
        Shape::Figure => sprites::figure_outline(footprint, base_z, top_z),
        Shape::Laptop => sprites::laptop_outline(footprint, base_z, top_z),
        Shape::Phone => sprites::phone_outline(footprint, base_z, top_z),
    }
}

/// Points sampled around each ellipse of a round solid's outline.
const ROUND_OUTLINE_POINTS: usize = 16;

/// The convex outline a pipe or tee covers on screen: a tube from dot center to dot
/// center with a ring's radius at each end, a band's flat body, or a tee's spine tube.
/// None for a pipe without its dot parts, which keeps its box outline.
fn surface_outline(node: &NodeGeometry, base_z: f32, zoom: f32) -> Option<Vec<ScreenPoint>> {
    let tube_hull = |start: (f32, f32), end: (f32, f32)| {
        let tube = tube::Tube {
            start,
            end,
            floor_z: base_z,
            radius: ISO_TUBE_RADIUS_PX * zoom,
        };
        let run = tube.run()?;
        let body = tube::body(&tube, ZERO_OFFSET)?;
        let ring_axis_z = base_z + ISO_TUBE_RADIUS_PX * zoom;
        let mut points = body.outline.to_vec();
        for at in [start, end] {
            points.extend(tube::cross_section(
                at,
                run,
                ring_axis_z,
                tube::FLANGE_RADIUS_PX * zoom,
                ZERO_OFFSET,
            ));
        }
        Some(convex_hull(&points))
    };
    match node.tag {
        NodeTag::Pipe => {
            let start = box_center(node.part(PartName::DotStart)?.bounds);
            let end = box_center(node.part(PartName::DotEnd)?.bounds);
            if node.pipe_form == Some(PipeForm::Band) {
                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                let length = dx.hypot(dy);
                if length <= GEOMETRY_EPSILON {
                    return None;
                }
                let run = (dx / length, dy / length);
                return Some(convex_hull(&tube::band_body(
                    (start, end),
                    run,
                    base_z,
                    ZERO_OFFSET,
                )));
            }
            tube_hull(start, end)
        }
        NodeTag::Tee => {
            let spine = node.part(PartName::Spine)?.bounds;
            let center_x = spine.x + spine.width / 2.0;
            tube_hull((center_x, spine.y), (center_x, spine.bottom()))
        }
        _ => None,
    }
}

fn box_center(bounds: BoxRect) -> (f32, f32) {
    (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// The screen semi-axes of a flat circle of radius r (section 12.2 rule 6).
pub fn ellipse_radii(radius: f32) -> (f32, f32) {
    (radius * 1.5_f32.sqrt(), radius * 0.5_f32.sqrt())
}

/// The axis a set of parts reads along, and the pivot of its turn: layout lays a y run out
/// as a strip taller than its text is wide (section 12.4), turned about the center of the
/// parts' union. Parts without a strip read along x.
pub(crate) fn label_axis(parts: &[&Part]) -> (Axis, (f32, f32)) {
    let strip = parts.iter().any(|part| {
        part.text.as_ref().is_some_and(|run| {
            run.metrics.width_px > part.bounds.width + stencil_layout::GEOMETRY_EPSILON_PX
                && part.bounds.height > part.bounds.width
        })
    });
    let pivot = parts
        .iter()
        .map(|part| part.bounds)
        .reduce(union)
        .map_or((0.0, 0.0), center);
    if strip {
        (Axis::Y, pivot)
    } else {
        (Axis::X, pivot)
    }
}

/// The part in the frame it is drawn in: its own box along x, its strip turned back about
/// `pivot` along y.
pub(crate) fn local_part(part: &Part, axis: Axis, pivot: (f32, f32)) -> Part {
    match axis {
        Axis::X => part.clone(),
        Axis::Y => Part {
            bounds: unturned_box(part.bounds, pivot),
            ..part.clone()
        },
    }
}

/// A link's tag on the terrain under the midpoint of the drawn route's longest leg
/// (section 12.4), reading along the axis layout gave it. The drawn route may differ from
/// the layout route (section 13.11 rule 2 and the slab exit), so the tag moves with it: the
/// move is a layout-px shift folded into the map. None for a link without a label.
fn link_label(
    route: &LinkRoute,
    drawn: &[PagePoint],
    z: f32,
    zoom: f32,
    origin: (f32, f32),
    plane: &dyn Fn(f32) -> PlaneMap,
    fraction: f32,
) -> Option<Label> {
    let tag = route.tag?;
    let (layout_x, layout_y) = center(tag);
    let (scene_center, _) = stencil_layout::longest_segment_point(drawn, fraction);
    let shift = (
        origin.0 + (scene_center.x - origin.0) / zoom - layout_x,
        origin.1 + (scene_center.y - origin.1) / zoom - layout_y,
    );
    let members: Vec<&Part> = route.parts.iter().collect();
    let (axis, pivot) = label_axis(&members);
    let local: Vec<Part> = members
        .iter()
        .map(|part| local_part(part, axis, pivot))
        .collect();
    let flat = local.iter().map(member_box).reduce(union)?;
    let map = match axis {
        Axis::X => plane(z).shifted_layout(shift),
        Axis::Y => plane(z).shifted_layout(shift).turned_about(pivot),
    };
    let marks = local
        .iter()
        .filter(|part| part.text.is_some())
        .map(|part| map.corners(member_box(part)))
        .collect();
    Some(Label {
        owner: NodePointer::root().child("links").index(route.index),
        node: None,
        z,
        axis,
        map,
        flat,
        corners: map.corners(flat),
        marks,
        opaque: true,
        contained: false,
        parts: LabelParts::All,
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
}

const ZERO_OFFSET: ScreenPoint = ScreenPoint { x: 0.0, y: 0.0 };

/// Flat length of a link arrowhead under iso, by line and effective tint (section 12.3,
/// rule 7): 18 for solid slot 1, 15 for every dash, 14 for the rest. Each is at least four
/// times the widest stroke the line has in any theme, and it does not depend on the theme,
/// so the drawn canvas does not either.
/// Radius of a link's tube at zoom 1 (section 12.3 rule 7): the primary attachment is the
/// thickest, a dashed line next, every other line the thinnest, the same in every theme.
pub fn link_tube_radius(line: Line, tint: Option<u8>) -> f32 {
    match (line, tint) {
        (Line::Solid, Some(1)) => 4.5,
        (Line::Dash, _) => 3.0,
        (Line::Gray | Line::Solid | Line::Deny, _) => 2.5,
    }
}

pub fn iso_link_arrowhead_length(line: Line, tint: Option<u8>) -> f32 {
    match (line, tint) {
        (Line::Solid, Some(1)) => 18.0,
        (Line::Dash, _) => 15.0,
        (Line::Gray | Line::Solid | Line::Deny, _) => 14.0,
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

/// Adds a pipe's or tee's tube primitives. The geometry does not record which ends carry
/// an arrowhead, so the widest ring a tube end can carry, a flange or a cone base, is added
/// at both ends. The extra vertices do not move the extent: a pipe end lies inside the
/// pipe's own box, and that box lies inside a zone top face or the gap between two zones.
fn add_surface_extent(extent: &mut Extent, node: &NodeGeometry, z: f32) {
    let Some((start, end)) = surface_stroke(node, z) else {
        return;
    };
    extent.add(start);
    extent.add(end);
    let flat_ends = match (node.part(PartName::DotStart), node.part(PartName::DotEnd)) {
        (Some(start), Some(end)) => (center(start.bounds), center(end.bounds)),
        _ => match node.part(PartName::Spine) {
            Some(spine) => {
                let (center_x, _) = center(spine.bounds);
                (
                    (center_x, spine.bounds.y),
                    (center_x, spine.bounds.bottom()),
                )
            }
            None => return,
        },
    };
    let Some(run) = unit_direction(flat_ends.0, flat_ends.1) else {
        return;
    };
    let ring = tube::FLANGE_RADIUS_PX
        .max(tube::CONE_RADIUS_PX)
        .max(tube::BAND_HEAD_HALF_WIDTH_PX);
    let axis_z = z + tube::ISO_TUBE_RADIUS_PX;
    for at in [flat_ends.0, flat_ends.1] {
        for point in tube::cross_section(at, run, axis_z, ring, ZERO_OFFSET) {
            extent.add(point);
        }
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

/// A link's routed polyline with the adjustments of section 12.3 rule 8 (one leg across two
/// facing sides that share a span, then every inner leg moved clear of the zone edges),
/// replaced by the slab exit route of section 13.11 rule 2 when the link may take one.
fn adjusted_route(
    route: &LinkRoute,
    geometry: &PageGeometry,
    solids: &[Solid],
    inputs: &SolidInputs,
    clearance: f32,
) -> Vec<PagePoint> {
    let adjusted = rule_8_route(route, geometry, solids, clearance);
    let may_exit = inputs.slab_exits.get(route.index).copied().unwrap_or(false);
    let exit = may_exit
        .then(|| exit::slab_exit_route(route, geometry, solids, &adjusted))
        .flatten();
    exit.unwrap_or(adjusted)
}

fn rule_8_route(
    route: &LinkRoute,
    geometry: &PageGeometry,
    solids: &[Solid],
    clearance: f32,
) -> Vec<PagePoint> {
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
    // How much nearer its front corner a hidden-side end lands after the cut of rule 7:
    // the block's height for a box form; a sprite's hull has no corner to keep clear of.
    let hidden_setback = |node: usize| {
        solids
            .iter()
            .find(|solid| solid.node == node && solid.shape == SolidShape::Block)
            .filter(|solid| has_box_outline(solid.form))
            .map_or(0.0, |solid| solid.height)
    };
    let straight = match ends {
        (Some(from), Some(to)) => route::straightened(
            &route.points,
            (attach_bounds(from), hidden_setback(route.from_node)),
            (attach_bounds(to), hidden_setback(route.to_node)),
            &blocks,
        ),
        _ => None,
    };
    let points = straight.unwrap_or_else(|| route.points.clone());
    route::kept_clear(&points, &zones, &blocks, clearance)
}

/// The box a link attaches to: an item's footprint when it stands on one, else its bounds.
fn attach_bounds(node: &NodeGeometry) -> BoxRect {
    node.part(PartName::Footprint)
        .map_or(node.bounds, |part| part.bounds)
}

/// How far apart two neighbouring links of a bundle run, center to center: their tubes
/// plus ISO_LINK_GAP_PX of air, never less than their two cone bases, which is what
/// `iso-links-apart` asks of two ends on one node, and never less than either link's tag
/// half-height plus the other's tube and the air, so a tag lying on one tube does not
/// cover the other.
fn bundle_spacing((radius_a, tag_a): (f32, f32), (radius_b, tag_b): (f32, f32)) -> f32 {
    (radius_a + radius_b + ISO_LINK_GAP_PX)
        .max(LINK_CONE_RADIUS_SCALE * (radius_a + radius_b))
        .max(tag_a / 2.0 + radius_b + ISO_LINK_GAP_PX)
        .max(tag_b / 2.0 + radius_a + ISO_LINK_GAP_PX)
}

/// A link's tube radius at the zoom and its tag's height across the run (0 without a tag),
/// what `bundle_spacing` reads.
fn link_width(route: &LinkRoute, zoom: f32) -> (f32, f32) {
    let radius = link_tube_radius(route.line, route.tint) * zoom;
    let tag = route.tag.map_or(0.0, |tag| tag.width.min(tag.height));
    (radius, tag)
}

/// A link's place among the links whose adjusted routes coincide with its own, corner for
/// corner: its position and the bundle's size, in link order. None for a route no other
/// link follows. Such a bundle is moved whole, and its tags are staggered along the leg.
fn shared_route_places(routes: &[Vec<PagePoint>]) -> Vec<Option<(usize, usize)>> {
    routes
        .iter()
        .enumerate()
        .map(|(index, points)| {
            let size = routes
                .iter()
                .filter(|other| same_route(other, points))
                .count();
            (size >= 2).then(|| {
                let position = routes
                    .iter()
                    .take(index)
                    .filter(|other| same_route(other, points))
                    .count();
                (position, size)
            })
        })
        .collect()
}

/// Whether two routes coincide corner for corner.
fn same_route(a: &[PagePoint], b: &[PagePoint]) -> bool {
    let (a, b) = (route::corners(a), route::corners(b));
    a.len() == b.len()
        && a.iter().zip(&b).all(|(p, q)| {
            (p.x - q.x).abs() <= GEOMETRY_EPSILON && (p.y - q.y).abs() <= GEOMETRY_EPSILON
        })
}

/// The side of its node a drawn end sits on, read from the leg that leaves or reaches it:
/// a start leg toward +x leaves the right side, an end leg toward +x reaches the left side.
fn end_side(points: &[PagePoint], at_start: bool) -> Option<Side> {
    let kept = route::corners(points);
    let (a, b) = if at_start {
        (kept.first()?, kept.get(1)?)
    } else {
        (kept.get(kept.len().checked_sub(2)?)?, kept.last()?)
    };
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    if dx.abs() <= GEOMETRY_EPSILON && dy.abs() <= GEOMETRY_EPSILON {
        return None;
    }
    let toward = if dx.abs() >= dy.abs() {
        if dx > 0.0 { Side::Right } else { Side::Left }
    } else if dy > 0.0 {
        Side::Bottom
    } else {
        Side::Top
    };
    Some(if at_start {
        toward
    } else {
        match toward {
            Side::Right => Side::Left,
            Side::Left => Side::Right,
            Side::Bottom => Side::Top,
            Side::Top => Side::Bottom,
        }
    })
}

/// How far each end of each link moves along the side it shares with other ends on the
/// same node (section 12.3 rule 8). The ends on each (node, side) pair, the side read from
/// the leg that leaves or reaches the end, are laid out in link order `bundle_spacing`
/// apart along +x for a top or bottom side and +y for a left or right side, from where the
/// layout put them. On a visible side the group is centered on the mean of its attach
/// points. On a block's top or left side the cut of rule 7 lands an arrival the block's
/// height nearer the front corner than its attach point, so there the group is centered
/// half the block's height behind that mean. Each offset is the move from the end's own
/// attach point to its place, with whether the side is hidden.
fn end_offsets(
    routes: &[&LinkRoute],
    adjusted: &[Vec<PagePoint>],
    zoom: f32,
    blocks: &[(usize, f32)],
) -> Vec<[(f32, bool); 2]> {
    // Where an end sits along its side.
    let along = |points: &[PagePoint], at_start: bool, side: Side| -> Option<f32> {
        let point = if at_start {
            points.first()?
        } else {
            points.last()?
        };
        Some(match side {
            Side::Top | Side::Bottom => point.x,
            Side::Left | Side::Right => point.y,
        })
    };
    let group_of = |node: usize, side: Side| -> Vec<(usize, bool, f32)> {
        routes
            .iter()
            .zip(adjusted)
            .enumerate()
            .flat_map(|(index, (route, points))| {
                [(route.from_node, true), (route.to_node, false)]
                    .into_iter()
                    .filter(move |(other_node, at_start)| {
                        *other_node == node && end_side(points, *at_start) == Some(side)
                    })
                    .filter_map(move |(_, at_start)| {
                        along(points, at_start, side).map(|at| (index, at_start, at))
                    })
            })
            .collect()
    };
    routes
        .iter()
        .zip(adjusted)
        .enumerate()
        .map(|(index, (route, points))| {
            let offset = |node: usize, at_start: bool| -> (f32, bool) {
                let Some(side) = end_side(points, at_start) else {
                    return (0.0, false);
                };
                let group = group_of(node, side);
                if group.len() < 2 {
                    return (0.0, false);
                }
                let widths: Vec<(f32, f32)> = group
                    .iter()
                    .filter_map(|(member, _, _)| {
                        routes.get(*member).map(|route| link_width(route, zoom))
                    })
                    .collect();
                let mut positions = Vec::with_capacity(widths.len());
                let mut at = 0.0;
                for pair in widths.windows(2) {
                    positions.push(at);
                    if let (Some(a), Some(b)) = (pair.first(), pair.get(1)) {
                        at += bundle_spacing(*a, *b);
                    }
                }
                positions.push(at);
                let Some(position) = group
                    .iter()
                    .position(|(member, start, _)| *member == index && *start == at_start)
                else {
                    return (0.0, false);
                };
                let own = group.get(position).map_or(0.0, |(_, _, at)| *at);
                let block_height = blocks
                    .iter()
                    .find(|(block, _)| *block == node)
                    .map(|(_, height)| *height);
                let hidden = block_height.is_some() && matches!(side, Side::Top | Side::Left);
                let setback = if hidden {
                    block_height.unwrap_or(0.0) / 2.0
                } else {
                    0.0
                };
                let mean = group.iter().map(|(_, _, at)| *at).sum::<f32>() / group.len() as f32;
                let target =
                    mean - setback + positions.get(position).copied().unwrap_or(0.0) - at / 2.0;
                (target - own, hidden)
            };
            [offset(route.from_node, true), offset(route.to_node, false)]
        })
        .collect()
}

/// The unit vector along the side an end sits on: +x for a top or bottom side, +y for a
/// left or right side.
fn side_axis(side: Side) -> (f32, f32) {
    match side {
        Side::Top | Side::Bottom => (1.0, 0.0),
        Side::Left | Side::Right => (0.0, 1.0),
    }
}

/// A link as drawn: its adjusted flat route, its path laid over the filled slabs and cut
/// back at its endpoint blocks, and its place in a bundle of coinciding routes.
struct DrawnLink {
    points: Vec<PagePoint>,
    path: Vec<IsoPoint>,
    place: Option<(usize, usize)>,
}

/// Every link: its adjusted flat route, and its path laid over the filled slabs and cut back
/// at its endpoint blocks.
fn link_paths(
    geometry: &PageGeometry,
    terrain: &[Terrain],
    solids: &[Solid],
    inputs: &SolidInputs,
    zoom: f32,
) -> Vec<DrawnLink> {
    let block_silhouette = |node: usize| {
        solids
            .iter()
            .find(|solid| solid.node == node && solid.shape == SolidShape::Block)
            .map(|solid| solid.outline.clone())
    };
    let blocks: Vec<BoxRect> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Block)
        .map(|solid| solid.footprint)
        .collect();
    let routes: Vec<&LinkRoute> = geometry.links.iter().take(LINKS_MAX).collect();
    let block_heights: Vec<(usize, f32)> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Block)
        .map(|solid| (solid.node, solid.height))
        .collect();
    // Routes adjusted at the base clearance give each end's side and attach point; a bundle
    // of coinciding routes is then laid out again with its widest offset added to the zone
    // clearance, so every member's inner legs stay clear once the bundle is moved whole.
    let base: Vec<Vec<PagePoint>> = routes
        .iter()
        .map(|route| adjusted_route(route, geometry, solids, inputs, ISO_LINK_CLEARANCE_PX))
        .collect();
    let offsets = end_offsets(&routes, &base, zoom, &block_heights);
    let shared = shared_route_places(&base);
    let adjusted: Vec<Vec<PagePoint>> = routes
        .iter()
        .zip(&base)
        .zip(&shared)
        .map(|((route, points), place)| {
            if place.is_none() {
                return points.clone();
            }
            let widest = base
                .iter()
                .zip(&offsets)
                .filter(|(other, _)| same_route(other, points))
                .map(|(_, [start, end])| start.0.abs().max(end.0.abs()))
                .fold(0.0, f32::max);
            adjusted_route(
                route,
                geometry,
                solids,
                inputs,
                ISO_LINK_CLEARANCE_PX + widest,
            )
        })
        .collect();
    let on_side = |point: PagePoint, node: usize| {
        geometry.nodes.get(node).is_none_or(|node| {
            let bounds = attach_bounds(node);
            point.x >= bounds.x - GEOMETRY_EPSILON
                && point.x <= bounds.right() + GEOMETRY_EPSILON
                && point.y >= bounds.y - GEOMETRY_EPSILON
                && point.y <= bounds.bottom() + GEOMETRY_EPSILON
        })
    };
    routes
        .iter()
        .zip(adjusted)
        .zip(&offsets)
        .zip(&shared)
        .map(|(((route, adjusted), [start_offset, end_offset]), place)| {
            let whole = place.is_some() || route::corners(&adjusted).len() < 3;
            let moved = if whole {
                // One sideways shift for the whole route: from an end on a hidden side,
                // whose group is anchored, else from the end that shares a side.
                let (offset, at_start) = match (start_offset, end_offset) {
                    (_, (offset, true)) => (*offset, false),
                    ((offset, true), _) => (*offset, true),
                    (_, (offset, false)) if offset.abs() > GEOMETRY_EPSILON => (*offset, false),
                    ((offset, false), _) => (*offset, true),
                };
                match (
                    offset.abs() > GEOMETRY_EPSILON,
                    end_side(&adjusted, at_start),
                ) {
                    (true, Some(side)) => {
                        let axis = side_axis(side);
                        let kept = route::corners(&adjusted);
                        let leg = if at_start {
                            (kept.first(), kept.get(1))
                        } else {
                            (kept.get(kept.len().wrapping_sub(2)), kept.last())
                        };
                        // The left normal of the leg, which `offset_polyline` shifts along.
                        let left = match leg {
                            (Some(a), Some(b)) => {
                                let length = (b.x - a.x).hypot(b.y - a.y).max(GEOMETRY_EPSILON);
                                ((b.y - a.y) / length, -(b.x - a.x) / length)
                            }
                            _ => (0.0, 0.0),
                        };
                        let sign = left.0 * axis.0 + left.1 * axis.1;
                        Some(route::offset_polyline(&adjusted, offset * sign))
                    }
                    _ => None,
                }
            } else {
                let mut points = adjusted.clone();
                for (offset, at_start) in [(start_offset.0, true), (end_offset.0, false)] {
                    if offset.abs() <= GEOMETRY_EPSILON {
                        continue;
                    }
                    if let Some(side) = end_side(&points, at_start) {
                        let axis = side_axis(side);
                        points = route::shift_end_leg(
                            &points,
                            (axis.0 * offset, axis.1 * offset),
                            at_start,
                        );
                    }
                }
                Some(points)
            };
            let points = match moved {
                Some(moved)
                    if moved
                        .first()
                        .is_some_and(|first| on_side(*first, route.from_node))
                        && moved
                            .last()
                            .is_some_and(|last| on_side(*last, route.to_node))
                        && !route::enters_any(&moved, &blocks) =>
                {
                    moved
                }
                _ => adjusted,
            };
            let place = *place;
            let mut path = drape::drape(&points, terrain);
            if let Some(outline) = block_silhouette(route.from_node) {
                drape::trim_start_at(&mut path, &outline);
                drape::land_start_at(&mut path, &outline);
            }
            if let Some(outline) = block_silhouette(route.to_node) {
                drape::trim_end_at(&mut path, &outline);
                drape::land_end_at(&mut path, &outline);
            }
            DrawnLink {
                points,
                path,
                place,
            }
        })
        .collect()
}

const GEOMETRY_EPSILON: f32 = stencil_layout::GEOMETRY_EPSILON_PX;

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
    let zoomed = zoom::zoomed(geometry, &facts.in_body, origin, zoom);
    Ok((zoomed, zoom))
}

/// Asserts nodes[0] is the root and /body is present, then zooms and projects the body.
pub fn project_page(
    geometry: &PageGeometry,
    inputs: &SolidInputs,
) -> Result<IsoScene, RenderError> {
    let (zoomed, zoom) = zoomed_geometry(geometry)?;
    project_zoomed(&zoomed, zoom, inputs)
}

/// `project_page` over a geometry `zoomed_geometry` returned.
pub fn project_zoomed(
    geometry: &PageGeometry,
    zoom: f32,
    inputs: &SolidInputs,
) -> Result<IsoScene, RenderError> {
    let (body_index, facts) = body_facts(geometry)?;
    let slab_top = slab_tops(geometry, inputs);
    let Some(body) = geometry.nodes.get(body_index) else {
        return Err(RenderError::GeometryMismatch {
            expected: NodePointer::root().child("body"),
            found: NodePointer::root().child("<absent>"),
        });
    };

    let origin = (body.bounds.x, body.bounds.y);
    let unshifted = |z: f32| PlaneMap::plane(z, zoom, origin, ZERO_OFFSET);
    let mut extent = Extent::empty();
    let mut solids = Vec::new();
    let mut labels = Vec::new();
    for (index, node) in geometry.nodes.iter().enumerate() {
        if !facts.in_body.get(index).copied().unwrap_or(false) {
            continue;
        }
        let top = slab_top.get(index).copied().unwrap_or(0.0);
        let Some((shape, flat_base_z, flat_height)) = solid_shape(node, index, top, inputs) else {
            continue;
        };
        // Heights scale with the zoom, so a zoomed scene keeps the proportions of section
        // 12.3.
        let (base_z, height) = (flat_base_z * zoom, flat_height * zoom);
        let form = if node.tag == NodeTag::Pcard {
            node.shape.unwrap_or_default()
        } else {
            Shape::Card
        };
        let footprint = node
            .part(PartName::Footprint)
            .map_or(node.bounds, |part| part.bounds);
        let vertices = silhouette(footprint, base_z, base_z + height, ZERO_OFFSET);
        let outline = match shape {
            SolidShape::Surface => surface_outline(node, base_z, zoom).unwrap_or_else(|| {
                solid_outline(form, footprint, base_z, base_z + height, &vertices)
            }),
            SolidShape::Slab | SolidShape::Block => {
                solid_outline(form, footprint, base_z, base_z + height, &vertices)
            }
        };
        match shape {
            SolidShape::Slab | SolidShape::Block => {
                for point in &outline {
                    extent.add(*point);
                }
            }
            SolidShape::Surface => {
                add_surface_extent(&mut extent, node, base_z);
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
            silhouette: vertices,
            outline,
            form,
            opaque,
            footprint,
        });
        for label in node_labels(index, node, shape, (base_z, base_z + height), &unshifted) {
            for corner in label.corners {
                extent.add(corner);
            }
            labels.push(label);
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
    let routes = link_paths(geometry, &terrain, &solids, inputs, zoom);
    for (route, drawn) in geometry.links.iter().zip(&routes) {
        let (points, path, place) = (&drawn.points, &drawn.path, &drawn.place);
        for point in path {
            extent.add(project_point(point.x, point.y, point.z, ZERO_OFFSET));
        }
        // LinkRoute does not carry the arrow value, so both ends get an arrowhead extent.
        let head = iso_link_arrowhead_length(route.line, route.tint);
        for (tip, direction) in [start_direction(path), end_direction(path)]
            .into_iter()
            .flatten()
        {
            for (x, y) in arrowhead_vertices((tip.x, tip.y), direction, head) {
                extent.add(project_point(x, y, tip.z, ZERO_OFFSET));
            }
        }
        // The tag pill lies on the tube's top, as a pipe's does.
        let tag_z = path.first().map_or(0.0, |point| point.z)
            + 2.0 * link_tube_radius(route.line, route.tint) * zoom;
        let fraction = place.map_or(0.5, |(position, size)| {
            (position as f32 + 0.5) / size as f32
        });
        if let Some(label) = link_label(route, points, tag_z, zoom, origin, &unshifted, fraction) {
            for corner in label.corners {
                extent.add(corner);
            }
            labels.push(label);
        }
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
        for point in &mut solid.outline {
            point.x += offset.x;
            point.y += offset.y;
        }
    }
    for label in &mut labels {
        label.map = label.map.shifted(offset);
        label.corners = label.map.corners(label.flat);
        for mark in &mut label.marks {
            for corner in mark {
                corner.x += offset.x;
                corner.y += offset.y;
            }
        }
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
        labels,
        link_paths: routes.into_iter().map(|drawn| drawn.path).collect(),
        link_kinds: geometry
            .links
            .iter()
            .map(|route| (route.line, route.tint))
            .collect(),
        zoom,
        origin,
    })
}

/// The pieces of a slab's edges that no opaque solid painted after it covers. Painter
/// order is geometry order (section 12.5), so those are the solids of later nodes.
fn visible_slab_edges(slab: &Solid, solids: &[&Solid]) -> Vec<(ScreenPoint, ScreenPoint)> {
    let occluders: Vec<&[ScreenPoint]> = solids
        .iter()
        .filter(|solid| solid.opaque && solid.node > slab.node)
        .map(|solid| solid.outline.as_slice())
        .collect();
    slab.slab_edges()
        .into_iter()
        .flat_map(|(start, end)| shapes::visible_pieces(start, end, &occluders))
        .collect()
}

fn describe(label: &Label) -> String {
    let x = label
        .corners
        .iter()
        .map(|corner| corner.x)
        .fold(f32::INFINITY, f32::min);
    let y = label
        .corners
        .iter()
        .map(|corner| corner.y)
        .fold(f32::INFINITY, f32::min);
    format!(
        "label {} at {:.2},{:.2} along {}",
        label.owner,
        x,
        y,
        label.axis.as_str()
    )
}

/// NotApplicable with reason "projection is flat" for None; section 12.7 otherwise.
/// The air two labels keep between them on screen (section 12.7).
pub const ISO_LABEL_GAP_PX: f32 = 4.0;

/// The distance between two convex polygons on screen: 0 when they overlap, else the
/// shortest vertex-to-edge distance either way.
fn polygon_distance(first: &[ScreenPoint], second: &[ScreenPoint]) -> f32 {
    if polygons_overlap(first, second) {
        return 0.0;
    }
    let one_way = |from: &[ScreenPoint], to: &[ScreenPoint]| {
        from.iter()
            .map(|point| polygon_edge_distance(*point, to))
            .fold(f32::INFINITY, f32::min)
    };
    one_way(first, second).min(one_way(second, first))
}

/// The distance from a point to a convex polygon on screen: 0 inside or on its boundary.
fn point_polygon_distance(point: ScreenPoint, polygon: &[ScreenPoint]) -> f32 {
    if shapes::point_in_convex(point, polygon) {
        return 0.0;
    }
    polygon_edge_distance(point, polygon)
}

/// The distance from a point to the nearest stroke of a drawn link path on screen.
fn path_distance(point: ScreenPoint, path: &[IsoPoint], offset: ScreenPoint) -> f32 {
    path_strokes(path)
        .iter()
        .map(|(start, end)| {
            let shift = |at: &ScreenPoint| ScreenPoint {
                x: at.x + offset.x,
                y: at.y + offset.y,
            };
            point_segment_distance(point, shift(start), shift(end))
        })
        .fold(f32::INFINITY, f32::min)
}

fn link_index(owner: &NodePointer) -> Option<usize> {
    owner.as_str().strip_prefix("/links/")?.parse().ok()
}

pub fn iso_labels_clear(scene: Option<&IsoScene>) -> CheckReport {
    let Some(scene) = scene else {
        return CheckReport::not_applicable(CheckName::IsoLabelsClear, "projection is flat");
    };
    let label_limit = NODES_MAX + LINKS_MAX;
    debug_assert!(
        scene.labels.len() <= label_limit,
        "iso-labels-clear would drop labels past {label_limit}"
    );
    let labels: Vec<&Label> = scene.labels.iter().take(label_limit).collect();
    debug_assert!(
        scene.solids.len() <= NODES_MAX,
        "iso-labels-clear would drop solids past {NODES_MAX}"
    );
    let solids: Vec<&Solid> = scene.solids.iter().take(NODES_MAX).collect();
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    // 1. Two labels overlap, or lie closer than the gap.
    for (later_index, later) in labels.iter().enumerate() {
        for earlier in labels.iter().take(later_index) {
            examined += 1;
            let apart = polygon_distance(&earlier.corners, &later.corners);
            if polygons_overlap(&earlier.corners, &later.corners) {
                defects.push(Defect {
                    pointer: later.owner.clone(),
                    message: format!("{} overlaps label {}", describe(later), earlier.owner),
                });
            } else if apart < ISO_LABEL_GAP_PX - GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: later.owner.clone(),
                    message: format!(
                        "{} lies {apart:.2} px from label {}, closer than {ISO_LABEL_GAP_PX}",
                        describe(later),
                        earlier.owner
                    ),
                });
            }
        }
    }
    // 2. An opaque solid painted after the label covers one of its marks, or stands closer
    // to a mark than the label clearance at the zoom. Link tags are painted after every
    // solid, so for them every block counts, the way a tag must not lie where a block
    // stands.
    let clearance = ISO_LABEL_CLEARANCE_PX * scene.zoom;
    for label in &labels {
        for solid in solids.iter().filter(|solid| solid.opaque) {
            let painted_later = match label.node {
                Some(owner) => solid.node > owner,
                None => solid.shape == SolidShape::Block,
            };
            if !painted_later {
                continue;
            }
            examined += 1;
            let nearest = label
                .marks
                .iter()
                .map(|mark| polygon_distance(mark, &solid.outline))
                .fold(f32::INFINITY, f32::min);
            if nearest <= GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!("{} is covered by {}", describe(label), solid.pointer),
                });
            } else if nearest < clearance - GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!(
                        "{} lies {nearest:.2} px from {}, closer than {clearance:.2}",
                        describe(label),
                        solid.pointer
                    ),
                });
            }
        }
    }
    // 3. A visible slab edge crosses a mark that has no box of its own.
    let slabs: Vec<(&Solid, Vec<(ScreenPoint, ScreenPoint)>)> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Slab)
        .map(|slab| (*slab, visible_slab_edges(slab, &solids)))
        .collect();
    for label in labels.iter().filter(|label| !label.opaque) {
        for (slab, edges) in &slabs {
            examined += 1;
            let crossed = edges.iter().any(|(start, end)| {
                label
                    .marks
                    .iter()
                    .any(|mark| segment_crosses_convex(*start, *end, mark))
            });
            if crossed {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!(
                        "{} is crossed by an edge of slab {}",
                        describe(label),
                        slab.pointer
                    ),
                });
            }
        }
    }
    // 4. A link's tube, at its radius on the drawn path, crosses a mark that has no box of
    // its own.
    let tube_outlines: Vec<Vec<[ScreenPoint; 4]>> = scene
        .link_paths
        .iter()
        .zip(&scene.link_kinds)
        .take(LINKS_MAX)
        .map(|(path, (line, tint))| {
            let radius = link_tube_radius(*line, *tint) * scene.zoom;
            let floor_z = path.first().map_or(0.0, |point| point.z);
            path.windows(2)
                .filter_map(|pair| {
                    let (Some(start), Some(end)) = (pair.first(), pair.get(1)) else {
                        return None;
                    };
                    let tube = tube::Tube {
                        start: (start.x, start.y),
                        end: (end.x, end.y),
                        floor_z,
                        radius,
                    };
                    tube::body(&tube, scene.offset).map(|body| body.outline)
                })
                .collect()
        })
        .collect();
    for label in labels.iter().filter(|label| !label.opaque) {
        for (index, legs) in tube_outlines.iter().enumerate() {
            examined += 1;
            let crossed = legs
                .iter()
                .any(|leg| label.marks.iter().any(|mark| polygons_overlap(mark, leg)));
            if crossed {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!("{} is crossed by link /links/{index}", describe(label)),
                });
            }
        }
    }
    // 6. A pipe's tube or band, or a tee's spine, crosses a mark of another owner that has
    // no box of its own.
    let surfaces: Vec<&Solid> = solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Surface)
        .copied()
        .collect();
    for label in labels.iter().filter(|label| !label.opaque) {
        for surface in surfaces
            .iter()
            .filter(|surface| Some(surface.node) != label.node)
        {
            examined += 1;
            let crossed = label
                .marks
                .iter()
                .any(|mark| polygons_overlap(mark, &surface.outline));
            if crossed {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!("{} is crossed by pipe {}", describe(label), surface.pointer),
                });
            }
        }
    }
    // 7. A link or a pipe passes under the opaque tag of another owner.
    for label in labels.iter().filter(|label| label.opaque) {
        let own_link = link_index(&label.owner);
        for (index, legs) in tube_outlines.iter().enumerate() {
            if own_link == Some(index) {
                continue;
            }
            examined += 1;
            if legs.iter().any(|leg| polygons_overlap(&label.corners, leg)) {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!("{} is passed under by /links/{index}", describe(label)),
                });
            }
        }
        for surface in surfaces
            .iter()
            .filter(|surface| Some(surface.node) != label.node)
        {
            examined += 1;
            if polygons_overlap(&label.corners, &surface.outline) {
                defects.push(Defect {
                    pointer: label.owner.clone(),
                    message: format!(
                        "{} is passed under by pipe {}",
                        describe(label),
                        surface.pointer
                    ),
                });
            }
        }
    }
    // 8. A tag lies nearer its own connector than any other. A tee's arms are its own.
    let owns = |label: &Label, surface: &Solid| {
        Some(surface.node) == label.node
            || surface
                .pointer
                .as_str()
                .strip_prefix(label.owner.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    };
    for label in labels.iter().filter(|label| label.opaque) {
        let own_link = link_index(&label.owner);
        let own_surface = surfaces
            .iter()
            .find(|surface| Some(surface.node) == label.node);
        if own_link.is_none() && own_surface.is_none() {
            continue;
        }
        examined += 1;
        let count = label.corners.len() as f32;
        let center = ScreenPoint {
            x: label.corners.iter().map(|corner| corner.x).sum::<f32>() / count,
            y: label.corners.iter().map(|corner| corner.y).sum::<f32>() / count,
        };
        let own = match (own_link, own_surface) {
            (Some(index), _) => scene.link_paths.get(index).map_or(f32::INFINITY, |path| {
                path_distance(center, path, scene.offset)
            }),
            (None, Some(surface)) => point_polygon_distance(center, &surface.outline),
            (None, None) => f32::INFINITY,
        };
        let mut nearer: Option<String> = None;
        for (index, path) in scene.link_paths.iter().enumerate().take(LINKS_MAX) {
            if own_link == Some(index) {
                continue;
            }
            if path_distance(center, path, scene.offset) < own - GEOMETRY_EPSILON {
                nearer = Some(format!("/links/{index}"));
                break;
            }
        }
        if nearer.is_none() {
            for surface in surfaces.iter().filter(|surface| !owns(label, surface)) {
                if point_polygon_distance(center, &surface.outline) < own - GEOMETRY_EPSILON {
                    nearer = Some(format!("pipe {}", surface.pointer));
                    break;
                }
            }
        }
        if let Some(other) = nearer {
            defects.push(Defect {
                pointer: label.owner.clone(),
                message: format!(
                    "tag of {} lies nearer {other} than its own path",
                    label.owner
                ),
            });
        }
    }
    // 5. The parts laid out to fit a block stay on its top face.
    for label in labels.iter().filter(|label| label.contained) {
        examined += 1;
        let Some(block) = solids
            .iter()
            .find(|solid| Some(solid.node) == label.node && solid.shape == SolidShape::Block)
        else {
            defects.push(Defect {
                pointer: label.owner.clone(),
                message: format!("{} has no block", describe(label)),
            });
            continue;
        };
        let leaves = label.marks.iter().any(|mark| {
            !mark
                .iter()
                .all(|corner| shapes::point_in_convex(*corner, &block.outline))
        });
        if leaves {
            defects.push(Defect {
                pointer: label.owner.clone(),
                message: format!("{} leaves its block", describe(label)),
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

/// How close to a box corner a link end may land (section 12.7).
pub const ISO_LINK_CORNER_CLEARANCE_PX: f32 = 8.0;

/// A link's cone base radius as a multiple of its tube radius (section 12.3 rule 7).
pub const LINK_CONE_RADIUS_SCALE: f32 = 1.9;

/// The air two parallel link tubes keep between their surfaces (section 12.7).
pub const ISO_LINK_GAP_PX: f32 = 4.0;

/// For two parallel axis-aligned legs with an overlapping extent, the gap between their
/// lines and the length of the overlap.
fn parallel_overlap(a: route::FlatSegment, b: route::FlatSegment) -> Option<(f32, f32)> {
    let extent = |a0: f32, a1: f32, b0: f32, b1: f32| {
        a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1))
    };
    let horizontal = |leg: route::FlatSegment| {
        (leg.start.y - leg.end.y).abs() <= GEOMETRY_EPSILON
            && (leg.start.x - leg.end.x).abs() > GEOMETRY_EPSILON
    };
    let vertical = |leg: route::FlatSegment| {
        (leg.start.x - leg.end.x).abs() <= GEOMETRY_EPSILON
            && (leg.start.y - leg.end.y).abs() > GEOMETRY_EPSILON
    };
    let (gap, shared) = if horizontal(a) && horizontal(b) {
        (
            (a.start.y - b.start.y).abs(),
            extent(a.start.x, a.end.x, b.start.x, b.end.x),
        )
    } else if vertical(a) && vertical(b) {
        (
            (a.start.x - b.start.x).abs(),
            extent(a.start.y, a.end.y, b.start.y, b.end.y),
        )
    } else {
        return None;
    };
    (shared > GEOMETRY_EPSILON).then_some((gap, shared))
}

/// NotApplicable with reason "projection is flat" for None and "page has fewer than two
/// links" otherwise short of a pair; section 12.7 otherwise. Each unordered pair of drawn
/// link paths is one examined unit: no two legs share a collinear stretch, no two parallel
/// legs run closer than their tube radii plus ISO_LINK_GAP_PX over an overlapping extent,
/// and two ends on one node keep two cone bases apart. A defect points at the later link.
pub fn iso_links_apart(geometry: &PageGeometry, scene: Option<&IsoScene>) -> CheckReport {
    let Some(scene) = scene else {
        return CheckReport::not_applicable(CheckName::IsoLinksApart, "projection is flat");
    };
    if scene.link_paths.len() < 2 {
        return CheckReport::not_applicable(
            CheckName::IsoLinksApart,
            "page has fewer than two links",
        );
    }
    debug_assert!(
        scene.link_paths.len() <= LINKS_MAX,
        "iso-links-apart would drop links past {LINKS_MAX}"
    );
    struct Drawn<'a> {
        pointer: NodePointer,
        legs: Vec<route::FlatSegment>,
        radius: f32,
        ends: [(usize, ScreenPoint); 2],
        route: &'a LinkRoute,
    }
    let drawn: Vec<Drawn> = scene
        .link_paths
        .iter()
        .zip(&scene.link_kinds)
        .zip(&geometry.links)
        .take(LINKS_MAX)
        .enumerate()
        .filter_map(|(index, ((path, kind), route))| {
            let flat: Vec<PagePoint> = path
                .iter()
                .map(|point| PagePoint {
                    x: point.x,
                    y: point.y,
                })
                .collect();
            let (first, last) = (path.first()?, path.last()?);
            Some(Drawn {
                pointer: NodePointer::root().child("links").index(index),
                legs: route::legs(&route::corners(&flat)),
                radius: link_tube_radius(kind.0, kind.1) * scene.zoom,
                ends: [
                    (
                        route.from_node,
                        project_point(first.x, first.y, first.z, scene.offset),
                    ),
                    (
                        route.to_node,
                        project_point(last.x, last.y, last.z, scene.offset),
                    ),
                ],
                route,
            })
        })
        .collect();
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for (later_index, later) in drawn.iter().enumerate() {
        for earlier in drawn.iter().take(later_index) {
            examined += 1;
            let least_gap = later.radius + earlier.radius + ISO_LINK_GAP_PX;
            let mut shared_total: f32 = 0.0;
            let mut closest: Option<f32> = None;
            for leg in &later.legs {
                for other in &earlier.legs {
                    let Some((gap, shared)) = parallel_overlap(*leg, *other) else {
                        continue;
                    };
                    if gap <= GEOMETRY_EPSILON {
                        shared_total += shared;
                    } else if gap < least_gap - GEOMETRY_EPSILON {
                        closest = Some(closest.map_or(gap, |known: f32| known.min(gap)));
                    }
                }
            }
            if shared_total > GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: later.pointer.clone(),
                    message: format!("shares {shared_total:.2} px with {}", earlier.pointer),
                });
            }
            if let Some(gap) = closest {
                defects.push(Defect {
                    pointer: later.pointer.clone(),
                    message: format!(
                        "runs {gap:.2} px beside {}, closer than {least_gap:.2}",
                        earlier.pointer
                    ),
                });
            }
            let least_apart = LINK_CONE_RADIUS_SCALE * (later.radius + earlier.radius);
            for (node, end) in later.ends {
                for (other_node, other_end) in earlier.ends {
                    if node != other_node {
                        continue;
                    }
                    let apart = (end.x - other_end.x).hypot(end.y - other_end.y);
                    if apart < least_apart - GEOMETRY_EPSILON {
                        let owner = geometry.nodes.get(node).map_or_else(
                            || NodePointer::root().child("<absent>"),
                            |node| node.pointer.clone(),
                        );
                        defects.push(Defect {
                            pointer: later.pointer.clone(),
                            message: format!(
                                "ends {apart:.2} px from the end of {} on {owner}, closer than {least_apart:.2}",
                                earlier.pointer
                            ),
                        });
                    }
                }
            }
            let _ = (later.route, earlier.route);
        }
    }
    CheckReport {
        check: CheckName::IsoLinksApart,
        examined,
        defects,
        not_applicable: None,
    }
}

/// Which end of a link a defect names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkEnd {
    Start,
    End,
}

impl LinkEnd {
    fn name(self) -> &'static str {
        match self {
            LinkEnd::Start => "start",
            LinkEnd::End => "end",
        }
    }
}

/// Whether the solid's outline is the six-vertex silhouette of a box, whose corners a link
/// end must keep clear of. Round forms and sprites have hull outlines with many vertices.
fn has_box_outline(form: Shape) -> bool {
    match form {
        Shape::Card | Shape::Tile | Shape::Tower | Shape::Block => true,
        Shape::Cylinder | Shape::Stack | Shape::Figure | Shape::Laptop | Shape::Phone => false,
    }
}

fn point_segment_distance(point: ScreenPoint, a: ScreenPoint, b: ScreenPoint) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared <= 0.0 {
        0.0
    } else {
        (((point.x - a.x) * dx + (point.y - a.y) * dy) / length_squared).clamp(0.0, 1.0)
    };
    (point.x - (a.x + t * dx)).hypot(point.y - (a.y + t * dy))
}

/// The distance from a point to the boundary of a polygon, over its edges.
fn polygon_edge_distance(point: ScreenPoint, polygon: &[ScreenPoint]) -> f32 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .map(|(a, b)| point_segment_distance(point, *a, *b))
        .fold(f32::INFINITY, f32::min)
}

fn polygon_vertex_distance(point: ScreenPoint, polygon: &[ScreenPoint]) -> f32 {
    polygon
        .iter()
        .map(|vertex| (point.x - vertex.x).hypot(point.y - vertex.y))
        .fold(f32::INFINITY, f32::min)
}

/// How far a flat point lies outside a box; 0 inside or on its edge.
fn outside_distance(point: PagePoint, bounds: BoxRect) -> f32 {
    let dx = (bounds.x - point.x)
        .max(point.x - (bounds.x + bounds.width))
        .max(0.0);
    let dy = (bounds.y - point.y)
        .max(point.y - (bounds.y + bounds.height))
        .max(0.0);
    dx.hypot(dy)
}

/// NotApplicable with reason "projection is flat" for None and "page has no links" for a
/// scene without links; section 12.7 otherwise. Each end of each drawn link path is one
/// examined unit: on a block it lies on the block's outline and clear of a box corner; on a
/// zone or a surface it lies inside the footprint. An end whose node has no solid is a
/// defect.
pub fn iso_link_ends(geometry: &PageGeometry, scene: Option<&IsoScene>) -> CheckReport {
    let Some(scene) = scene else {
        return CheckReport::not_applicable(CheckName::IsoLinkEnds, "projection is flat");
    };
    if scene.link_paths.is_empty() {
        return CheckReport::not_applicable(CheckName::IsoLinkEnds, "page has no links");
    }
    debug_assert!(
        scene.link_paths.len() <= LINKS_MAX,
        "iso-link-ends would drop links past {LINKS_MAX}"
    );
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for (index, (path, route)) in scene
        .link_paths
        .iter()
        .zip(&geometry.links)
        .take(LINKS_MAX)
        .enumerate()
    {
        let pointer = NodePointer::root().child("links").index(index);
        let ends = [
            (LinkEnd::Start, route.from_node, path.first().copied()),
            (LinkEnd::End, route.to_node, path.last().copied()),
        ];
        for (end, node, point) in ends {
            examined += 1;
            let mut defect = |message: String| {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message,
                });
            };
            let Some(point) = point else {
                defect(format!("{} has no drawn point", end.name()));
                continue;
            };
            let node_pointer = geometry.nodes.get(node).map_or_else(
                || NodePointer::root().child("<absent>"),
                |node| node.pointer.clone(),
            );
            let Some(solid) = scene
                .solids
                .iter()
                .take(NODES_MAX)
                .find(|solid| solid.node == node)
            else {
                defect(format!("{} node {node_pointer} has no solid", end.name()));
                continue;
            };
            match solid.shape {
                SolidShape::Slab | SolidShape::Surface => {
                    let flat = PagePoint {
                        x: point.x,
                        y: point.y,
                    };
                    let outside = outside_distance(flat, solid.footprint);
                    if outside > GEOMETRY_EPSILON {
                        defect(format!(
                            "{} lies {outside:.2} px outside zone {node_pointer}",
                            end.name()
                        ));
                    }
                }
                SolidShape::Block => {
                    let screen = project_point(point.x, point.y, point.z, scene.offset);
                    let off = polygon_edge_distance(screen, &solid.outline);
                    if off > GEOMETRY_EPSILON {
                        defect(format!(
                            "{} lies {off:.2} px off the outline of {node_pointer}",
                            end.name()
                        ));
                    }
                    if has_box_outline(solid.form) {
                        let corner = polygon_vertex_distance(screen, &solid.outline);
                        if corner < ISO_LINK_CORNER_CLEARANCE_PX - GEOMETRY_EPSILON {
                            defect(format!(
                                "{} lies {corner:.2} px from a corner of {node_pointer}, under {ISO_LINK_CORNER_CLEARANCE_PX}",
                                end.name()
                            ));
                        }
                    }
                }
            }
        }
    }
    CheckReport {
        check: CheckName::IsoLinkEnds,
        examined,
        defects,
        not_applicable: None,
    }
}

/// How much longer than the span between its ends, plus an approach stub at each end, a
/// drawn link may run before it is a detour (section 12.7).
pub const ISO_LINK_DETOUR_RATIO: f32 = 1.5;

/// The shortest inner leg a drawn link may have, as a multiple of the widest link tube's
/// diameter; a shorter one between two turns reads as a jog (section 12.7).
pub const ISO_LINK_JOG_DIAMETERS: f32 = 2.0;

/// NotApplicable with reason "projection is flat" for None and "page has no links" for a
/// scene without links; section 12.7 otherwise. Each leg of each drawn link path, taken in
/// flat px with its risers dropped, is one examined unit. The geometry names each link's
/// endpoint nodes, whose blocks a leg may cross on screen.
pub fn iso_links_clear(geometry: &PageGeometry, scene: Option<&IsoScene>) -> CheckReport {
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
    let blocks: Vec<&Solid> = scene
        .solids
        .iter()
        .take(NODES_MAX)
        .filter(|solid| solid.shape == SolidShape::Block && solid.opaque)
        .collect();
    let widest_radius = scene
        .link_kinds
        .iter()
        .map(|(line, tint)| link_tube_radius(*line, *tint))
        .fold(0.0, f32::max)
        * scene.zoom;
    let least_inner = ISO_LINK_JOG_DIAMETERS * 2.0 * widest_radius;
    let stub = stencil_layout::ISO_APPROACH_PX * scene.zoom;
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
        let endpoints = geometry
            .links
            .get(index)
            .map(|route| [route.from_node, route.to_node]);
        let radius = link_tube_radius(kind.0, kind.1) * scene.zoom;
        let flat: Vec<PagePoint> = path
            .iter()
            .map(|point| PagePoint {
                x: point.x,
                y: point.y,
            })
            .collect();
        let legs = route::legs(&route::corners(&flat));
        let least_last = 2.0 * iso_link_arrowhead_length(kind.0, kind.1);
        let length: f32 = legs.iter().map(|leg| leg.length()).sum();
        if let (Some(first), Some(last)) = (flat.first(), flat.last()) {
            let span = (last.x - first.x).abs() + (last.y - first.y).abs();
            let allowed = ISO_LINK_DETOUR_RATIO * (span + 2.0 * stub);
            if length > allowed + GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!(
                        "link runs {length:.2} px between ends {span:.2} px apart, more than {ISO_LINK_DETOUR_RATIO} times their span plus two stubs ({allowed:.2} px)"
                    ),
                });
            }
        }
        let path_z = path.first().map_or(0.0, |point| point.z);
        for (leg_index, leg) in legs.iter().enumerate() {
            examined += 1;
            let inner = leg_index > 0 && leg_index + 1 < legs.len();
            if inner && leg.length() < least_inner - GEOMETRY_EPSILON {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!(
                        "leg {leg_index} is {:.2} px between two turns, shorter than {least_inner:.2}",
                        leg.length()
                    ),
                });
            }
            let tube = tube::Tube {
                start: (leg.start.x, leg.start.y),
                end: (leg.end.x, leg.end.y),
                floor_z: path_z,
                radius,
            };
            if let Some(body) = tube::body(&tube, scene.offset) {
                for block in &blocks {
                    if endpoints.is_some_and(|ends| ends.contains(&block.node)) {
                        continue;
                    }
                    if polygons_overlap(&body.outline, &block.outline) {
                        defects.push(Defect {
                            pointer: pointer.clone(),
                            message: format!(
                                "leg {leg_index} crosses block {} on screen",
                                block.pointer
                            ),
                        });
                    }
                }
            }
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
