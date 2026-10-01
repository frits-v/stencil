//! Builds the taffy tree for a vetted page (sections 2.2 to 2.8) and records, per geometry
//! node, the taffy nodes its bounds, content box and parts are read from.

use stencil_model::grammar::{LabelStyle, Role};
use stencil_model::pointer::NodePointer;
use stencil_model::text::{TextStyle, TextStyleName};
use stencil_model::{
    Arrow, BoxNode, Callout, Canvas, Chrome, DEPTH_MAX, Fact, FactSource, Frame, GAP_DEFAULT_PX,
    Grammar, Item, Justify, LANE_GAP_DEFAULT_PX, Lanes, LegendEntry, ListKind, Node, Note,
    NoteKind, Page, Pipe, PipeDir, Projection, Tee, TeeArm, Text, VetRule, Violation, box_key,
    box_tint, legend_label, line_key, line_tint,
};
use taffy::prelude::{
    AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Line, NodeId, Position, Rect, Style, TaffyTree, auto,
    fr, length, line,
};

use crate::lanes::LanesPlan;
use crate::styles::{text_color, text_style_for};
use crate::{
    Axis, ContainerLook, ISO_BLOCK_HEIGHT_PX, ISO_LABEL_CLEARANCE_PX, LayoutError, NodeTag,
    PartName, TextAlign,
};

/// Taffy's node context is an index into `BuiltPage::text_leaves`.
pub(crate) type LayoutTree = TaffyTree<usize>;

pub(crate) struct TextLeaf {
    pub text: String,
    /// The resolved style: the named one flat, grown under iso (`text_style_for`).
    pub style: TextStyle,
    pub color: &'static str,
    pub align: TextAlign,
    /// A y run is measured with its width and height swapped (section 12.4).
    pub axis: Axis,
    /// The field the string comes from; a measure error is reported at this pointer.
    pub source: NodePointer,
}

pub(crate) struct PartRecord {
    pub name: PartName,
    pub taffy_node: NodeId,
    pub text_leaf: Option<usize>,
}

pub(crate) struct NodeRecord {
    pub pointer: NodePointer,
    pub tag: NodeTag,
    pub kind: Option<String>,
    pub tint: Option<u8>,
    pub container: Option<ContainerLook>,
    pub parent: Option<usize>,
    pub taffy_node: NodeId,
    /// The taffy node whose content box children must stay inside.
    pub content_node: NodeId,
    pub parts: Vec<PartRecord>,
    /// For a Pipe, which ends carry an arrowhead and along which axis it runs.
    pub arrow_ends: Option<ArrowEnds>,
}

/// The ends of a Pipe whose dot an arrowhead replaces (section 11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArrowEnds {
    pub start: bool,
    pub end: bool,
    pub horizontal: bool,
}

/// Box sizes of the section 11.3 blocks, in px.
const BLOCK_BORDER_PX: f32 = 1.25;
const BLOCK_PADDING_PX: f32 = 12.0;
const CALLOUT_ACCENT_PX: f32 = 4.0;
const BLOCK_HEADING_GAP_PX: f32 = 6.0;
const BLOCK_LINE_GAP_PX: f32 = 4.0;
const LIST_INDENT_PX: f32 = 22.0;
const FRAME_PADDING_PX: f32 = 8.0;

pub(crate) struct BuiltPage {
    pub tree: LayoutTree,
    pub root: NodeId,
    pub records: Vec<NodeRecord>,
    pub text_leaves: Vec<TextLeaf>,
}

/// How a body node sits in its parent (sections 2.3, 2.7 and 2.8).
#[derive(Debug, Clone, Copy)]
enum Placement {
    /// A child of a Row: the main axis is horizontal.
    RowItem { weight: u16 },
    /// A child of a Col, a Box or the page body: the main axis is vertical.
    ColumnItem { weight: u16 },
    /// A Tee arm in grid row line 1 or 3, column line 2.
    TeeArm { row_line: i16 },
}

pub(crate) fn build_page(
    page: &Page,
    grammar: &Grammar,
    lanes_plan: &LanesPlan,
) -> Result<BuiltPage, LayoutError> {
    let mut builder = Builder {
        tree: TaffyTree::new(),
        records: Vec::new(),
        text_leaves: Vec::new(),
        canvas: page.canvas,
        grammar,
        lanes_plan,
        iso: page.projection == Projection::Iso,
    };
    builder.tree.disable_rounding();
    let root = builder.add_page(page)?;
    Ok(BuiltPage {
        tree: builder.tree,
        root,
        records: builder.records,
        text_leaves: builder.text_leaves,
    })
}

struct Builder<'page> {
    tree: LayoutTree,
    records: Vec<NodeRecord>,
    text_leaves: Vec<TextLeaf>,
    canvas: Canvas,
    grammar: &'page Grammar,
    lanes_plan: &'page LanesPlan,
    /// True under iso: every zone label reserves the floor its placard needs.
    iso: bool,
}

/// Extra floor under a zone label under iso: the strip a child block of
/// `ISO_BLOCK_HEIGHT_PX` covers on screen, plus the clearance (section 12.4).
fn iso_label_reserve() -> f32 {
    ISO_BLOCK_HEIGHT_PX + ISO_LABEL_CLEARANCE_PX
}

/// Every taffy node starts from this: content that does not fit overflows (section 2.1).
fn base_style() -> Style {
    Style {
        flex_shrink: 0.0,
        ..Style::default()
    }
}

fn flex_column(align_items: AlignItems) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(align_items),
        ..base_style()
    }
}

fn flex_row(align_items: AlignItems) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        align_items: Some(align_items),
        ..base_style()
    }
}

fn sides(top: f32, right: f32, bottom: f32, left: f32) -> Rect<LengthPercentage> {
    Rect {
        left: length(left),
        right: length(right),
        top: length(top),
        bottom: length(bottom),
    }
}

fn margins(top: f32, right: f32, bottom: f32, left: f32) -> Rect<LengthPercentageAuto> {
    Rect {
        left: length(left),
        right: length(right),
        top: length(top),
        bottom: length(bottom),
    }
}

fn fixed_size(width: f32, height: f32) -> taffy::Size<Dimension> {
    taffy::Size {
        width: length(width),
        height: length(height),
    }
}

/// A grow weight above 0 gives `flex_grow = weight` and `flex_basis = 0`; weight 0 keeps
/// the max-content size (section 2.3).
fn apply_weight(style: &mut Style, weight: u16) {
    if weight > 0 {
        style.flex_grow = f32::from(weight);
        style.flex_basis = length(0.0);
    } else {
        style.flex_grow = 0.0;
        style.flex_basis = auto();
    }
}

fn apply_flex_placement(style: &mut Style, placement: Placement) {
    match placement {
        Placement::RowItem { weight } | Placement::ColumnItem { weight } => {
            apply_weight(style, weight);
        }
        Placement::TeeArm { row_line } => {
            style.grid_row = line(row_line);
            style.grid_column = line(2);
            style.align_self = Some(AlignSelf::CENTER);
            style.justify_self = Some(AlignSelf::STRETCH);
        }
    }
}

fn justify_content(justify: Option<Justify>) -> JustifyContent {
    match justify {
        None | Some(Justify::Start) => JustifyContent::START,
        Some(Justify::Center) => JustifyContent::CENTER,
        Some(Justify::End) => JustifyContent::END,
        Some(Justify::SpaceBetween) => JustifyContent::SPACE_BETWEEN,
    }
}

/// Default weight of a Row or Col child when `grow` is absent (section 2.3). Every tag is
/// named so a new container or node tag has to choose its weight.
fn default_weight(container: NodeTag, child: &Node) -> u16 {
    let row_weight = match child {
        Node::Pipe(_) | Node::Tee(_) => 0,
        Node::Row(_)
        | Node::Col(_)
        | Node::Lanes(_)
        | Node::Box(_)
        | Node::Item(_)
        | Node::Fact(_)
        | Node::Note(_)
        | Node::Text(_)
        | Node::Callout(_)
        | Node::Frame(_) => 1,
    };
    match container {
        NodeTag::Row => row_weight,
        // Lane heads are equal columns (section 13.6).
        NodeTag::Lanes => 1,
        NodeTag::Text | NodeTag::Callout | NodeTag::Frame => 0,
        NodeTag::Col
        | NodeTag::Page
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::LegendEntry
        | NodeTag::Foot
        | NodeTag::Zone
        | NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Pipe
        | NodeTag::Tee => 0,
    }
}

/// The text style of a container label (section 13.2): `bar` is the frame bar label,
/// `accent` the perimeter label, `plain` the zone label.
pub fn container_label_style(label: LabelStyle) -> TextStyleName {
    match label {
        LabelStyle::Bar => TextStyleName::GcpBar,
        LabelStyle::Accent => TextStyleName::PerimeterLabel,
        LabelStyle::Plain => TextStyleName::ZoneLabel,
    }
}

/// The part names, text style and prefix of a fact by source (section 13.7).
pub fn fact_presentation(
    source: FactSource,
) -> ((PartName, PartName), TextStyleName, &'static str) {
    match source {
        FactSource::Doc => ((PartName::FactBox, PartName::Fact), TextStyleName::Fact, ""),
        FactSource::Built => (
            (PartName::BuiltBox, PartName::Built),
            TextStyleName::Fact,
            "\u{2022} ",
        ),
        FactSource::Ask => (
            (PartName::AskBox, PartName::Ask),
            TextStyleName::Ask,
            "Ask: ",
        ),
    }
}

fn note_style(kind: NoteKind) -> TextStyleName {
    match kind {
        NoteKind::Kicker => TextStyleName::Kicker,
        NoteKind::H1 => TextStyleName::Title,
        NoteKind::Lede => TextStyleName::Lede,
        NoteKind::Legend => TextStyleName::NoteLegend,
        NoteKind::Foot => TextStyleName::Foot,
    }
}

fn canvas_badge_text(canvas: Canvas) -> &'static str {
    match canvas {
        Canvas::Customer => "Customer",
        Canvas::Internal => "Internal",
    }
}

struct TextSpec<'a> {
    text: &'a str,
    style_name: TextStyleName,
    line: Option<stencil_model::Line>,
    align: TextAlign,
    source: NodePointer,
}

impl Builder<'_> {
    fn taffy_error(pointer: &NodePointer) -> impl Fn(taffy::TaffyError) -> LayoutError + '_ {
        move |error| LayoutError::Taffy {
            pointer: pointer.clone(),
            message: error.to_string(),
        }
    }

    fn attach(
        &mut self,
        parent: NodeId,
        child: NodeId,
        pointer: &NodePointer,
    ) -> Result<(), LayoutError> {
        self.tree
            .add_child(parent, child)
            .map_err(Self::taffy_error(pointer))
    }

    /// A node that will hold children, attached as the last child of `parent`.
    fn container(
        &mut self,
        style: Style,
        parent: NodeId,
        pointer: &NodePointer,
    ) -> Result<NodeId, LayoutError> {
        let node = self
            .tree
            .new_with_children(style, &[])
            .map_err(Self::taffy_error(pointer))?;
        self.attach(parent, node, pointer)?;
        Ok(node)
    }

    /// A childless node sized by its style alone.
    fn plain_leaf(
        &mut self,
        style: Style,
        parent: NodeId,
        pointer: &NodePointer,
    ) -> Result<NodeId, LayoutError> {
        let node = self
            .tree
            .new_leaf(style)
            .map_err(Self::taffy_error(pointer))?;
        self.attach(parent, node, pointer)?;
        Ok(node)
    }

    /// A measured text leaf reading along x. Uppercasing runs here, before measurement
    /// (section 2.9).
    fn text_leaf(
        &mut self,
        style: Style,
        parent: Option<NodeId>,
        spec: TextSpec<'_>,
    ) -> Result<(NodeId, usize), LayoutError> {
        self.text_leaf_along(style, parent, spec, Axis::X)
    }

    fn text_leaf_along(
        &mut self,
        style: Style,
        parent: Option<NodeId>,
        spec: TextSpec<'_>,
        axis: Axis,
    ) -> Result<(NodeId, usize), LayoutError> {
        let text = if spec.style_name.text_style().uppercase {
            spec.text.to_uppercase()
        } else {
            spec.text.to_string()
        };
        let index = self.text_leaves.len();
        let node = self
            .tree
            .new_leaf_with_context(style, index)
            .map_err(Self::taffy_error(&spec.source))?;
        if let Some(parent) = parent {
            self.attach(parent, node, &spec.source)?;
        }
        self.text_leaves.push(TextLeaf {
            text,
            style: text_style_for(spec.style_name, self.iso),
            color: text_color(spec.style_name, self.canvas, spec.line),
            align: spec.align,
            axis,
            source: spec.source,
        });
        Ok((node, index))
    }

    fn push_record(&mut self, record: NodeRecord) -> usize {
        self.records.push(record);
        self.records.len() - 1
    }

    fn record(
        pointer: NodePointer,
        tag: NodeTag,
        kind: Option<String>,
        parent: Option<usize>,
        taffy_node: NodeId,
    ) -> NodeRecord {
        NodeRecord {
            pointer,
            tag,
            kind,
            tint: None,
            container: None,
            parent,
            taffy_node,
            content_node: taffy_node,
            parts: Vec::new(),
            arrow_ends: None,
        }
    }

    fn add_page(&mut self, page: &Page) -> Result<NodeId, LayoutError> {
        let root_pointer = NodePointer::root();
        let canvas_width = page.width as f32 + 40.0;
        let root_style = Style {
            size: taffy::Size {
                width: length(canvas_width),
                height: auto(),
            },
            padding: sides(20.0, 20.0, 20.0, 20.0),
            ..flex_column(AlignItems::STRETCH)
        };
        let root = self
            .tree
            .new_with_children(root_style, &[])
            .map_err(Self::taffy_error(&root_pointer))?;
        let root_index = self.push_record(Self::record(
            root_pointer.clone(),
            NodeTag::Page,
            None,
            None,
            root,
        ));

        // A captioned figure draws no badge, kicker, title or lede (section 13.7).
        if page.chrome == Chrome::Full {
            self.add_kicker(page, root, root_index)?;
            self.add_page_text(
                root,
                root_index,
                (NodeTag::Title, "title"),
                &page.title,
                TextStyleName::Title,
                margins(0.0, 0.0, 0.0, 0.0),
            )?;
            self.add_page_text(
                root,
                root_index,
                (NodeTag::Lede, "lede"),
                &page.lede,
                TextStyleName::Lede,
                margins(4.0, 0.0, 16.0, 0.0),
            )?;
        }
        self.add_body(page, root, root_index)?;
        self.add_legend(&page.legend, root, root_index)?;
        if let Some(foot) = &page.foot {
            self.add_page_text(
                root,
                root_index,
                (NodeTag::Foot, "foot"),
                foot,
                TextStyleName::Foot,
                margins(8.0, 0.0, 0.0, 0.0),
            )?;
        }
        Ok(root)
    }

    fn add_kicker(
        &mut self,
        page: &Page,
        root: NodeId,
        root_index: usize,
    ) -> Result<(), LayoutError> {
        let pointer = NodePointer::root().child("kicker");
        let style = Style {
            gap: taffy::Size {
                width: length(6.0),
                height: length(0.0),
            },
            margin: margins(0.0, 0.0, 6.0, 0.0),
            ..flex_row(AlignItems::CENTER)
        };
        let kicker = self.container(style, root, &pointer)?;
        let badge_style = Style {
            padding: sides(2.0, 7.0, 2.0, 7.0),
            ..flex_column(AlignItems::STRETCH)
        };
        let badge = self.container(badge_style, kicker, &pointer)?;
        let (badge_text, badge_leaf) = self.text_leaf(
            base_style(),
            Some(badge),
            TextSpec {
                text: canvas_badge_text(page.canvas),
                style_name: TextStyleName::Badge,
                line: None,
                align: TextAlign::Start,
                source: NodePointer::root().child("canvas"),
            },
        )?;
        let kicker_text_style = Style {
            flex_grow: 1.0,
            flex_basis: length(0.0),
            ..base_style()
        };
        let (kicker_text, kicker_leaf) = self.text_leaf(
            kicker_text_style,
            Some(kicker),
            TextSpec {
                text: &page.kicker,
                style_name: TextStyleName::Kicker,
                line: None,
                align: TextAlign::Start,
                source: pointer.clone(),
            },
        )?;
        let mut record = Self::record(pointer, NodeTag::Kicker, None, Some(root_index), kicker);
        record.parts = vec![
            PartRecord {
                name: PartName::Badge,
                taffy_node: badge,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::BadgeText,
                taffy_node: badge_text,
                text_leaf: Some(badge_leaf),
            },
            PartRecord {
                name: PartName::Text,
                taffy_node: kicker_text,
                text_leaf: Some(kicker_leaf),
            },
        ];
        self.push_record(record);
        Ok(())
    }

    /// `/title`, `/lede` and `/foot`: the node is its own text leaf.
    fn add_page_text(
        &mut self,
        root: NodeId,
        root_index: usize,
        (tag, token): (NodeTag, &str),
        text: &str,
        style_name: TextStyleName,
        margin: Rect<LengthPercentageAuto>,
    ) -> Result<(), LayoutError> {
        let pointer = NodePointer::root().child(token);
        let style = Style {
            margin,
            ..base_style()
        };
        let (leaf, leaf_index) = self.text_leaf(
            style,
            Some(root),
            TextSpec {
                text,
                style_name,
                line: None,
                align: TextAlign::Start,
                source: pointer.clone(),
            },
        )?;
        let mut record = Self::record(pointer, tag, None, Some(root_index), leaf);
        record.parts = vec![PartRecord {
            name: PartName::Text,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        }];
        self.push_record(record);
        Ok(())
    }

    fn add_body(
        &mut self,
        page: &Page,
        root: NodeId,
        root_index: usize,
    ) -> Result<(), LayoutError> {
        let pointer = NodePointer::root().child("body");
        let style = Style {
            gap: taffy::Size {
                width: length(0.0),
                height: length(8.0),
            },
            ..flex_column(AlignItems::STRETCH)
        };
        let body = self.container(style, root, &pointer)?;
        let body_index = self.push_record(Self::record(
            pointer.clone(),
            NodeTag::Body,
            None,
            Some(root_index),
            body,
        ));
        for (index, node) in page.body.iter().enumerate() {
            self.add_body_node(
                node,
                pointer.index(index),
                body_index,
                body,
                Placement::ColumnItem { weight: 0 },
                1,
            )?;
        }
        Ok(())
    }

    fn add_legend(
        &mut self,
        legend: &[LegendEntry],
        root: NodeId,
        root_index: usize,
    ) -> Result<(), LayoutError> {
        if legend.is_empty() {
            return Ok(());
        }
        let pointer = NodePointer::root().child("legend");
        let style = Style {
            flex_wrap: FlexWrap::Wrap,
            gap: taffy::Size {
                width: length(16.0),
                height: length(6.0),
            },
            margin: margins(10.0, 0.0, 0.0, 0.0),
            ..flex_row(AlignItems::STRETCH)
        };
        let legend_node = self.container(style, root, &pointer)?;
        let legend_index = self.push_record(Self::record(
            pointer.clone(),
            NodeTag::Legend,
            None,
            Some(root_index),
            legend_node,
        ));
        for (index, entry) in legend.iter().enumerate() {
            self.add_legend_entry(entry, pointer.index(index), legend_node, legend_index)?;
        }
        Ok(())
    }

    fn add_legend_entry(
        &mut self,
        entry: &LegendEntry,
        pointer: NodePointer,
        legend_node: NodeId,
        legend_index: usize,
    ) -> Result<(), LayoutError> {
        let style = Style {
            gap: taffy::Size {
                width: length(6.0),
                height: length(0.0),
            },
            ..flex_row(AlignItems::CENTER)
        };
        let entry_node = self.container(style, legend_node, &pointer)?;
        let swatch = self.plain_leaf(
            Style {
                size: fixed_size(26.0, 2.0),
                ..base_style()
            },
            entry_node,
            &pointer,
        )?;
        let (label, label_leaf) = self.text_leaf(
            base_style(),
            Some(entry_node),
            TextSpec {
                text: legend_label(entry.line, entry.tint),
                style_name: TextStyleName::LegendLabel,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("line"),
            },
        )?;
        let (text, text_leaf) = self.text_leaf(
            base_style(),
            Some(entry_node),
            TextSpec {
                text: &entry.text,
                style_name: TextStyleName::LegendText,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        let mut record = Self::record(
            pointer,
            NodeTag::LegendEntry,
            Some(line_key(entry.line, entry.tint).to_string()),
            Some(legend_index),
            entry_node,
        );
        record.tint = line_tint(entry.line, entry.tint);
        record.parts = vec![
            PartRecord {
                name: PartName::Swatch,
                taffy_node: swatch,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::LegendLabel,
                taffy_node: label,
                text_leaf: Some(label_leaf),
            },
            PartRecord {
                name: PartName::LegendText,
                taffy_node: text,
                text_leaf: Some(text_leaf),
            },
        ];
        self.push_record(record);
        Ok(())
    }

    /// Recursion depth is bounded by DEPTH_MAX: layout_page only builds vetted pages, and
    /// the guard below keeps that true if this is ever called otherwise.
    fn add_body_node(
        &mut self,
        node: &Node,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
        depth: usize,
    ) -> Result<(), LayoutError> {
        if depth > DEPTH_MAX {
            return Err(LayoutError::Invalid(vec![Violation {
                message: format!("depth {depth} is above {DEPTH_MAX}"),
                pointer,
                rule: VetRule::DepthExceeded,
            }]));
        }
        match node {
            Node::Row(row) => self.add_flex_container(
                FlexContainer {
                    tag: NodeTag::Row,
                    gap: row.gap,
                    grow: row.grow.as_deref(),
                    justify: row.justify,
                    children: &row.children,
                },
                pointer,
                parent_index,
                parent_container,
                placement,
                depth,
            ),
            Node::Col(col) => self.add_flex_container(
                FlexContainer {
                    tag: NodeTag::Col,
                    gap: col.gap,
                    grow: col.grow.as_deref(),
                    justify: col.justify,
                    children: &col.children,
                },
                pointer,
                parent_index,
                parent_container,
                placement,
                depth,
            ),
            Node::Lanes(lanes) => self.add_lanes(
                lanes,
                pointer,
                parent_index,
                parent_container,
                placement,
                depth,
            ),
            Node::Box(box_node) => self.add_box(
                box_node,
                pointer,
                parent_index,
                parent_container,
                placement,
                depth,
            ),
            Node::Item(item) => {
                self.add_item(item, pointer, parent_index, parent_container, placement)
            }
            Node::Fact(fact) => {
                self.add_fact(fact, pointer, parent_index, parent_container, placement)
            }
            Node::Note(note) => {
                self.add_note(note, pointer, parent_index, parent_container, placement)
            }
            Node::Pipe(pipe) => {
                self.add_pipe(pipe, pointer, parent_index, parent_container, placement)
            }
            Node::Tee(tee) => self.add_tee(tee, pointer, parent_index, parent_container, placement),
            Node::Text(text) => {
                self.add_text(text, pointer, parent_index, parent_container, placement)
            }
            Node::Callout(callout) => {
                self.add_callout(callout, pointer, parent_index, parent_container, placement)
            }
            Node::Frame(frame) => {
                self.add_frame(frame, pointer, parent_index, parent_container, placement)
            }
        }
    }

    fn add_flex_container(
        &mut self,
        container: FlexContainer<'_>,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
        depth: usize,
    ) -> Result<(), LayoutError> {
        let gap = f32::from(container.gap.unwrap_or(GAP_DEFAULT_PX));
        let lays_out_as_row = container.tag == NodeTag::Row;
        let (direction, gap_size) = if lays_out_as_row {
            (
                FlexDirection::Row,
                taffy::Size {
                    width: length(gap),
                    height: length(0.0),
                },
            )
        } else {
            (
                FlexDirection::Column,
                taffy::Size {
                    width: length(0.0),
                    height: length(gap),
                },
            )
        };
        let mut style = Style {
            flex_direction: direction,
            gap: gap_size,
            justify_content: Some(justify_content(container.justify)),
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let node = self.container(style, parent_container, &pointer)?;
        let index = self.push_record(Self::record(
            pointer.clone(),
            container.tag,
            None,
            Some(parent_index),
            node,
        ));
        let weights: Vec<u16> = match container.grow {
            Some(weights) if weights.len() == container.children.len() => weights.to_vec(),
            Some(weights) => {
                return Err(LayoutError::Invalid(vec![Violation {
                    message: format!(
                        "grow has {} weights for {} children",
                        weights.len(),
                        container.children.len()
                    ),
                    pointer: pointer.child("grow"),
                    rule: VetRule::GrowLengthMismatch,
                }]));
            }
            None => container
                .children
                .iter()
                .map(|child| default_weight(container.tag, child))
                .collect(),
        };
        let children_pointer = pointer.child("children");
        for (child_index, (child, &weight)) in container.children.iter().zip(&weights).enumerate() {
            let child_placement = if lays_out_as_row {
                Placement::RowItem { weight }
            } else {
                Placement::ColumnItem { weight }
            };
            self.add_body_node(
                child,
                children_pointer.index(child_index),
                index,
                node,
                child_placement,
                depth + 1,
            )?;
        }
        Ok(())
    }

    /// A Lanes node (section 13.6): a column of two parts, `Heads`, a row of equal columns
    /// `gap` apart holding the heads, and `Band`, a leaf as wide as the node whose height
    /// is the sum of its message rows. The heads stay the node's children in geometry.
    fn add_lanes(
        &mut self,
        lanes: &Lanes,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
        depth: usize,
    ) -> Result<(), LayoutError> {
        let mut style = flex_column(AlignItems::STRETCH);
        apply_flex_placement(&mut style, placement);
        let lanes_node = self.container(style, parent_container, &pointer)?;
        let gap = f32::from(lanes.gap.unwrap_or(LANE_GAP_DEFAULT_PX));
        let heads_style = Style {
            gap: taffy::Size {
                width: length(gap),
                height: length(0.0),
            },
            ..flex_row(AlignItems::STRETCH)
        };
        let heads = self.container(heads_style, lanes_node, &pointer)?;
        let band_height = self.lanes_plan.band_height(&pointer);
        let band = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: auto(),
                    height: length(band_height),
                },
                ..base_style()
            },
            lanes_node,
            &pointer,
        )?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Lanes,
            None,
            Some(parent_index),
            lanes_node,
        );
        record.parts = vec![
            PartRecord {
                name: PartName::Heads,
                taffy_node: heads,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::Band,
                taffy_node: band,
                text_leaf: None,
            },
        ];
        let index = self.push_record(record);
        let children_pointer = pointer.child("children");
        for (child_index, child) in lanes.children.iter().enumerate() {
            self.add_body_node(
                child,
                children_pointer.index(child_index),
                index,
                heads,
                Placement::RowItem {
                    weight: default_weight(NodeTag::Lanes, child),
                },
                depth + 1,
            )?;
        }
        Ok(())
    }

    /// A Box laid out from its container kind (section 13.1): a frame kind as section 2.4's
    /// gcp zone with bar and body, every other role as the non-gcp zone, with the kind's
    /// border width, padding and label style.
    fn add_box(
        &mut self,
        box_node: &BoxNode,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
        depth: usize,
    ) -> Result<(), LayoutError> {
        let grammar = self.grammar;
        let Some(kind) = grammar.container(&box_node.kind) else {
            return Err(LayoutError::Invalid(vec![Violation {
                message: format!(
                    "kind \"{}\" is not a container kind of grammar {}",
                    box_node.kind, grammar.name
                ),
                pointer: pointer.child("kind"),
                rule: VetRule::KindUnknown,
            }]));
        };
        let is_frame = kind.role == Role::Frame;
        let border = kind.border.width;
        let padding = kind.padding;
        let gap = if is_frame { 0.0 } else { 8.0 };
        let mut style = Style {
            border: sides(border, border, border, border),
            padding: sides(padding, padding, padding, padding),
            gap: taffy::Size {
                width: length(0.0),
                height: length(gap),
            },
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let zone_node = self.container(style, parent_container, &pointer)?;
        let label_source = pointer.child("label");
        let tint = box_tint(kind, box_node.tint);
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Zone,
            Some(box_key(&kind.name, tint)),
            Some(parent_index),
            zone_node,
        );
        record.tint = tint;
        record.container = Some(ContainerLook {
            role: kind.role,
            tone: kind.tone,
            pattern: kind.border.pattern,
            border_width: kind.border.width,
            radius: kind.radius,
            label: kind.label,
        });
        let label_style = container_label_style(kind.label);
        let label_leaf_style = if self.iso {
            Style {
                margin: margins(0.0, 0.0, iso_label_reserve(), 0.0),
                ..base_style()
            }
        } else {
            base_style()
        };

        let children_container = if is_frame {
            let bar_style = Style {
                padding: sides(7.0, 16.0, 7.0, 16.0),
                ..flex_column(AlignItems::STRETCH)
            };
            let bar = self.container(bar_style, zone_node, &pointer)?;
            let (label, label_leaf) = self.text_leaf(
                label_leaf_style,
                Some(bar),
                TextSpec {
                    text: &box_node.label,
                    style_name: label_style,
                    line: None,
                    align: TextAlign::Start,
                    source: label_source,
                },
            )?;
            let body_style = Style {
                flex_grow: 1.0,
                padding: sides(16.0, 14.0, 14.0, 14.0),
                gap: taffy::Size {
                    width: length(0.0),
                    height: length(8.0),
                },
                ..flex_column(AlignItems::STRETCH)
            };
            let body = self.container(body_style, zone_node, &pointer)?;
            record.content_node = body;
            record.parts = vec![
                PartRecord {
                    name: PartName::Bar,
                    taffy_node: bar,
                    text_leaf: None,
                },
                PartRecord {
                    name: PartName::Label,
                    taffy_node: label,
                    text_leaf: Some(label_leaf),
                },
                PartRecord {
                    name: PartName::Body,
                    taffy_node: body,
                    text_leaf: None,
                },
            ];
            body
        } else {
            let (label, label_leaf) = self.text_leaf(
                label_leaf_style,
                Some(zone_node),
                TextSpec {
                    text: &box_node.label,
                    style_name: label_style,
                    line: None,
                    align: TextAlign::Start,
                    source: label_source,
                },
            )?;
            record.parts = vec![PartRecord {
                name: PartName::Label,
                taffy_node: label,
                text_leaf: Some(label_leaf),
            }];
            zone_node
        };

        let index = self.push_record(record);
        let children_pointer = pointer.child("children");
        for (child_index, child) in box_node.children.iter().enumerate() {
            self.add_body_node(
                child,
                children_pointer.index(child_index),
                index,
                children_container,
                Placement::ColumnItem { weight: 0 },
                depth + 1,
            )?;
        }
        Ok(())
    }

    /// An Item laid out as section 2.5's card: icon, title, subtitle, then one box per fact
    /// in list order (section 13.7).
    fn add_item(
        &mut self,
        item: &Item,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = Style {
            gap: taffy::Size {
                width: length(10.0),
                height: length(0.0),
            },
            padding: sides(6.0, 10.0, 6.0, 10.0),
            border: sides(1.5, 1.5, 1.5, 1.5),
            min_size: taffy::Size {
                width: auto(),
                height: length(44.0),
            },
            ..flex_row(AlignItems::CENTER)
        };
        apply_flex_placement(&mut style, placement);
        let card = self.container(style, parent_container, &pointer)?;
        let mut parts = Vec::new();

        if item.icon.is_some() {
            let icon = self.plain_leaf(
                Style {
                    size: fixed_size(28.0, 28.0),
                    ..base_style()
                },
                card,
                &pointer,
            )?;
            parts.push(PartRecord {
                name: PartName::Icon,
                taffy_node: icon,
                text_leaf: None,
            });
        }

        let column_style = Style {
            flex_grow: 1.0,
            flex_basis: length(0.0),
            ..flex_column(AlignItems::STRETCH)
        };
        let column = self.container(column_style, card, &pointer)?;
        parts.push(PartRecord {
            name: PartName::Text,
            taffy_node: column,
            text_leaf: None,
        });

        let (function_name, function_leaf) = self.text_leaf(
            base_style(),
            Some(column),
            TextSpec {
                text: &item.title,
                style_name: TextStyleName::CardFunction,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("title"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::FunctionName,
            taffy_node: function_name,
            text_leaf: Some(function_leaf),
        });

        if let Some(subtitle) = &item.subtitle {
            let (product, product_leaf) = self.text_leaf(
                Style {
                    margin: margins(1.0, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                Some(column),
                TextSpec {
                    text: subtitle,
                    style_name: TextStyleName::CardProduct,
                    line: None,
                    align: TextAlign::Start,
                    source: pointer.child("subtitle"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::ProductName,
                taffy_node: product,
                text_leaf: Some(product_leaf),
            });
        }

        let facts_pointer = pointer.child("facts");
        for (index, fact) in item.facts.iter().enumerate() {
            let (names, style_name, prefix) = fact_presentation(fact.source);
            let drawn = format!("{prefix}{}", fact.text);
            self.add_chip(
                column,
                &mut parts,
                names,
                &drawn,
                style_name,
                facts_pointer.index(index).child("text"),
            )?;
        }

        let mut record = Self::record(pointer, NodeTag::Pcard, None, Some(parent_index), card);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// The fact boxes of an Item: margin-top 4, padding 4/8, one text leaf.
    fn add_chip(
        &mut self,
        column: NodeId,
        parts: &mut Vec<PartRecord>,
        names: (PartName, PartName),
        text: &str,
        style_name: TextStyleName,
        source: NodePointer,
    ) -> Result<(), LayoutError> {
        let box_style = Style {
            margin: margins(4.0, 0.0, 0.0, 0.0),
            padding: sides(4.0, 8.0, 4.0, 8.0),
            ..flex_column(AlignItems::STRETCH)
        };
        let chip = self.container(box_style, column, &source)?;
        let (leaf, leaf_index) = self.text_leaf(
            base_style(),
            Some(chip),
            TextSpec {
                text,
                style_name,
                line: None,
                align: TextAlign::Start,
                source,
            },
        )?;
        parts.push(PartRecord {
            name: names.0,
            taffy_node: chip,
            text_leaf: None,
        });
        parts.push(PartRecord {
            name: names.1,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        });
        Ok(())
    }

    fn add_fact(
        &mut self,
        fact: &Fact,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = Style {
            padding: sides(4.0, 8.0, 4.0, 8.0),
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let fact_node = self.container(style, parent_container, &pointer)?;
        let (_, style_name, prefix) = fact_presentation(fact.source);
        let drawn = format!("{prefix}{}", fact.text);
        let (leaf, leaf_index) = self.text_leaf(
            base_style(),
            Some(fact_node),
            TextSpec {
                text: &drawn,
                style_name,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        let mut record = Self::record(pointer, NodeTag::Fact, None, Some(parent_index), fact_node);
        record.parts = vec![PartRecord {
            name: PartName::Text,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        }];
        self.push_record(record);
        Ok(())
    }

    fn add_note(
        &mut self,
        note: &Note,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = base_style();
        apply_flex_placement(&mut style, placement);
        let (leaf, leaf_index) = self.text_leaf(
            style,
            Some(parent_container),
            TextSpec {
                text: &note.text,
                style_name: note_style(note.kind),
                line: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        let mut record = Self::record(pointer, NodeTag::Note, None, Some(parent_index), leaf);
        record.parts = vec![PartRecord {
            name: PartName::Text,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        }];
        self.push_record(record);
        Ok(())
    }

    fn add_pipe(
        &mut self,
        pipe: &Pipe,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let horizontal = pipe.dir == PipeDir::Horizontal;
        let mut style = if horizontal {
            flex_row(AlignItems::CENTER)
        } else {
            Style {
                min_size: taffy::Size {
                    width: auto(),
                    height: length(36.0),
                },
                ..flex_column(AlignItems::CENTER)
            }
        };
        style.justify_content = Some(JustifyContent::CENTER);
        // The run axis decides the fill: across the parent it stretches the gutter, along the
        // parent's main axis it keeps its max-content run length (section 2.7).
        style.align_self = match (placement, horizontal) {
            (Placement::RowItem { .. }, true) | (Placement::ColumnItem { .. }, false) => {
                Some(AlignSelf::CENTER)
            }
            (Placement::RowItem { .. }, false) | (Placement::ColumnItem { .. }, true) => {
                Some(AlignSelf::STRETCH)
            }
            (Placement::TeeArm { .. }, _) => None,
        };
        apply_flex_placement(&mut style, placement);
        let pipe_node = self.container(style, parent_container, &pointer)?;

        let dot_style = Style {
            size: fixed_size(8.0, 8.0),
            ..base_style()
        };
        let wire_minimum = match (horizontal, pipe.line) {
            (true, _) => 14.0,
            (false, stencil_model::Line::Deny) => 16.0,
            (false, _) => 12.0,
        };
        let wire_style = if horizontal {
            Style {
                flex_grow: 1.0,
                flex_basis: length(0.0),
                min_size: taffy::Size {
                    width: length(wire_minimum),
                    height: auto(),
                },
                size: taffy::Size {
                    width: auto(),
                    height: length(2.0),
                },
                ..base_style()
            }
        } else {
            Style {
                flex_grow: 1.0,
                flex_basis: length(0.0),
                min_size: taffy::Size {
                    width: auto(),
                    height: length(wire_minimum),
                },
                size: taffy::Size {
                    width: length(2.0),
                    height: auto(),
                },
                ..base_style()
            }
        };

        let dot_start = self.plain_leaf(dot_style.clone(), pipe_node, &pointer)?;
        let wire_start = self.plain_leaf(wire_style.clone(), pipe_node, &pointer)?;
        // Under iso a vertical pipe's tag reads along y (section 12.4): the pill and its
        // runs are laid out as strips, the label left of the sub, with the pill's padding
        // turned with it.
        let tag_axis = if self.iso && !horizontal {
            Axis::Y
        } else {
            Axis::X
        };
        let tag_style = match tag_axis {
            Axis::X => Style {
                padding: sides(6.0, 8.0, 6.0, 8.0),
                border: sides(1.5, 1.5, 1.5, 1.5),
                flex_shrink: if horizontal { 1.0 } else { 0.0 },
                max_size: taffy::Size {
                    width: LengthPercentageAuto::percent(1.0),
                    height: auto(),
                },
                ..flex_column(AlignItems::CENTER)
            },
            Axis::Y => Style {
                padding: sides(8.0, 6.0, 8.0, 6.0),
                border: sides(1.5, 1.5, 1.5, 1.5),
                flex_shrink: 0.0,
                ..flex_row(AlignItems::CENTER)
            },
        };
        let tag = self.container(tag_style, pipe_node, &pointer)?;
        let (label, label_leaf) = self.text_leaf_along(
            base_style(),
            Some(tag),
            TextSpec {
                text: &pipe.label,
                style_name: TextStyleName::TagLabel,
                line: Some(pipe.line),
                align: TextAlign::Center,
                source: pointer.child("label"),
            },
            tag_axis,
        )?;
        let mut parts = vec![
            PartRecord {
                name: PartName::DotStart,
                taffy_node: dot_start,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::WireStart,
                taffy_node: wire_start,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::Tag,
                taffy_node: tag,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::TagLabel,
                taffy_node: label,
                text_leaf: Some(label_leaf),
            },
        ];
        if let Some(sub) = &pipe.sub {
            let sub_margin = match tag_axis {
                Axis::X => margins(2.0, 0.0, 0.0, 0.0),
                Axis::Y => margins(0.0, 0.0, 0.0, 2.0),
            };
            let (sub_node, sub_leaf) = self.text_leaf_along(
                Style {
                    margin: sub_margin,
                    ..base_style()
                },
                Some(tag),
                TextSpec {
                    text: sub,
                    style_name: TextStyleName::TagSub,
                    line: Some(pipe.line),
                    align: TextAlign::Center,
                    source: pointer.child("sub"),
                },
                tag_axis,
            )?;
            parts.push(PartRecord {
                name: PartName::TagSub,
                taffy_node: sub_node,
                text_leaf: Some(sub_leaf),
            });
        }
        let wire_end = self.plain_leaf(wire_style, pipe_node, &pointer)?;
        let dot_end = self.plain_leaf(dot_style, pipe_node, &pointer)?;
        parts.push(PartRecord {
            name: PartName::WireEnd,
            taffy_node: wire_end,
            text_leaf: None,
        });
        parts.push(PartRecord {
            name: PartName::DotEnd,
            taffy_node: dot_end,
            text_leaf: None,
        });

        let mut record = Self::record(
            pointer,
            NodeTag::Pipe,
            Some(line_key(pipe.line, pipe.tint).to_string()),
            Some(parent_index),
            pipe_node,
        );
        record.tint = line_tint(pipe.line, pipe.tint);
        record.parts = parts;
        record.arrow_ends = arrow_ends(pipe.arrow, horizontal);
        self.push_record(record);
        Ok(())
    }

    fn add_tee(
        &mut self,
        tee: &Tee,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = Style {
            display: Display::Grid,
            grid_template_columns: vec![length(14.0), fr(1.0)],
            grid_template_rows: vec![fr(1.0), auto(), fr(1.0)],
            min_size: taffy::Size {
                width: length(118.0),
                height: auto(),
            },
            ..base_style()
        };
        if let Placement::RowItem { .. } = placement {
            style.align_self = Some(AlignSelf::STRETCH);
        }
        apply_flex_placement(&mut style, placement);
        let tee_node = self.container(style, parent_container, &pointer)?;

        let spine_style = Style {
            grid_column: line(1),
            grid_row: Line {
                start: line(1),
                end: line(4),
            },
            size: taffy::Size {
                width: length(2.0),
                height: auto(),
            },
            justify_self: Some(AlignSelf::CENTER),
            margin: margins(18.0, 0.0, 18.0, 0.0),
            ..base_style()
        };
        let spine = self.plain_leaf(spine_style, tee_node, &pointer)?;
        let hub_style = Style {
            grid_row: line(2),
            grid_column: Line {
                start: line(1),
                end: line(3),
            },
            justify_self: Some(AlignSelf::CENTER),
            padding: sides(6.0, 8.0, 6.0, 8.0),
            border: sides(1.5, 1.5, 1.5, 1.5),
            ..flex_column(AlignItems::CENTER)
        };
        let hub = self.container(hub_style, tee_node, &pointer)?;
        let (hub_text, hub_leaf) = self.text_leaf(
            base_style(),
            Some(hub),
            TextSpec {
                text: &tee.hub,
                style_name: TextStyleName::TagLabel,
                line: Some(tee.line),
                align: TextAlign::Center,
                source: pointer.child("hub"),
            },
        )?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Tee,
            Some(line_key(tee.line, tee.tint).to_string()),
            Some(parent_index),
            tee_node,
        );
        record.tint = line_tint(tee.line, tee.tint);
        record.parts = vec![
            PartRecord {
                name: PartName::Spine,
                taffy_node: spine,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::Hub,
                taffy_node: hub,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::HubText,
                taffy_node: hub_text,
                text_leaf: Some(hub_leaf),
            },
        ];
        let tee_index = self.push_record(record);

        let arms_pointer = pointer.child("arms");
        for (arm_index, (arm, row_line)) in tee.arms.iter().zip([1_i16, 3]).enumerate() {
            let TeeArm::Pipe(arm_pipe) = arm;
            self.add_pipe(
                arm_pipe,
                arms_pointer.index(arm_index),
                tee_index,
                tee_node,
                Placement::TeeArm { row_line },
            )?;
        }
        Ok(())
    }

    /// Text block (section 11.3): an optional heading, then one row per body line. A list
    /// line holds a 22 px marker cell and the wrapped line; a plain line is the text leaf.
    fn add_text(
        &mut self,
        text: &Text,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = block_style(BLOCK_PADDING_PX);
        apply_flex_placement(&mut style, placement);
        let block = self.container(style, parent_container, &pointer)?;
        let mut parts = Vec::new();
        if let Some(heading) = &text.heading {
            self.add_block_heading(block, &mut parts, heading, pointer.child("heading"))?;
        }
        let body_pointer = pointer.child("body");
        for (line_index, line_text) in text.body.iter().enumerate() {
            let line_pointer = body_pointer.index(line_index);
            let margin_top = if line_index == 0 {
                0.0
            } else {
                BLOCK_LINE_GAP_PX
            };
            match text.list {
                ListKind::Plain => {
                    let (leaf, leaf_index) = self.text_leaf(
                        Style {
                            margin: margins(margin_top, 0.0, 0.0, 0.0),
                            ..base_style()
                        },
                        Some(block),
                        TextSpec {
                            text: line_text,
                            style_name: TextStyleName::BlockBody,
                            line: None,
                            align: TextAlign::Start,
                            source: line_pointer,
                        },
                    )?;
                    parts.push(PartRecord {
                        name: PartName::BodyLine,
                        taffy_node: leaf,
                        text_leaf: Some(leaf_index),
                    });
                }
                ListKind::Numbered | ListKind::Bulleted => {
                    let row = self.container(
                        Style {
                            margin: margins(margin_top, 0.0, 0.0, 0.0),
                            ..flex_row(AlignItems::START)
                        },
                        block,
                        &line_pointer,
                    )?;
                    let marker_style = Style {
                        size: taffy::Size {
                            width: length(LIST_INDENT_PX),
                            height: length(
                                TextStyleName::BlockBody.text_style().style.line_height_px,
                            ),
                        },
                        ..base_style()
                    };
                    let marker = if text.list == ListKind::Numbered {
                        let number = format!("{}.", line_index + 1);
                        let (marker_node, marker_leaf) = self.text_leaf(
                            marker_style,
                            Some(row),
                            TextSpec {
                                text: &number,
                                style_name: TextStyleName::BlockBody,
                                line: None,
                                align: TextAlign::Start,
                                source: line_pointer.clone(),
                            },
                        )?;
                        PartRecord {
                            name: PartName::Marker,
                            taffy_node: marker_node,
                            text_leaf: Some(marker_leaf),
                        }
                    } else {
                        PartRecord {
                            name: PartName::Marker,
                            taffy_node: self.plain_leaf(marker_style, row, &line_pointer)?,
                            text_leaf: None,
                        }
                    };
                    parts.push(marker);
                    let (leaf, leaf_index) = self.text_leaf(
                        Style {
                            flex_grow: 1.0,
                            flex_basis: length(0.0),
                            ..base_style()
                        },
                        Some(row),
                        TextSpec {
                            text: line_text,
                            style_name: TextStyleName::BlockBody,
                            line: None,
                            align: TextAlign::Start,
                            source: line_pointer,
                        },
                    )?;
                    parts.push(PartRecord {
                        name: PartName::BodyLine,
                        taffy_node: leaf,
                        text_leaf: Some(leaf_index),
                    });
                }
            }
        }
        let mut record = Self::record(pointer, NodeTag::Text, None, Some(parent_index), block);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// Callout block (section 11.3): the Text box with a 4 px accent bar along the inside
    /// of the left border. The bar is absolutely placed in the left padding, which is 4 px
    /// wider than the other sides, so it takes no flow space.
    fn add_callout(
        &mut self,
        callout: &Callout,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = block_style(BLOCK_PADDING_PX);
        style.padding = sides(
            BLOCK_PADDING_PX,
            BLOCK_PADDING_PX,
            BLOCK_PADDING_PX,
            BLOCK_PADDING_PX + CALLOUT_ACCENT_PX,
        );
        apply_flex_placement(&mut style, placement);
        let block = self.container(style, parent_container, &pointer)?;
        let accent = self.plain_leaf(
            Style {
                position: Position::Absolute,
                inset: Rect {
                    left: length(0.0),
                    right: auto(),
                    top: length(0.0),
                    bottom: length(0.0),
                },
                size: taffy::Size {
                    width: length(CALLOUT_ACCENT_PX),
                    height: auto(),
                },
                ..base_style()
            },
            block,
            &pointer,
        )?;
        let mut parts = vec![PartRecord {
            name: PartName::Accent,
            taffy_node: accent,
            text_leaf: None,
        }];
        if let Some(title) = &callout.title {
            self.add_block_heading(block, &mut parts, title, pointer.child("title"))?;
        }
        let (leaf, leaf_index) = self.text_leaf(
            base_style(),
            Some(block),
            TextSpec {
                text: &callout.text,
                style_name: TextStyleName::BlockBody,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::Text,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        });
        let mut record = Self::record(pointer, NodeTag::Callout, None, Some(parent_index), block);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// Frame block (section 11.3): authored height, width from the container, and the
    /// label in a chip centered on both axes.
    fn add_frame(
        &mut self,
        frame: &Frame,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = Style {
            border: sides(
                BLOCK_BORDER_PX,
                BLOCK_BORDER_PX,
                BLOCK_BORDER_PX,
                BLOCK_BORDER_PX,
            ),
            padding: sides(
                FRAME_PADDING_PX,
                FRAME_PADDING_PX,
                FRAME_PADDING_PX,
                FRAME_PADDING_PX,
            ),
            justify_content: Some(JustifyContent::CENTER),
            ..flex_column(AlignItems::CENTER)
        };
        apply_flex_placement(&mut style, placement);
        // The authored height holds against a grow weight in a column and against stretch
        // in a Row alike.
        let height_px = f32::from(frame.height);
        style.size.height = length(height_px);
        style.min_size.height = length(height_px);
        style.max_size.height = length(height_px);
        let frame_node = self.container(style, parent_container, &pointer)?;
        let chip_style = Style {
            padding: sides(4.0, 8.0, 4.0, 8.0),
            max_size: taffy::Size {
                width: LengthPercentageAuto::percent(1.0),
                height: auto(),
            },
            ..flex_column(AlignItems::STRETCH)
        };
        let chip = self.container(chip_style, frame_node, &pointer)?;
        let (label, label_leaf) = self.text_leaf(
            base_style(),
            Some(chip),
            TextSpec {
                text: &frame.label,
                style_name: TextStyleName::ZoneLabel,
                line: None,
                align: TextAlign::Center,
                source: pointer.child("label"),
            },
        )?;
        let mut record = Self::record(
            pointer,
            NodeTag::Frame,
            None,
            Some(parent_index),
            frame_node,
        );
        record.parts = vec![
            PartRecord {
                name: PartName::LabelChip,
                taffy_node: chip,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::Label,
                taffy_node: label,
                text_leaf: Some(label_leaf),
            },
        ];
        self.push_record(record);
        Ok(())
    }

    /// The Text heading and the Callout title: 13 px bold with 6 px below it.
    fn add_block_heading(
        &mut self,
        block: NodeId,
        parts: &mut Vec<PartRecord>,
        text: &str,
        source: NodePointer,
    ) -> Result<(), LayoutError> {
        let (leaf, leaf_index) = self.text_leaf(
            Style {
                margin: margins(0.0, 0.0, BLOCK_HEADING_GAP_PX, 0.0),
                ..base_style()
            },
            Some(block),
            TextSpec {
                text,
                style_name: TextStyleName::CardFunction,
                line: None,
                align: TextAlign::Start,
                source,
            },
        )?;
        parts.push(PartRecord {
            name: PartName::Heading,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        });
        Ok(())
    }
}

/// The Text and Callout box: a stretched column with the 1.25 px block border.
fn block_style(padding: f32) -> Style {
    Style {
        border: sides(
            BLOCK_BORDER_PX,
            BLOCK_BORDER_PX,
            BLOCK_BORDER_PX,
            BLOCK_BORDER_PX,
        ),
        padding: sides(padding, padding, padding, padding),
        ..flex_column(AlignItems::STRETCH)
    }
}

fn arrow_ends(arrow: Arrow, horizontal: bool) -> Option<ArrowEnds> {
    let (start, end) = match arrow {
        Arrow::None => return None,
        Arrow::Start => (true, false),
        Arrow::End => (false, true),
        Arrow::Both => (true, true),
    };
    Some(ArrowEnds {
        start,
        end,
        horizontal,
    })
}

struct FlexContainer<'a> {
    tag: NodeTag,
    gap: Option<u16>,
    grow: Option<&'a [u16]>,
    justify: Option<Justify>,
    children: &'a [Node],
}
