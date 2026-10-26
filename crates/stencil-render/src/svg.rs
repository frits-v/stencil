//! The section 5.2 SVG writer, and through `iso_writer` the section 12.5 one.

mod assembly;
mod iso_writer;
mod related;

use std::collections::{BTreeSet, HashMap};
use stencil_layout::{
    BoxRect, ContainerLook, DrawnLink, EndCap, LinkRoute, NodeGeometry, NodeTag, PageGeometry,
    Part, PartName, PathPiece, STOP_PLATE_THICKNESS_PX, TextAlign, TextRun, container_label_style,
    drawn_link, fact_presentation, marks::sampled_kinds,
};
use stencil_model::grammar::Role;
use stencil_model::pointer::NodePointer;
use stencil_model::text::{FontFamily, FontWeight, TextStyleName};

use base64::Engine as _;
use stencil_model::text::TextMeasurer;
use stencil_model::{
    Arrow, BandLook, Chrome, FactSource, IconName, LEGEND_ENTRIES_MAX, LINKS_MAX, LegendEntry,
    MarkKind, Node, NodeRef, NoteKind, Page, PageActors, PageMarks, PagePoint, Pipe, PipeDir,
    PipeForm, Projection, SheetList, StepFill, StepLook, Theme, body_nodes,
};
use stencil_text::{CosmicTextMeasurer, bundled_font};

use crate::icons::icon_data_uri;
use crate::palette::{self, BoxPaint, DotStyle, LineStyle, LineUse, Palette, PartShape, Stroke};
use crate::{HoverRule, HoverSource, RenderError, SvgDocument, format_number};

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
/// Flat radius of a pipe end dot; an arrowhead's tip sits this far out from the dot center.
pub(crate) const DOT_RADIUS_PX: f32 = 4.0;
const BADGE_RADIUS_PX: f32 = 4.0;
const CARD_RADIUS_PX: f32 = 8.0;
const FACT_RADIUS_PX: f32 = 4.0;
const TAG_RADIUS_PX: f32 = 6.0;
/// A legend sample's stop plate is shorter than a line's, so it stays inside the entry's line.
const LEGEND_STOP_PLATE_PX: f32 = 12.0;
/// The height of a legend swatch for a band form: a flat bar rather than a wire.
const BAND_SWATCH_HEIGHT_PX: f32 = 8.0;
const BAND_SWATCH_OUTLINE_PX: f32 = 1.5;
pub(crate) const ICON_CHIP_SIZE_PX: f32 = 36.0;
const ICON_CHIP_RADIUS_PX: f32 = 6.0;
/// Arrowhead triangle along the run axis and across it (section 11.2).
const ARROW_LENGTH_PX: f32 = stencil_layout::ARROWHEAD_LENGTH_PX;
const ARROW_WIDTH_PX: f32 = stencil_layout::ARROWHEAD_WIDTH_PX;
const FRAME_RADIUS_PX: f32 = 4.0;
const FRAME_CHIP_RADIUS_PX: f32 = 4.0;
/// Radius of the dot of a bulleted Text line, and its center's offset into the 22 px
/// marker cell.
const BULLET_RADIUS_PX: f32 = 2.0;
const BULLET_CENTER_INSET_PX: f32 = 5.0;
/// Corner radii of the gcp frame in CSS order: top-left, top-right, bottom-right, bottom-left.
const GCP_RADII_PX: [f32; 4] = [4.0, 4.0, 10.0, 10.0];
/// The pitch the drafting grid's minor lines keep near: each zone is cut into the whole
/// number of parts closest to its size over this.
const SHEET_GRID_PITCH_PX: f32 = 20.0;
/// The bound on one zone's minor parts, reached by a zone 5120 px long.
const SHEET_GRID_PARTS_MAX: f32 = 256.0;
/// The strokes of a minor grid line and of a major line on a zone edge.
const SHEET_GRID_MINOR_PX: f32 = 1.0;
const SHEET_GRID_ZONE_PX: f32 = 1.5;

/// The document side of one geometry node, in section 4.4 geometry order.
#[derive(Debug, Clone, Copy)]
enum DocumentNode<'a> {
    Page,
    Kicker,
    Title,
    Lede,
    Body,
    Content(NodeRef<'a>),
    Legend,
    LegendEntry(&'a LegendEntry),
    Foot,
    /// A design sheet's title block, `/sheet`.
    Sheet,
    /// The problem, answer or attention cell of the title block.
    SheetCell,
    /// A point of the problem cell.
    ProblemPoint,
    /// An actor's chip in the legend strip.
    Actor,
    /// A rung of a ladder, `/…/rungs/<i>`, with the fill its box takes.
    Rung(&'a stencil_model::Rung, StepFill),
    /// One how-to of a HowTo, `/…/items/<i>`.
    HowToItem(&'a stencil_model::HowToItem),
}

/// Walks page and geometry in the section 4.4 geometry order and asserts the pointers match,
/// painting every role from `theme`.
pub fn render_svg(
    page: &Page,
    theme: &Theme,
    geometry: &PageGeometry,
) -> Result<SvgDocument, RenderError> {
    if page.projection == Projection::Iso
        && let Some(pointer) = sheet_surface_in(geometry)
    {
        return Err(RenderError::SheetSurface { pointer });
    }
    let expected = geometry_order(page);
    if let Some(mismatch) = first_mismatch(&expected, geometry) {
        return Err(mismatch);
    }
    if page.projection == Projection::Iso {
        return iso_writer::render_iso(page, theme, geometry, &expected);
    }

    let mut writer = SvgWriter::new(Palette::new(theme, Projection::Flat));
    writer.sheet = page.sheet.as_ref().map(|sheet| SheetLook {
        wordmark: sheet.wordmark.is_some(),
    });
    writer.mark_paints = MarkPaints::of(page);
    writer.actor_slots = page
        .actors
        .iter()
        .take(SheetList::Actors.max() + 1)
        .map(|actor| (actor.id.clone(), actor.tint))
        .collect();
    let mut relabeler = Relabeler::default();
    let width = format_number(geometry.canvas.width);
    let height = format_number(geometry.canvas.height);
    writer.line(
        0,
        &format!(
            r#"<svg xmlns="{SVG_NAMESPACE}" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
        ),
    );
    writer.line(
        1,
        &format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            writer.palette.page_background()
        ),
    );
    let arrow_kinds = arrow_kinds(page, geometry, &expected);
    if !arrow_kinds.is_empty() {
        writer.write_arrow_markers(1, &arrow_kinds);
    }

    // Geometry order is pre-order, so a stack of open groups reproduces the nesting.
    let highlights = related::highlights(page, geometry);
    let mut open_groups: Vec<usize> = Vec::new();
    for (index, (node, (_, document_node))) in geometry.nodes.iter().zip(&expected).enumerate() {
        close_groups_until_parent(&mut writer, &mut open_groups, geometry, index)?;
        let depth = open_groups.len() + 1;
        writer.line(depth, &group_open_tag(node, &highlights, index));
        let relabeled = relabeler.relabeled(writer.palette, node, *document_node)?;
        writer.part_keys = highlights.part_keys.get(index).cloned().unwrap_or_default();
        writer.write_node(
            depth + 1,
            relabeled.as_ref().unwrap_or(node),
            *document_node,
        )?;
        writer.part_keys.clear();
        open_groups.push(index);
    }
    for depth in (1..=open_groups.len()).rev() {
        writer.line(depth, "</g>");
    }
    // Links paint after every node, so zone fills never cover them (section 11.2).
    for route in geometry.links.iter().take(LINKS_MAX) {
        if page.links.get(route.index).is_none() {
            return Err(link_mismatch(route));
        }
        writer.part_keys = highlights
            .link_part_keys
            .get(&route.index)
            .cloned()
            .unwrap_or_default();
        writer.write_link(1, geometry, route, page.links.get(route.index))?;
        writer.part_keys.clear();
    }
    writer.line(0, "</svg>");
    writer.hover_rules = highlights.rules;

    Ok(writer.into_document())
}

/// Where a geometry holds a design sheet surface, which the iso writer refuses (section 14.2
/// rule 11): `/sheet` when it holds a title block, else the first node in geometry order
/// with a sheet tag or a sheet part, else the first link whose route draws a sheet part.
fn sheet_surface_in(geometry: &PageGeometry) -> Option<NodePointer> {
    if geometry
        .nodes
        .iter()
        .any(|node| node.tag == NodeTag::TitleBlock)
    {
        return Some(NodePointer::root().child("sheet"));
    }
    geometry
        .nodes
        .iter()
        .find(|node| is_sheet_tag(node.tag) || node.parts.iter().any(|part| part.name.is_sheet()))
        .map(|node| node.pointer.clone())
        .or_else(|| {
            // A step badge, a valve, a rail dot or a hop is a part of a link's route.
            geometry
                .links
                .iter()
                .find(|route| route.parts.iter().any(|part| part.name.is_sheet()))
                .map(|route| NodePointer::root().child("links").index(route.index))
        })
}

/// True for the node tags a design sheet adds.
pub(crate) fn is_sheet_tag(tag: NodeTag) -> bool {
    match tag {
        NodeTag::TitleBlock
        | NodeTag::TitleCell
        | NodeTag::ProblemPoint
        | NodeTag::View
        | NodeTag::ActorChip
        | NodeTag::Phases
        | NodeTag::Step
        | NodeTag::PartsList
        | NodeTag::GeneralNotes
        | NodeTag::Ladder
        | NodeTag::Rung
        | NodeTag::HowTo
        | NodeTag::HowToItem => true,
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
        | NodeTag::Lanes
        | NodeTag::Zone
        | NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Pipe
        | NodeTag::Tee
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame => false,
    }
}

/// Pointers and document nodes in section 4.4 geometry order. The body portion is
/// `body_nodes`, which is bounded by NODES_MAX + 1, and the legend portion stops after
/// LEGEND_ENTRIES_MAX + 1 entries; a longer geometry then fails as a mismatch.
fn geometry_order(page: &Page) -> Vec<(NodePointer, DocumentNode<'_>)> {
    let root = NodePointer::root();
    let mut order = vec![(root.clone(), DocumentNode::Page)];
    if let Some(sheet) = &page.sheet {
        let sheet_pointer = root.child("sheet");
        order.push((sheet_pointer.clone(), DocumentNode::Sheet));
        order.extend([
            (root.child("kicker"), DocumentNode::Kicker),
            (root.child("title"), DocumentNode::Title),
            (root.child("lede"), DocumentNode::Lede),
        ]);
        let problem = sheet_pointer.child("problem");
        order.push((problem.clone(), DocumentNode::SheetCell));
        let points = problem.child("points");
        order.extend(
            (0..sheet
                .problem
                .points
                .len()
                .min(SheetList::ProblemPoints.max() + 1))
                .map(|index| (points.index(index), DocumentNode::ProblemPoint)),
        );
        order.push((sheet_pointer.child("answer"), DocumentNode::SheetCell));
        order.push((sheet_pointer.child("attention"), DocumentNode::SheetCell));
        push_legend_order(page, &mut order);
        order.push((root.child("body"), DocumentNode::Body));
        for entry in body_nodes(page) {
            let rungs = match entry.node {
                NodeRef::Node(Node::Ladder(ladder)) => {
                    let list = SheetList::Rungs;
                    ladder
                        .rungs
                        .iter()
                        .take(list.max() + 1)
                        .enumerate()
                        .map(|(index, rung)| {
                            (
                                list.pointer_in(&entry.pointer).index(index),
                                DocumentNode::Rung(rung, ladder.fill_of(rung)),
                            )
                        })
                        .collect()
                }
                NodeRef::Node(Node::HowTo(how_to)) => {
                    let list = SheetList::HowToItems;
                    how_to
                        .items
                        .iter()
                        .take(list.max() + 1)
                        .enumerate()
                        .map(|(index, item)| {
                            (
                                list.pointer_in(&entry.pointer).index(index),
                                DocumentNode::HowToItem(item),
                            )
                        })
                        .collect()
                }
                _ => Vec::new(),
            };
            order.push((entry.pointer, DocumentNode::Content(entry.node)));
            order.extend(rungs);
        }
        if page.foot.is_some() {
            order.push((root.child("foot"), DocumentNode::Foot));
        }
        return order;
    }
    // A captioned figure lays out no badge, kicker, title or lede (section 13.7).
    if page.chrome == Chrome::Full {
        order.extend([
            (root.child("kicker"), DocumentNode::Kicker),
            (root.child("title"), DocumentNode::Title),
            (root.child("lede"), DocumentNode::Lede),
        ]);
    }
    order.push((root.child("body"), DocumentNode::Body));
    order.extend(
        body_nodes(page)
            .into_iter()
            .map(|entry| (entry.pointer, DocumentNode::Content(entry.node))),
    );
    push_legend_order(page, &mut order);
    if page.foot.is_some() {
        order.push((root.child("foot"), DocumentNode::Foot));
    }
    order
}

fn push_legend_order<'a>(page: &'a Page, order: &mut Vec<(NodePointer, DocumentNode<'a>)>) {
    // Only a sheet has actors, whose chips open its legend strip.
    if page.legend.is_empty() && page.actors.is_empty() {
        return;
    }
    let legend_pointer = NodePointer::root().child("legend");
    order.push((legend_pointer.clone(), DocumentNode::Legend));
    let actors_pointer = SheetList::Actors.pointer();
    order.extend(
        page.actors
            .iter()
            .enumerate()
            .take(SheetList::Actors.max() + 1)
            .map(|(index, _)| (actors_pointer.index(index), DocumentNode::Actor)),
    );
    order.extend(
        page.legend
            .iter()
            .enumerate()
            .take(LEGEND_ENTRIES_MAX + 1)
            .map(|(index, entry)| {
                (
                    legend_pointer.index(index),
                    DocumentNode::LegendEntry(entry),
                )
            }),
    );
}

/// Lines of the Pipes, Tee arms and links that draw an arrowhead, one marker each, by their
/// effective tint, which an actor gives on a sheet. A deny line draws a stop plate instead. `expected` and the geometry are in the
/// same order, which the caller has checked.
fn arrow_kinds(
    page: &Page,
    geometry: &PageGeometry,
    expected: &[(NodePointer, DocumentNode<'_>)],
) -> BTreeSet<LineUse> {
    let pipe_kinds = expected
        .iter()
        .zip(&geometry.nodes)
        .filter_map(|((_, document_node), node)| match *document_node {
            DocumentNode::Content(NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe)) => {
                Some((pipe, node.tint))
            }
            _ => None,
        })
        .filter(|(pipe, _)| pipe.arrow != Arrow::None)
        .map(|(pipe, tint)| LineUse::new(pipe.line, tint));
    let link_kinds = geometry
        .links
        .iter()
        .take(LINKS_MAX)
        .filter(|route| {
            page.links
                .get(route.index)
                .is_some_and(|link| link.arrow != Arrow::None)
        })
        .map(|route| LineUse::new(route.line, route.tint));
    pipe_kinds
        .chain(link_kinds)
        .filter(|kind| EndCap::of(kind.line) == EndCap::Arrowhead)
        .collect()
}

/// A route whose index names no `Page.links` entry, or whose tag parts do not match.
fn link_mismatch(route: &LinkRoute) -> RenderError {
    let pointer = NodePointer::root().child("links").index(route.index);
    RenderError::GeometryMismatch {
        expected: pointer.clone(),
        found: pointer.child("<absent>"),
    }
}

/// Which ends of a pipe carry an arrowhead instead of a dot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ArrowEnds {
    start: bool,
    end: bool,
}

impl ArrowEnds {
    fn of(pipe: &Pipe) -> Self {
        Self::from_arrow(pipe.arrow)
    }

    fn from_arrow(arrow: Arrow) -> Self {
        match arrow {
            Arrow::None => ArrowEnds::default(),
            Arrow::Start => ArrowEnds {
                start: true,
                end: false,
            },
            Arrow::End => ArrowEnds {
                start: false,
                end: true,
            },
            Arrow::Both => ArrowEnds {
                start: true,
                end: true,
            },
        }
    }
}

/// The (base, tip) of the arrowhead or stop plate that replaces a pipe's end dot: the tip on
/// the dot box's outer edge, the base an arrowhead's length back along the run.
fn arrowhead_ends(dot_bounds: BoxRect, dir: PipeDir, at_start: bool) -> (PagePoint, PagePoint) {
    let center_x = dot_bounds.x + dot_bounds.width / 2.0;
    let center_y = dot_bounds.y + dot_bounds.height / 2.0;
    let at = |x: f32, y: f32| PagePoint { x, y };
    match (dir, at_start) {
        (PipeDir::Horizontal, true) => (
            at(dot_bounds.x + ARROW_LENGTH_PX, center_y),
            at(dot_bounds.x, center_y),
        ),
        (PipeDir::Horizontal, false) => (
            at(dot_bounds.right() - ARROW_LENGTH_PX, center_y),
            at(dot_bounds.right(), center_y),
        ),
        (PipeDir::Vertical, true) => (
            at(center_x, dot_bounds.y + ARROW_LENGTH_PX),
            at(center_x, dot_bounds.y),
        ),
        (PipeDir::Vertical, false) => (
            at(center_x, dot_bounds.bottom() - ARROW_LENGTH_PX),
            at(center_x, dot_bounds.bottom()),
        ),
    }
}

/// The stop plate's inner face and tip on the run from `base` to `tip`.
fn stop_plate_ends(base: PagePoint, tip: PagePoint) -> (PagePoint, PagePoint) {
    let (delta_x, delta_y) = (tip.x - base.x, tip.y - base.y);
    let length = delta_x.hypot(delta_y);
    if length <= f32::EPSILON {
        return (tip, tip);
    }
    let back = STOP_PLATE_THICKNESS_PX / length;
    (
        PagePoint {
            x: tip.x - delta_x * back,
            y: tip.y - delta_y * back,
        },
        tip,
    )
}

/// Pointer used in a mismatch when one side has no node at that position.
fn absent_pointer() -> NodePointer {
    NodePointer::root().child("<absent>")
}

fn first_mismatch(
    expected: &[(NodePointer, DocumentNode<'_>)],
    geometry: &PageGeometry,
) -> Option<RenderError> {
    let pair_mismatch = expected
        .iter()
        .zip(&geometry.nodes)
        .find(|((pointer, _), node)| *pointer != node.pointer)
        .map(|((pointer, _), node)| RenderError::GeometryMismatch {
            expected: pointer.clone(),
            found: node.pointer.clone(),
        });
    if pair_mismatch.is_some() {
        return pair_mismatch;
    }
    let shared = expected.len().min(geometry.nodes.len());
    if let Some((pointer, _)) = expected.get(shared) {
        return Some(RenderError::GeometryMismatch {
            expected: pointer.clone(),
            found: absent_pointer(),
        });
    }
    geometry
        .nodes
        .get(shared)
        .map(|node| RenderError::GeometryMismatch {
            expected: absent_pointer(),
            found: node.pointer.clone(),
        })
}

/// Closes open groups until the top of the stack is the parent of `index`. The root is the
/// only node without a parent and must come first.
fn close_groups_until_parent(
    writer: &mut SvgWriter,
    open_groups: &mut Vec<usize>,
    geometry: &PageGeometry,
    index: usize,
) -> Result<(), RenderError> {
    let Some(node) = geometry.nodes.get(index) else {
        return Ok(());
    };
    let Some(parent) = node.parent else {
        if open_groups.is_empty() {
            return Ok(());
        }
        return Err(parent_mismatch(geometry, index));
    };
    while let Some(&top) = open_groups.last() {
        if top == parent {
            return Ok(());
        }
        open_groups.pop();
        writer.line(open_groups.len() + 1, "</g>");
    }
    Err(parent_mismatch(geometry, index))
}

fn parent_mismatch(geometry: &PageGeometry, index: usize) -> RenderError {
    let found = geometry
        .nodes
        .get(index)
        .map_or_else(absent_pointer, |node| node.pointer.clone());
    let expected = geometry
        .nodes
        .get(index)
        .and_then(|node| node.parent)
        .and_then(|parent| geometry.nodes.get(parent))
        .map_or_else(absent_pointer, |parent| parent.pointer.clone());
    RenderError::GeometryMismatch { expected, found }
}

/// A node's group: its `id` when it fades, `data-id`, `data-tag`, `data-kind`, and for a
/// Pipe, Tee arm or Tee the pointers of the nodes its ends meet.
fn group_open_tag(node: &NodeGeometry, highlights: &related::Highlights, index: usize) -> String {
    let anchor = highlights
        .anchors
        .get(index)
        .cloned()
        .flatten()
        .map(|anchor| format!(r#" id="{}""#, escape_xml(&anchor)))
        .unwrap_or_default();
    let kind = node
        .kind
        .as_deref()
        .map(|kind| format!(r#" data-kind="{}""#, escape_xml(kind)))
        .unwrap_or_default();
    let mut ends = String::new();
    if let Some((from, to)) = highlights.ends.get(index) {
        for (name, end) in [("data-from", from), ("data-to", to)] {
            if let Some(end) = end {
                ends.push_str(&format!(r#" {name}="{}""#, escape_xml(end)));
            }
        }
    }
    format!(
        r#"<g{anchor} data-id="{}" data-tag="{}"{kind}{ends}>"#,
        escape_xml(node.pointer.as_str()),
        node.tag.as_str()
    )
}

/// A link's group: its `id`, and the pointer of the node at each end in `data-from` and
/// `data-to`.
fn link_group_open_tag(geometry: &PageGeometry, route: &LinkRoute) -> Result<String, RenderError> {
    let end_pointer = |index: usize| {
        geometry
            .nodes
            .get(index)
            .map(|node| escape_xml(node.pointer.as_str()))
            .ok_or_else(|| link_mismatch(route))
    };
    let pointer = NodePointer::root().child("links").index(route.index);
    Ok(format!(
        r#"<g id="{}" data-id="{}" data-tag="Link" data-kind="{}" data-from="{}" data-to="{}">"#,
        escape_xml(&related::pointer_anchor(pointer.as_str())),
        escape_xml(pointer.as_str()),
        route.key(),
        end_pointer(route.from_node)?,
        end_pointer(route.to_node)?
    ))
}

/// Draws each legend entry's label as the theme names it (section 13.4 rule 9), measuring
/// the drawn label with the bundled fonts, which load on the first relabeled entry.
#[derive(Default)]
struct Relabeler {
    measurer: Option<CosmicTextMeasurer>,
}

impl Relabeler {
    /// The legend entry with its drawn label, or None when the node is not a legend entry
    /// or its drawn label equals the canonical one.
    fn relabeled(
        &mut self,
        palette: Palette<'_>,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
    ) -> Result<Option<NodeGeometry>, RenderError> {
        let DocumentNode::LegendEntry(entry) = document_node else {
            return Ok(None);
        };
        let Some(label) = palette.legend_entry_relabel(entry) else {
            return Ok(None);
        };
        let measurer = match self.measurer.as_mut() {
            Some(measurer) => measurer,
            None => self.measurer.insert(CosmicTextMeasurer::new()?),
        };
        relabel_legend_entry(node, &label, measurer).map(Some)
    }
}

/// A legend entry whose label run reads `label`, measured in the run's style, with the
/// description moved by the change in label width so the gap between them keeps its size.
fn relabel_legend_entry(
    entry: &NodeGeometry,
    label: &str,
    measurer: &mut dyn TextMeasurer,
) -> Result<NodeGeometry, RenderError> {
    let mut relabeled = entry.clone();
    let mut width_change = 0.0;
    for part in &mut relabeled.parts {
        if part.name == PartName::LegendLabel
            && let Some(run) = &part.text
        {
            let metrics =
                measurer
                    .measure(label, &run.style, None)
                    .map_err(|error| RenderError::Svg {
                        message: format!(
                            "legend label {label:?} at {} could not be measured: {error}",
                            entry.pointer
                        ),
                    })?;
            width_change = metrics.width_px - run.metrics.width_px;
            part.bounds.width += width_change;
            part.text = Some(TextRun {
                text: label.to_string(),
                metrics,
                ..run.clone()
            });
        }
    }
    for part in &mut relabeled.parts {
        if part.name == PartName::LegendText {
            part.bounds.x += width_change;
        }
    }
    Ok(relabeled)
}

/// Document facts a part needs that geometry does not carry.
#[derive(Debug, Clone, Copy, Default)]
struct PartContext<'a> {
    line: Option<LineUse>,
    pipe_dir: Option<PipeDir>,
    pipe_form: Option<PipeForm>,
    arrows: ArrowEnds,
    icon: Option<IconName>,
    /// Accent bar color of a Callout.
    accent: Option<&'a str>,
    /// How a Step is drawn, and what its box is filled with.
    step_look: Option<StepLook>,
    step_fill: Option<StepFill>,
    /// What a Phases node draws its lanes on.
    band: Option<BandLook>,
    /// A legend entry that names every actor's use of its line.
    by_actor: bool,
    /// An outlined Item's outline on a sheet; None for a card.
    outline: Option<stencil_model::Outline>,
    /// A planned Item, Box, Pipe or legend entry.
    planned: bool,
    /// A legend entry of the rail form.
    rail: bool,
}

struct SvgWriter<'a> {
    output: String,
    text_elements: usize,
    /// The family and weight of every `<text>` written, each once.
    font_faces: Vec<(FontFamily, FontWeight)>,
    palette: Palette<'a>,
    /// Section 12.2 rule 7: the zoom of an iso scene, which tube radii follow; 1 in flat.
    iso_zoom: f32,
    /// Set on a design sheet, whose chrome and Boxes paint from the sheet role.
    sheet: Option<SheetLook>,
    /// Each actor's tint slot, by actor id.
    actor_slots: Vec<(String, u8)>,
    /// The page's `Grid` part, whose lines a washed lane band draws again over itself.
    sheet_grid: Option<BoxRect>,
    /// The `step_fill` of the Phases node last written; geometry order puts each Step
    /// after its own Phases node and before the next one.
    phases_step_fill: Option<StepFill>,
    /// The kind and key colour of every glyph.
    mark_paints: MarkPaints,
    /// The rules the written SVG highlights with, set once every link is written.
    hover_rules: Vec<HoverRule>,
    /// The key of each part of the node or link tag being written, aligned with its parts;
    /// empty off a design sheet.
    part_keys: Vec<Option<related::ElementKey>>,
    /// The attribute every element written at `element_depth` carries while a part with
    /// a key is written.
    element_attribute: Option<String>,
    element_depth: usize,
    /// The rows whose first element already carries the row's `id`.
    anchored_rows: BTreeSet<String>,
    /// The row of the part being written; its first element written carries the row's
    /// `id`.
    pending_row: Option<String>,
}

/// The kind and key actor slot of every glyph the page draws, read from the document
/// before writing: a glyph's colours follow its key's actor, not its carrier's.
#[derive(Debug, Clone, Default)]
struct MarkPaints {
    /// Carrier pointer to its glyphs in drawn order.
    carriers: HashMap<String, Vec<(MarkKind, Option<u8>)>>,
    /// Link index to the glyphs in its tag.
    links: HashMap<usize, Vec<(MarkKind, Option<u8>)>>,
    /// The actor slot of each part and each attention item, in list order.
    parts: Vec<Option<u8>>,
    attention: Vec<Option<u8>>,
    /// The kinds the legend strip samples, in order.
    samples: Vec<MarkKind>,
}

impl MarkPaints {
    fn of(page: &Page) -> Self {
        let page_marks = PageMarks::of(page);
        let actors = PageActors::of(page);
        let paint = |marks: &[stencil_model::CarrierMark]| -> Vec<(MarkKind, Option<u8>)> {
            marks
                .iter()
                .map(|mark| {
                    (
                        mark.mark.kind(),
                        actors.slot(page_marks.key_actor(&mark.mark)),
                    )
                })
                .collect()
        };
        let mut carriers = HashMap::new();
        if let Some(sheet) = &page.sheet {
            for index in 0..sheet
                .problem
                .points
                .len()
                .min(SheetList::ProblemPoints.max() + 1)
            {
                let pointer = SheetList::ProblemPoints.pointer().index(index);
                carriers.insert(
                    pointer.as_str().to_string(),
                    paint(&page_marks.point_marks(page, index)),
                );
            }
        }
        for entry in body_nodes(page) {
            let marks = page_marks.node_marks(&entry);
            if !marks.is_empty() {
                carriers.insert(entry.pointer.as_str().to_string(), paint(&marks));
            }
            if let NodeRef::Node(Node::Ladder(ladder)) = entry.node {
                let rungs = SheetList::Rungs;
                for index in 0..ladder.rungs.len().min(rungs.max() + 1) {
                    let marks = PageMarks::rung_marks(ladder, &entry.pointer, index);
                    if !marks.is_empty() {
                        carriers.insert(
                            rungs
                                .pointer_in(&entry.pointer)
                                .index(index)
                                .as_str()
                                .to_string(),
                            paint(&marks),
                        );
                    }
                }
            }
        }
        let links = page
            .links
            .iter()
            .enumerate()
            .take(LINKS_MAX + 1)
            .map(|(index, link)| (index, paint(&PageMarks::link_marks(link, index))))
            .collect();
        let parts = page_marks
            .parts()
            .iter()
            .map(|part| actors.slot(page_marks.part_actor(part)))
            .collect();
        let attention = page
            .sheet
            .iter()
            .flat_map(|sheet| sheet.attention.items.iter())
            .take(SheetList::AttentionItems.max() + 1)
            .map(|item| actors.slot(item.actor.as_deref()))
            .collect();
        MarkPaints {
            carriers,
            links,
            parts,
            attention,
            samples: sampled_kinds(page),
        }
    }
}

/// What the writer reads of a page's sheet.
#[derive(Debug, Clone, Copy)]
struct SheetLook {
    /// A wordmark sets the title in `motto`; without one the title is `sheet_title`.
    wordmark: bool,
}

impl<'a> SvgWriter<'a> {
    fn new(palette: Palette<'a>) -> Self {
        SvgWriter {
            output: String::new(),
            text_elements: 0,
            font_faces: Vec::new(),
            palette,
            iso_zoom: 1.0,
            sheet: None,
            actor_slots: Vec::new(),
            sheet_grid: None,
            phases_step_fill: None,
            mark_paints: MarkPaints::default(),
            hover_rules: Vec::new(),
            part_keys: Vec::new(),
            element_attribute: None,
            element_depth: 0,
            anchored_rows: BTreeSet::new(),
            pending_row: None,
        }
    }

    fn line(&mut self, depth: usize, content: &str) {
        for _ in 0..depth {
            self.output.push_str("  ");
        }
        let keyed = depth == self.element_depth && !content.starts_with("</");
        match (&self.element_attribute, keyed) {
            (Some(attribute), true) => {
                let name_length = content.find([' ', '>', '/']).unwrap_or(content.len());
                let (name, rest) = content.split_at(name_length);
                self.output.push_str(name);
                if let Some(row) = self.pending_row.take()
                    && self.anchored_rows.insert(row.clone())
                {
                    let anchor = related::pointer_anchor(&row);
                    self.output
                        .push_str(&format!(r#" id="{}""#, escape_xml(&anchor)));
                }
                self.output.push_str(attribute);
                self.output.push_str(rest);
            }
            _ => self.output.push_str(content),
        }
        self.output.push('\n');
    }

    /// Every element the part at `index` of `part_keys` writes at `depth` carries its key
    /// until `end_part`: `data-row` for a row, with the row's `id` on its first element,
    /// or `data-mark` for a glyph and its numeral.
    fn begin_part(&mut self, index: usize, depth: usize) {
        let Some(Some(key)) = self.part_keys.get(index) else {
            return;
        };
        let attribute = match key {
            related::ElementKey::Row(row) => {
                self.pending_row = Some(row.clone());
                format!(r#" data-row="{}""#, escape_xml(row))
            }
            related::ElementKey::Mark(row) => format!(r#" data-mark="{}""#, escape_xml(row)),
        };
        self.element_attribute = Some(attribute);
        self.element_depth = depth;
    }

    fn end_part(&mut self) {
        self.element_attribute = None;
        self.pending_row = None;
    }

    /// The written SVG, with its faces ordered by family name and then weight.
    fn into_document(self) -> SvgDocument {
        let mut font_faces = self.font_faces;
        font_faces.sort_by_key(|(family, weight)| (family.css_name(), weight.css_value()));
        SvgDocument {
            svg: self.output,
            text_elements: self.text_elements,
            font_faces,
            hover_rules: self.hover_rules,
        }
    }

    /// Shapes first, then the node's text; child groups follow from the caller.
    fn write_node(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
    ) -> Result<(), RenderError> {
        let context = self.write_node_box(depth, node, document_node)?;
        let slots = self.part_slots(node, document_node);
        for (index, (part, slot)) in node.parts.iter().zip(&slots).enumerate() {
            self.begin_part(index, depth);
            self.write_part_shape(depth, node, part, (context, *slot))?;
            self.end_part();
        }
        if let DocumentNode::Content(NodeRef::Node(Node::Box(_))) = document_node
            && let Some(look) = node.container
            && look.role == Role::Frame
        {
            self.write_gcp_frame_stroke(depth, node.bounds, look, node.tint);
        }
        let glyph_kinds = self.glyph_kinds(node);
        let mut numerals = 0;
        for (index, (part, slot)) in node.parts.iter().zip(&slots).enumerate() {
            if let Some(run) = &part.text {
                let fill = if matches!(
                    part.name,
                    PartName::MarkNumeral | PartName::MarkSampleNumeral
                ) {
                    let kind = glyph_kinds.get(numerals).copied().flatten();
                    numerals += 1;
                    let kind = kind.ok_or_else(|| part_mismatch(node, part))?;
                    self.palette.sheet_color(palette::numeral_ink(kind), None)
                } else {
                    self.text_fill(node, document_node, part, (context, *slot))?
                };
                self.begin_part(index, depth);
                self.write_text_run(depth, &node.pointer, part.bounds, run, &fill)?;
                self.end_part();
            }
        }
        Ok(())
    }

    /// The kind of each carrier glyph or legend sample of a node, in part order; a numeral
    /// takes the kind of the glyph it is drawn in.
    fn glyph_kinds(&self, node: &NodeGeometry) -> Vec<Option<MarkKind>> {
        let paints = self.mark_paints.carriers.get(node.pointer.as_str());
        let mut glyphs = 0;
        let mut samples = 0;
        let mut kinds = Vec::new();
        for part in &node.parts {
            if part.name.is_mark_glyph() {
                kinds.push(
                    paints
                        .and_then(|paints| paints.get(glyphs))
                        .map(|paint| paint.0),
                );
                glyphs += 1;
            } else if part.name == PartName::MarkSample {
                kinds.push(self.mark_paints.samples.get(samples).copied());
                samples += 1;
            }
        }
        kinds
    }

    /// The tint slot of an actor.
    fn actor_slot(&self, actor: &str) -> Option<u8> {
        self.actor_slots
            .iter()
            .find(|(id, _)| id == actor)
            .map(|(_, slot)| *slot)
    }

    /// The tint slot each part paints with: a lane's part takes its lane's actor, every
    /// other part its node's effective tint.
    fn part_slots(&self, node: &NodeGeometry, document_node: DocumentNode<'_>) -> Vec<Option<u8>> {
        let lanes: Vec<Option<u8>> = match document_node {
            DocumentNode::Content(NodeRef::Node(Node::Phases(phases))) => phases
                .lanes
                .iter()
                .map(|lane| self.actor_slot(lane))
                .collect(),
            _ => Vec::new(),
        };
        let mut seen: Vec<(PartName, usize)> = Vec::new();
        let glyph_slots = self.mark_paints.carriers.get(node.pointer.as_str());
        let mut glyph_index = 0;
        node.parts
            .iter()
            .map(|part| {
                let nth = match seen.iter_mut().find(|(name, _)| *name == part.name) {
                    Some((_, count)) => {
                        *count += 1;
                        *count - 1
                    }
                    None => {
                        seen.push((part.name, 1));
                        0
                    }
                };
                match part.name {
                    PartName::LaneBand
                    | PartName::LaneBar
                    | PartName::LaneName
                    | PartName::LaneText => lanes.get(nth).copied().flatten(),
                    PartName::Balloon | PartName::Flag | PartName::Diamond => {
                        let glyph = glyph_index;
                        glyph_index += 1;
                        glyph_slots
                            .and_then(|paints| paints.get(glyph))
                            .and_then(|paint| paint.1)
                    }
                    PartName::PartBalloon | PartName::PartTitle => {
                        self.mark_paints.parts.get(nth).copied().flatten()
                    }
                    PartName::AttentionDiamond => {
                        self.mark_paints.attention.get(nth).copied().flatten()
                    }
                    PartName::MarkSample
                    | PartName::MarkNumeral
                    | PartName::MarkSampleNumeral
                    | PartName::PartNumeral
                    | PartName::AttentionNumeral => None,
                    _ => node.tint,
                }
            })
            .collect()
    }

    /// The ink of a text part: a Box label by its container's paint, every other run by
    /// its section 2.9 style.
    fn text_fill(
        &self,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        part: &Part,
        (context, slot): (PartContext<'a>, Option<u8>),
    ) -> Result<String, RenderError> {
        if let DocumentNode::Content(NodeRef::Node(Node::Box(_))) = document_node
            && part.name == PartName::Label
        {
            let look = node.container.ok_or_else(|| container_mismatch(node))?;
            return Ok(match self.sheet {
                Some(_) => self.palette.sheet_container_label_ink(look, node.tint),
                None => self.palette.container_label_ink(look, node.tint),
            }
            .to_string());
        }
        // On the actor's fill a Step's lines take the actor's ink, the pair the theme's
        // `tints.<n>.ink on tints.<n>.fill` row holds.
        if context.step_fill == Some(StepFill::Tint)
            && context.step_look != Some(StepLook::Text)
            && part.name == PartName::StepLine
        {
            return Ok(self.palette.sheet_color(palette::SheetInk::TintInk, slot));
        }
        // A text-look Step is an aside: its title takes the secondary ink.
        if context.step_look == Some(StepLook::Text) && part.name == PartName::Text {
            return Ok(self
                .palette
                .sheet_ink(palette::SheetInk::InkSecondary)
                .to_string());
        }
        // An actor's Item names its work in the actor's ink (section 14.5 rule 2).
        if self.sheet.is_some()
            && node.tag == NodeTag::Pcard
            && part.name == PartName::FunctionName
            && let Some(slot) = node.tint
        {
            return Ok(self
                .palette
                .sheet_color(palette::SheetInk::TintInk, Some(slot)));
        }
        if let DocumentNode::HowToItem(item) = document_node
            && matches!(part.name, PartName::TerminalLine | PartName::TerminalPrompt)
        {
            let line = node
                .parts
                .iter()
                .take_while(|candidate| !std::ptr::eq(*candidate, part))
                .filter(|candidate| candidate.name == PartName::TerminalLine)
                .count();
            let kind = item
                .terminal
                .get(line)
                .map(|line| line.kind)
                .ok_or_else(|| part_mismatch(node, part))?;
            let ink = palette::terminal_ink(kind, part.name == PartName::TerminalPrompt);
            return Ok(self.palette.sheet_ink(ink).to_string());
        }
        if self.sheet.is_some()
            && let Some(ink) =
                palette::sheet_part_paint(node.tag, part.name).and_then(|paint| paint.text)
        {
            return Ok(self.palette.sheet_color(ink, slot));
        }
        let style_name = match (self.sheet, document_node, part.name) {
            (Some(sheet), DocumentNode::Title, PartName::Text) => {
                if sheet.wordmark {
                    TextStyleName::Motto
                } else {
                    TextStyleName::SheetTitle
                }
            }
            (Some(_), DocumentNode::Lede, PartName::Text) => TextStyleName::BlockBody,
            _ => text_style_name(document_node, node.container, part.name)
                .ok_or_else(|| part_mismatch(node, part))?,
        };
        Ok(self
            .palette
            .text_ink(style_name, context.line.map(|line_use| line_use.line))
            .to_string())
    }

    /// Draws the box of the node itself, below its parts, and returns what the parts need.
    fn write_node_box(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
    ) -> Result<PartContext<'a>, RenderError> {
        if let DocumentNode::Content(NodeRef::Node(content)) = document_node {
            match content {
                Node::Text(_) => {
                    let block_paint = self.palette.block();
                    self.write_box(depth, node.bounds, CARD_RADIUS_PX, block_paint);
                }
                Node::Callout(callout) => {
                    let paint = self.palette.callout(callout.kind);
                    self.write_rect(
                        depth,
                        node.bounds,
                        CARD_RADIUS_PX,
                        Some(paint.fill),
                        Some(paint.border),
                    );
                }
                Node::Frame(_) => self.write_frame_box(depth, node.bounds),
                Node::Box(box_node) => {
                    let look = node.container.ok_or_else(|| container_mismatch(node))?;
                    let style = self.zone_style(look, node.tint);
                    match (self.sheet, box_node.planned, style.border) {
                        // Planned work on a sheet: its border dashed, no fill.
                        (Some(_), true, Some(border)) => {
                            self.write_planned_box(depth, node.bounds, border.color);
                        }
                        _ => self.write_zone_box(depth, node.bounds, look, node.tint),
                    }
                }
                Node::Phases(phases) => self.phases_step_fill = phases.step_fill,
                Node::View(_) | Node::Step(_) | Node::HowTo(_) => {}
                Node::PartsList(_) | Node::GeneralNotes(_) | Node::Ladder(_) => {
                    let width_px = match content {
                        Node::PartsList(_) => palette::PARTS_LIST_BORDER_PX,
                        Node::Ladder(_) => palette::LADDER_BORDER_PX,
                        _ => palette::GENERAL_NOTES_BORDER_PX,
                    };
                    let fill = self.palette.sheet_ink(palette::SheetInk::Cell).to_string();
                    let color = self.palette.sheet_ink(palette::SheetInk::Frame).to_string();
                    let border = Stroke {
                        width_px,
                        line: LineStyle::Solid,
                        color: &color,
                    };
                    self.write_rect(depth, node.bounds, 0.0, Some(&fill), Some(border));
                }
                // An outlined Item draws its outline as parts.
                Node::Item(_) if self.sheet.is_some() && node.outline.is_some() => {}
                Node::Item(item) if self.sheet.is_some() && item.planned => {
                    let color = self
                        .palette
                        .sheet_color(palette::SheetInk::TintWire, node.tint);
                    self.write_planned_box(depth, node.bounds, &color);
                }
                Node::Item(_) => {
                    let card_paint = self.palette.card();
                    match (self.sheet, node.tint) {
                        // An actor's Item outlines in the actor's wire (section 14.5 rule 2).
                        (Some(_), Some(slot)) => {
                            let wire = self
                                .palette
                                .sheet_color(palette::SheetInk::TintWire, Some(slot));
                            let border = Stroke {
                                width_px: palette::STEP_BORDER_PX,
                                line: LineStyle::Solid,
                                color: &wire,
                            };
                            self.write_rect(
                                depth,
                                node.bounds,
                                CARD_RADIUS_PX,
                                Some(card_paint.fill),
                                Some(border),
                            );
                        }
                        _ => self.write_box(depth, node.bounds, CARD_RADIUS_PX, card_paint),
                    }
                }
                Node::Fact(fact) => {
                    if let Some(fill) = self.fact_box_fill(fact.source) {
                        self.write_rect(depth, node.bounds, FACT_RADIUS_PX, Some(fill), None);
                    }
                }
                Node::Row(_)
                | Node::Col(_)
                | Node::Lanes(_)
                | Node::Note(_)
                | Node::Pipe(_)
                | Node::Tee(_) => {}
            }
        }
        Ok(self.part_context(node, document_node))
    }

    /// The document facts the parts of a node need, without drawing anything. A line takes
    /// its node's effective tint, which an actor gives on a sheet.
    fn part_context(
        &self,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
    ) -> PartContext<'a> {
        match document_node {
            DocumentNode::Page
            | DocumentNode::Kicker
            | DocumentNode::Title
            | DocumentNode::Lede
            | DocumentNode::Body
            | DocumentNode::Legend
            | DocumentNode::Foot
            | DocumentNode::Sheet
            | DocumentNode::SheetCell
            | DocumentNode::ProblemPoint
            | DocumentNode::Actor
            | DocumentNode::HowToItem(_) => PartContext::default(),
            // A rung's box is drawn as a boxed Step is: filled in its actor's fill and
            // outlined in its wire, or dashed and unfilled when planned.
            DocumentNode::Rung(rung, fill) => PartContext {
                step_look: Some(if rung.planned {
                    StepLook::Planned
                } else {
                    StepLook::Box
                }),
                step_fill: Some(fill),
                ..PartContext::default()
            },
            DocumentNode::LegendEntry(entry) => PartContext {
                line: Some(LineUse::new(entry.line, entry.tint)),
                pipe_form: Some(entry.form),
                by_actor: entry.by_actor,
                planned: entry.planned,
                rail: entry.form == PipeForm::Rail,
                ..PartContext::default()
            },
            DocumentNode::Content(NodeRef::TeeArm(pipe)) => PartContext {
                line: Some(LineUse::new(pipe.line, node.tint)),
                pipe_dir: Some(pipe.dir),
                pipe_form: Some(pipe.form),
                arrows: ArrowEnds::of(pipe),
                planned: pipe.planned,
                ..PartContext::default()
            },
            DocumentNode::Content(NodeRef::Node(content)) => match content {
                Node::Row(_)
                | Node::Col(_)
                | Node::Lanes(_)
                | Node::Note(_)
                | Node::Text(_)
                | Node::Frame(_)
                | Node::Box(_)
                | Node::Fact(_)
                | Node::View(_)
                | Node::PartsList(_)
                | Node::GeneralNotes(_)
                | Node::Ladder(_)
                | Node::HowTo(_) => PartContext::default(),
                Node::Phases(phases) => PartContext {
                    band: Some(phases.band),
                    ..PartContext::default()
                },
                Node::Step(step) => PartContext {
                    step_look: Some(step.look),
                    step_fill: Some(step.fill_under(self.phases_step_fill)),
                    ..PartContext::default()
                },
                Node::Callout(callout) => PartContext {
                    accent: Some(self.palette.callout(callout.kind).accent),
                    ..PartContext::default()
                },
                Node::Item(item) => PartContext {
                    icon: node.icon,
                    outline: node.outline,
                    planned: item.planned,
                    ..PartContext::default()
                },
                Node::Pipe(pipe) => PartContext {
                    line: Some(LineUse::new(pipe.line, node.tint)),
                    pipe_dir: Some(pipe.dir),
                    pipe_form: Some(pipe.form),
                    arrows: ArrowEnds::of(pipe),
                    planned: pipe.planned,
                    ..PartContext::default()
                },
                Node::Tee(tee) => PartContext {
                    line: Some(LineUse::new(tee.line, node.tint)),
                    ..PartContext::default()
                },
            },
        }
    }

    /// The box of a fact by source (section 13.7): a doc fact on the fact fill, an ask on
    /// the ask fill, an as-built name unfilled.
    fn fact_box_fill(&self, source: FactSource) -> Option<&'a str> {
        match source {
            FactSource::Doc => Some(self.palette.fact_fill()),
            FactSource::Built => None,
            FactSource::Ask => Some(self.palette.ask_fill()),
        }
    }

    fn write_zone_box(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        look: ContainerLook,
        tint: Option<u8>,
    ) {
        let style = self.zone_style(look, tint);
        if look.role == Role::Frame {
            let fill = style.fill.unwrap_or("none");
            let path = rounded_path(bounds, GCP_RADII_PX);
            self.line(depth, &format!(r#"<path d="{path}" fill="{fill}"/>"#));
            return;
        }
        self.write_rect(depth, bounds, style.radius_px, style.fill, style.border);
    }

    /// The two diagonals first, then the dashed border over their ends. The diagonals run
    /// between the corners of the box inside the border, pulled in to where the inner edge
    /// of the rounded corner crosses a 45 degree line, so no end shows through a dash gap.
    fn write_frame_box(&mut self, depth: usize, bounds: BoxRect) {
        let border = self.palette.frame_border();
        let inner = frame_diagonal_box(bounds, border.width_px);
        let diagonal = self.palette.frame_diagonal();
        for (x1, y1, x2, y2) in [
            (inner.x, inner.y, inner.right(), inner.bottom()),
            (inner.x, inner.bottom(), inner.right(), inner.y),
        ] {
            self.line(
                depth,
                &format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                    format_number(x1),
                    format_number(y1),
                    format_number(x2),
                    format_number(y2),
                    stroke_attributes(diagonal)
                ),
            );
        }
        self.write_rect(depth, bounds, FRAME_RADIUS_PX, None, Some(border));
    }

    /// The Callout accent bar fills the left 4 px of the box inside the border and follows
    /// the inner curve of the rounded left corners, so it never covers the border stroke.
    fn write_callout_accent(&mut self, depth: usize, bounds: BoxRect, fill: &str) {
        let inner_radius = (CARD_RADIUS_PX - palette::BLOCK_BORDER_PX).max(0.0);
        let path = left_rounded_strip_path(bounds, inner_radius);
        self.line(depth, &format!(r#"<path d="{path}" fill="{fill}"/>"#));
    }

    /// The frame stroke goes over the bar and body fills, as a CSS border would.
    fn write_gcp_frame_stroke(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        look: ContainerLook,
        tint: Option<u8>,
    ) {
        let Some(border) = self.zone_style(look, tint).border else {
            return;
        };
        let inset = border.width_px / 2.0;
        let inner = inset_rect(bounds, inset);
        let radii = GCP_RADII_PX.map(|radius| (radius - inset).max(0.0));
        let path = rounded_path(inner, radii);
        self.line(
            depth,
            &format!(
                r#"<path d="{path}" fill="none"{}/>"#,
                stroke_attributes(border)
            ),
        );
    }

    /// The paint of a Box: from the sheet role on a design sheet (section 14.5 rule 3).
    fn zone_style(&self, look: ContainerLook, tint: Option<u8>) -> palette::ZoneStyle<'a> {
        match self.sheet {
            Some(_) => self.palette.sheet_zone_style(look, tint),
            None => self.palette.zone_style(look, tint),
        }
    }

    fn write_part_shape(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        part: &Part,
        (context, slot): (PartContext<'a>, Option<u8>),
    ) -> Result<(), RenderError> {
        let bounds = part.bounds;
        if self.sheet.is_some() && self.write_assembly_part(depth, node, part, (context, slot)) {
            return Ok(());
        }
        if let (Some(look), Some(fill)) = (context.step_look, context.step_fill) {
            match part.name {
                PartName::Front => {
                    self.write_step_front(depth, bounds, (look, fill), slot);
                    return Ok(());
                }
                PartName::StackBack => {
                    let nearer =
                        sheet_in_front_of(node, part).ok_or_else(|| part_mismatch(node, part))?;
                    self.write_stack_back(depth, (bounds, nearer), fill, slot);
                    return Ok(());
                }
                _ => {}
            }
        }
        if part.name == PartName::LaneBand && context.band == Some(BandLook::None) {
            return Ok(());
        }
        if part.name == PartName::MarkSample {
            let position = node
                .parts
                .iter()
                .take_while(|candidate| !std::ptr::eq(*candidate, part))
                .filter(|candidate| candidate.name == PartName::MarkSample)
                .count();
            let kind = self
                .mark_paints
                .samples
                .get(position)
                .copied()
                .ok_or_else(|| part_mismatch(node, part))?;
            self.write_sheet_part(depth, bounds, palette::glyph_paint(kind), None);
            return Ok(());
        }
        if self.sheet.is_some()
            && let Some(paint) = palette::sheet_part_paint(node.tag, part.name)
        {
            self.write_sheet_part(depth, bounds, paint, slot);
            match part.name {
                PartName::Grid => self.sheet_grid = Some(bounds),
                // A washed band redraws the grid over itself, so the grid reads through
                // the lane while the band keeps its colour between the lines.
                PartName::LaneBand => {
                    if let Some(grid) = self.sheet_grid {
                        self.write_grid_lines(depth, grid, bounds);
                    }
                }
                _ => {}
            }
            return Ok(());
        }
        if context.by_actor && part.name == PartName::Swatch {
            let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
            let stroke = self.palette.by_actor_sample(kind.line).stroke;
            self.write_axis_line(depth, bounds, stroke, None);
            return Ok(());
        }
        match part.name {
            // The solid draws the footprint under iso; it has no flat drawing.
            PartName::Footprint => {}
            PartName::Badge => {
                let badge_paint = self.palette.badge();
                self.write_box(depth, bounds, BADGE_RADIUS_PX, badge_paint);
            }
            // On a sheet a frame-role Box leaves its bar and body unfilled; its label stands
            // on the body with no rule under it.
            PartName::Bar | PartName::Body if self.sheet.is_some() => {}
            PartName::Bar => {
                let fill = self.palette.frame_bar_fill();
                self.write_rect(depth, bounds, 0.0, Some(fill), None);
                if let Some(rule) = self.palette.frame_bar_rule() {
                    self.write_bottom_rule(depth, bounds, rule);
                }
            }
            PartName::Body => {
                let fill = self.palette.frame_body_fill();
                self.write_rect(depth, bounds, 0.0, Some(fill), None);
            }
            PartName::Icon => {
                let icon = context.icon.ok_or_else(|| part_mismatch(node, part))?;
                if self.palette.projection() == Projection::Iso {
                    self.write_iso_icon_chip(depth, bounds);
                } else if let Some(chip_fill) = self.palette.icon_chip() {
                    self.write_icon_chip(depth, bounds, chip_fill);
                }
                self.line(
                    depth,
                    &format!(
                        r#"<image x="{}" y="{}" width="{}" height="{}" href="{}"/>"#,
                        format_number(bounds.x),
                        format_number(bounds.y),
                        format_number(bounds.width),
                        format_number(bounds.height),
                        icon_data_uri(icon)
                    ),
                );
            }
            PartName::FactBox => {
                let fill = self.palette.fact_fill();
                self.write_rect(depth, bounds, FACT_RADIUS_PX, Some(fill), None);
            }
            PartName::AskBox => {
                let fill = self.palette.ask_fill();
                self.write_rect(depth, bounds, FACT_RADIUS_PX, Some(fill), None);
            }
            PartName::BuiltBox => {}
            PartName::DotStart | PartName::DotEnd => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                let at_start = part.name == PartName::DotStart;
                let arrow_here = if at_start {
                    context.arrows.start
                } else {
                    context.arrows.end
                };
                if arrow_here {
                    let dir = context.pipe_dir.ok_or_else(|| part_mismatch(node, part))?;
                    let (base, tip) = arrowhead_ends(bounds, dir, at_start);
                    match EndCap::of(kind.line) {
                        EndCap::Arrowhead => self.write_arrow_carrier(depth, base, tip, kind),
                        EndCap::StopPlate => self.write_pipe_stop_plate(depth, (base, tip), kind),
                    }
                } else {
                    let center_x = bounds.x + bounds.width / 2.0;
                    let center_y = bounds.y + bounds.height / 2.0;
                    self.write_dot(depth, center_x, center_y, kind);
                }
            }
            PartName::WireStart | PartName::WireEnd => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                let dir = context.pipe_dir.ok_or_else(|| part_mismatch(node, part))?;
                self.write_wire(depth, bounds, dir, kind);
            }
            PartName::Swatch => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                match context.pipe_form {
                    Some(PipeForm::Band) => self.write_band_swatch(depth, bounds, kind),
                    // A rail's sample is drawn by `write_assembly_part` on its sheet.
                    Some(PipeForm::Rail) => return Err(part_mismatch(node, part)),
                    Some(PipeForm::Tube) | None => {
                        self.write_wire(depth, bounds, PipeDir::Horizontal, kind);
                        // A kind told apart by its hollow dots shows them on its swatch
                        // too, inset so they stay within the swatch's width.
                        if self.palette.wire_style(kind).dot == DotStyle::Hollow {
                            let center_y = bounds.y + bounds.height / 2.0;
                            self.write_dot(depth, bounds.x + DOT_RADIUS_PX, center_y, kind);
                            self.write_dot(depth, bounds.right() - DOT_RADIUS_PX, center_y, kind);
                        }
                        if EndCap::of(kind.line) == EndCap::StopPlate {
                            let plate = BoxRect {
                                x: bounds.right() - STOP_PLATE_THICKNESS_PX,
                                y: bounds.y + bounds.height / 2.0 - LEGEND_STOP_PLATE_PX / 2.0,
                                width: STOP_PLATE_THICKNESS_PX,
                                height: LEGEND_STOP_PLATE_PX,
                            };
                            let color = self.palette.wire_style(kind).stroke.color;
                            self.write_rect(depth, plate, 0.0, Some(color), None);
                        }
                    }
                }
            }
            PartName::Spine => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                self.write_wire(depth, bounds, PipeDir::Vertical, kind);
            }
            PartName::Tag | PartName::Hub => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                let tag_paint = self.palette.tag(kind.line);
                self.write_box(depth, bounds, TAG_RADIUS_PX, tag_paint);
            }
            PartName::Accent => {
                let fill = context.accent.ok_or_else(|| part_mismatch(node, part))?;
                self.write_callout_accent(depth, bounds, fill);
            }
            PartName::LabelChip => {
                let fill = self.palette.frame_label_chip();
                self.write_rect(depth, bounds, FRAME_CHIP_RADIUS_PX, Some(fill), None);
            }
            PartName::Marker if part.text.is_none() => {
                let fill = self.palette.list_bullet();
                self.line(
                    depth,
                    &format!(
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{fill}"/>"#,
                        format_number(bounds.x + BULLET_CENTER_INSET_PX),
                        format_number(bounds.y + bounds.height / 2.0),
                        format_number(BULLET_RADIUS_PX),
                    ),
                );
            }
            PartName::Lifeline => self.write_lifeline(depth, bounds),
            PartName::Heads
            | PartName::Band
            | PartName::BadgeText
            | PartName::Text
            | PartName::Label
            | PartName::FunctionName
            | PartName::ProductName
            | PartName::Fact
            | PartName::Built
            | PartName::Ask
            | PartName::TagLabel
            | PartName::TagSub
            | PartName::HubText
            | PartName::LegendLabel
            | PartName::LegendText
            | PartName::Heading
            | PartName::Marker
            | PartName::BodyLine => {}
            // A sheet part reaches here only off a sheet, which never lays one out.
            PartName::DetailFrame
            | PartName::DetailTag
            | PartName::DetailLetter
            | PartName::RailLeft
            | PartName::RailRight
            | PartName::RungTick
            | PartName::EscalationArrow
            | PartName::RungTitle
            | PartName::RungWho
            | PartName::RungText
            | PartName::HowToNumber
            | PartName::HowToTitle
            | PartName::HowToRule
            | PartName::CommandChip
            | PartName::CommandText
            | PartName::CommandWord
            | PartName::HowToText
            | PartName::Terminal
            | PartName::TerminalPrompt
            | PartName::TerminalLine
            | PartName::Border
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
            | PartName::StackBack
            | PartName::Balloon
            | PartName::Flag
            | PartName::Diamond
            | PartName::MarkNumeral
            | PartName::AttentionDiamond
            | PartName::AttentionNumeral
            | PartName::AttentionZones
            | PartName::MarkSample
            | PartName::MarkSampleNumeral
            | PartName::MarkSampleText
            | PartName::Title
            | PartName::Note
            | PartName::TitleRule
            | PartName::HeaderCell
            | PartName::RowRule
            | PartName::PartBalloon
            | PartName::PartNumeral
            | PartName::PartTitle
            | PartName::PartWhat
            | PartName::PartPlanned
            | PartName::PartWhy
            | PartName::PartWhyRule
            | PartName::PartZones
            | PartName::NoteFlag
            | PartName::NoteNumeral
            | PartName::NoteText
            | PartName::NoteDetail
            | PartName::NoteZones
            | PartName::FootRule
            | PartName::Foot
            | PartName::Head
            | PartName::Shoulders
            | PartName::Compartment
            | PartName::CompartmentText
            | PartName::TabBorder
            | PartName::Tab
            | PartName::Rail
            | PartName::StepBadge
            | PartName::StepNumeral
            | PartName::ValveNear
            | PartName::ValveFar
            | PartName::RailDot
            | PartName::Hop => return Err(part_mismatch(node, part)),
        }
        Ok(())
    }

    /// The colour a Step's fill paints: the actor's fill, the sheet cell, or nothing.
    fn step_fill_color(&self, fill: StepFill, slot: Option<u8>) -> Option<String> {
        use palette::SheetInk::{Cell, TintFill};
        match fill {
            StepFill::Tint => Some(self.palette.sheet_color(TintFill, slot)),
            StepFill::Cell => Some(self.palette.sheet_color(Cell, slot)),
            StepFill::None => None,
        }
    }

    /// The box a Step's look outlines (section 14.5 rule 6), filled as `fill` says: a box
    /// and a stack's front stroked in the actor's wire, a chip as a pill in the wire, a
    /// note in the rule, a planned Step dashed in the wire. A text look draws no box.
    fn write_step_front(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        (look, fill): (StepLook, StepFill),
        slot: Option<u8>,
    ) {
        use palette::SheetInk::{Rule, TintWire};
        // A Step always has its lane's actor; a rung without one outlines in the frame ink,
        // which reads on every paper where the hairline rule would not.
        let wire = match slot {
            Some(_) => self.palette.sheet_color(TintWire, slot),
            None => self.palette.sheet_ink(palette::SheetInk::Frame).to_string(),
        };
        let fill = self.step_fill_color(fill, slot);
        let (stroke_color, radius, dash) = match look {
            StepLook::Box | StepLook::Stack => (wire, 0.0, None),
            StepLook::Chip => (wire, bounds.height.min(bounds.width) / 2.0, None),
            StepLook::Note => (self.palette.sheet_color(Rule, slot), 0.0, None),
            StepLook::Planned => (wire, 0.0, Some(palette::PLANNED_DASH)),
            StepLook::Text => return,
        };
        let stroke = Stroke {
            width_px: palette::STEP_BORDER_PX,
            line: LineStyle::Solid,
            color: &stroke_color,
        };
        match dash {
            None => self.write_rect(depth, bounds, radius, fill.as_deref(), Some(stroke)),
            Some(dash) => {
                let rect = inset_rect(bounds, stroke.width_px / 2.0);
                self.line(
                    depth,
                    &format!(
                        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"{} stroke-dasharray="{dash}"/>"#,
                        format_number(rect.x),
                        format_number(rect.y),
                        format_number(rect.width),
                        format_number(rect.height),
                        fill.as_deref().unwrap_or("none"),
                        stroke_attributes(stroke)
                    ),
                );
            }
        }
    }

    /// One sheet behind a stack's front (section 14.5 rule 6). Filled, it is a rect in the
    /// fill with the actor's wire, which the nearer sheet covers in part. Unfilled, nothing
    /// would cover its edges inside the nearer sheet, so it draws only the edges that stand
    /// clear of it: up its left side from the nearer sheet's top, along its top, down its
    /// right side, and back along its bottom to the nearer sheet's right edge.
    fn write_stack_back(
        &mut self,
        depth: usize,
        (bounds, nearer): (BoxRect, BoxRect),
        fill: StepFill,
        slot: Option<u8>,
    ) {
        let wire = self.palette.sheet_color(palette::SheetInk::TintWire, slot);
        let stroke = Stroke {
            width_px: palette::STEP_BORDER_PX,
            line: LineStyle::Solid,
            color: &wire,
        };
        if let Some(fill) = self.step_fill_color(fill, slot) {
            self.write_rect(depth, bounds, 0.0, Some(&fill), Some(stroke));
            return;
        }
        let rect = inset_rect(bounds, stroke.width_px / 2.0);
        let nearer = inset_rect(nearer, stroke.width_px / 2.0);
        let path = format!(
            "M {} {} V {} H {} V {} H {}",
            format_number(rect.x),
            format_number(nearer.y.max(rect.y)),
            format_number(rect.y),
            format_number(rect.right()),
            format_number(rect.bottom()),
            format_number(nearer.right().min(rect.right())),
        );
        self.line(
            depth,
            &format!(
                r#"<path d="{path}" fill="none"{}/>"#,
                stroke_attributes(stroke)
            ),
        );
    }

    /// One sheet part by its paint row: a rect, a line along its long axis, a circle, or
    /// the drafting grid. A text part draws nothing here.
    fn write_sheet_part(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        paint: palette::PartPaint,
        slot: Option<u8>,
    ) {
        let fill_color = paint.fill.map(|ink| self.palette.sheet_color(ink, slot));
        let stroke_color = paint
            .stroke
            .map(|(ink, width_px)| (self.palette.sheet_color(ink, slot), width_px));
        let fill = fill_color.as_deref();
        let stroke = stroke_color.as_ref().map(|(color, width_px)| Stroke {
            width_px: *width_px,
            line: LineStyle::Solid,
            color,
        });
        match paint.shape {
            PartShape::Rect => self.write_rect(depth, bounds, 0.0, fill, stroke),
            PartShape::Line => {
                if let Some(stroke) = stroke {
                    self.write_axis_line(depth, bounds, stroke, None);
                }
            }
            PartShape::DashedLine(dash) => {
                if let Some(stroke) = stroke {
                    self.write_axis_line(depth, bounds, stroke, Some(dash));
                }
            }
            PartShape::Circle => {
                let inset = stroke.map_or(0.0, |stroke| stroke.width_px / 2.0);
                let radius = bounds.width.min(bounds.height) / 2.0 - inset;
                let stroke_part = stroke.map(stroke_attributes).unwrap_or_default();
                self.line(
                    depth,
                    &format!(
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{}"{stroke_part}/>"#,
                        format_number(bounds.x + bounds.width / 2.0),
                        format_number(bounds.y + bounds.height / 2.0),
                        format_number(radius.max(0.0)),
                        fill.unwrap_or("none"),
                    ),
                );
            }
            PartShape::Triangle => {
                let path = format!(
                    "M{} {} L{} {} L{} {} Z",
                    format_number(bounds.x + bounds.width / 2.0),
                    format_number(bounds.y),
                    format_number(bounds.right()),
                    format_number(bounds.bottom()),
                    format_number(bounds.x),
                    format_number(bounds.bottom()),
                );
                self.write_glyph_path(depth, &path, fill, stroke);
            }
            PartShape::Diamond => {
                let inset = stroke.map_or(0.0, |stroke| stroke.width_px / 2.0);
                let center_x = bounds.x + bounds.width / 2.0;
                let center_y = bounds.y + bounds.height / 2.0;
                let path = format!(
                    "M{} {} L{} {} L{} {} L{} {} Z",
                    format_number(center_x),
                    format_number(bounds.y + inset),
                    format_number(bounds.right() - inset),
                    format_number(center_y),
                    format_number(center_x),
                    format_number(bounds.bottom() - inset),
                    format_number(bounds.x + inset),
                    format_number(center_y),
                );
                self.write_glyph_path(depth, &path, fill, stroke);
            }
            PartShape::Grid => self.write_sheet_grid(depth, bounds),
            PartShape::DashedRect(dash) => {
                if let Some(stroke) = stroke {
                    let rect = inset_rect(bounds, stroke.width_px / 2.0);
                    self.line(
                        depth,
                        &format!(
                            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"{} stroke-dasharray="{dash}"/>"#,
                            format_number(rect.x),
                            format_number(rect.y),
                            format_number(rect.width),
                            format_number(rect.height),
                            fill.unwrap_or("none"),
                            stroke_attributes(stroke)
                        ),
                    );
                }
            }
            PartShape::RoundedRect => {
                self.write_rect(depth, bounds, palette::STEP_BADGE_RADIUS_PX, fill, stroke);
            }
            PartShape::Shoulders => {
                let inset = stroke.map_or(0.0, |stroke| stroke.width_px / 2.0);
                let shape = inset_rect(bounds, inset);
                let middle = shape.x + shape.width / 2.0;
                let path = format!(
                    "M {} {} Q {} {} {} {} Q {} {} {} {} Z",
                    format_number(shape.x),
                    format_number(shape.bottom()),
                    format_number(shape.x),
                    format_number(shape.y),
                    format_number(middle),
                    format_number(shape.y),
                    format_number(shape.right()),
                    format_number(shape.y),
                    format_number(shape.right()),
                    format_number(shape.bottom()),
                );
                self.write_glyph_path(depth, &path, fill, stroke);
            }
            PartShape::RoundedDashedRect(dash) => {
                if let Some(stroke) = stroke {
                    let rect = inset_rect(bounds, stroke.width_px / 2.0);
                    let radius = stencil_layout::DETAIL_FRAME_RADIUS_PX - stroke.width_px / 2.0;
                    self.line(
                        depth,
                        &format!(
                            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="none"{} stroke-dasharray="{dash}"/>"#,
                            format_number(rect.x),
                            format_number(rect.y),
                            format_number(rect.width),
                            format_number(rect.height),
                            format_number(radius.max(0.0)),
                            stroke_attributes(stroke)
                        ),
                    );
                }
            }
            PartShape::UpArrow(dash) => {
                if let Some(stroke) = stroke {
                    self.write_up_arrow(depth, bounds, stroke, dash);
                }
            }
            // A valve and a hop are drawn with their link.
            PartShape::Valve | PartShape::OnPath | PartShape::Text => {}
        }
    }

    /// A dashed line up the middle of `bounds` from its bottom to the base of a filled
    /// arrowhead whose tip is the box's top.
    fn write_up_arrow(&mut self, depth: usize, bounds: BoxRect, stroke: Stroke<'_>, dash: &str) {
        let center_x = bounds.x + bounds.width / 2.0;
        let base_y = bounds.y + ARROW_LENGTH_PX;
        self.line(
            depth,
            &format!(
                r#"<line x1="{x}" y1="{}" x2="{x}" y2="{}" fill="none"{} stroke-dasharray="{dash}"/>"#,
                format_number(bounds.bottom()),
                format_number(base_y),
                stroke_attributes(stroke),
                x = format_number(center_x),
            ),
        );
        let half = ARROW_WIDTH_PX / 2.0;
        self.line(
            depth,
            &format!(
                r#"<path d="M{} {} L{} {} L{} {} Z" fill="{}"/>"#,
                format_number(center_x),
                format_number(bounds.y),
                format_number(center_x + half),
                format_number(base_y),
                format_number(center_x - half),
                format_number(base_y),
                stroke.color,
            ),
        );
    }

    /// A flag or diamond outline.
    fn write_glyph_path(
        &mut self,
        depth: usize,
        path: &str,
        fill: Option<&str>,
        stroke: Option<Stroke<'_>>,
    ) {
        let stroke_part = stroke
            .map(|stroke| format!("{} stroke-linejoin=\"round\"", stroke_attributes(stroke)))
            .unwrap_or_default();
        self.line(
            depth,
            &format!(
                r#"<path d="{path}" fill="{}"{stroke_part}/>"#,
                fill.unwrap_or("none")
            ),
        );
    }

    /// A line through the middle of `bounds` along its longer side.
    fn write_axis_line(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        stroke: Stroke<'_>,
        dash: Option<&str>,
    ) {
        let (x1, y1, x2, y2) = if bounds.width < bounds.height {
            let center_x = bounds.x + bounds.width / 2.0;
            (center_x, bounds.y, center_x, bounds.bottom())
        } else {
            let center_y = bounds.y + bounds.height / 2.0;
            (bounds.x, center_y, bounds.right(), center_y)
        };
        let dash = dash.map_or(String::new(), |dash| {
            format!(r#" stroke-dasharray="{dash}""#)
        });
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}{dash}/>"#,
                format_number(x1),
                format_number(y1),
                format_number(x2),
                format_number(y2),
                stroke_attributes(stroke)
            ),
        );
    }

    /// The drafting grid (section 14.5 rule 6): a major line on every zone edge the
    /// reference ticks mark and minor lines cutting each zone into equal parts near 20 px,
    /// drawn as one path per colour. A theme without a grid draws nothing.
    fn write_sheet_grid(&mut self, depth: usize, bounds: BoxRect) {
        self.write_grid_lines(depth, bounds, bounds);
    }

    /// The lines of the drafting grid `grid` that fall inside `clip`, each cut to it: the
    /// whole grid when `clip` is the grid, and the grid over a lane band when it is the
    /// band. Lines keep their place and colour from the grid's own zones, so the lines
    /// over a band continue the lines around it.
    fn write_grid_lines(&mut self, depth: usize, grid: BoxRect, clip: BoxRect) {
        let Some((minor, major)) = self.palette.sheet_grid() else {
            return;
        };
        let left = clip.x.max(grid.x);
        let right = clip.right().min(grid.right());
        let top = clip.y.max(grid.y);
        let bottom = clip.bottom().min(grid.bottom());
        if left >= right || top >= bottom {
            return;
        }
        let edges = stencil_layout::sheet_zone_edges(grid);
        let mut minor_path = String::new();
        let mut major_path = String::new();
        for (x, is_major) in grid_lines(&edges.columns) {
            if x < left || x >= right {
                continue;
            }
            let path = if is_major {
                &mut major_path
            } else {
                &mut minor_path
            };
            path.push_str(&format!(
                "M {} {} V {} ",
                format_number(x),
                format_number(top),
                format_number(bottom)
            ));
        }
        for (y, is_major) in grid_lines(&edges.rows) {
            if y < top || y >= bottom {
                continue;
            }
            let path = if is_major {
                &mut major_path
            } else {
                &mut minor_path
            };
            path.push_str(&format!(
                "M {} {} H {} ",
                format_number(left),
                format_number(y),
                format_number(right)
            ));
        }
        for (path, color, width) in [
            (minor_path, minor, SHEET_GRID_MINOR_PX),
            (major_path, major, SHEET_GRID_ZONE_PX),
        ] {
            if path.is_empty() {
                continue;
            }
            self.line(
                depth,
                &format!(
                    r#"<path d="{}" fill="none" stroke="{color}" stroke-width="{}"/>"#,
                    path.trim_end(),
                    format_number(width)
                ),
            );
        }
    }

    /// A lifeline down its lane, from the head's bottom to the band's bottom. One of length
    /// 0, under a Lanes node with no message, draws nothing.
    fn write_lifeline(&mut self, depth: usize, bounds: BoxRect) {
        if bounds.height <= 0.0 {
            return;
        }
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                format_number(bounds.x),
                format_number(bounds.y),
                format_number(bounds.x),
                format_number(bounds.bottom()),
                stroke_attributes(self.palette.lifeline())
            ),
        );
    }

    /// One `<marker>` per arrow kind in this theme: a triangle 10 long and 8 wide in the wire
    /// color, its tip on the carrier line's last point.
    fn write_arrow_markers(&mut self, depth: usize, kinds: &BTreeSet<LineUse>) {
        let length = format_number(ARROW_LENGTH_PX);
        let width = format_number(ARROW_WIDTH_PX);
        let half_width = format_number(ARROW_WIDTH_PX / 2.0);
        self.line(depth, "<defs>");
        for kind in kinds {
            let color = self.palette.wire_style(*kind).stroke.color;
            let marker = format!(
                r#"<marker id="{}" viewBox="0 0 {length} {width}" refX="{length}" refY="{half_width}" markerWidth="{length}" markerHeight="{width}" markerUnits="userSpaceOnUse" orient="auto"><path d="M 0 0 L {length} {half_width} L 0 {width} Z" fill="{color}"/></marker>"#,
                self.arrow_marker_id(*kind)
            );
            self.line(depth + 1, &marker);
        }
        self.line(depth, "</defs>");
    }

    /// `arrow-<theme>-<line key>` (section 13.1 rule 6).
    fn arrow_marker_id(&self, kind: LineUse) -> String {
        format!("arrow-{}-{}", self.palette.theme_name(), kind.key())
    }

    /// A deny pipe's stop plate in place of an arrowhead: the wire carried solid from the
    /// arrowhead's base, where layout stops it, to the plate, then the plate.
    fn write_pipe_stop_plate(
        &mut self,
        depth: usize,
        (base, tip): (PagePoint, PagePoint),
        kind: LineUse,
    ) {
        let (plate_base, _) = stop_plate_ends(base, tip);
        let stroke = Stroke {
            line: LineStyle::Solid,
            ..self.palette.wire_style(kind).stroke
        };
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                format_number(base.x),
                format_number(base.y),
                format_number(plate_base.x),
                format_number(plate_base.y),
                stroke_attributes(stroke)
            ),
        );
        self.write_flat_stop_plate(depth, (plate_base, tip), kind);
    }

    /// A deny line's stop plate: a bar across the line from `base` to its outer face at
    /// `tip`, on the endpoint's outline, filled in the wire color (section 11.2).
    fn write_flat_stop_plate(&mut self, depth: usize, ends: (PagePoint, PagePoint), kind: LineUse) {
        let color = self.palette.wire_style(kind).stroke.color;
        self.write_rect(
            depth,
            EndCap::StopPlate.bounds(ends),
            0.0,
            Some(color),
            None,
        );
    }

    /// One link: the routed path in the wire style of its kind, an arrowhead at each end the
    /// `arrow` value names, then the tag, drawn like a pipe tag over the path under it.
    fn write_link(
        &mut self,
        depth: usize,
        geometry: &PageGeometry,
        route: &LinkRoute,
        link: Option<&stencil_model::Link>,
    ) -> Result<(), RenderError> {
        let pointer = NodePointer::root().child("links").index(route.index);
        self.line(depth, &link_group_open_tag(geometry, route)?);
        let line_use = LineUse::new(route.line, route.tint);
        let drawn = drawn_link(geometry, route);
        if self.sheet.is_some() {
            self.write_sheet_link_line(depth + 1, &drawn, line_use, assembly::is_planned(link));
        } else {
            let stroke = self.palette.wire_style(line_use).stroke;
            self.line(
                depth + 1,
                &format!(
                    r#"<path d="{}" fill="none"{} stroke-linejoin="round"/>"#,
                    link_path(&drawn),
                    stroke_attributes(stroke)
                ),
            );
        }
        for ends in [drawn.start_arrow, drawn.end_arrow].into_iter().flatten() {
            match drawn.cap {
                EndCap::Arrowhead => self.write_arrow_carrier(depth + 1, ends.0, ends.1, line_use),
                EndCap::StopPlate => self.write_flat_stop_plate(depth + 1, ends, line_use),
            }
        }
        let glyphs = self
            .mark_paints
            .links
            .get(&route.index)
            .cloned()
            .unwrap_or_default();
        let mut glyph_index = 0;
        let mut numeral_index = 0;
        for (index, part) in route.parts.iter().enumerate() {
            self.begin_part(index, depth + 1);
            if self.sheet.is_some()
                && part.text.is_none()
                && self.write_link_mark(depth + 1, route, part, line_use)
            {
                self.end_part();
                continue;
            }
            match (part.name, &part.text) {
                (PartName::StepNumeral, Some(run)) => {
                    let fill = self
                        .palette
                        .sheet_ink(palette::SheetInk::TagInk)
                        .to_string();
                    self.write_text_run(depth + 1, &pointer, part.bounds, run, &fill)?;
                }
                (PartName::Tag, None) => {
                    let tag_paint = self.palette.tag(route.line);
                    self.write_box(depth + 1, part.bounds, TAG_RADIUS_PX, tag_paint);
                }
                (PartName::Balloon | PartName::Flag | PartName::Diamond, None) => {
                    let (kind, slot) = glyphs
                        .get(glyph_index)
                        .copied()
                        .ok_or_else(|| link_mismatch(route))?;
                    glyph_index += 1;
                    self.write_sheet_part(depth + 1, part.bounds, palette::glyph_paint(kind), slot);
                }
                (PartName::MarkNumeral, Some(run)) => {
                    let (kind, _) = glyphs
                        .get(numeral_index)
                        .copied()
                        .ok_or_else(|| link_mismatch(route))?;
                    numeral_index += 1;
                    let fill = self.palette.sheet_color(palette::numeral_ink(kind), None);
                    self.write_text_run(depth + 1, &pointer, part.bounds, run, &fill)?;
                }
                (PartName::TagLabel | PartName::TagSub, Some(run)) => {
                    let style_name =
                        pipe_text_style_name(part.name).ok_or_else(|| link_mismatch(route))?;
                    let fill = self.palette.text_ink(style_name, Some(route.line));
                    self.write_text_run(depth + 1, &pointer, part.bounds, run, fill)?;
                }
                _ => return Err(link_mismatch(route)),
            }
            self.end_part();
        }
        self.line(depth, "</g>");
        Ok(())
    }

    /// An unstroked line from the arrowhead's base to its tip that only places the marker.
    fn write_arrow_carrier(
        &mut self,
        depth: usize,
        base: PagePoint,
        tip: PagePoint,
        kind: LineUse,
    ) {
        let marker_id = self.arrow_marker_id(kind);
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="none" marker-end="url(#{marker_id})"/>"#,
                format_number(base.x),
                format_number(base.y),
                format_number(tip.x),
                format_number(tip.y),
            ),
        );
    }

    /// An end dot of a wire, filled or hollow as the palette says.
    fn write_dot(&mut self, depth: usize, center_x: f32, center_y: f32, kind: LineUse) {
        let wire = self.palette.wire_style(kind);
        let circle = match wire.dot {
            DotStyle::None => return,
            DotStyle::Filled => format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"#,
                format_number(center_x),
                format_number(center_y),
                format_number(DOT_RADIUS_PX),
                wire.stroke.color
            ),
            DotStyle::Hollow => {
                let ring = Stroke {
                    width_px: palette::HOLLOW_DOT_RING_PX,
                    line: LineStyle::Solid,
                    color: wire.stroke.color,
                };
                // The ring is inset like a border so the dot keeps its 4 px outer radius.
                format!(
                    r#"<circle cx="{}" cy="{}" r="{}" fill="{}"{}/>"#,
                    format_number(center_x),
                    format_number(center_y),
                    format_number(DOT_RADIUS_PX - ring.width_px / 2.0),
                    self.palette.page_background(),
                    stroke_attributes(ring)
                )
            }
        };
        self.line(depth, &circle);
    }

    /// A flat bar centered on the swatch's center line, filled in the wire color; a
    /// patterned line draws it hollow with its pattern, as the band on the floor is.
    fn write_band_swatch(&mut self, depth: usize, bounds: BoxRect, kind: LineUse) {
        let stroke = self.palette.wire_style(kind).stroke;
        let bar = BoxRect {
            x: bounds.x,
            y: bounds.y + bounds.height / 2.0 - BAND_SWATCH_HEIGHT_PX / 2.0,
            width: bounds.width,
            height: BAND_SWATCH_HEIGHT_PX,
        };
        if stroke.line == LineStyle::Solid {
            self.write_rect(depth, bar, 0.0, Some(stroke.color), None);
        } else {
            let outline = Stroke {
                width_px: BAND_SWATCH_OUTLINE_PX,
                line: stroke.line,
                color: stroke.color,
            };
            self.write_rect(
                depth,
                bar,
                0.0,
                Some(self.palette.page_background()),
                Some(outline),
            );
        }
    }

    /// A `<line>` along the box's center line in the run direction.
    fn write_wire(&mut self, depth: usize, bounds: BoxRect, dir: PipeDir, kind: LineUse) {
        let stroke = self.palette.wire_style(kind).stroke;
        let (x1, y1, x2, y2) = match dir {
            PipeDir::Horizontal => {
                let center_y = bounds.y + bounds.height / 2.0;
                (bounds.x, center_y, bounds.right(), center_y)
            }
            PipeDir::Vertical => {
                let center_x = bounds.x + bounds.width / 2.0;
                (center_x, bounds.y, center_x, bounds.bottom())
            }
        };
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                format_number(x1),
                format_number(y1),
                format_number(x2),
                format_number(y2),
                stroke_attributes(stroke)
            ),
        );
    }

    /// The rounded square under an icon, centered on the icon part. The icon part is 28 by
    /// 28 (section 2.5), so the 36 px chip reaches 4 px into the card padding on each side.
    fn write_icon_chip(&mut self, depth: usize, icon_bounds: BoxRect, fill: &str) {
        self.write_rect(
            depth,
            icon_chip_box(icon_bounds),
            ICON_CHIP_RADIUS_PX,
            Some(fill),
            None,
        );
    }

    fn write_box(&mut self, depth: usize, bounds: BoxRect, radius_px: f32, paint: BoxPaint<'_>) {
        self.write_rect(depth, bounds, radius_px, Some(paint.fill), paint.border);
    }

    /// A horizontal rule along the bottom edge of `bounds`, inside the box.
    fn write_bottom_rule(&mut self, depth: usize, bounds: BoxRect, rule: Stroke<'_>) {
        let center_y = bounds.bottom() - rule.width_px / 2.0;
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                format_number(bounds.x),
                format_number(center_y),
                format_number(bounds.right()),
                format_number(center_y),
                stroke_attributes(rule)
            ),
        );
    }

    /// A stroke of width w is drawn on the rect inset by w / 2, so it stays inside the
    /// border box as a CSS border does, and the radius shrinks by the same amount so the
    /// outer edge keeps the section 2 radius.
    fn write_rect(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        radius_px: f32,
        fill: Option<&str>,
        stroke: Option<Stroke<'_>>,
    ) {
        let inset = stroke.map_or(0.0, |stroke| stroke.width_px / 2.0);
        let rect = inset_rect(bounds, inset);
        let radius = (radius_px - inset).max(0.0);
        let radius_attribute = if radius > 0.0 {
            format!(r#" rx="{}""#, format_number(radius))
        } else {
            String::new()
        };
        let stroke_part = stroke.map(stroke_attributes).unwrap_or_default();
        self.line(
            depth,
            &format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}"{radius_attribute} fill="{}"{stroke_part}/>"#,
                format_number(rect.x),
                format_number(rect.y),
                format_number(rect.width),
                format_number(rect.height),
                fill.unwrap_or("none"),
            ),
        );
    }

    /// One `<text>` per line. Centering uses the measured line width instead of
    /// `text-anchor`, so the drawn line sits where the text-fits-box check measured it.
    fn write_text_run(
        &mut self,
        depth: usize,
        pointer: &NodePointer,
        bounds: BoxRect,
        run: &TextRun,
        fill: &str,
    ) -> Result<(), RenderError> {
        self.write_text_run_transformed(depth, pointer, bounds, run, fill, None)
    }

    /// `transform`, when present, is written on each `<text>` element; the bounds are then
    /// read in the transform's local space.
    pub(crate) fn write_text_run_transformed(
        &mut self,
        depth: usize,
        pointer: &NodePointer,
        bounds: BoxRect,
        run: &TextRun,
        fill: &str,
        transform: Option<&str>,
    ) -> Result<(), RenderError> {
        let transform =
            transform.map_or(String::new(), |matrix| format!(r#" transform="{matrix}""#));
        let style = run.style;
        if !run.metrics.lines.is_empty() && !self.font_faces.contains(&(style.family, style.weight))
        {
            self.font_faces.push((style.family, style.weight));
        }
        let letter_spacing_px = style.letter_spacing_em * style.size_px;
        let letter_spacing = if format_number(letter_spacing_px) == crate::NumberRepr::Integer(0) {
            String::new()
        } else {
            format!(r#" letter-spacing="{}""#, format_number(letter_spacing_px))
        };
        for (line_index, line) in run.metrics.lines.iter().enumerate() {
            let Some(content) = run.text.get(line.byte_start..line.byte_end) else {
                return Err(RenderError::Svg {
                    message: format!(
                        "text run at {pointer} has line bytes {}..{} outside its {}-byte string",
                        line.byte_start,
                        line.byte_end,
                        run.text.len()
                    ),
                });
            };
            // The first line of a lead run's body continues on its lead's last line.
            let indent = if line_index == 0 {
                run.first_line_indent_px
            } else {
                0.0
            };
            let x = match run.align {
                TextAlign::Start => bounds.x + indent,
                TextAlign::Center => bounds.x + (bounds.width - line.width_px) / 2.0,
            };
            let y = bounds.y + line.baseline_px;
            self.line(
                depth,
                &format!(
                    r#"<text x="{}" y="{}"{transform} xml:space="preserve" font-family="{}" font-size="{}" font-weight="{}"{letter_spacing} fill="{}">{}</text>"#,
                    format_number(x),
                    format_number(y),
                    style.family.css_name(),
                    format_number(style.size_px),
                    style.weight.css_value(),
                    fill,
                    escape_xml(content)
                ),
            );
            self.text_elements += 1;
        }
        Ok(())
    }
}

/// The section 2.9 style of a text part, from the node that owns it and, for a Box, its
/// container kind's label style. Layout resolves the center color into each run; the other
/// themes pick theirs by the style's role.
fn text_style_name(
    document_node: DocumentNode<'_>,
    container: Option<ContainerLook>,
    part: PartName,
) -> Option<TextStyleName> {
    match (document_node, part) {
        (DocumentNode::Kicker, PartName::BadgeText) => Some(TextStyleName::Badge),
        (DocumentNode::Kicker, PartName::Text) => Some(TextStyleName::Kicker),
        (DocumentNode::Title, PartName::Text) => Some(TextStyleName::Title),
        (DocumentNode::Lede, PartName::Text) => Some(TextStyleName::Lede),
        (DocumentNode::Foot, PartName::Text) => Some(TextStyleName::Foot),
        (DocumentNode::LegendEntry(_), PartName::LegendLabel) => Some(TextStyleName::LegendLabel),
        (DocumentNode::LegendEntry(_), PartName::LegendText) => Some(TextStyleName::LegendText),
        (DocumentNode::Content(NodeRef::TeeArm(_)), part) => pipe_text_style_name(part),
        (DocumentNode::Content(NodeRef::Node(Node::Box(_))), PartName::Label) => {
            container.map(|look| container_label_style(look.label))
        }
        (DocumentNode::Content(NodeRef::Node(content)), part) => {
            content_text_style_name(content, part)
        }
        _ => None,
    }
}

fn content_text_style_name(content: &Node, part: PartName) -> Option<TextStyleName> {
    match (content, part) {
        (Node::Note(note), PartName::Text) => Some(match note.kind {
            NoteKind::Kicker => TextStyleName::Kicker,
            NoteKind::H1 => TextStyleName::Title,
            NoteKind::Lede => TextStyleName::Lede,
            NoteKind::Legend => TextStyleName::NoteLegend,
            NoteKind::Foot => TextStyleName::Foot,
        }),
        (Node::Item(_), PartName::FunctionName) => Some(TextStyleName::CardFunction),
        (Node::Item(_), PartName::ProductName) => Some(TextStyleName::CardProduct),
        (Node::Item(_), PartName::Fact) => Some(fact_presentation(FactSource::Doc).1),
        (Node::Item(_), PartName::Built) => Some(fact_presentation(FactSource::Built).1),
        (Node::Item(_), PartName::Ask) => Some(fact_presentation(FactSource::Ask).1),
        (Node::Fact(fact), PartName::Text) => Some(fact_presentation(fact.source).1),
        (Node::Pipe(_), part) => pipe_text_style_name(part),
        (Node::Tee(_), PartName::HubText) => Some(TextStyleName::TagLabel),
        (Node::Text(_) | Node::Callout(_), PartName::Heading) => Some(TextStyleName::CardFunction),
        (Node::Text(_), PartName::BodyLine | PartName::Marker) => Some(TextStyleName::BlockBody),
        (Node::Callout(_), PartName::Text) => Some(TextStyleName::BlockBody),
        (Node::Frame(_), PartName::Label) => Some(TextStyleName::ZoneLabel),
        _ => None,
    }
}

fn pipe_text_style_name(part: PartName) -> Option<TextStyleName> {
    match part {
        PartName::TagLabel => Some(TextStyleName::TagLabel),
        PartName::TagSub => Some(TextStyleName::TagSub),
        _ => None,
    }
}

/// A Box whose geometry carries no container look.
fn container_mismatch(node: &NodeGeometry) -> RenderError {
    RenderError::GeometryMismatch {
        expected: node.pointer.clone(),
        found: node.pointer.child("kind"),
    }
}

fn part_mismatch(node: &NodeGeometry, part: &Part) -> RenderError {
    RenderError::GeometryMismatch {
        expected: node.pointer.clone(),
        found: node.pointer.child(part.name.as_str()),
    }
}

/// The 36 px square centered on an icon part.
fn icon_chip_box(icon_bounds: BoxRect) -> BoxRect {
    let center_x = icon_bounds.x + icon_bounds.width / 2.0;
    let center_y = icon_bounds.y + icon_bounds.height / 2.0;
    BoxRect {
        x: center_x - ICON_CHIP_SIZE_PX / 2.0,
        y: center_y - ICON_CHIP_SIZE_PX / 2.0,
        width: ICON_CHIP_SIZE_PX,
        height: ICON_CHIP_SIZE_PX,
    }
}

/// The lines of one grid axis between its zone `edges`, as (position, major): a major line
/// on each inner edge and minor lines cutting each zone into equal parts. Each line is
/// placed from its zone's start, so no rounding carries from one zone into the next.
fn grid_lines(edges: &[f32]) -> Vec<(f32, bool)> {
    let mut lines = Vec::new();
    for (zone, bounds) in edges.windows(2).enumerate() {
        let [start, end] = bounds else { continue };
        let size = end - start;
        let parts = (size / SHEET_GRID_PITCH_PX)
            .round()
            .clamp(1.0, SHEET_GRID_PARTS_MAX);
        if zone > 0 {
            lines.push((*start, true));
        }
        // The clamp keeps the count small and whole, so the cast is exact.
        for part in 1..parts as usize {
            lines.push((start + part as f32 * size / parts, false));
        }
    }
    lines
}

fn stroke_attributes(stroke: Stroke<'_>) -> String {
    let dash = match stroke.line {
        LineStyle::Solid => String::new(),
        LineStyle::Dashed => format!(r#" stroke-dasharray="{}""#, palette::DASH_ARRAY),
        LineStyle::Dotted => format!(r#" stroke-dasharray="{}""#, palette::DOT_ARRAY),
    };
    format!(
        r#" stroke="{}" stroke-width="{}"{dash}"#,
        stroke.color,
        format_number(stroke.width_px)
    )
}

/// The box whose corners the Frame diagonals join: inside the border, pulled in to where
/// the inner edge of the rounded corner crosses a 45 degree line.
fn frame_diagonal_box(bounds: BoxRect, border_width_px: f32) -> BoxRect {
    let inner_radius = (FRAME_RADIUS_PX - border_width_px).max(0.0);
    let corner_pull = inner_radius * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    inset_rect(bounds, border_width_px + corner_pull)
}

fn inset_rect(bounds: BoxRect, inset: f32) -> BoxRect {
    BoxRect {
        x: bounds.x + inset,
        y: bounds.y + inset,
        width: (bounds.width - 2.0 * inset).max(0.0),
        height: (bounds.height - 2.0 * inset).max(0.0),
    }
}

/// A rectangle path with per-corner radii in CSS order: top-left, top-right, bottom-right,
/// bottom-left.
fn rounded_path(bounds: BoxRect, radii: [f32; 4]) -> String {
    let [top_left, top_right, bottom_right, bottom_left] = radii;
    let left = bounds.x;
    let top = bounds.y;
    let right = bounds.right();
    let bottom = bounds.bottom();
    let number = format_number;
    format!(
        "M {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
        number(left + top_left),
        number(top),
        number(right - top_right),
        number(top_right),
        number(top_right),
        number(right),
        number(top + top_right),
        number(bottom - bottom_right),
        number(bottom_right),
        number(bottom_right),
        number(right - bottom_right),
        number(bottom),
        number(left + bottom_left),
        number(bottom_left),
        number(bottom_left),
        number(left),
        number(bottom - bottom_left),
        number(top + top_left),
        number(top_left),
        number(top_left),
        number(left + top_left),
        number(top),
    )
}

/// `M x y` at the drawn start, then each piece: `L` for a straight, `A` for an arc, `C` for a
/// curve with both control points on its corner, `Q` for a spline piece.
fn link_path(drawn: &DrawnLink) -> String {
    let number = format_number;
    let mut path = format!("M {} {}", number(drawn.start.x), number(drawn.start.y));
    for piece in drawn.legs.iter().flatten() {
        let to = piece.to();
        let (to_x, to_y) = (number(to.x), number(to.y));
        match *piece {
            PathPiece::Line(_) => path.push_str(&format!(" L {to_x} {to_y}")),
            PathPiece::Arc {
                radius, clockwise, ..
            } => path.push_str(&format!(
                " A {radius} {radius} 0 0 {} {to_x} {to_y}",
                u8::from(clockwise),
                radius = number(radius),
            )),
            PathPiece::Cubic { corner, .. } => path.push_str(&format!(
                " C {x} {y} {x} {y} {to_x} {to_y}",
                x = number(corner.x),
                y = number(corner.y),
            )),
            PathPiece::Quad { corner, .. } => path.push_str(&format!(
                " Q {} {} {to_x} {to_y}",
                number(corner.x),
                number(corner.y),
            )),
        }
    }
    path
}

/// The left `bounds.width` px of a rounded rectangle whose left corners have `radius`,
/// as a closed path: flat right edge, left corners following the curve.
fn left_rounded_strip_path(bounds: BoxRect, radius: f32) -> String {
    let left = bounds.x;
    let right = bounds.right();
    let top = bounds.y;
    let bottom = bounds.bottom();
    let radius = radius.min(bounds.height / 2.0).max(0.0);
    // How far below the box top the corner curve meets the strip's right edge.
    let reach = (radius - bounds.width).max(0.0);
    let curve_drop = radius - (radius * radius - reach * reach).max(0.0).sqrt();
    let number = format_number;
    if bounds.width >= radius {
        return format!(
            "M {} {} H {} V {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
            number(left + radius),
            number(top),
            number(right),
            number(bottom),
            number(left + radius),
            number(radius),
            number(radius),
            number(left),
            number(bottom - radius),
            number(top + radius),
            number(radius),
            number(radius),
            number(left + radius),
            number(top),
        );
    }
    format!(
        "M {} {} V {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
        number(right),
        number(top + curve_drop),
        number(bottom - curve_drop),
        number(radius),
        number(radius),
        number(left),
        number(bottom - radius),
        number(top + radius),
        number(radius),
        number(radius),
        number(right),
        number(top + curve_drop),
    )
}

/// How far a faded unit drops while another element is highlighted (section 5.2).
const HOVER_FADED_OPACITY: &str = "0.2";
/// The families the written file's text falls back through when the embedded faces do not
/// load (section 5.2).
const TEXT_FONT_STACK: &str =
    r#"Inter, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif"#;
/// How long the fade takes each way.
const HOVER_TRANSITION: &str = "opacity 0.15s ease-out";

impl SvgDocument {
    /// The SVG with one `@font-face` per entry of `font_faces`, its bundled file as a `data:`
    /// URI, and one rule per entry of `hover_rules`, in a `<style>` right after the opening
    /// `<svg>` tag, so the file renders in Inter where Inter is not installed and highlights
    /// what an element refers to under the pointer (section 5.2). A document with no text and
    /// no rule is returned as is.
    pub fn self_contained(&self) -> Result<String, RenderError> {
        let Some((open_tag, rest)) = self.svg.split_once('\n') else {
            return Err(RenderError::Svg {
                message: "SVG has no line after its opening tag".to_string(),
            });
        };
        if !open_tag.starts_with("<svg ") {
            return Err(RenderError::Svg {
                message: format!("SVG starts with {open_tag:.40} instead of <svg"),
            });
        }
        if self.font_faces.is_empty() && self.hover_rules.is_empty() {
            return Ok(self.svg.clone());
        }
        let mut output = String::new();
        output.push_str(open_tag);
        output.push_str("\n  <defs>\n    <style>\n");
        for (family, weight) in &self.font_faces {
            let file = bundled_font(*family, *weight)?;
            output.push_str(&format!(
                r#"      @font-face {{ font-family: "{}"; font-style: normal; font-weight: {}; src: url("data:font/ttf;base64,{}") format("truetype"); }}"#,
                family.css_name(),
                weight.css_value(),
                base64::engine::general_purpose::STANDARD.encode(file.bytes)
            ));
            output.push('\n');
        }
        if !self.font_faces.is_empty() {
            // A style rule outranks the `font-family` attribute, so where the embedded faces
            // are blocked and Inter is not installed the text falls back to a sans face.
            output.push_str(&format!(
                "      text {{ font-family: {TEXT_FONT_STACK}; }}\n"
            ));
        }
        if !self.hover_rules.is_empty() {
            output.push_str(&format!(
                "      {} > :not([data-id]), [data-row] {{ transition: {HOVER_TRANSITION}; }}\n",
                fade_groups_selector()
            ));
        }
        for rule in &self.hover_rules {
            output.push_str("      ");
            output.push_str(&escape_xml_text(&hover_rule(rule)));
            output.push('\n');
        }
        output.push_str("    </style>\n  </defs>\n");
        output.push_str(rest);
        Ok(output)
    }
}

/// Every group that fades: the groups of the tags in `related::FADE_TAGS`, and links.
fn fade_groups_selector() -> String {
    let tags: Vec<String> = related::FADE_TAGS
        .iter()
        .map(|tag| tag.as_str())
        .chain(["Link"])
        .map(|tag| format!(r#"[data-tag="{tag}"]"#))
        .collect();
    format!(":is({})", tags.join(","))
}

/// One rule that fades every group and row outside the rule's kept sets while the pointer
/// rests on the source or the URL fragment names its anchor. A group fades its own drawn
/// elements, not its child groups; a group source is hovered through those elements and a
/// row through its own or through any mark of its key.
fn hover_rule(rule: &HoverRule) -> String {
    let anchor = css_string(&rule.anchor);
    let source = match &rule.source {
        HoverSource::Group(pointer) => format!(
            "[data-id={}] > :not([data-id], [data-mark]):hover, [id={anchor}]:target",
            css_string(pointer)
        ),
        HoverSource::Row(pointer) => {
            let pointer = css_string(pointer);
            format!("[data-row={pointer}]:hover, [data-mark={pointer}]:hover, [id={anchor}]:target")
        }
    };
    let highlighted = format!("svg:has({source})");
    let kept_groups: String = rule
        .kept_groups
        .iter()
        .map(|pointer| format!(":not([data-id={}])", css_string(pointer)))
        .collect();
    let kept_rows: String = rule
        .kept_rows
        .iter()
        .map(|pointer| format!(":not([data-row={}])", css_string(pointer)))
        .collect();
    format!(
        "{highlighted} {}{kept_groups} > :not([data-id]), {highlighted} [data-row]{kept_rows} {{ opacity: {HOVER_FADED_OPACITY}; }}",
        fade_groups_selector()
    )
}

/// `text` as a double-quoted CSS string.
fn css_string(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\a "),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// `text` escaped for element content: `&` and `<`. Quotes and `>`, the CSS child
/// combinator, stand as they are; a rule never holds `]]>`, since every attribute selector
/// closes with `]` before a `)`, `:` or space.
fn escape_xml_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// The sheet drawn right after a stack's back sheet `part`: the next back sheet or the
/// front, which covers it in part.
fn sheet_in_front_of(node: &NodeGeometry, part: &Part) -> Option<BoxRect> {
    let index = node
        .parts
        .iter()
        .position(|candidate| std::ptr::eq(candidate, part))?;
    node.parts
        .iter()
        .skip(index + 1)
        .find(|candidate| matches!(candidate.name, PartName::StackBack | PartName::Front))
        .map(|candidate| candidate.bounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_without_text_is_self_contained_as_written() {
        let document = SvgDocument {
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\">\n</svg>\n".to_string(),
            text_elements: 0,
            font_faces: Vec::new(),
            hover_rules: Vec::new(),
        };
        assert_eq!(document.self_contained().unwrap(), document.svg);
    }

    #[test]
    fn self_contained_rejects_a_document_that_does_not_open_with_svg() {
        let document = SvgDocument {
            svg: "<g>\n</g>\n".to_string(),
            text_elements: 1,
            font_faces: vec![(FontFamily::Inter, FontWeight::Bold)],
            hover_rules: Vec::new(),
        };
        assert!(matches!(
            document.self_contained(),
            Err(RenderError::Svg { .. })
        ));
    }

    #[test]
    fn css_string_escapes_quotes_backslashes_and_newlines() {
        assert_eq!(css_string("/body/0"), r#""/body/0""#);
        assert_eq!(css_string("a\"b\\c\nd"), r#""a\"b\\c\a d""#);
    }

    #[test]
    fn a_group_rule_fades_every_unit_outside_its_kept_sets() {
        let rule = HoverRule {
            source: HoverSource::Group("/body/0".to_string()),
            anchor: "api".to_string(),
            kept_groups: vec!["/body/0".to_string(), "/links/1".to_string()],
            kept_rows: vec!["/body/2/parts/0".to_string()],
        };
        let highlighted = r#"svg:has([data-id="/body/0"] > :not([data-id], [data-mark]):hover, [id="api"]:target)"#;
        assert_eq!(
            hover_rule(&rule),
            format!(
                r#"{highlighted} :is([data-tag="Pcard"],[data-tag="Zone"],[data-tag="Pipe"],[data-tag="Tee"],[data-tag="Step"],[data-tag="Rung"],[data-tag="ProblemPoint"],[data-tag="ActorChip"],[data-tag="LegendEntry"],[data-tag="HowToItem"],[data-tag="Link"]):not([data-id="/body/0"]):not([data-id="/links/1"]) > :not([data-id]), {highlighted} [data-row]:not([data-row="/body/2/parts/0"]) {{ opacity: 0.2; }}"#
            )
        );
    }

    #[test]
    fn a_row_rule_is_hovered_through_its_row_and_the_marks_of_its_key() {
        let rule = HoverRule {
            source: HoverSource::Row("/body/2/notes/1".to_string()),
            anchor: "_body_2_notes_1".to_string(),
            kept_groups: vec!["/body/0".to_string()],
            kept_rows: vec!["/body/2/notes/1".to_string()],
        };
        let written = hover_rule(&rule);
        assert!(written.starts_with(r#"svg:has([data-row="/body/2/notes/1"]:hover, [data-mark="/body/2/notes/1"]:hover, [id="_body_2_notes_1"]:target) "#));
        assert!(
            written.ends_with(r#"[data-row]:not([data-row="/body/2/notes/1"]) { opacity: 0.2; }"#)
        );
    }

    #[test]
    fn a_document_with_a_rule_and_no_text_still_gets_its_style_block() {
        let document = SvgDocument {
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\">\n</svg>\n".to_string(),
            text_elements: 0,
            font_faces: Vec::new(),
            hover_rules: vec![HoverRule {
                source: HoverSource::Group("/body/0".to_string()),
                anchor: "_body_0".to_string(),
                kept_groups: vec!["/body/0".to_string(), "/body/1".to_string()],
                kept_rows: Vec::new(),
            }],
        };
        let written = document.self_contained().unwrap();
        assert_eq!(
            written
                .lines()
                .filter(|line| line.contains(":hover"))
                .count(),
            1
        );
        assert!(!written.contains("@font-face"));
        assert!(written.contains(r#"svg:has([data-id="/body/0"] > "#));
    }

    #[test]
    fn escape_xml_escapes_markup_characters() {
        assert_eq!(
            escape_xml(r#"a < b & "c" > 'd'"#),
            "a &lt; b &amp; &quot;c&quot; &gt; &apos;d&apos;"
        );
    }

    #[test]
    fn each_piece_writes_its_path_command() {
        let at = |x: f32, y: f32| PagePoint { x, y };
        let drawn = DrawnLink {
            start: at(0.0, 0.0),
            legs: vec![
                vec![
                    PathPiece::Line(at(96.0, 0.0)),
                    PathPiece::Arc {
                        radius: 4.0,
                        clockwise: true,
                        corner: at(100.0, 0.0),
                        to: at(100.0, 4.0),
                    },
                ],
                vec![
                    PathPiece::Line(at(100.0, 4.0)),
                    PathPiece::Cubic {
                        corner: at(100.0, 8.0),
                        to: at(104.0, 8.0),
                    },
                ],
                vec![
                    PathPiece::Line(at(150.0, 8.0)),
                    PathPiece::Quad {
                        corner: at(200.0, 8.0),
                        to: at(200.0, 30.5),
                    },
                ],
                vec![PathPiece::Line(at(200.0, 53.0))],
            ],
            start_arrow: None,
            end_arrow: None,
            cap: stencil_layout::EndCap::Arrowhead,
        };
        assert_eq!(
            link_path(&drawn),
            "M 0 0 L 96 0 A 4 4 0 0 1 100 4 L 100 4 C 100 8 100 8 104 8 L 150 8 Q 200 8 200 30.5 L 200 53"
        );
    }

    #[test]
    fn left_rounded_strip_follows_the_corner_curve() {
        let bounds = BoxRect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 40.0,
        };
        assert_eq!(
            left_rounded_strip_path(bounds, 6.75),
            "M 4 0.59 V 39.41 A 6.75 6.75 0 0 1 0 33.25 V 6.75 A 6.75 6.75 0 0 1 4 0.59 Z"
        );
    }

    #[test]
    fn rounded_path_places_each_corner_radius() {
        let bounds = BoxRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(
            rounded_path(bounds, [4.0, 4.0, 10.0, 10.0]),
            "M 4 0 H 96 A 4 4 0 0 1 100 4 V 40 A 10 10 0 0 1 90 50 H 10 A 10 10 0 0 1 0 40 V 4 A 4 4 0 0 1 4 0 Z"
        );
    }
}
