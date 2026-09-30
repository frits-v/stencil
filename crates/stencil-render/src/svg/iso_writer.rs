//! The section 12.5 SVG: page-level nodes flat, body nodes as faces and surfaces in
//! geometry order, links on their planes, then every billboard upright on top.

use stencil_layout::{BoxRect, LinkRoute, NodeGeometry, NodeTag, PageGeometry, Part, PartName};
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, Link, Node, NodeRef, Page, PipeDir, PipeKind, ZoneKind};

use super::DOT_RADIUS_PX;
use super::{
    ArrowEnds, DocumentNode, PartContext, SvgWriter, TAG_RADIUS_PX, close_groups_until_parent,
    escape_xml, frame_diagonal_box, group_open_tag, link_mismatch, part_mismatch,
    pipe_text_style_name, stroke_attributes, text_style_name, trim_end, trim_start,
};
use crate::iso::{
    Billboard, ISO_DOT_RADIUS_X_PX, ISO_DOT_RADIUS_Y_PX, ScreenPoint, Solid, SolidShape,
    arrowhead_vertices, billboard_member, project_page, project_point, unit_direction,
};
use crate::palette::{DotStyle, Face, LineStyle, Stroke};
use crate::{RenderError, SvgDocument, format_number};

/// Radius of the chip behind a gcp label billboard (section 12.4).
const GCP_CHIP_RADIUS_PX: f32 = 4.0;
/// Width of the page-background outline behind billboard text that has no chip of its own.
/// Upright text crosses slab edges, dashed zone borders and the gcp bar band; the outline
/// keeps each glyph on a clean ground in every theme.
const TEXT_HALO_PX: f32 = 3.0;

pub(super) fn render_iso(
    page: &Page,
    geometry: &PageGeometry,
    expected: &[(NodePointer, DocumentNode<'_>)],
) -> Result<SvgDocument, RenderError> {
    let scene = project_page(geometry)?;
    let mut writer = SvgWriter::new(page.canvas, crate::palette::Palette::new(page.theme));
    let width = format_number(scene.canvas.width);
    let height = format_number(scene.canvas.height);
    writer.line(
        0,
        &format!(
            r#"<svg xmlns="{}" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
            super::SVG_NAMESPACE
        ),
    );
    writer.line(
        1,
        &format!(
            r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{}"/>"#,
            writer.palette.page_background()
        ),
    );

    let mut solids: Vec<Option<&Solid>> = vec![None; geometry.nodes.len()];
    for solid in &scene.solids {
        if let Some(slot) = solids.get_mut(solid.node) {
            *slot = Some(solid);
        }
    }

    let mut open_groups: Vec<usize> = Vec::new();
    for (index, (node, (_, document_node))) in geometry.nodes.iter().zip(expected).enumerate() {
        close_groups_until_parent(&mut writer, &mut open_groups, geometry, index)?;
        let depth = open_groups.len() + 1;
        if node.parent.is_none() {
            writer.line(
                depth,
                &format!(
                    r#"<g data-id="{}" data-tag="{}" data-projection="iso">"#,
                    escape_xml(node.pointer.as_str()),
                    node.tag.as_str()
                ),
            );
        } else {
            writer.line(depth, &group_open_tag(node));
        }
        let solid = solids.get(index).copied().flatten();
        match (node.tag, solid) {
            (NodeTag::Legend | NodeTag::LegendEntry | NodeTag::Foot, _) => {
                let shifted = shifted_node(node, 0.0, scene.footer_shift);
                writer.write_node(depth + 1, &shifted, *document_node)?;
            }
            (_, Some(solid)) => {
                writer.write_solid(depth + 1, node, *document_node, solid, scene.offset)?;
            }
            (_, None) => writer.write_node(depth + 1, node, *document_node)?,
        }
        open_groups.push(index);
    }
    for depth in (1..=open_groups.len()).rev() {
        writer.line(depth, "</g>");
    }

    for (route, plane) in geometry
        .links
        .iter()
        .zip(&scene.link_planes)
        .take(LINKS_MAX)
    {
        let link = page
            .links
            .get(route.index)
            .ok_or_else(|| link_mismatch(route))?;
        writer.write_iso_link(1, route, link, *plane, scene.offset);
    }

    writer.line(1, r#"<g data-layer="billboards">"#);
    for billboard in &scene.billboards {
        writer.write_billboard(2, billboard, geometry, expected)?;
    }
    writer.line(1, "</g>");
    writer.line(0, "</svg>");

    Ok(SvgDocument {
        svg: writer.output,
        text_elements: writer.text_elements,
    })
}

fn shifted_box(bounds: BoxRect, delta_x: f32, delta_y: f32) -> BoxRect {
    BoxRect {
        x: bounds.x + delta_x,
        y: bounds.y + delta_y,
        ..bounds
    }
}

fn shifted_part(part: &Part, delta_x: f32, delta_y: f32) -> Part {
    Part {
        bounds: shifted_box(part.bounds, delta_x, delta_y),
        ..part.clone()
    }
}

/// A copy of the node with its box and every part moved, so the flat drawing code draws it
/// at the new place.
fn shifted_node(node: &NodeGeometry, delta_x: f32, delta_y: f32) -> NodeGeometry {
    NodeGeometry {
        bounds: shifted_box(node.bounds, delta_x, delta_y),
        content: shifted_box(node.content, delta_x, delta_y),
        parts: node
            .parts
            .iter()
            .map(|part| shifted_part(part, delta_x, delta_y))
            .collect(),
        ..node.clone()
    }
}

/// `x,y x,y ...` for a polygon's `points`.
fn points_attribute(points: &[ScreenPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The flat fill and border of a slab or block (section 12.3, rule 2).
#[derive(Debug, Clone, Copy)]
struct SolidPaint {
    fill: Option<&'static str>,
    border: Option<Stroke>,
}

impl SvgWriter {
    fn solid_paint(&self, document_node: DocumentNode<'_>) -> SolidPaint {
        let DocumentNode::Content(NodeRef::Node(content)) = document_node else {
            return SolidPaint {
                fill: None,
                border: None,
            };
        };
        match content {
            Node::Zone(zone) => {
                let style = self.palette.zone_style(zone.kind);
                let fill = if zone.kind == ZoneKind::Gcp {
                    Some(self.palette.gcp_body_fill())
                } else {
                    style.fill
                };
                SolidPaint {
                    fill,
                    border: style.border,
                }
            }
            Node::Pcard(_) => {
                let card = self.palette.card();
                SolidPaint {
                    fill: Some(card.fill),
                    border: card.border,
                }
            }
            Node::Text(_) => {
                let block = self.palette.block();
                SolidPaint {
                    fill: Some(block.fill),
                    border: block.border,
                }
            }
            Node::Fact(_) => SolidPaint {
                fill: Some(self.palette.fact_fill()),
                border: None,
            },
            Node::Callout(callout) => {
                let paint = self.palette.callout(callout.kind);
                SolidPaint {
                    fill: Some(paint.fill),
                    border: Some(paint.border),
                }
            }
            Node::Frame(_) => SolidPaint {
                fill: None,
                border: Some(self.palette.frame_border()),
            },
            Node::Note(_) | Node::Row(_) | Node::Col(_) | Node::Pipe(_) | Node::Tee(_) => {
                SolidPaint {
                    fill: None,
                    border: None,
                }
            }
        }
    }

    /// Faces and top-face drawing of a slab or block, or the primitives of a surface.
    fn write_solid(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        solid: &Solid,
        offset: ScreenPoint,
    ) -> Result<(), RenderError> {
        let context = self.part_context(document_node);
        if solid.shape == SolidShape::Surface {
            let kind = context.pipe_kind.ok_or_else(|| surface_mismatch(node))?;
            if let Some(spine) = node.part(PartName::Spine) {
                self.write_spine(depth, spine.bounds, kind, solid.base_z, offset);
            } else {
                self.write_iso_pipe(depth, node, context, solid.base_z, offset)?;
            }
            return Ok(());
        }
        let paint = self.solid_paint(document_node);
        let top_z = solid.base_z + solid.height;
        self.write_faces(depth, node.bounds, solid.base_z, top_z, paint, offset)?;
        self.write_top_face_drawing(depth, node, document_node, top_z, offset);
        Ok(())
    }

    /// Left, right and top face, in that order (section 12.3, rule 1).
    fn write_faces(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        base_z: f32,
        top_z: f32,
        paint: SolidPaint,
        offset: ScreenPoint,
    ) -> Result<(), RenderError> {
        if paint.fill.is_none() && paint.border.is_none() {
            return Ok(());
        }
        let (left, top, right, bottom) = (bounds.x, bounds.y, bounds.right(), bounds.bottom());
        let at = |x: f32, y: f32, z: f32| project_point(x, y, z, offset);
        let faces = [
            (
                Face::Left,
                [
                    at(left, bottom, base_z),
                    at(right, bottom, base_z),
                    at(right, bottom, top_z),
                    at(left, bottom, top_z),
                ],
            ),
            (
                Face::Right,
                [
                    at(right, top, base_z),
                    at(right, bottom, base_z),
                    at(right, bottom, top_z),
                    at(right, top, top_z),
                ],
            ),
            (
                Face::Top,
                [
                    at(left, top, top_z),
                    at(right, top, top_z),
                    at(right, bottom, top_z),
                    at(left, bottom, top_z),
                ],
            ),
        ];
        let stroke = self
            .palette
            .face_outline(paint.border)
            .map(stroke_attributes)
            .unwrap_or_default();
        for (face, corners) in faces {
            let fill = match paint.fill {
                Some(base) => {
                    self.palette
                        .face_fill(base, face)
                        .ok_or_else(|| RenderError::Svg {
                            message: format!("face fill {base} is not a #RRGGBB color"),
                        })?
                }
                None => "none".to_string(),
            };
            self.line(
                depth,
                &format!(
                    r#"<polygon points="{}" fill="{fill}"{stroke}/>"#,
                    points_attribute(&corners)
                ),
            );
        }
        Ok(())
    }

    /// The gcp bar band, the Callout accent and the Frame diagonals, projected onto the top
    /// face in their flat paint (section 12.3, rule 4).
    fn write_top_face_drawing(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        top_z: f32,
        offset: ScreenPoint,
    ) {
        let DocumentNode::Content(NodeRef::Node(content)) = document_node else {
            return;
        };
        match content {
            Node::Zone(zone) if zone.kind == ZoneKind::Gcp => {
                if let Some(bar) = node.part(PartName::Bar) {
                    let fill = self.palette.gcp_bar_fill();
                    self.write_top_rectangle(depth, bar.bounds, top_z, fill, offset);
                    if let Some(rule) = self.palette.gcp_bar_rule() {
                        let rule_y = bar.bounds.bottom() - rule.width_px / 2.0;
                        let start = project_point(bar.bounds.x, rule_y, top_z, offset);
                        let end = project_point(bar.bounds.right(), rule_y, top_z, offset);
                        self.write_screen_line(depth, start, end, rule);
                    }
                }
            }
            Node::Callout(callout) => {
                if let Some(accent) = node.part(PartName::Accent) {
                    let fill = self.palette.callout(callout.kind).accent;
                    self.write_top_rectangle(depth, accent.bounds, top_z, fill, offset);
                }
            }
            Node::Frame(_) => {
                let border = self.palette.frame_border();
                let inner = frame_diagonal_box(node.bounds, border.width_px);
                let diagonal = self.palette.frame_diagonal();
                for ((x1, y1), (x2, y2)) in [
                    ((inner.x, inner.y), (inner.right(), inner.bottom())),
                    ((inner.x, inner.bottom()), (inner.right(), inner.y)),
                ] {
                    let start = project_point(x1, y1, top_z, offset);
                    let end = project_point(x2, y2, top_z, offset);
                    self.write_screen_line(depth, start, end, diagonal);
                }
            }
            Node::Zone(_)
            | Node::Row(_)
            | Node::Col(_)
            | Node::Pcard(_)
            | Node::Fact(_)
            | Node::Note(_)
            | Node::Pipe(_)
            | Node::Tee(_)
            | Node::Text(_) => {}
        }
    }

    fn write_top_rectangle(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        z: f32,
        fill: &str,
        offset: ScreenPoint,
    ) {
        let corners = [
            project_point(bounds.x, bounds.y, z, offset),
            project_point(bounds.right(), bounds.y, z, offset),
            project_point(bounds.right(), bounds.bottom(), z, offset),
            project_point(bounds.x, bounds.bottom(), z, offset),
        ];
        self.line(
            depth,
            &format!(
                r#"<polygon points="{}" fill="{fill}"/>"#,
                points_attribute(&corners)
            ),
        );
    }

    fn write_screen_line(
        &mut self,
        depth: usize,
        start: ScreenPoint,
        end: ScreenPoint,
        stroke: Stroke,
    ) {
        self.line(
            depth,
            &format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
                format_number(start.x),
                format_number(start.y),
                format_number(end.x),
                format_number(end.y),
                stroke_attributes(stroke)
            ),
        );
    }

    /// One wire from dot center to dot center, stopping at an arrowhead base, then the two
    /// ends: an ellipse per dot or a projected arrowhead (section 12.3, rule 5).
    fn write_iso_pipe(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        context: PartContext,
        z: f32,
        offset: ScreenPoint,
    ) -> Result<(), RenderError> {
        let kind = context.pipe_kind.ok_or_else(|| surface_mismatch(node))?;
        let dir = context.pipe_dir.ok_or_else(|| surface_mismatch(node))?;
        let arrows = context.arrows;
        let dot_start = node
            .part(PartName::DotStart)
            .ok_or_else(|| surface_mismatch(node))?;
        let dot_end = node
            .part(PartName::DotEnd)
            .ok_or_else(|| surface_mismatch(node))?;
        let (run_x, run_y) = match dir {
            PipeDir::Horizontal => (1.0, 0.0),
            PipeDir::Vertical => (0.0, 1.0),
        };
        let start_center = box_center(dot_start.bounds);
        let end_center = box_center(dot_end.bounds);
        // An arrowhead's tip is on the dot box's outer edge and its base 10 px inward.
        let start_tip = (
            start_center.0 - run_x * DOT_RADIUS_PX,
            start_center.1 - run_y * DOT_RADIUS_PX,
        );
        let end_tip = (
            end_center.0 + run_x * DOT_RADIUS_PX,
            end_center.1 + run_y * DOT_RADIUS_PX,
        );
        let arrow_length = stencil_layout::ARROWHEAD_LENGTH_PX;
        let wire_start = if arrows.start {
            (
                start_tip.0 + run_x * arrow_length,
                start_tip.1 + run_y * arrow_length,
            )
        } else {
            start_center
        };
        let wire_end = if arrows.end {
            (
                end_tip.0 - run_x * arrow_length,
                end_tip.1 - run_y * arrow_length,
            )
        } else {
            end_center
        };
        let wire = self.palette.wire_style(kind);
        self.write_screen_line(
            depth,
            project_point(wire_start.0, wire_start.1, z, offset),
            project_point(wire_end.0, wire_end.1, z, offset),
            wire.stroke,
        );
        for (arrow_here, center, tip, outward) in [
            (arrows.start, start_center, start_tip, (-run_x, -run_y)),
            (arrows.end, end_center, end_tip, (run_x, run_y)),
        ] {
            if arrow_here {
                self.write_arrowhead_polygon(depth, tip, outward, z, kind, offset);
            } else {
                self.write_ellipse_dot(depth, project_point(center.0, center.1, z, offset), kind);
            }
        }
        Ok(())
    }

    fn write_arrowhead_polygon(
        &mut self,
        depth: usize,
        tip: (f32, f32),
        direction: (f32, f32),
        z: f32,
        kind: PipeKind,
        offset: ScreenPoint,
    ) {
        let corners =
            arrowhead_vertices(tip, direction).map(|(x, y)| project_point(x, y, z, offset));
        let color = self.palette.wire_style(kind).stroke.color;
        self.line(
            depth,
            &format!(
                r#"<polygon points="{}" fill="{color}"/>"#,
                points_attribute(&corners)
            ),
        );
    }

    /// A 4 px dot on a horizontal plane is an ellipse on screen (section 12.2, rule 6),
    /// filled or hollow as the flat dot.
    fn write_ellipse_dot(&mut self, depth: usize, center: ScreenPoint, kind: PipeKind) {
        let wire = self.palette.wire_style(kind);
        let (fill, ring) = match wire.dot {
            DotStyle::Filled => (wire.stroke.color, String::new()),
            DotStyle::Hollow => (
                self.palette.page_background(),
                stroke_attributes(Stroke {
                    width_px: crate::palette::HOLLOW_DOT_RING_PX,
                    line: LineStyle::Solid,
                    color: wire.stroke.color,
                }),
            ),
        };
        self.line(
            depth,
            &format!(
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{fill}"{ring}/>"#,
                format_number(center.x),
                format_number(center.y),
                format_number(ISO_DOT_RADIUS_X_PX),
                format_number(ISO_DOT_RADIUS_Y_PX),
            ),
        );
    }

    fn write_spine(
        &mut self,
        depth: usize,
        spine: BoxRect,
        kind: PipeKind,
        z: f32,
        offset: ScreenPoint,
    ) {
        let center_x = spine.x + spine.width / 2.0;
        let stroke = self.palette.wire_style(kind).stroke;
        self.write_screen_line(
            depth,
            project_point(center_x, spine.y, z, offset),
            project_point(center_x, spine.bottom(), z, offset),
            stroke,
        );
    }

    /// The routed polyline on the link's plane, trimmed under each arrowhead as in flat, and
    /// the arrowheads as projected polygons. The tag is a billboard.
    fn write_iso_link(
        &mut self,
        depth: usize,
        route: &LinkRoute,
        link: &Link,
        plane: f32,
        offset: ScreenPoint,
    ) {
        let pointer = NodePointer::root().child("links").index(route.index);
        self.line(
            depth,
            &format!(
                r#"<g data-id="{}" data-tag="Link" data-kind="{}">"#,
                escape_xml(pointer.as_str()),
                route.kind.as_str()
            ),
        );
        let arrows = ArrowEnds::from_arrow(link.arrow);
        let mut points = route.points.clone();
        let start_arrow = if arrows.start {
            trim_start(&mut points)
        } else {
            None
        };
        let end_arrow = if arrows.end {
            trim_end(&mut points)
        } else {
            None
        };
        let mut path = String::new();
        for (index, point) in points.iter().enumerate() {
            let command = if index == 0 { "M" } else { " L" };
            let screen = project_point(point.x, point.y, plane, offset);
            path.push_str(&format!(
                "{command} {} {}",
                format_number(screen.x),
                format_number(screen.y)
            ));
        }
        let stroke = self.palette.wire_style(route.kind).stroke;
        self.line(
            depth + 1,
            &format!(
                r#"<path d="{path}" fill="none"{} stroke-linejoin="round"/>"#,
                stroke_attributes(stroke)
            ),
        );
        for (base, tip) in [start_arrow, end_arrow].into_iter().flatten() {
            if let Some(direction) = unit_direction((base.x, base.y), (tip.x, tip.y)) {
                self.write_arrowhead_polygon(
                    depth + 1,
                    (tip.x, tip.y),
                    direction,
                    plane,
                    route.kind,
                    offset,
                );
            }
        }
        self.line(depth, "</g>");
    }

    /// One billboard: its members drawn with the flat drawing, moved by the difference
    /// between its screen box and its flat box (section 12.4, rule 4).
    fn write_billboard(
        &mut self,
        depth: usize,
        billboard: &Billboard,
        geometry: &PageGeometry,
        expected: &[(NodePointer, DocumentNode<'_>)],
    ) -> Result<(), RenderError> {
        self.line(
            depth,
            &format!(
                r#"<g data-billboard="{}" data-role="{}">"#,
                escape_xml(billboard.owner.as_str()),
                billboard.role.as_str()
            ),
        );
        let delta_x = billboard.screen.x - billboard.flat.x;
        let delta_y = billboard.screen.y - billboard.flat.y;
        match billboard.node {
            Some(index) => {
                let node = geometry
                    .nodes
                    .get(index)
                    .ok_or_else(|| billboard_mismatch(billboard))?;
                let document_node = expected
                    .get(index)
                    .map(|(_, document_node)| *document_node)
                    .ok_or_else(|| billboard_mismatch(billboard))?;
                self.write_node_billboard(
                    depth + 1,
                    node,
                    document_node,
                    billboard,
                    delta_x,
                    delta_y,
                )?;
            }
            None => {
                let route = geometry
                    .links
                    .iter()
                    .take(LINKS_MAX)
                    .find(|route| {
                        NodePointer::root().child("links").index(route.index) == billboard.owner
                    })
                    .ok_or_else(|| billboard_mismatch(billboard))?;
                self.write_link_tag_billboard(depth + 1, route, delta_x, delta_y)?;
            }
        }
        self.line(depth, "</g>");
        Ok(())
    }

    fn write_node_billboard(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        billboard: &Billboard,
        delta_x: f32,
        delta_y: f32,
    ) -> Result<(), RenderError> {
        let context = self.part_context(document_node);
        if node.tag == NodeTag::Zone && node.kind == Some("gcp") {
            let chip = shifted_box(billboard.flat, delta_x, delta_y);
            let paint = self.palette.gcp_label_chip();
            self.write_box(depth, chip, GCP_CHIP_RADIUS_PX, paint);
        }
        let members: Vec<Part> = node
            .parts
            .iter()
            .filter(|part| billboard_member(node.tag, part.name))
            .map(|part| shifted_part(part, delta_x, delta_y))
            .collect();
        for part in &members {
            self.write_part_shape(depth, node, part, context)?;
        }
        let halo = format!(
            r#" stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke""#,
            self.palette.text_halo(),
            format_number(TEXT_HALO_PX)
        );
        for part in &members {
            if let Some(run) = &part.text {
                let style_name = text_style_name(document_node, part.name)
                    .ok_or_else(|| part_mismatch(node, part))?;
                let fill = self
                    .palette
                    .text_ink(style_name, self.canvas, context.pipe_kind);
                let attributes = if needs_halo(node, part.name) {
                    halo.as_str()
                } else {
                    ""
                };
                self.write_text_run_with_halo(
                    depth,
                    &node.pointer,
                    part.bounds,
                    run,
                    fill,
                    attributes,
                )?;
            }
        }
        Ok(())
    }

    fn write_link_tag_billboard(
        &mut self,
        depth: usize,
        route: &LinkRoute,
        delta_x: f32,
        delta_y: f32,
    ) -> Result<(), RenderError> {
        let pointer = NodePointer::root().child("links").index(route.index);
        for part in &route.parts {
            let bounds = shifted_box(part.bounds, delta_x, delta_y);
            match (part.name, &part.text) {
                (PartName::Tag, None) => {
                    let tag_paint = self.palette.tag(route.kind);
                    self.write_box(depth, bounds, TAG_RADIUS_PX, tag_paint);
                }
                (PartName::TagLabel | PartName::TagSub, Some(run)) => {
                    let style_name =
                        pipe_text_style_name(part.name).ok_or_else(|| link_mismatch(route))?;
                    let fill = self
                        .palette
                        .text_ink(style_name, self.canvas, Some(route.kind));
                    self.write_text_run(depth, &pointer, bounds, run, fill)?;
                }
                _ => return Err(link_mismatch(route)),
            }
        }
        Ok(())
    }
}

/// Billboard runs that are not drawn on a box of their own: every content run except the
/// gcp label on its chip, a card's fact and ask in their boxes, and a Frame label on its
/// chip. Tag runs sit on their tag box.
fn needs_halo(node: &NodeGeometry, part: PartName) -> bool {
    match node.tag {
        NodeTag::Zone => node.kind != Some("gcp"),
        NodeTag::Pcard => matches!(part, PartName::FunctionName | PartName::ProductName),
        NodeTag::Fact | NodeTag::Note | NodeTag::Text | NodeTag::Callout => true,
        NodeTag::Frame
        | NodeTag::Pipe
        | NodeTag::Tee
        | NodeTag::Page
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

fn box_center(bounds: BoxRect) -> (f32, f32) {
    (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// A pipe or tee whose document side or dot parts do not match its geometry.
fn surface_mismatch(node: &NodeGeometry) -> RenderError {
    RenderError::GeometryMismatch {
        expected: node.pointer.clone(),
        found: node.pointer.child("<surface>"),
    }
}

fn billboard_mismatch(billboard: &Billboard) -> RenderError {
    RenderError::GeometryMismatch {
        expected: billboard.owner.clone(),
        found: billboard.owner.child("<absent>"),
    }
}
