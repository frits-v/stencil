//! The section 5.2 SVG writer, and through `iso_writer` the section 12.5 one.

mod iso_writer;

use std::collections::BTreeSet;
use stencil_layout::{
    BoxRect, ContainerLook, DrawnLink, LinkRoute, NodeGeometry, PageGeometry, Part, PartName,
    PathPiece, TextAlign, TextRun, container_label_style, drawn_link, fact_presentation,
};
use stencil_model::grammar::Role;
use stencil_model::pointer::NodePointer;
use stencil_model::text::{FontFamily, FontWeight, TextStyleName};

use base64::Engine as _;
use stencil_model::text::TextMeasurer;
use stencil_model::{
    Arrow, Canvas, Chrome, FactSource, IconName, LEGEND_ENTRIES_MAX, LINKS_MAX, LegendEntry, Link,
    Node, NodeRef, NoteKind, Page, PagePoint, Pipe, PipeDir, PipeForm, Projection, Theme,
    body_nodes,
};
use stencil_text::{CosmicTextMeasurer, bundled_font};

use crate::icons::icon_data_uri;
use crate::palette::{self, BoxPaint, DotStyle, LineStyle, LineUse, Palette, Stroke};
use crate::{RenderError, SvgDocument, format_number};

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
/// Flat radius of a pipe end dot; an arrowhead's tip sits this far out from the dot center.
pub(crate) const DOT_RADIUS_PX: f32 = 4.0;
const BADGE_RADIUS_PX: f32 = 4.0;
const CARD_RADIUS_PX: f32 = 8.0;
const FACT_RADIUS_PX: f32 = 4.0;
const TAG_RADIUS_PX: f32 = 6.0;
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
}

/// Walks page and geometry in the section 4.4 geometry order and asserts the pointers match,
/// painting every role from `theme`.
pub fn render_svg(
    page: &Page,
    theme: &Theme,
    geometry: &PageGeometry,
) -> Result<SvgDocument, RenderError> {
    let expected = geometry_order(page);
    if let Some(mismatch) = first_mismatch(&expected, geometry) {
        return Err(mismatch);
    }
    if page.projection == Projection::Iso {
        return iso_writer::render_iso(page, theme, geometry, &expected);
    }

    let mut writer = SvgWriter::new(page.canvas, Palette::new(theme, Projection::Flat));
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
    let arrow_kinds = arrow_kinds(&expected, &page.links);
    if !arrow_kinds.is_empty() {
        writer.write_arrow_markers(1, &arrow_kinds);
    }

    // Geometry order is pre-order, so a stack of open groups reproduces the nesting.
    let mut open_groups: Vec<usize> = Vec::new();
    for (index, (node, (_, document_node))) in geometry.nodes.iter().zip(&expected).enumerate() {
        close_groups_until_parent(&mut writer, &mut open_groups, geometry, index)?;
        let depth = open_groups.len() + 1;
        writer.line(depth, &group_open_tag(node));
        let relabeled = relabeler.relabeled(writer.palette, node, *document_node)?;
        writer.write_node(
            depth + 1,
            relabeled.as_ref().unwrap_or(node),
            *document_node,
        )?;
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
        writer.write_link(1, geometry, route)?;
    }
    writer.line(0, "</svg>");

    Ok(writer.into_document())
}

/// Pointers and document nodes in section 4.4 geometry order. The body portion is
/// `body_nodes`, which is bounded by NODES_MAX + 1, and the legend portion stops after
/// LEGEND_ENTRIES_MAX + 1 entries; a longer geometry then fails as a mismatch.
fn geometry_order(page: &Page) -> Vec<(NodePointer, DocumentNode<'_>)> {
    let root = NodePointer::root();
    let mut order = vec![(root.clone(), DocumentNode::Page)];
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
    if !page.legend.is_empty() {
        let legend_pointer = root.child("legend");
        order.push((legend_pointer.clone(), DocumentNode::Legend));
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
    if page.foot.is_some() {
        order.push((root.child("foot"), DocumentNode::Foot));
    }
    order
}

/// Lines of the Pipes, Tee arms and links that draw an arrowhead, one marker each.
fn arrow_kinds(expected: &[(NodePointer, DocumentNode<'_>)], links: &[Link]) -> BTreeSet<LineUse> {
    let pipe_kinds = expected
        .iter()
        .filter_map(|(_, document_node)| match *document_node {
            DocumentNode::Content(NodeRef::Node(Node::Pipe(pipe)) | NodeRef::TeeArm(pipe)) => {
                Some(pipe)
            }
            _ => None,
        })
        .filter(|pipe| pipe.arrow != Arrow::None)
        .map(|pipe| LineUse::new(pipe.line, pipe.tint));
    let link_kinds = links
        .iter()
        .take(LINKS_MAX)
        .filter(|link| link.arrow != Arrow::None)
        .map(|link| LineUse::new(link.line, link.tint));
    pipe_kinds.chain(link_kinds).collect()
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

fn group_open_tag(node: &NodeGeometry) -> String {
    let kind = node
        .kind
        .as_deref()
        .map(|kind| format!(r#" data-kind="{}""#, escape_xml(kind)))
        .unwrap_or_default();
    format!(
        r#"<g data-id="{}" data-tag="{}"{kind}>"#,
        escape_xml(node.pointer.as_str()),
        node.tag.as_str()
    )
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
        let Some(label) = palette.legend_relabel(LineUse::new(entry.line, entry.tint)) else {
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
}

struct SvgWriter<'a> {
    output: String,
    text_elements: usize,
    /// The family and weight of every `<text>` written, each once.
    font_faces: Vec<(FontFamily, FontWeight)>,
    canvas: Canvas,
    palette: Palette<'a>,
    /// Section 12.2 rule 7: the zoom of an iso scene, which tube radii follow; 1 in flat.
    iso_zoom: f32,
}

impl<'a> SvgWriter<'a> {
    fn new(canvas: Canvas, palette: Palette<'a>) -> Self {
        SvgWriter {
            output: String::new(),
            text_elements: 0,
            font_faces: Vec::new(),
            canvas,
            palette,
            iso_zoom: 1.0,
        }
    }

    fn line(&mut self, depth: usize, content: &str) {
        for _ in 0..depth {
            self.output.push_str("  ");
        }
        self.output.push_str(content);
        self.output.push('\n');
    }

    /// The written SVG, with its faces ordered by family name and then weight.
    fn into_document(self) -> SvgDocument {
        let mut font_faces = self.font_faces;
        font_faces.sort_by_key(|(family, weight)| (family.css_name(), weight.css_value()));
        SvgDocument {
            svg: self.output,
            text_elements: self.text_elements,
            font_faces,
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
        for part in &node.parts {
            self.write_part_shape(depth, node, part, context)?;
        }
        if let DocumentNode::Content(NodeRef::Node(Node::Box(_))) = document_node
            && let Some(look) = node.container
            && look.role == Role::Frame
        {
            self.write_gcp_frame_stroke(depth, node.bounds, look, node.tint);
        }
        for part in &node.parts {
            if let Some(run) = &part.text {
                let fill = self.text_fill(node, document_node, part, context)?;
                self.write_text_run(depth, &node.pointer, part.bounds, run, fill)?;
            }
        }
        Ok(())
    }

    /// The ink of a text part: a Box label by its container's paint, every other run by
    /// its section 2.9 style.
    fn text_fill(
        &self,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        part: &Part,
        context: PartContext<'a>,
    ) -> Result<&'a str, RenderError> {
        if let DocumentNode::Content(NodeRef::Node(Node::Box(_))) = document_node
            && part.name == PartName::Label
        {
            let look = node.container.ok_or_else(|| container_mismatch(node))?;
            return Ok(self.palette.container_label_ink(look, node.tint));
        }
        let style_name = text_style_name(document_node, node.container, part.name)
            .ok_or_else(|| part_mismatch(node, part))?;
        Ok(self.palette.text_ink(
            style_name,
            self.canvas,
            context.line.map(|line_use| line_use.line),
        ))
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
                Node::Box(_) => {
                    let look = node.container.ok_or_else(|| container_mismatch(node))?;
                    self.write_zone_box(depth, node.bounds, look, node.tint);
                }
                Node::Item(_) => {
                    let card_paint = self.palette.card();
                    self.write_box(depth, node.bounds, CARD_RADIUS_PX, card_paint);
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
        Ok(self.part_context(document_node))
    }

    /// The document facts the parts of a node need, without drawing anything.
    fn part_context(&self, document_node: DocumentNode<'_>) -> PartContext<'a> {
        match document_node {
            DocumentNode::Page
            | DocumentNode::Kicker
            | DocumentNode::Title
            | DocumentNode::Lede
            | DocumentNode::Body
            | DocumentNode::Legend
            | DocumentNode::Foot => PartContext::default(),
            DocumentNode::LegendEntry(entry) => PartContext {
                line: Some(LineUse::new(entry.line, entry.tint)),
                pipe_form: Some(entry.form),
                ..PartContext::default()
            },
            DocumentNode::Content(NodeRef::TeeArm(pipe)) => PartContext {
                line: Some(LineUse::new(pipe.line, pipe.tint)),
                pipe_dir: Some(pipe.dir),
                pipe_form: Some(pipe.form),
                arrows: ArrowEnds::of(pipe),
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
                | Node::Fact(_) => PartContext::default(),
                Node::Callout(callout) => PartContext {
                    accent: Some(self.palette.callout(callout.kind).accent),
                    ..PartContext::default()
                },
                Node::Item(item) => PartContext {
                    icon: item.icon,
                    ..PartContext::default()
                },
                Node::Pipe(pipe) => PartContext {
                    line: Some(LineUse::new(pipe.line, pipe.tint)),
                    pipe_dir: Some(pipe.dir),
                    pipe_form: Some(pipe.form),
                    arrows: ArrowEnds::of(pipe),
                    ..PartContext::default()
                },
                Node::Tee(tee) => PartContext {
                    line: Some(LineUse::new(tee.line, tee.tint)),
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
        let style = self.palette.zone_style(look, tint);
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
        let Some(border) = self.palette.zone_style(look, tint).border else {
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

    fn write_part_shape(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        part: &Part,
        context: PartContext<'a>,
    ) -> Result<(), RenderError> {
        let bounds = part.bounds;
        match part.name {
            // The solid draws the footprint under iso; it has no flat drawing.
            PartName::Footprint => {}
            PartName::Badge => {
                let badge_paint = self.palette.badge(self.canvas);
                self.write_box(depth, bounds, BADGE_RADIUS_PX, badge_paint);
            }
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
                    self.write_arrowhead(depth, bounds, dir, at_start, kind);
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
            PartName::Swatch if context.pipe_form == Some(PipeForm::Band) => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                self.write_band_swatch(depth, bounds, kind);
            }
            PartName::Swatch => {
                let kind = context.line.ok_or_else(|| part_mismatch(node, part))?;
                self.write_wire(depth, bounds, PipeDir::Horizontal, kind);
                // A kind told apart by its hollow dots shows them on its swatch too, inset
                // so they stay within the swatch's width.
                if self.palette.wire_style(kind).dot == DotStyle::Hollow {
                    let center_y = bounds.y + bounds.height / 2.0;
                    self.write_dot(depth, bounds.x + DOT_RADIUS_PX, center_y, kind);
                    self.write_dot(depth, bounds.right() - DOT_RADIUS_PX, center_y, kind);
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
                let fill = self.palette.list_bullet(self.canvas);
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
        }
        Ok(())
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

    /// An arrowhead in place of an end dot, pointing out of the pipe with its tip on the dot
    /// box's outer edge. The carrier line is unstroked and only places the marker.
    fn write_arrowhead(
        &mut self,
        depth: usize,
        dot_bounds: BoxRect,
        dir: PipeDir,
        at_start: bool,
        kind: LineUse,
    ) {
        let center_x = dot_bounds.x + dot_bounds.width / 2.0;
        let center_y = dot_bounds.y + dot_bounds.height / 2.0;
        let ((base_x, base_y), (tip_x, tip_y)) = match (dir, at_start) {
            (PipeDir::Horizontal, true) => (
                (dot_bounds.x + ARROW_LENGTH_PX, center_y),
                (dot_bounds.x, center_y),
            ),
            (PipeDir::Horizontal, false) => (
                (dot_bounds.right() - ARROW_LENGTH_PX, center_y),
                (dot_bounds.right(), center_y),
            ),
            (PipeDir::Vertical, true) => (
                (center_x, dot_bounds.y + ARROW_LENGTH_PX),
                (center_x, dot_bounds.y),
            ),
            (PipeDir::Vertical, false) => (
                (center_x, dot_bounds.bottom() - ARROW_LENGTH_PX),
                (center_x, dot_bounds.bottom()),
            ),
        };
        let marker_id = self.arrow_marker_id(kind);
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="none" marker-end="url(#{marker_id})"/>"#,
                format_number(base_x),
                format_number(base_y),
                format_number(tip_x),
                format_number(tip_y),
            ),
        );
    }

    /// One link: the routed path in the wire style of its kind, an arrowhead at each end the
    /// `arrow` value names, then the tag, drawn like a pipe tag over the path under it.
    fn write_link(
        &mut self,
        depth: usize,
        geometry: &PageGeometry,
        route: &LinkRoute,
    ) -> Result<(), RenderError> {
        let pointer = NodePointer::root().child("links").index(route.index);
        self.line(
            depth,
            &format!(
                r#"<g data-id="{}" data-tag="Link" data-kind="{}">"#,
                escape_xml(pointer.as_str()),
                route.key()
            ),
        );
        let line_use = LineUse::new(route.line, route.tint);
        let drawn = drawn_link(geometry, route);
        let stroke = self.palette.wire_style(line_use).stroke;
        self.line(
            depth + 1,
            &format!(
                r#"<path d="{}" fill="none"{} stroke-linejoin="round"/>"#,
                link_path(&drawn),
                stroke_attributes(stroke)
            ),
        );
        for (base, tip) in [drawn.start_arrow, drawn.end_arrow].into_iter().flatten() {
            self.write_arrow_carrier(depth + 1, base, tip, line_use);
        }
        for part in &route.parts {
            match (part.name, &part.text) {
                (PartName::Tag, None) => {
                    let tag_paint = self.palette.tag(route.line);
                    self.write_box(depth + 1, part.bounds, TAG_RADIUS_PX, tag_paint);
                }
                (PartName::TagLabel | PartName::TagSub, Some(run)) => {
                    let style_name =
                        pipe_text_style_name(part.name).ok_or_else(|| link_mismatch(route))?;
                    let fill = self
                        .palette
                        .text_ink(style_name, self.canvas, Some(route.line));
                    self.write_text_run(depth + 1, &pointer, part.bounds, run, fill)?;
                }
                _ => return Err(link_mismatch(route)),
            }
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
        for line in &run.metrics.lines {
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
            let x = match run.align {
                TextAlign::Start => bounds.x,
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

impl SvgDocument {
    /// The SVG with one `@font-face` per entry of `font_faces`, its bundled file as a `data:`
    /// URI, in a `<style>` right after the opening `<svg>` tag, so the file renders in Inter
    /// where Inter is not installed (section 5.2). A document with no text is returned as is.
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
        if self.font_faces.is_empty() {
            return Ok(self.svg.clone());
        }
        let mut output = String::new();
        output.push_str(open_tag);
        output.push_str("\n  <defs>\n    <style>\n");
        for (family, weight) in &self.font_faces {
            let file = bundled_font(*family, *weight);
            output.push_str(&format!(
                r#"      @font-face {{ font-family: "{}"; font-style: normal; font-weight: {}; src: url("data:font/ttf;base64,{}") format("truetype"); }}"#,
                family.css_name(),
                weight.css_value(),
                base64::engine::general_purpose::STANDARD.encode(file.bytes)
            ));
            output.push('\n');
        }
        output.push_str("    </style>\n  </defs>\n");
        output.push_str(rest);
        Ok(output)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_without_text_is_self_contained_as_written() {
        let document = SvgDocument {
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\">\n</svg>\n".to_string(),
            text_elements: 0,
            font_faces: Vec::new(),
        };
        assert_eq!(document.self_contained().unwrap(), document.svg);
    }

    #[test]
    fn self_contained_rejects_a_document_that_does_not_open_with_svg() {
        let document = SvgDocument {
            svg: "<g>\n</g>\n".to_string(),
            text_elements: 1,
            font_faces: vec![(FontFamily::Inter, FontWeight::Bold)],
        };
        assert!(matches!(
            document.self_contained(),
            Err(RenderError::Svg { .. })
        ));
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
