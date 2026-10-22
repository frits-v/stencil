//! Layout of a stencil page: one taffy tree built from the document model, text measured
//! through the `TextMeasurer` trait, and absolute geometry in canvas px (SPEC section 2).

pub mod checks;
pub mod styles;

mod build;
mod compute;
mod drawn;
mod lanes;
mod lead_run;
mod phases;
mod pipe_ends;
mod route;
mod sheet_frame;
mod theme_labels;
mod units;

use stencil_model::grammar::{BorderPattern, LabelStyle, Role, Tone};
use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, TextMetrics, TextStyle};
use stencil_model::{
    Arrow, Bend, Grammar, Line, Page, PagePoint, PipeForm, Violation, canvas_width, line_key,
    validate_page,
};

pub use build::{SHEET_BLOCK_BORDER_PX, container_label_style, fact_presentation};
pub use drawn::{DrawnLink, Fillet, PathPiece, drawn_link, fillet, push_flattened};
pub use lead_run::{LeadRun, measure_lead_run};
pub use route::{
    OBSTACLE_CLEARANCE_PX, longest_segment_midpoint, longest_segment_point, reroute_link,
};
pub use sheet_frame::{SHEET_BORDER_PX, SHEET_RULE_PX};
pub use stencil_model::text::WRAP_EPSILON_PX;
pub use theme_labels::theme_legend_labels;
pub use units::{
    DescribedUnit, DescriptionUnits, ENTRY_PARTS, PipeEndSide, PipeLanding, SheetZone, UnitKind,
    description_units, sheet_frame, sheet_zone, sheet_zone_at, title_cell_caption,
};

pub const GEOMETRY_EPSILON_PX: f32 = 0.01;

/// Length of a link or pipe arrowhead along its line, from base to tip (section 11.2).
pub const ARROWHEAD_LENGTH_PX: f32 = 10.0;
/// Width of an arrowhead across its line, at the base.
pub const ARROWHEAD_WIDTH_PX: f32 = 8.0;
/// The straight a link keeps at an arrowed end (section 11.2): the arrowhead and a default
/// corner on each side of the bend before it, so the stroke meets the arrowhead's base.
pub const LINK_ARROW_STUB_PX: f32 =
    ARROWHEAD_LENGTH_PX + 2.0 * stencil_model::LINK_CORNER_DEFAULT_PX;
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
pub const ISO_ZONE_LABEL_SCALE: f32 = 1.25;
/// Under iso the frame bar label is this much larger than flat: the platform name, a tier
/// above the zone names.
pub const ISO_FRAME_LABEL_SCALE: f32 = 1.4;
/// The square footprint, in flat px, of an item's solid under iso when its shape is not a
/// card (section 12.3).
pub fn shape_footprint_px(shape: Shape) -> f32 {
    match shape {
        Shape::Card => 0.0,
        Shape::Tile => 64.0,
        Shape::Tower => 44.0,
        Shape::Cylinder | Shape::Stack => 56.0,
        Shape::Block => 64.0,
        Shape::Figure => 44.0,
        Shape::Laptop => 56.0,
        Shape::Phone => 36.0,
    }
}

/// Floor kept clear under an item's floor text under iso: the strip the tallest shape in
/// the next row covers on screen, plus the clearance (section 12.3).
pub const ISO_FLOOR_TEXT_RESERVE_PX: f32 = 56.0 + ISO_LABEL_CLEARANCE_PX;

/// The height of an item's solid under iso, by shape (section 12.3).
pub fn shape_height_px(shape: Shape) -> f32 {
    match shape {
        Shape::Card => ISO_BLOCK_HEIGHT_PX,
        Shape::Tile => 6.0,
        Shape::Tower => 44.0,
        Shape::Cylinder => 36.0,
        Shape::Stack => 30.0,
        Shape::Block => ISO_BLOCK_HEIGHT_PX,
        Shape::Figure => 56.0,
        Shape::Laptop => 40.0,
        Shape::Phone => 44.0,
    }
}

/// Under iso a link meets a block through a straight stub of this length, perpendicular
/// to the visible side it attaches to, so the arrowhead stands clear of the block's base
/// and the last leg is longer than two arrowheads.
pub const ISO_APPROACH_PX: f32 = 40.0;
/// Under iso a wrapping run keeps lines at least this long unless the whole run is
/// shorter, so names on a plane never stack one word per line.
pub const ISO_LINE_MIN_PX: f32 = 120.0;
/// Under iso every container padding, card padding and gap is this much larger than flat:
/// a floor plan keeps open floor around what stands on it, and the grown type needs it.
pub const ISO_SPACE_SCALE: f32 = 1.75;
/// Under iso a pipe's wire is at least this many times its flat minimum, so its tag lying
/// on the floor clears the slab edges its dots touch.
pub const ISO_WIRE_SCALE: f32 = 3.0;
/// Under iso a pipe tag keeps this much wire visible on each side along the run, so the
/// pill never reaches the slab edges its dots touch.
pub const ISO_TAG_CLEARANCE_PX: f32 = 16.0;

pub use stencil_model::{Axis, Shape};

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
    // A Phases node is as wide as the sheet's content: its View stretches to it.
    let phases_plan = phases::plan_phases(page, page.width as f32, measurer)?;
    let built = build::build_page(page, grammar, &lanes_plan, &phases_plan)?;
    let mut geometry = compute::compute_geometry(built, canvas_width(page), measurer)?;
    compute::translate_pipe_slots(page, &mut geometry);
    pipe_ends::extend_pipe_ends(page, &mut geometry);
    lanes::add_lifelines(&mut geometry);
    if page.sheet.is_some() {
        sheet_frame::add_sheet_frame(&mut geometry, measurer)?;
    }
    geometry.links = route::route_links(page, grammar, &geometry, &lanes_plan, measurer)?;
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
    /// The radius the writer rounds each bend with, before its clamp to half the shorter
    /// leg; 0 draws square corners. A drawing parameter: the measured JSON does not write it.
    pub corner: f32,
    pub bend: Bend,
    /// The link's arrowheads, which shorten the drawn line at their ends.
    pub arrow: Arrow,
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
    /// The solid an Item stands as under iso (section 12.3); None for every other tag.
    pub shape: Option<Shape>,
    /// How a Pipe is drawn under iso (section 12.3 rule 5); None for every other tag.
    pub pipe_form: Option<PipeForm>,
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
    /// A design sheet's title block, `/sheet` (section 14.4 rule 4).
    TitleBlock,
    /// One of the problem, answer and attention cells of the title block.
    TitleCell,
    /// A numbered point of the problem cell.
    ProblemPoint,
    /// A numbered view of a design sheet.
    View,
    /// An actor's chip in the legend strip's Who acts group, `/actors/<i>`.
    ActorChip,
    /// A swimlane of lanes by actor and phases left to right (section 14.4 rule 11).
    Phases,
    /// One piece of work in a Phases node.
    Step,
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
            NodeTag::TitleBlock => "TitleBlock",
            NodeTag::TitleCell => "TitleCell",
            NodeTag::ProblemPoint => "ProblemPoint",
            NodeTag::View => "View",
            NodeTag::ActorChip => "ActorChip",
            NodeTag::Phases => "Phases",
            NodeTag::Step => "Step",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// The box an item's solid rises from under iso when its shape is not a card
    /// (section 12.3); the icon sits in it and the text lies beside it.
    Footprint,
    /// The design sheet frame, 12 px in from the canvas edge.
    Border,
    /// The inner rule of the reference band.
    RefRule,
    /// The rectangle inside the inner rule: the drafting grid and the reference cells.
    Grid,
    RefTick,
    RefColumn,
    RefColumnFoot,
    RefRow,
    RefRowRight,
    BlockBorder,
    Wordmark,
    CellRule,
    Caption,
    Lead,
    Statement,
    ParagraphLead,
    Paragraph,
    AttentionLead,
    AttentionText,
    PointRule,
    PointLabel,
    StripRule,
    GroupCaption,
    ViewNumber,
    ViewNumeral,
    ViewTitle,
    ViewSubtitle,
    ViewRule,
    ViewNote,
    ActorName,
    ActorText,
    LaneBand,
    LaneBar,
    LaneName,
    LaneText,
    LaneRule,
    PhaseHead,
    PhaseNumeral,
    PhaseTitle,
    PhaseSeparator,
    Front,
    StepLine,
    StackBack,
}

/// The number of part names, so a new name has to join `PartName::ALL`.
pub const PART_NAMES: usize = 79;

impl PartName {
    /// Every part name, in declaration order.
    pub const ALL: [PartName; PART_NAMES] = [
        PartName::Badge,
        PartName::BadgeText,
        PartName::Text,
        PartName::Label,
        PartName::Bar,
        PartName::Body,
        PartName::Icon,
        PartName::FunctionName,
        PartName::ProductName,
        PartName::FactBox,
        PartName::Fact,
        PartName::BuiltBox,
        PartName::Built,
        PartName::AskBox,
        PartName::Ask,
        PartName::DotStart,
        PartName::WireStart,
        PartName::Tag,
        PartName::TagLabel,
        PartName::TagSub,
        PartName::WireEnd,
        PartName::DotEnd,
        PartName::Spine,
        PartName::Hub,
        PartName::HubText,
        PartName::Swatch,
        PartName::LegendLabel,
        PartName::LegendText,
        PartName::Heading,
        PartName::Marker,
        PartName::BodyLine,
        PartName::Accent,
        PartName::LabelChip,
        PartName::Heads,
        PartName::Band,
        PartName::Lifeline,
        PartName::Footprint,
        PartName::Border,
        PartName::RefRule,
        PartName::Grid,
        PartName::RefTick,
        PartName::RefColumn,
        PartName::RefColumnFoot,
        PartName::RefRow,
        PartName::RefRowRight,
        PartName::BlockBorder,
        PartName::Wordmark,
        PartName::CellRule,
        PartName::Caption,
        PartName::Lead,
        PartName::Statement,
        PartName::ParagraphLead,
        PartName::Paragraph,
        PartName::AttentionLead,
        PartName::AttentionText,
        PartName::PointRule,
        PartName::PointLabel,
        PartName::StripRule,
        PartName::GroupCaption,
        PartName::ViewNumber,
        PartName::ViewNumeral,
        PartName::ViewTitle,
        PartName::ViewSubtitle,
        PartName::ViewRule,
        PartName::ViewNote,
        PartName::ActorName,
        PartName::ActorText,
        PartName::LaneBand,
        PartName::LaneBar,
        PartName::LaneName,
        PartName::LaneText,
        PartName::LaneRule,
        PartName::PhaseHead,
        PartName::PhaseNumeral,
        PartName::PhaseTitle,
        PartName::PhaseSeparator,
        PartName::Front,
        PartName::StepLine,
        PartName::StackBack,
    ];

    /// True for the parts a design sheet adds (section 14.2 rule 10); only a sheet draws
    /// them.
    pub fn is_sheet(self) -> bool {
        match self {
            PartName::Border
            | PartName::RefRule
            | PartName::Grid
            | PartName::RefTick
            | PartName::RefColumn
            | PartName::RefColumnFoot
            | PartName::RefRow
            | PartName::RefRowRight
            | PartName::BlockBorder
            | PartName::Wordmark
            | PartName::CellRule
            | PartName::Caption
            | PartName::Lead
            | PartName::Statement
            | PartName::ParagraphLead
            | PartName::Paragraph
            | PartName::AttentionLead
            | PartName::AttentionText
            | PartName::PointRule
            | PartName::PointLabel
            | PartName::StripRule
            | PartName::GroupCaption
            | PartName::ViewNumber
            | PartName::ViewNumeral
            | PartName::ViewTitle
            | PartName::ViewSubtitle
            | PartName::ViewRule
            | PartName::ViewNote
            | PartName::ActorName
            | PartName::ActorText
            | PartName::LaneBand
            | PartName::LaneBar
            | PartName::LaneName
            | PartName::LaneText
            | PartName::LaneRule
            | PartName::PhaseHead
            | PartName::PhaseNumeral
            | PartName::PhaseTitle
            | PartName::PhaseSeparator
            | PartName::Front
            | PartName::StepLine
            | PartName::StackBack => true,
            PartName::Badge
            | PartName::BadgeText
            | PartName::Text
            | PartName::Label
            | PartName::Bar
            | PartName::Body
            | PartName::Icon
            | PartName::FunctionName
            | PartName::ProductName
            | PartName::FactBox
            | PartName::Fact
            | PartName::BuiltBox
            | PartName::Built
            | PartName::AskBox
            | PartName::Ask
            | PartName::DotStart
            | PartName::WireStart
            | PartName::Tag
            | PartName::TagLabel
            | PartName::TagSub
            | PartName::WireEnd
            | PartName::DotEnd
            | PartName::Spine
            | PartName::Hub
            | PartName::HubText
            | PartName::Swatch
            | PartName::LegendLabel
            | PartName::LegendText
            | PartName::Heading
            | PartName::Marker
            | PartName::BodyLine
            | PartName::Accent
            | PartName::LabelChip
            | PartName::Heads
            | PartName::Band
            | PartName::Lifeline
            | PartName::Footprint => false,
        }
    }

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
            PartName::Footprint => "footprint",
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
            PartName::Border => "border",
            PartName::RefRule => "ref_rule",
            PartName::Grid => "grid",
            PartName::RefTick => "ref_tick",
            PartName::RefColumn => "ref_column",
            PartName::RefColumnFoot => "ref_column_foot",
            PartName::RefRow => "ref_row",
            PartName::RefRowRight => "ref_row_right",
            PartName::BlockBorder => "block_border",
            PartName::Wordmark => "wordmark",
            PartName::CellRule => "cell_rule",
            PartName::Caption => "caption",
            PartName::Lead => "lead",
            PartName::Statement => "statement",
            PartName::ParagraphLead => "paragraph_lead",
            PartName::Paragraph => "paragraph",
            PartName::AttentionLead => "attention_lead",
            PartName::AttentionText => "attention_text",
            PartName::PointRule => "point_rule",
            PartName::PointLabel => "point_label",
            PartName::StripRule => "strip_rule",
            PartName::GroupCaption => "group_caption",
            PartName::ViewNumber => "view_number",
            PartName::ViewNumeral => "view_numeral",
            PartName::ViewTitle => "view_title",
            PartName::ViewSubtitle => "view_subtitle",
            PartName::ViewRule => "view_rule",
            PartName::ViewNote => "view_note",
            PartName::ActorName => "actor_name",
            PartName::ActorText => "actor_text",
            PartName::LaneBand => "lane_band",
            PartName::LaneBar => "lane_bar",
            PartName::LaneName => "lane_name",
            PartName::LaneText => "lane_text",
            PartName::LaneRule => "lane_rule",
            PartName::PhaseHead => "phase_head",
            PartName::PhaseNumeral => "phase_numeral",
            PartName::PhaseTitle => "phase_title",
            PartName::PhaseSeparator => "phase_separator",
            PartName::Front => "front",
            PartName::StepLine => "step_line",
            PartName::StackBack => "stack_back",
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
    /// Where line 0 starts after the part's left edge: the body of a lead run continues on
    /// its lead's last line (section 14.4 rule 9); 0 for every other run.
    pub first_line_indent_px: f32,
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
