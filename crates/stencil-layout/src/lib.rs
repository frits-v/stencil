//! Layout of a stencil page: one taffy tree built from the document model, text measured
//! through the `TextMeasurer` trait, and absolute geometry in canvas px (SPEC section 2).

pub mod checks;
pub mod styles;

mod build;
mod compute;
mod lanes;
mod route;
mod theme_labels;

use stencil_model::grammar::{BorderPattern, LabelStyle, Role, Tone};
use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, TextMetrics, TextStyle};
use stencil_model::{Grammar, Line, Page, PagePoint, Violation, line_key, validate_page};

pub use build::{container_label_style, fact_presentation};
pub use route::{longest_segment_midpoint, reroute_link};
pub use stencil_model::text::WRAP_EPSILON_PX;
pub use theme_labels::theme_legend_labels;

pub const GEOMETRY_EPSILON_PX: f32 = 0.01;

/// Length of a link or pipe arrowhead along its line, from base to tip (section 11.2).
pub const ARROWHEAD_LENGTH_PX: f32 = 10.0;
/// Width of an arrowhead across its line, at the base.
pub const ARROWHEAD_WIDTH_PX: f32 = 8.0;
/// Height of a leaf block under iso (section 12.3): a Pcard, Fact, Note, Text, Callout or
/// Frame. Layout reserves this much floor behind a zone's label, because a block covers
/// the strip of floor its height spans on screen.
pub const ISO_BLOCK_HEIGHT_PX: f32 = 18.0;
/// Floor kept clear between a zone's label and the strip its first child covers under iso.
pub const ISO_LABEL_CLEARANCE_PX: f32 = 8.0;
/// Under iso every body text style is this much larger than flat (section 12.4): the plane
/// halves a run's visible x-height, so the type grows to read. Chrome and legend stay flat.
pub const ISO_TYPE_SCALE: f32 = 1.3;
/// Under iso a zone label is this much larger than flat, the floor name of its slab.
pub const ISO_ZONE_LABEL_SCALE: f32 = 1.5;
/// Under iso every container padding, card padding and gap is this much larger than flat:
/// a floor plan keeps open floor around what stands on it, and the grown type needs it.
pub const ISO_SPACE_SCALE: f32 = 1.75;
/// Under iso a pipe's wire is at least this many times its flat minimum, so its tag lying
/// on the floor clears the slab edges its dots touch.
pub const ISO_WIRE_SCALE: f32 = 3.0;
/// Under iso a pipe tag keeps this much wire visible on each side along the run, so the
/// pill never reaches the slab edges its dots touch.
pub const ISO_TAG_CLEARANCE_PX: f32 = 16.0;

pub use stencil_model::Axis;

/// `bounds` after a quarter turn about `pivot` that takes layout right to flat -y: the
/// strip a y run covers.
pub fn turned_box(bounds: BoxRect, pivot: (f32, f32)) -> BoxRect {
    let (px, py) = pivot;
    BoxRect {
        x: px + bounds.y - py,
        y: py + px - bounds.right(),
        width: bounds.height,
        height: bounds.width,
    }
}

/// The inverse of `turned_box`: the layout box a y run is drawn in before its turn.
pub fn unturned_box(bounds: BoxRect, pivot: (f32, f32)) -> BoxRect {
    let (px, py) = pivot;
    BoxRect {
        x: px + py - bounds.bottom(),
        y: py + bounds.x - px,
        width: bounds.height,
        height: bounds.width,
    }
}

/// Asserts validate_page(page, grammar) is empty, sizes each Lanes band from its messages,
/// builds the taffy tree with each Box laid out from its container kind, computes layout,
/// re-measures text at final widths, centers the slot of every pipe that names its targets
/// on them, adds the lifelines, routes the links, and returns
/// absolute geometry in canvas px.
pub fn layout_page(
    page: &Page,
    grammar: &Grammar,
    measurer: &mut dyn TextMeasurer,
) -> Result<PageGeometry, LayoutError> {
    let violations = validate_page(page, grammar);
    if !violations.is_empty() {
        return Err(LayoutError::Invalid(violations));
    }
    let lanes_plan = lanes::plan_lanes(page, measurer)?;
    let built = build::build_page(page, grammar, &lanes_plan)?;
    let mut geometry = compute::compute_geometry(built, page.width, measurer)?;
    compute::translate_pipe_slots(page, &mut geometry);
    lanes::add_lifelines(&mut geometry);
    geometry.links = route::route_links(page, &geometry, &lanes_plan, measurer)?;
    Ok(geometry)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageGeometry {
    pub canvas: Size,
    /// In geometry order (section 4.4). nodes[0] is the page root (pointer "").
    pub nodes: Vec<NodeGeometry>,
    /// One route per `Page.links` entry, in link order (section 11.2).
    pub links: Vec<LinkRoute>,
}

/// A routed link (section 11.2). A link is not a node: it takes no layout space.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkRoute {
    /// Position in `Page.links`; the link's pointer is `/links/<index>`.
    pub index: usize,
    pub line: Line,
    /// The effective tint (section 13.1 rule 2): slot 1 for an untinted solid or dash line,
    /// None for gray and deny.
    pub tint: Option<u8>,
    /// Geometry index of the node named by `from`.
    pub from_node: usize,
    /// Geometry index of the node named by `to`.
    pub to_node: usize,
    /// Border-box polyline in page coordinates, first point on the from box edge, last on
    /// the to box edge. 2 to LINK_SEGMENTS_MAX + 1 points.
    pub points: Vec<PagePoint>,
    /// The tag box for label and sub, centered on the longest segment; None without a label.
    pub tag: Option<BoxRect>,
    /// Tag, TagLabel and, with a sub, TagSub, laid out as in a pipe tag. Empty without a
    /// label.
    pub parts: Vec<Part>,
    pub status: RouteStatus,
}

impl LinkRoute {
    /// The output key of the link's line (section 13.1 rule 6), for example "blue".
    pub fn key(&self) -> &'static str {
        line_key(self.line, self.tint)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteStatus {
    Routed,
    Fallback,
}

impl PageGeometry {
    pub fn node(&self, pointer: &NodePointer) -> Option<&NodeGeometry> {
        self.nodes.iter().find(|node| &node.pointer == pointer)
    }

    /// Indices of the nodes whose parent is `index`, in geometry order.
    pub fn children(&self, index: usize) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.parent == Some(index))
            .map(|(child_index, _)| child_index)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeGeometry {
    pub pointer: NodePointer,
    pub tag: NodeTag,
    /// The output key (section 13.1 rule 6) of a Box, Pipe (Tee arms included), Tee and
    /// LegendEntry, for example "region-a" or "blue"; None for every other tag.
    pub kind: Option<String>,
    /// The effective tint of a Box or line (section 13.1 rule 2). The measured JSON does not
    /// write it.
    pub tint: Option<u8>,
    /// How a Box's container kind is drawn, read from the grammar; None for every other tag.
    pub container: Option<ContainerLook>,
    pub parent: Option<usize>,
    /// Border box.
    pub bounds: BoxRect,
    /// Region children must stay inside: border box minus border and padding,
    /// and for gcp zones the body part's content box.
    pub content: BoxRect,
    /// In the order of section 2.11.
    pub parts: Vec<Part>,
}

/// The parts of a container kind the renderer reads (section 13.2): role, tone, border
/// pattern and width, corner radius and label style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContainerLook {
    pub role: Role,
    pub tone: Option<Tone>,
    pub pattern: BorderPattern,
    pub border_width: f32,
    pub radius: f32,
    pub label: LabelStyle,
}

impl ContainerLook {
    /// A boundary drawn as a ring under iso (section 12.3): the strong tone, untinted.
    pub fn is_ring(&self, tint: Option<u8>) -> bool {
        self.tone == Some(Tone::Strong) && tint.is_none()
    }
}

impl NodeGeometry {
    /// The first part with this name.
    pub fn part(&self, name: PartName) -> Option<&Part> {
        self.parts.iter().find(|part| part.name == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTag {
    Page,
    Kicker,
    Title,
    Lede,
    Body,
    Legend,
    LegendEntry,
    Foot,
    Row,
    Col,
    Lanes,
    /// A Box; the output name stays `Zone` (section 13.1 rule 5).
    Zone,
    /// An Item; the output name stays `Pcard` (section 13.1 rule 5).
    Pcard,
    Fact,
    Note,
    Pipe,
    Tee,
    Text,
    Callout,
    Frame,
}

impl NodeTag {
    /// The `data-tag` name, identical to the variant name.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeTag::Page => "Page",
            NodeTag::Kicker => "Kicker",
            NodeTag::Title => "Title",
            NodeTag::Lede => "Lede",
            NodeTag::Body => "Body",
            NodeTag::Legend => "Legend",
            NodeTag::LegendEntry => "LegendEntry",
            NodeTag::Foot => "Foot",
            NodeTag::Row => "Row",
            NodeTag::Col => "Col",
            NodeTag::Lanes => "Lanes",
            NodeTag::Zone => "Zone",
            NodeTag::Pcard => "Pcard",
            NodeTag::Fact => "Fact",
            NodeTag::Note => "Note",
            NodeTag::Pipe => "Pipe",
            NodeTag::Tee => "Tee",
            NodeTag::Text => "Text",
            NodeTag::Callout => "Callout",
            NodeTag::Frame => "Frame",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BoxRect {
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: PartName,
    pub bounds: BoxRect,
    pub text: Option<TextRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartName {
    Badge,
    BadgeText,
    Text,
    Label,
    Bar,
    Body,
    Icon,
    FunctionName,
    ProductName,
    FactBox,
    Fact,
    BuiltBox,
    Built,
    AskBox,
    Ask,
    DotStart,
    WireStart,
    Tag,
    TagLabel,
    TagSub,
    WireEnd,
    DotEnd,
    Spine,
    Hub,
    HubText,
    Swatch,
    LegendLabel,
    LegendText,
    Heading,
    Marker,
    BodyLine,
    Accent,
    LabelChip,
    Heads,
    Band,
    Lifeline,
}

impl PartName {
    /// The snake_case name used as a measured JSON key and in check messages.
    pub fn as_str(self) -> &'static str {
        match self {
            PartName::Badge => "badge",
            PartName::BadgeText => "badge_text",
            PartName::Text => "text",
            PartName::Label => "label",
            PartName::Bar => "bar",
            PartName::Body => "body",
            PartName::Icon => "icon",
            PartName::FunctionName => "function_name",
            PartName::ProductName => "product_name",
            PartName::FactBox => "fact_box",
            PartName::Fact => "fact",
            PartName::BuiltBox => "built_box",
            PartName::Built => "built",
            PartName::AskBox => "ask_box",
            PartName::Ask => "ask",
            PartName::DotStart => "dot_start",
            PartName::WireStart => "wire_start",
            PartName::Tag => "tag",
            PartName::TagLabel => "tag_label",
            PartName::TagSub => "tag_sub",
            PartName::WireEnd => "wire_end",
            PartName::DotEnd => "dot_end",
            PartName::Spine => "spine",
            PartName::Hub => "hub",
            PartName::HubText => "hub_text",
            PartName::Swatch => "swatch",
            PartName::LegendLabel => "legend_label",
            PartName::LegendText => "legend_text",
            PartName::Heading => "heading",
            PartName::Marker => "marker",
            PartName::BodyLine => "body_line",
            PartName::Accent => "accent",
            PartName::LabelChip => "label_chip",
            PartName::Heads => "heads",
            PartName::Band => "band",
            PartName::Lifeline => "lifeline",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// The string as drawn, after any uppercase transform and the "• " or "Ask: " prefix.
    pub text: String,
    pub style: TextStyle,
    pub color: &'static str,
    pub align: TextAlign,
    pub metrics: TextMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    Center,
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("page fails vet: {} violation(s)", .0.len())]
    Invalid(Vec<Violation>),
    #[error("text at {pointer} could not be measured: {source}")]
    Measure {
        pointer: NodePointer,
        #[source]
        source: MeasureError,
    },
    #[error("taffy failed at {pointer}: {message}")]
    Taffy {
        pointer: NodePointer,
        message: String,
    },
    #[error("layout produced a non-finite box at {pointer}")]
    NonFinite { pointer: NodePointer },
}
