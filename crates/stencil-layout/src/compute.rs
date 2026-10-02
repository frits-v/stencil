//! Runs taffy with the measure closure of section 3, re-measures every text leaf at its final
//! width, converts parent-relative taffy boxes into absolute geometry, and moves the slots of
//! pipes that name their targets (section 13.8).

use std::collections::HashMap;

use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, WRAP_EPSILON_PX};
use stencil_model::{Node, NodeRef, Page, PipeDir, body_nodes};
use taffy::prelude::{AvailableSpace, NodeId};
use taffy::{LayoutInput, LayoutOutput};

use crate::Axis;
use crate::build::{ArrowEnds, BuiltPage, LayoutTree, TextLeaf};
use crate::{
    ARROWHEAD_LENGTH_PX, BoxRect, LayoutError, NodeGeometry, NodeTag, PageGeometry, Part, PartName,
    Size, TextRun,
};

/// Side of the Pipe dot box that an arrowhead replaces (section 2.7).
const PIPE_DOT_PX: f32 = 8.0;

pub(crate) fn compute_geometry(
    built: BuiltPage,
    page_width: u32,
    measurer: &mut dyn TextMeasurer,
) -> Result<PageGeometry, LayoutError> {
    let BuiltPage {
        mut tree,
        root,
        records,
        text_leaves,
    } = built;
    let canvas_width = page_width as f32 + 40.0;
    let mut first_error: Option<(NodePointer, MeasureError)> = None;

    tree.compute_layout_with_measure(
        root,
        taffy::Size {
            width: AvailableSpace::Definite(canvas_width),
            height: AvailableSpace::MaxContent,
        },
        |inputs: LayoutInput, _node: NodeId, context: Option<&mut usize>, style| -> LayoutOutput {
            let leaf = context.and_then(|index| text_leaves.get(*index));
            taffy::compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |known, available| match leaf {
                    None => taffy::Size::ZERO,
                    Some(leaf) => {
                        measure_in_layout(leaf, known, available, measurer, &mut first_error)
                    }
                },
            )
        },
    )
    .map_err(|error| LayoutError::Taffy {
        pointer: NodePointer::root(),
        message: error.to_string(),
    })?;

    if let Some((pointer, source)) = first_error {
        return Err(LayoutError::Measure { pointer, source });
    }

    let origins = absolute_origins(&tree, root)?;
    let mut nodes = Vec::with_capacity(records.len());
    for record in &records {
        let bounds = absolute_box(&tree, &origins, record.taffy_node, &record.pointer)?;
        let content_border_box =
            absolute_box(&tree, &origins, record.content_node, &record.pointer)?;
        let content_layout = tree.unrounded_layout(record.content_node);
        let content = inset(
            content_border_box,
            &content_layout.border,
            &content_layout.padding,
        );
        if !is_finite_box(&content) {
            return Err(LayoutError::NonFinite {
                pointer: record.pointer.clone(),
            });
        }
        let mut parts = Vec::with_capacity(record.parts.len());
        for part in &record.parts {
            let part_bounds = absolute_box(&tree, &origins, part.taffy_node, &record.pointer)?;
            let text = match part.text_leaf.and_then(|index| text_leaves.get(index)) {
                Some(leaf) => Some(remeasure(leaf, part_bounds, measurer)?),
                None => None,
            };
            parts.push(Part {
                name: part.name,
                bounds: part_bounds,
                text,
            });
        }
        if let Some(ends) = record.arrow_ends {
            shorten_wires_for_arrowheads(&mut parts, ends);
        }
        nodes.push(NodeGeometry {
            pointer: record.pointer.clone(),
            tag: record.tag,
            kind: record.kind.clone(),
            tint: record.tint,
            container: record.container,
            shape: record.shape,
            pipe_form: record.pipe_form,
            parent: record.parent,
            bounds,
            content,
            parts,
        });
    }

    let root_layout = tree.unrounded_layout(root);
    let canvas = Size {
        width: root_layout.size.width,
        height: root_layout.size.height,
    };
    if !canvas.width.is_finite() || !canvas.height.is_finite() {
        return Err(LayoutError::NonFinite {
            pointer: NodePointer::root(),
        });
    }
    Ok(PageGeometry {
        canvas,
        nodes,
        links: Vec::new(),
    })
}

/// An arrowhead has its tip on the outer edge of the dot box it replaces and its base
/// ARROWHEAD_LENGTH_PX back along the run axis, so the wire on that end starts or stops at
/// the base. The dot box itself keeps its place, and no other box moves.
fn shorten_wires_for_arrowheads(parts: &mut [Part], ends: ArrowEnds) {
    let overlap = ARROWHEAD_LENGTH_PX - PIPE_DOT_PX;
    for part in parts.iter_mut() {
        let bounds = &mut part.bounds;
        match (part.name, ends.horizontal) {
            (PartName::WireStart, true) if ends.start => {
                let shortened = overlap.min(bounds.width);
                bounds.x += shortened;
                bounds.width -= shortened;
            }
            (PartName::WireStart, false) if ends.start => {
                let shortened = overlap.min(bounds.height);
                bounds.y += shortened;
                bounds.height -= shortened;
            }
            (PartName::WireEnd, true) if ends.end => {
                bounds.width -= overlap.min(bounds.width);
            }
            (PartName::WireEnd, false) if ends.end => {
                bounds.height -= overlap.min(bounds.height);
            }
            _ => {}
        }
    }
}

/// Section 3: a known or definite width wraps at that width plus the epsilon, min-content
/// wraps at 0 and max-content does not wrap. The first error is kept and the leaf gets a
/// zero size, because taffy's closure cannot fail.
fn measure_in_layout(
    leaf: &TextLeaf,
    known: taffy::Size<Option<f32>>,
    available: taffy::Size<AvailableSpace>,
    measurer: &mut dyn TextMeasurer,
    first_error: &mut Option<(NodePointer, MeasureError)>,
) -> taffy::Size<f32> {
    if let (Some(width), Some(height)) = (known.width, known.height) {
        return taffy::Size { width, height };
    }
    // A y run's text runs along the leaf's height, so it wraps at the height the layout
    // offers and reports its size swapped (section 12.4).
    let (known_along, known_across, available_along) = match leaf.axis {
        Axis::X => (known.width, known.height, available.width),
        Axis::Y => (known.height, known.width, available.height),
    };
    let max_width_px = match (known_along, available_along) {
        (Some(width), _) | (None, AvailableSpace::Definite(width)) => {
            Some(width.max(0.0) + WRAP_EPSILON_PX)
        }
        // Min-content is the widest word flat; under iso it is the run wrapped at its
        // shortest line, or the whole run for one that never wraps (section 12.4).
        (None, AvailableSpace::MinContent) => match leaf.min_line_px {
            None => Some(0.0),
            Some(min_line) if min_line.is_infinite() => None,
            Some(min_line) => Some(min_line + WRAP_EPSILON_PX),
        },
        (None, AvailableSpace::MaxContent) => None,
    };
    match measurer.measure(&leaf.text, &leaf.style, max_width_px) {
        Ok(metrics) => {
            let along = known_along.unwrap_or(metrics.width_px);
            let across = known_across.unwrap_or(metrics.height_px);
            match leaf.axis {
                Axis::X => taffy::Size {
                    width: along,
                    height: across,
                },
                Axis::Y => taffy::Size {
                    width: across,
                    height: along,
                },
            }
        }
        Err(error) => {
            if first_error.is_none() {
                *first_error = Some((leaf.source.clone(), error));
            }
            taffy::Size::ZERO
        }
    }
}

fn remeasure(
    leaf: &TextLeaf,
    final_box: BoxRect,
    measurer: &mut dyn TextMeasurer,
) -> Result<TextRun, LayoutError> {
    let style = leaf.style;
    let final_width = match leaf.axis {
        Axis::X => final_box.width,
        Axis::Y => final_box.height,
    };
    let metrics = measurer
        .measure(
            &leaf.text,
            &style,
            Some(final_width.max(0.0) + WRAP_EPSILON_PX),
        )
        .map_err(|source| LayoutError::Measure {
            pointer: leaf.source.clone(),
            source,
        })?;
    Ok(TextRun {
        text: leaf.text.clone(),
        style,
        color: leaf.color,
        align: leaf.align,
        metrics,
    })
}

/// Absolute top-left corner of every taffy node, from a pre-order walk that sums
/// parent-relative locations. The walk visits each node once, so it is bounded by the
/// tree's node count.
fn absolute_origins(
    tree: &LayoutTree,
    root: NodeId,
) -> Result<HashMap<NodeId, (f32, f32)>, LayoutError> {
    let node_count = tree.total_node_count();
    let mut origins = HashMap::with_capacity(node_count);
    let mut pending = vec![(root, 0.0_f32, 0.0_f32)];
    for _ in 0..node_count {
        let Some((node, parent_x, parent_y)) = pending.pop() else {
            break;
        };
        let location = tree.unrounded_layout(node).location;
        let x = parent_x + location.x;
        let y = parent_y + location.y;
        origins.insert(node, (x, y));
        let children = tree.children(node).map_err(|error| LayoutError::Taffy {
            pointer: NodePointer::root(),
            message: error.to_string(),
        })?;
        pending.extend(children.into_iter().map(|child| (child, x, y)));
    }
    Ok(origins)
}

fn absolute_box(
    tree: &LayoutTree,
    origins: &HashMap<NodeId, (f32, f32)>,
    node: NodeId,
    pointer: &NodePointer,
) -> Result<BoxRect, LayoutError> {
    let &(x, y) = origins.get(&node).ok_or_else(|| LayoutError::Taffy {
        pointer: pointer.clone(),
        message: "node is not attached to the page tree".to_string(),
    })?;
    let size = tree.unrounded_layout(node).size;
    let bounds = BoxRect {
        x,
        y,
        width: size.width,
        height: size.height,
    };
    if is_finite_box(&bounds) {
        Ok(bounds)
    } else {
        Err(LayoutError::NonFinite {
            pointer: pointer.clone(),
        })
    }
}

fn inset(bounds: BoxRect, border: &taffy::Rect<f32>, padding: &taffy::Rect<f32>) -> BoxRect {
    let left = border.left + padding.left;
    let right = border.right + padding.right;
    let top = border.top + padding.top;
    let bottom = border.bottom + padding.bottom;
    BoxRect {
        x: bounds.x + left,
        y: bounds.y + top,
        width: bounds.width - left - right,
        height: bounds.height - top - bottom,
    }
}

fn is_finite_box(bounds: &BoxRect) -> bool {
    bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
}

/// A Pipe node that names a target in `from` or `to` (section 13.8), with every index
/// resolved against the geometry. An index is None only for a page that was never vetted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TargetedPipe {
    pub pointer: NodePointer,
    pub dir: PipeDir,
    pub pipe: Option<usize>,
    pub from: Option<PipeTarget>,
    pub to: Option<PipeTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PipeTarget {
    pub id: String,
    pub node: Option<usize>,
}

/// Every Pipe node of the page with a `from` or `to`, in body order. Tee arms cannot carry
/// a target, so they are never listed.
pub(crate) fn targeted_pipes(page: &Page, geometry: &PageGeometry) -> Vec<TargetedPipe> {
    let entries = body_nodes(page);
    let pointer_by_id: HashMap<&str, &NodePointer> = entries
        .iter()
        .filter_map(|entry| entry.node.id().map(|id| (id, &entry.pointer)))
        .collect();
    let index_by_pointer: HashMap<&str, usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.pointer.as_str(), index))
        .collect();
    let resolve = |id: &Option<String>| {
        id.as_ref().map(|id| PipeTarget {
            id: id.clone(),
            node: pointer_by_id
                .get(id.as_str())
                .and_then(|pointer| index_by_pointer.get(pointer.as_str()).copied()),
        })
    };
    let mut pipes = Vec::new();
    for entry in &entries {
        let NodeRef::Node(Node::Pipe(pipe)) = entry.node else {
            continue;
        };
        if pipe.from.is_none() && pipe.to.is_none() {
            continue;
        }
        pipes.push(TargetedPipe {
            pointer: entry.pointer.clone(),
            dir: pipe.dir,
            pipe: index_by_pointer.get(entry.pointer.as_str()).copied(),
            from: resolve(&pipe.from),
            to: resolve(&pipe.to),
        });
    }
    pipes
}

/// The start and end of a box on the move axis of a pipe: y for h, x for v.
pub(crate) fn move_extent(bounds: &BoxRect, dir: PipeDir) -> (f32, f32) {
    match dir {
        PipeDir::Horizontal => (bounds.y, bounds.bottom()),
        PipeDir::Vertical => (bounds.x, bounds.right()),
    }
}

/// The slot of a pipe (section 13.8 rule 1): its parent when that is a Row or Col holding
/// only Pipes and Tees, otherwise the pipe itself.
fn pipe_slot(geometry: &PageGeometry, pipe: usize) -> usize {
    let Some(parent) = geometry.nodes.get(pipe).and_then(|node| node.parent) else {
        return pipe;
    };
    let parent_is_line = geometry
        .nodes
        .get(parent)
        .is_some_and(|node| matches!(node.tag, NodeTag::Row | NodeTag::Col));
    let only_pipes = geometry
        .nodes
        .iter()
        .filter(|node| node.parent == Some(parent))
        .all(|node| matches!(node.tag, NodeTag::Pipe | NodeTag::Tee));
    if parent_is_line && only_pipes {
        parent
    } else {
        pipe
    }
}

/// The aim span of one pipe (section 13.8 rule 2): the intersection of its targets' extents
/// on the move axis, the `to` extent when the two do not overlap, or the one target's extent.
fn aim_span(geometry: &PageGeometry, pipe: &TargetedPipe) -> Option<(f32, f32)> {
    let extent = |target: &Option<PipeTarget>| {
        target
            .as_ref()
            .and_then(|target| target.node)
            .and_then(|index| geometry.nodes.get(index))
            .map(|node| move_extent(&node.bounds, pipe.dir))
    };
    match (extent(&pipe.from), extent(&pipe.to)) {
        (Some(from), Some(to)) => {
            let start = from.0.max(to.0);
            let end = from.1.min(to.1);
            if start <= end {
                Some((start, end))
            } else {
                Some(to)
            }
        }
        (Some(only), None) | (None, Some(only)) => Some(only),
        (None, None) => None,
    }
}

/// Section 13.8 rule 4: every slot holding a pipe with targets moves along the move axis so
/// its center lies on the midpoint of the union of its pipes' aim spans, clamped to its
/// parent's content box. The slot's box, its descendants and their parts move by the same
/// amount; no size changes and nothing else moves. Slots are processed in geometry order.
pub(crate) fn translate_pipe_slots(page: &Page, geometry: &mut PageGeometry) {
    let pipes = targeted_pipes(page, geometry);
    let mut slots: Vec<(usize, PipeDir)> = Vec::new();
    for pipe in &pipes {
        let Some(index) = pipe.pipe else {
            continue;
        };
        let slot = pipe_slot(geometry, index);
        if !slots.iter().any(|(known, _)| *known == slot) {
            slots.push((slot, pipe.dir));
        }
    }
    slots.sort_by_key(|(slot, _)| *slot);

    for (slot, dir) in slots {
        let mut union: Option<(f32, f32)> = None;
        for pipe in &pipes {
            let in_slot = pipe
                .pipe
                .is_some_and(|index| pipe_slot(geometry, index) == slot);
            if !in_slot || pipe.dir != dir {
                continue;
            }
            if let Some((start, end)) = aim_span(geometry, pipe) {
                union = Some(match union {
                    None => (start, end),
                    Some((union_start, union_end)) => (union_start.min(start), union_end.max(end)),
                });
            }
        }
        let Some((aim_start, aim_end)) = union else {
            continue;
        };
        let Some(slot_node) = geometry.nodes.get(slot) else {
            continue;
        };
        let (slot_start, slot_end) = move_extent(&slot_node.bounds, dir);
        let slot_size = slot_end - slot_start;
        let mut new_start = (aim_start + aim_end) / 2.0 - slot_size / 2.0;
        if let Some(parent) = slot_node
            .parent
            .and_then(|parent| geometry.nodes.get(parent))
        {
            let (content_start, content_end) = move_extent(&parent.content, dir);
            new_start = new_start.min(content_end - slot_size).max(content_start);
        }
        let shift = new_start - slot_start;
        if shift != 0.0 {
            translate_subtree(geometry, slot, dir, shift);
        }
    }
}

/// Moves the node at `root`, its descendants and every part of each by `shift` along the
/// move axis. Geometry order is pre-order, so a parent comes before its children.
fn translate_subtree(geometry: &mut PageGeometry, root: usize, dir: PipeDir, shift: f32) {
    let mut in_subtree = vec![false; geometry.nodes.len()];
    for index in 0..geometry.nodes.len() {
        let parent_inside = geometry
            .nodes
            .get(index)
            .and_then(|node| node.parent)
            .and_then(|parent| in_subtree.get(parent).copied())
            .unwrap_or(false);
        if let Some(flag) = in_subtree.get_mut(index) {
            *flag = index == root || parent_inside;
        }
    }
    let shifted = |bounds: &mut BoxRect| match dir {
        PipeDir::Horizontal => bounds.y += shift,
        PipeDir::Vertical => bounds.x += shift,
    };
    for (node, inside) in geometry.nodes.iter_mut().zip(in_subtree) {
        if !inside {
            continue;
        }
        shifted(&mut node.bounds);
        shifted(&mut node.content);
        for part in &mut node.parts {
            shifted(&mut part.bounds);
        }
    }
}
