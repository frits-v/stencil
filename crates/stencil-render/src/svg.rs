//! The section 5.2 SVG writer.

use stencil_layout::styles::badge_fill;
use stencil_layout::{BoxRect, NodeGeometry, PageGeometry, Part, PartName, TextAlign, TextRun};
use stencil_model::pointer::NodePointer;
use stencil_model::{
    Canvas, IconName, LEGEND_ENTRIES_MAX, LegendEntry, Node, NodeRef, Page, PipeDir, PipeKind,
    ZoneKind, body_nodes,
};

use crate::icons::icon_data_uri;
use crate::palette::{self, LineStyle, Stroke};
use crate::{RenderError, SvgDocument, format_number};

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const WIRE_WIDTH_PX: f32 = 2.0;
const DOT_RADIUS_PX: f32 = 4.0;
const BADGE_RADIUS_PX: f32 = 4.0;
const CARD_RADIUS_PX: f32 = 8.0;
const CARD_BORDER_PX: f32 = 1.5;
const FACT_RADIUS_PX: f32 = 4.0;
const TAG_RADIUS_PX: f32 = 6.0;
const TAG_BORDER_PX: f32 = 1.5;
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

/// Walks page and geometry in the section 4.4 geometry order and asserts the pointers match.
pub fn render_svg(page: &Page, geometry: &PageGeometry) -> Result<SvgDocument, RenderError> {
    let expected = geometry_order(page);
    if let Some(mismatch) = first_mismatch(&expected, geometry) {
        return Err(mismatch);
    }

    let mut writer = SvgWriter::new(page.canvas);
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
            palette::CANVAS_FILL
        ),
    );

    // Geometry order is pre-order, so a stack of open groups reproduces the nesting.
    let mut open_groups: Vec<usize> = Vec::new();
    for (index, (node, (_, document_node))) in geometry.nodes.iter().zip(&expected).enumerate() {
        close_groups_until_parent(&mut writer, &mut open_groups, geometry, index)?;
        let depth = open_groups.len() + 1;
        writer.line(depth, &group_open_tag(node));
        writer.write_node(depth + 1, node, *document_node)?;
        open_groups.push(index);
    }
    for depth in (1..=open_groups.len()).rev() {
        writer.line(depth, "</g>");
    }
    writer.line(0, "</svg>");

    Ok(SvgDocument {
        svg: writer.output,
        text_elements: writer.text_elements,
    })
}

/// Pointers and document nodes in section 4.4 geometry order. The body portion is
/// `body_nodes`, which is bounded by NODES_MAX + 1, and the legend portion stops after
/// LEGEND_ENTRIES_MAX + 1 entries; a longer geometry then fails as a mismatch.
fn geometry_order(page: &Page) -> Vec<(NodePointer, DocumentNode<'_>)> {
    let root = NodePointer::root();
    let mut order = vec![
        (root.clone(), DocumentNode::Page),
        (root.child("kicker"), DocumentNode::Kicker),
        (root.child("title"), DocumentNode::Title),
        (root.child("lede"), DocumentNode::Lede),
        (root.child("body"), DocumentNode::Body),
    ];
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
        .map(|kind| format!(r#" data-kind="{}""#, escape_xml(kind)))
        .unwrap_or_default();
    format!(
        r#"<g data-id="{}" data-tag="{}"{kind}>"#,
        escape_xml(node.pointer.as_str()),
        node.tag.as_str()
    )
}

/// Document facts a part needs that geometry does not carry.
#[derive(Debug, Clone, Copy, Default)]
struct PartContext {
    pipe_kind: Option<PipeKind>,
    pipe_dir: Option<PipeDir>,
    icon: Option<IconName>,
}

struct SvgWriter {
    output: String,
    text_elements: usize,
    canvas: Canvas,
}

impl SvgWriter {
    fn new(canvas: Canvas) -> Self {
        SvgWriter {
            output: String::new(),
            text_elements: 0,
            canvas,
        }
    }

    fn line(&mut self, depth: usize, content: &str) {
        for _ in 0..depth {
            self.output.push_str("  ");
        }
        self.output.push_str(content);
        self.output.push('\n');
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
        if let DocumentNode::Content(NodeRef::Node(Node::Zone(zone))) = document_node
            && zone.kind == ZoneKind::Gcp
        {
            self.write_gcp_frame_stroke(depth, node.bounds);
        }
        for part in &node.parts {
            if let Some(run) = &part.text {
                self.write_text_run(depth, &node.pointer, part.bounds, run)?;
            }
        }
        Ok(())
    }

    /// Draws the box of the node itself, below its parts, and returns what the parts need.
    fn write_node_box(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
    ) -> Result<PartContext, RenderError> {
        let context = match document_node {
            DocumentNode::Page
            | DocumentNode::Kicker
            | DocumentNode::Title
            | DocumentNode::Lede
            | DocumentNode::Body
            | DocumentNode::Legend
            | DocumentNode::Foot => PartContext::default(),
            DocumentNode::LegendEntry(entry) => PartContext {
                pipe_kind: Some(entry.kind),
                ..PartContext::default()
            },
            DocumentNode::Content(NodeRef::TeeArm(pipe)) => PartContext {
                pipe_kind: Some(pipe.kind),
                pipe_dir: Some(pipe.dir),
                icon: None,
            },
            DocumentNode::Content(NodeRef::Node(content)) => match content {
                Node::Row(_) | Node::Col(_) | Node::Note(_) => PartContext::default(),
                Node::Zone(zone) => {
                    self.write_zone_box(depth, node.bounds, zone.kind);
                    PartContext::default()
                }
                Node::Pcard(card) => {
                    let border = Stroke {
                        width_px: CARD_BORDER_PX,
                        line: LineStyle::Solid,
                        color: palette::CARD_BORDER,
                    };
                    self.write_rect(
                        depth,
                        node.bounds,
                        CARD_RADIUS_PX,
                        Some(palette::CARD_FILL),
                        Some(border),
                    );
                    PartContext {
                        icon: card.icon,
                        ..PartContext::default()
                    }
                }
                Node::Fact(_) => {
                    self.write_rect(
                        depth,
                        node.bounds,
                        FACT_RADIUS_PX,
                        Some(palette::FACT_FILL),
                        None,
                    );
                    PartContext::default()
                }
                Node::Pipe(pipe) => PartContext {
                    pipe_kind: Some(pipe.kind),
                    pipe_dir: Some(pipe.dir),
                    icon: None,
                },
                Node::Tee(tee) => PartContext {
                    pipe_kind: Some(tee.kind),
                    ..PartContext::default()
                },
            },
        };
        Ok(context)
    }

    fn write_zone_box(&mut self, depth: usize, bounds: BoxRect, kind: ZoneKind) {
        let style = palette::zone_style(kind);
        if kind == ZoneKind::Gcp {
            let fill = style.fill.unwrap_or(palette::GCP_FRAME_FILL);
            let path = rounded_path(bounds, GCP_RADII_PX);
            self.line(depth, &format!(r#"<path d="{path}" fill="{fill}"/>"#));
            return;
        }
        self.write_rect(depth, bounds, style.radius_px, style.fill, style.border);
    }

    /// The frame stroke goes over the bar and body fills, as a CSS border would.
    fn write_gcp_frame_stroke(&mut self, depth: usize, bounds: BoxRect) {
        let Some(border) = palette::zone_style(ZoneKind::Gcp).border else {
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
        context: PartContext,
    ) -> Result<(), RenderError> {
        let bounds = part.bounds;
        match part.name {
            PartName::Badge => {
                let fill = badge_fill(self.canvas);
                self.write_rect(depth, bounds, BADGE_RADIUS_PX, Some(fill), None);
            }
            PartName::Bar => {
                self.write_rect(depth, bounds, 0.0, Some(palette::GCP_BAR_FILL), None);
            }
            PartName::Body => {
                self.write_rect(depth, bounds, 0.0, Some(palette::GCP_BODY_FILL), None);
            }
            PartName::Icon => {
                let icon = context.icon.ok_or_else(|| part_mismatch(node, part))?;
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
                self.write_rect(
                    depth,
                    bounds,
                    FACT_RADIUS_PX,
                    Some(palette::FACT_FILL),
                    None,
                );
            }
            PartName::AskBox => {
                self.write_rect(depth, bounds, FACT_RADIUS_PX, Some(palette::ASK_FILL), None);
            }
            PartName::DotStart | PartName::DotEnd => {
                let kind = context.pipe_kind.ok_or_else(|| part_mismatch(node, part))?;
                let (color, _) = palette::wire_style(kind);
                self.line(
                    depth,
                    &format!(
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{color}"/>"#,
                        format_number(bounds.x + bounds.width / 2.0),
                        format_number(bounds.y + bounds.height / 2.0),
                        format_number(DOT_RADIUS_PX)
                    ),
                );
            }
            PartName::WireStart | PartName::WireEnd => {
                let kind = context.pipe_kind.ok_or_else(|| part_mismatch(node, part))?;
                let dir = context.pipe_dir.ok_or_else(|| part_mismatch(node, part))?;
                self.write_wire(depth, bounds, dir, kind);
            }
            PartName::Swatch => {
                let kind = context.pipe_kind.ok_or_else(|| part_mismatch(node, part))?;
                self.write_wire(depth, bounds, PipeDir::Horizontal, kind);
            }
            PartName::Spine => {
                let kind = context.pipe_kind.ok_or_else(|| part_mismatch(node, part))?;
                self.write_wire(depth, bounds, PipeDir::Vertical, kind);
            }
            PartName::Tag | PartName::Hub => {
                let kind = context.pipe_kind.ok_or_else(|| part_mismatch(node, part))?;
                let border = Stroke {
                    width_px: TAG_BORDER_PX,
                    line: LineStyle::Solid,
                    color: palette::tag_border(kind),
                };
                self.write_rect(
                    depth,
                    bounds,
                    TAG_RADIUS_PX,
                    Some(palette::TAG_FILL),
                    Some(border),
                );
            }
            PartName::BadgeText
            | PartName::Text
            | PartName::Label
            | PartName::FunctionName
            | PartName::ProductName
            | PartName::Fact
            | PartName::Ask
            | PartName::TagLabel
            | PartName::TagSub
            | PartName::HubText
            | PartName::LegendLabel
            | PartName::LegendText => {}
        }
        Ok(())
    }

    /// A `<line>` along the box's center line in the run direction.
    fn write_wire(&mut self, depth: usize, bounds: BoxRect, dir: PipeDir, kind: PipeKind) {
        let (color, line_style) = palette::wire_style(kind);
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
        let stroke = Stroke {
            width_px: WIRE_WIDTH_PX,
            line: line_style,
            color,
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

    /// A stroke of width w is drawn on the rect inset by w / 2, so it stays inside the
    /// border box as a CSS border does, and the radius shrinks by the same amount so the
    /// outer edge keeps the section 2 radius.
    fn write_rect(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        radius_px: f32,
        fill: Option<&str>,
        stroke: Option<Stroke>,
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
    ) -> Result<(), RenderError> {
        let style = run.style;
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
                    r#"<text x="{}" y="{}" xml:space="preserve" font-family="{}" font-size="{}" font-weight="{}"{letter_spacing} fill="{}">{}</text>"#,
                    format_number(x),
                    format_number(y),
                    style.family.css_name(),
                    format_number(style.size_px),
                    style.weight.css_value(),
                    run.color,
                    escape_xml(content)
                ),
            );
            self.text_elements += 1;
        }
        Ok(())
    }
}

fn part_mismatch(node: &NodeGeometry, part: &Part) -> RenderError {
    RenderError::GeometryMismatch {
        expected: node.pointer.clone(),
        found: node.pointer.child(part.name.as_str()),
    }
}

fn stroke_attributes(stroke: Stroke) -> String {
    let dash = match stroke.line {
        LineStyle::Solid => String::new(),
        LineStyle::Dashed => format!(r#" stroke-dasharray="{}""#, palette::DASH_ARRAY),
    };
    format!(
        r#" stroke="{}" stroke-width="{}"{dash}"#,
        stroke.color,
        format_number(stroke.width_px)
    )
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
    fn escape_xml_escapes_markup_characters() {
        assert_eq!(
            escape_xml(r#"a < b & "c" > 'd'"#),
            "a &lt; b &amp; &quot;c&quot; &gt; &apos;d&apos;"
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
