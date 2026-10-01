//! The section 12.5 SVG: page-level nodes flat, body nodes as faces and surfaces in
//! geometry order, each node's label on its plane right after its solid, then links with
//! their tags on the terrain.

use stencil_layout::{BoxRect, LinkRoute, NodeGeometry, NodeTag, PageGeometry, Part, PartName};
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, Link, Node, NodeRef, Page, PipeDir, Projection, Theme};

use super::DOT_RADIUS_PX;
use super::{
    ArrowEnds, DocumentNode, ICON_CHIP_RADIUS_PX, PartContext, Relabeler, SvgWriter, TAG_RADIUS_PX,
    close_groups_until_parent, escape_xml, frame_diagonal_box, group_open_tag, icon_chip_box,
    link_mismatch, part_mismatch, pipe_text_style_name, stroke_attributes, text_style_name,
};
use crate::iso::{
    ISO_DOT_RADIUS_X_PX, ISO_DOT_RADIUS_Y_PX, IsoPoint, Label, ScreenPoint, Solid, SolidInputs,
    SolidShape, arrowhead_vertices, end_direction, has_zone_ancestor, iso_link_arrowhead_length,
    label_axis, local_part, plane_member, project_point, project_zoomed, start_direction,
    zoomed_geometry,
};
use crate::palette::{DotStyle, FacePaint, LineStyle, LineUse, Palette, Stroke, ZoneTab};
use crate::{RenderError, SvgDocument, format_number};

/// The id of the blur filter every block shadow references (section 13.11).
const SHADOW_FILTER_ID: &str = "stencil-shadow";

pub(super) fn render_iso(
    page: &Page,
    theme: &Theme,
    geometry: &PageGeometry,
    expected: &[(NodePointer, DocumentNode<'_>)],
) -> Result<SvgDocument, RenderError> {
    let (zoomed, zoom) = zoomed_geometry(geometry)?;
    let solid_inputs = SolidInputs::new(&zoomed, &page.links, theme.iso.slab_thickness);
    let scene = project_zoomed(&zoomed, zoom, &solid_inputs)?;
    let geometry = &zoomed;
    let palette = Palette::new(theme, Projection::Iso);
    let mut writer = SvgWriter::new(page.canvas, palette);
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
    if let Some(shadow) = writer.palette.iso_block_shadow() {
        writer.line(
            1,
            &format!(
                r#"<defs><filter id="{SHADOW_FILTER_ID}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="{}"/></filter></defs>"#,
                format_number(shadow.blur)
            ),
        );
    }

    let mut solids: Vec<Option<&Solid>> = vec![None; geometry.nodes.len()];
    for solid in &scene.solids {
        if let Some(slot) = solids.get_mut(solid.node) {
            *slot = Some(solid);
        }
    }
    let mut node_labels: Vec<Option<&Label>> = vec![None; geometry.nodes.len()];
    for label in &scene.labels {
        if let Some(slot) = label.node.and_then(|index| node_labels.get_mut(index)) {
            *slot = Some(label);
        }
    }
    let mut relabeler = Relabeler::default();

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
        match (node.tag, solid, document_node) {
            (NodeTag::LegendEntry, _, DocumentNode::LegendEntry(_)) => {
                let shifted = shifted_node(node, 0.0, scene.footer_shift);
                let relabeled = relabeler.relabeled(writer.palette, &shifted, *document_node)?;
                writer.write_node(
                    depth + 1,
                    relabeled.as_ref().unwrap_or(&shifted),
                    *document_node,
                )?;
            }
            (NodeTag::Legend | NodeTag::LegendEntry | NodeTag::Foot, _, _) => {
                let shifted = shifted_node(node, 0.0, scene.footer_shift);
                writer.write_node(depth + 1, &shifted, *document_node)?;
            }
            (_, Some(solid), _) => {
                writer.write_solid(depth + 1, node, *document_node, solid, scene.offset)?;
                if let Some(label) = node_labels.get(index).copied().flatten() {
                    let nested = has_zone_ancestor(geometry, index);
                    writer.write_node_label(depth + 1, node, *document_node, label, nested)?;
                }
            }
            (_, None, _) => {
                writer.write_node(depth + 1, node, *document_node)?;
            }
        }
        open_groups.push(index);
    }
    for depth in (1..=open_groups.len()).rev() {
        writer.line(depth, "</g>");
    }

    for (route, path) in geometry.links.iter().zip(&scene.link_paths).take(LINKS_MAX) {
        let link = page
            .links
            .get(route.index)
            .ok_or_else(|| link_mismatch(route))?;
        let pointer = NodePointer::root().child("links").index(route.index);
        let tag = scene.labels.iter().find(|label| label.owner == pointer);
        writer.write_iso_link(1, route, link, path, scene.offset, tag)?;
    }
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

/// Moves the end of a path back by `length` along its last horizontal stretch, or drops
/// the last point when that stretch is shorter, so an arrowhead of that length covers it.
fn shorten_end(path: &mut Vec<IsoPoint>, length: f32) {
    let count = path.len();
    let (Some(&tip), Some(&previous)) = (
        path.last(),
        count.checked_sub(2).and_then(|index| path.get(index)),
    ) else {
        return;
    };
    let run = ((tip.x - previous.x).powi(2) + (tip.y - previous.y).powi(2)).sqrt();
    if run > length {
        let keep = (run - length) / run;
        if let Some(slot) = path.last_mut() {
            *slot = IsoPoint {
                x: previous.x + (tip.x - previous.x) * keep,
                y: previous.y + (tip.y - previous.y) * keep,
                z: tip.z,
            };
        }
    } else if count > 2 {
        path.pop();
    }
}

impl<'a> SvgWriter<'a> {
    /// The face paint of a slab or block, or None for a node that draws no faces.
    fn face_paint(
        &self,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        solid: &Solid,
    ) -> Result<Option<FacePaint<'a>>, RenderError> {
        let DocumentNode::Content(NodeRef::Node(content)) = document_node else {
            return Ok(None);
        };
        let block = |fill: Option<&'a str>, border: Option<Stroke<'a>>| {
            self.palette.block_faces(fill, border)
        };
        let paint = match content {
            Node::Box(_) => {
                let look = node.container.ok_or_else(|| surface_mismatch(node))?;
                self.palette.slab_faces(look, node.tint)
            }
            Node::Item(_) => {
                let card = self.palette.card();
                block(Some(card.fill), card.border)
            }
            Node::Text(_) => {
                let text_block = self.palette.block();
                block(Some(text_block.fill), text_block.border)
            }
            Node::Fact(fact) => block(self.fact_box_fill(fact.source), None),
            Node::Callout(callout) => {
                let paint = self.palette.callout(callout.kind);
                block(Some(paint.fill), Some(paint.border))
            }
            Node::Frame(_) => block(None, Some(self.palette.frame_border())),
            Node::Note(_)
            | Node::Row(_)
            | Node::Col(_)
            | Node::Lanes(_)
            | Node::Pipe(_)
            | Node::Tee(_) => {
                return Ok(None);
            }
        };
        paint.map(Some).ok_or_else(|| RenderError::Svg {
            message: format!("a face color of {} is not a #RRGGBB color", solid.pointer),
        })
    }

    /// Faces and top-face drawing of a slab or block, or the primitives of a surface.
    /// Returns the fill of the top face, if any.
    fn write_solid(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        solid: &Solid,
        offset: ScreenPoint,
    ) -> Result<Option<String>, RenderError> {
        let context = self.part_context(document_node);
        if solid.shape == SolidShape::Surface {
            let kind = context.line.ok_or_else(|| surface_mismatch(node))?;
            if let Some(spine) = node.part(PartName::Spine) {
                self.write_spine(depth, spine.bounds, kind, solid.base_z, offset);
            } else {
                self.write_iso_pipe(depth, node, context, solid.base_z, offset)?;
            }
            return Ok(None);
        }
        let Some(paint) = self.face_paint(node, document_node, solid)? else {
            return Ok(None);
        };
        let top_z = solid.base_z + solid.height;
        if solid.shape == SolidShape::Block && solid.opaque {
            self.write_block_shadow(depth, node.bounds, solid.base_z, offset);
        }
        self.write_faces(depth, node.bounds, solid.base_z, top_z, &paint, offset);
        self.write_top_face_drawing(depth, node, document_node, top_z, offset);
        Ok(paint.top)
    }

    /// The block's footprint at its base, moved `dy` down on screen and blurred, so the
    /// block reads as standing on its surface (section 13.11). Nothing without a theme
    /// shadow.
    fn write_block_shadow(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        base_z: f32,
        offset: ScreenPoint,
    ) {
        let Some(shadow) = self.palette.iso_block_shadow() else {
            return;
        };
        let lowered = ScreenPoint {
            x: offset.x,
            y: offset.y + shadow.dy,
        };
        let corners = [
            project_point(bounds.x, bounds.y, base_z, lowered),
            project_point(bounds.right(), bounds.y, base_z, lowered),
            project_point(bounds.right(), bounds.bottom(), base_z, lowered),
            project_point(bounds.x, bounds.bottom(), base_z, lowered),
        ];
        self.line(
            depth,
            &format!(
                r#"<polygon points="{}" fill="{}" fill-opacity="{}" filter="url(#{SHADOW_FILTER_ID})"/>"#,
                points_attribute(&corners),
                shadow.color,
                format_number(shadow.opacity)
            ),
        );
    }

    /// Left, right and top face, in that order (section 12.3, rule 1). A slab with no
    /// height draws its top face only.
    fn write_faces(
        &mut self,
        depth: usize,
        bounds: BoxRect,
        base_z: f32,
        top_z: f32,
        paint: &FacePaint<'_>,
        offset: ScreenPoint,
    ) {
        let (left, top, right, bottom) = (bounds.x, bounds.y, bounds.right(), bounds.bottom());
        let at = |x: f32, y: f32, z: f32| project_point(x, y, z, offset);
        let has_sides = top_z > base_z;
        let faces = [
            (
                has_sides,
                paint.left.as_deref(),
                paint.side_stroke,
                [
                    at(left, bottom, base_z),
                    at(right, bottom, base_z),
                    at(right, bottom, top_z),
                    at(left, bottom, top_z),
                ],
            ),
            (
                has_sides,
                paint.right.as_deref(),
                paint.side_stroke,
                [
                    at(right, top, base_z),
                    at(right, bottom, base_z),
                    at(right, bottom, top_z),
                    at(right, top, top_z),
                ],
            ),
            (
                true,
                paint.top.as_deref(),
                paint.top_stroke,
                [
                    at(left, top, top_z),
                    at(right, top, top_z),
                    at(right, bottom, top_z),
                    at(left, bottom, top_z),
                ],
            ),
        ];
        for (drawn, fill, stroke, corners) in faces {
            if !drawn || (fill.is_none() && stroke.is_none()) {
                continue;
            }
            let stroke_part = stroke
                .map(|stroke| format!(r#"{} stroke-linejoin="round""#, stroke_attributes(stroke)))
                .unwrap_or_default();
            self.line(
                depth,
                &format!(
                    r#"<polygon points="{}" fill="{}"{stroke_part}/>"#,
                    points_attribute(&corners),
                    fill.unwrap_or("none")
                ),
            );
        }
    }

    /// The Callout accent and the Frame diagonals, projected onto the top face in their
    /// flat paint (section 12.3, rule 4). The gcp bar is not drawn: the brand blue side
    /// faces and the label chip carry the frame.
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
            Node::Box(_)
            | Node::Row(_)
            | Node::Col(_)
            | Node::Lanes(_)
            | Node::Item(_)
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
        stroke: Stroke<'_>,
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

    /// The chip under an icon under iso (section 12.4): a soft shadow in center, then the
    /// tile with its ring.
    pub(super) fn write_iso_icon_chip(&mut self, depth: usize, icon_bounds: BoxRect) {
        let chip = icon_chip_box(icon_bounds);
        if let Some(shadow) = self.palette.iso_icon_chip_shadow() {
            self.line(
                depth,
                &format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" fill-opacity="{}"/>"#,
                    format_number(chip.x),
                    format_number(chip.y + shadow.dy),
                    format_number(chip.width),
                    format_number(chip.height),
                    format_number(ICON_CHIP_RADIUS_PX),
                    shadow.color,
                    format_number(shadow.opacity),
                ),
            );
        }
        let paint = self.palette.iso_icon_chip();
        self.write_box(depth, chip, ICON_CHIP_RADIUS_PX, paint);
    }

    /// One wire from dot center to dot center, stopping at an arrowhead base, then the two
    /// ends: an ellipse per dot or a projected arrowhead (section 12.3, rule 5).
    fn write_iso_pipe(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        context: PartContext<'a>,
        z: f32,
        offset: ScreenPoint,
    ) -> Result<(), RenderError> {
        let kind = context.line.ok_or_else(|| surface_mismatch(node))?;
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
                let head = (tip, outward, arrow_length);
                self.write_arrowhead_polygon(depth, head, z, kind, offset);
            } else {
                self.write_ellipse_dot(depth, project_point(center.0, center.1, z, offset), kind);
            }
        }
        Ok(())
    }

    /// `head` is the flat tip, the unit direction it points along and its length.
    fn write_arrowhead_polygon(
        &mut self,
        depth: usize,
        head: ((f32, f32), (f32, f32), f32),
        z: f32,
        kind: LineUse,
        offset: ScreenPoint,
    ) {
        let (tip, direction, length) = head;
        let corners =
            arrowhead_vertices(tip, direction, length).map(|(x, y)| project_point(x, y, z, offset));
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
    fn write_ellipse_dot(&mut self, depth: usize, center: ScreenPoint, kind: LineUse) {
        let wire = self.palette.wire_style(kind);
        let (fill, ring) = match wire.dot {
            DotStyle::None => return,
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
        kind: LineUse,
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

    /// The link path of section 12.3 rule 7 as one `<path>`, shortened under each arrowhead
    /// as in flat, and the arrowheads as projected polygons at the height of their tip. The
    /// tag is a billboard.
    fn write_iso_link(
        &mut self,
        depth: usize,
        route: &LinkRoute,
        link: &Link,
        path: &[IsoPoint],
        offset: ScreenPoint,
        tag: Option<&Label>,
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
        let arrows = ArrowEnds::from_arrow(link.arrow);
        let arrow_length = iso_link_arrowhead_length(route.line, route.tint);
        let mut points = path.to_vec();
        let start_head = if arrows.start {
            start_direction(&points)
        } else {
            None
        };
        let end_head = if arrows.end {
            end_direction(&points)
        } else {
            None
        };
        if end_head.is_some() {
            shorten_end(&mut points, arrow_length);
        }
        if start_head.is_some() {
            points.reverse();
            shorten_end(&mut points, arrow_length);
            points.reverse();
        }
        let stroke = self.palette.wire_style(line_use).stroke;
        // A dashed link skips each riser, so the dash pattern never lands on a slab edge as
        // a solid tick; the gap reads as one more space between dashes.
        let skips_risers = stroke.line != LineStyle::Solid;
        let mut data = String::new();
        let mut previous: Option<IsoPoint> = None;
        for point in &points {
            let riser = previous.is_some_and(|last| {
                (last.x - point.x).abs() <= stencil_layout::GEOMETRY_EPSILON_PX
                    && (last.y - point.y).abs() <= stencil_layout::GEOMETRY_EPSILON_PX
            });
            let command = match (previous, skips_risers && riser) {
                (None, _) => "M",
                (Some(_), true) => " M",
                (Some(_), false) => " L",
            };
            let screen = project_point(point.x, point.y, point.z, offset);
            data.push_str(&format!(
                "{command} {} {}",
                format_number(screen.x),
                format_number(screen.y)
            ));
            previous = Some(*point);
        }
        self.line(
            depth + 1,
            &format!(
                r#"<path d="{data}" fill="none"{} stroke-linejoin="round"/>"#,
                stroke_attributes(stroke)
            ),
        );
        for (tip, direction) in [start_head, end_head].into_iter().flatten() {
            self.write_arrowhead_polygon(
                depth + 1,
                ((tip.x, tip.y), direction, arrow_length),
                tip.z,
                line_use,
                offset,
            );
        }
        if let Some(label) = tag {
            self.write_link_tag_label(depth + 1, route, label)?;
        }
        self.line(depth, "</g>");
        Ok(())
    }

    /// The group every label lies in: the plane's matrix, so each part is written in its
    /// layout box (section 12.4, rule 4).
    fn open_plane_group(&mut self, depth: usize, label: &Label) {
        let map = label.map;
        self.line(
            depth,
            &format!(
                r#"<g data-plane="{}" data-axis="{}" transform="matrix({} {} {} {} {} {})">"#,
                format_number(label.z),
                label.axis.as_str(),
                format_number(map.a),
                format_number(map.b),
                format_number(map.c),
                format_number(map.d),
                format_number(map.e),
                format_number(map.f)
            ),
        );
    }

    /// A node's plane members with the flat drawing, in the plane group. A zone label takes
    /// the ink of the tab it would carry upright: the top-level tab fill, or a nested zone's
    /// border color.
    fn write_node_label(
        &mut self,
        depth: usize,
        node: &NodeGeometry,
        document_node: DocumentNode<'_>,
        label: &Label,
        nested: bool,
    ) -> Result<(), RenderError> {
        let context = self.part_context(document_node);
        let zone_ink = match document_node {
            DocumentNode::Content(NodeRef::Node(Node::Box(_))) => {
                let look = node.container.ok_or_else(|| surface_mismatch(node))?;
                Some(match self.palette.iso_zone_tab(look, node.tint, nested) {
                    ZoneTab::Filled { fill, .. } => fill,
                    ZoneTab::Outline { border, .. } => border.color,
                })
            }
            _ => None,
        };
        self.open_plane_group(depth, label);
        let members: Vec<&Part> = node
            .parts
            .iter()
            .filter(|part| plane_member(node.tag, part.name))
            .collect();
        let (axis, pivot) = label_axis(&members);
        let members: Vec<Part> = members
            .iter()
            .map(|part| local_part(part, axis, pivot))
            .collect();
        for part in &members {
            self.write_part_shape(depth + 1, node, part, context)?;
        }
        for part in &members {
            if let Some(run) = &part.text {
                let style_name = text_style_name(document_node, node.container, part.name)
                    .ok_or_else(|| part_mismatch(node, part))?;
                let fill = match zone_ink {
                    Some(ink) => ink,
                    None => self.palette.text_ink(
                        style_name,
                        self.canvas,
                        context.line.map(|line_use| line_use.line),
                    ),
                };
                self.write_text_run(depth + 1, &node.pointer, part.bounds, run, fill)?;
            }
        }
        self.line(depth, "</g>");
        Ok(())
    }

    /// A link's tag box and runs in the plane group on the terrain.
    fn write_link_tag_label(
        &mut self,
        depth: usize,
        route: &LinkRoute,
        label: &Label,
    ) -> Result<(), RenderError> {
        let pointer = NodePointer::root().child("links").index(route.index);
        self.open_plane_group(depth, label);
        let members: Vec<&Part> = route.parts.iter().collect();
        let (axis, pivot) = label_axis(&members);
        for part in members.iter().map(|part| local_part(part, axis, pivot)) {
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
