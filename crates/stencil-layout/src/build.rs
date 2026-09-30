//! Builds the taffy tree for a vetted page (sections 2.2 to 2.8) and records, per geometry
//! node, the taffy nodes its bounds, content box and parts are read from.

use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{
    Canvas, DEPTH_MAX, Fact, GAP_DEFAULT_PX, Justify, LegendEntry, Node, Note, NoteKind, Page,
    Pcard, Pipe, PipeDir, PipeKind, Tee, TeeArm, VetRule, Violation, Zone, ZoneKind,
};
use taffy::prelude::{
    AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Line, NodeId, Rect, Style, TaffyTree, auto, fr, length,
    line,
};

use crate::styles::text_color;
use crate::{LayoutError, NodeTag, PartName, TextAlign};

/// Taffy's node context is an index into `BuiltPage::text_leaves`.
pub(crate) type LayoutTree = TaffyTree<usize>;

pub(crate) struct TextLeaf {
    pub text: String,
    pub style_name: TextStyleName,
    pub color: &'static str,
    pub align: TextAlign,
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
    pub kind: Option<&'static str>,
    pub parent: Option<usize>,
    pub taffy_node: NodeId,
    /// The taffy node whose content box children must stay inside.
    pub content_node: NodeId,
    pub parts: Vec<PartRecord>,
}

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
    /// A child of a Col, a Zone or the page body: the main axis is vertical.
    ColumnItem { weight: u16 },
    /// A Tee arm in grid row line 1 or 3, column line 2.
    TeeArm { row_line: i16 },
}

pub(crate) fn build_page(page: &Page) -> Result<BuiltPage, LayoutError> {
    let mut builder = Builder {
        tree: TaffyTree::new(),
        records: Vec::new(),
        text_leaves: Vec::new(),
        canvas: page.canvas,
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

struct Builder {
    tree: LayoutTree,
    records: Vec<NodeRecord>,
    text_leaves: Vec<TextLeaf>,
    canvas: Canvas,
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
        | Node::Zone(_)
        | Node::Pcard(_)
        | Node::Fact(_)
        | Node::Note(_) => 1,
    };
    match container {
        NodeTag::Row => row_weight,
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

fn zone_border_px(kind: ZoneKind) -> f32 {
    match kind {
        ZoneKind::Gcp => 3.0,
        ZoneKind::Vpc | ZoneKind::Optional => 2.0,
        ZoneKind::RegionA
        | ZoneKind::RegionB
        | ZoneKind::Subnet
        | ZoneKind::OnpremA
        | ZoneKind::OnpremB
        | ZoneKind::Project => 1.5,
        ZoneKind::K8s => 0.0,
        ZoneKind::Perimeter => 2.5,
    }
}

fn zone_padding_px(kind: ZoneKind) -> f32 {
    match kind {
        ZoneKind::Gcp => 0.0,
        ZoneKind::Vpc => 10.0,
        ZoneKind::RegionA
        | ZoneKind::RegionB
        | ZoneKind::Subnet
        | ZoneKind::OnpremA
        | ZoneKind::OnpremB
        | ZoneKind::Project
        | ZoneKind::Optional
        | ZoneKind::K8s
        | ZoneKind::Perimeter => 12.0,
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

fn legend_label(kind: PipeKind) -> &'static str {
    match kind {
        PipeKind::Gray => "Solid gray",
        PipeKind::Blue => "Solid blue",
        PipeKind::Pink => "Solid pink",
        PipeKind::Dash => "Dashed blue",
        PipeKind::Deny => "Dashed red",
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
    pipe_kind: Option<PipeKind>,
    align: TextAlign,
    source: NodePointer,
}

impl Builder {
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

    /// A measured text leaf. Uppercasing runs here, before measurement (section 2.9).
    fn text_leaf(
        &mut self,
        style: Style,
        parent: Option<NodeId>,
        spec: TextSpec<'_>,
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
            style_name: spec.style_name,
            color: text_color(spec.style_name, self.canvas, spec.pipe_kind),
            align: spec.align,
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
        kind: Option<&'static str>,
        parent: Option<usize>,
        taffy_node: NodeId,
    ) -> NodeRecord {
        NodeRecord {
            pointer,
            tag,
            kind,
            parent,
            taffy_node,
            content_node: taffy_node,
            parts: Vec::new(),
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
                pipe_kind: None,
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
                pipe_kind: None,
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
                pipe_kind: None,
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
                text: legend_label(entry.kind),
                style_name: TextStyleName::LegendLabel,
                pipe_kind: None,
                align: TextAlign::Start,
                source: pointer.child("kind"),
            },
        )?;
        let (text, text_leaf) = self.text_leaf(
            base_style(),
            Some(entry_node),
            TextSpec {
                text: &entry.text,
                style_name: TextStyleName::LegendText,
                pipe_kind: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        let mut record = Self::record(
            pointer,
            NodeTag::LegendEntry,
            Some(entry.kind.as_str()),
            Some(legend_index),
            entry_node,
        );
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
            Node::Zone(zone) => self.add_zone(
                zone,
                pointer,
                parent_index,
                parent_container,
                placement,
                depth,
            ),
            Node::Pcard(pcard) => {
                self.add_pcard(pcard, pointer, parent_index, parent_container, placement)
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
        let (direction, gap_size) = if container.tag == NodeTag::Row {
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
            let child_placement = if container.tag == NodeTag::Row {
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

    fn add_zone(
        &mut self,
        zone: &Zone,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
        depth: usize,
    ) -> Result<(), LayoutError> {
        let border = zone_border_px(zone.kind);
        let padding = zone_padding_px(zone.kind);
        let gap = if zone.kind == ZoneKind::Gcp { 0.0 } else { 8.0 };
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
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Zone,
            Some(zone.kind.as_str()),
            Some(parent_index),
            zone_node,
        );

        let children_container = if zone.kind == ZoneKind::Gcp {
            let bar_style = Style {
                padding: sides(7.0, 16.0, 7.0, 16.0),
                ..flex_column(AlignItems::STRETCH)
            };
            let bar = self.container(bar_style, zone_node, &pointer)?;
            let (label, label_leaf) = self.text_leaf(
                base_style(),
                Some(bar),
                TextSpec {
                    text: &zone.label,
                    style_name: TextStyleName::GcpBar,
                    pipe_kind: None,
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
            let style_name = if zone.kind == ZoneKind::Perimeter {
                TextStyleName::PerimeterLabel
            } else {
                TextStyleName::ZoneLabel
            };
            let (label, label_leaf) = self.text_leaf(
                base_style(),
                Some(zone_node),
                TextSpec {
                    text: &zone.label,
                    style_name,
                    pipe_kind: None,
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
        for (child_index, child) in zone.children.iter().enumerate() {
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

    fn add_pcard(
        &mut self,
        pcard: &Pcard,
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

        if pcard.icon.is_some() {
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
                text: &pcard.function_name,
                style_name: TextStyleName::CardFunction,
                pipe_kind: None,
                align: TextAlign::Start,
                source: pointer.child("fn"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::FunctionName,
            taffy_node: function_name,
            text_leaf: Some(function_leaf),
        });

        if let Some(product_name) = &pcard.product_name {
            let (product, product_leaf) = self.text_leaf(
                Style {
                    margin: margins(1.0, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                Some(column),
                TextSpec {
                    text: product_name,
                    style_name: TextStyleName::CardProduct,
                    pipe_kind: None,
                    align: TextAlign::Start,
                    source: pointer.child("pn"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::ProductName,
                taffy_node: product,
                text_leaf: Some(product_leaf),
            });
        }

        if let Some(fact) = &pcard.fact {
            self.add_chip(
                column,
                &mut parts,
                (PartName::FactBox, PartName::Fact),
                fact,
                TextStyleName::Fact,
                pointer.child("fact"),
            )?;
        }
        if let Some(ask) = &pcard.ask {
            let ask_text = format!("Ask: {ask}");
            self.add_chip(
                column,
                &mut parts,
                (PartName::AskBox, PartName::Ask),
                &ask_text,
                TextStyleName::Ask,
                pointer.child("ask"),
            )?;
        }

        let mut record = Self::record(pointer, NodeTag::Pcard, None, Some(parent_index), card);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// The Pcard fact and ask boxes: margin-top 4, padding 4/8, one text leaf.
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
                pipe_kind: None,
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
        let (leaf, leaf_index) = self.text_leaf(
            base_style(),
            Some(fact_node),
            TextSpec {
                text: &fact.text,
                style_name: TextStyleName::Fact,
                pipe_kind: None,
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
                pipe_kind: None,
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
        let wire_minimum = match (horizontal, pipe.kind) {
            (true, _) => 14.0,
            (false, PipeKind::Deny) => 16.0,
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
        let tag_style = Style {
            padding: sides(6.0, 8.0, 6.0, 8.0),
            border: sides(1.5, 1.5, 1.5, 1.5),
            flex_shrink: if horizontal { 1.0 } else { 0.0 },
            max_size: taffy::Size {
                width: LengthPercentageAuto::percent(1.0),
                height: auto(),
            },
            ..flex_column(AlignItems::CENTER)
        };
        let tag = self.container(tag_style, pipe_node, &pointer)?;
        let (label, label_leaf) = self.text_leaf(
            base_style(),
            Some(tag),
            TextSpec {
                text: &pipe.label,
                style_name: TextStyleName::TagLabel,
                pipe_kind: Some(pipe.kind),
                align: TextAlign::Center,
                source: pointer.child("label"),
            },
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
            let (sub_node, sub_leaf) = self.text_leaf(
                Style {
                    margin: margins(2.0, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                Some(tag),
                TextSpec {
                    text: sub,
                    style_name: TextStyleName::TagSub,
                    pipe_kind: Some(pipe.kind),
                    align: TextAlign::Center,
                    source: pointer.child("sub"),
                },
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
            Some(pipe.kind.as_str()),
            Some(parent_index),
            pipe_node,
        );
        record.parts = parts;
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
                pipe_kind: Some(tee.kind),
                align: TextAlign::Center,
                source: pointer.child("hub"),
            },
        )?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Tee,
            Some(tee.kind.as_str()),
            Some(parent_index),
            tee_node,
        );
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
}

struct FlexContainer<'a> {
    tag: NodeTag,
    gap: Option<u16>,
    grow: Option<&'a [u16]>,
    justify: Option<Justify>,
    children: &'a [Node],
}
