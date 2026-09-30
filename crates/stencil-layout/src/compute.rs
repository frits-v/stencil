//! Runs taffy with the measure closure of section 3, re-measures every text leaf at its final
//! width and converts parent-relative taffy boxes into absolute geometry.

use std::collections::HashMap;

use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, WRAP_EPSILON_PX};
use taffy::prelude::{AvailableSpace, NodeId};
use taffy::{LayoutInput, LayoutOutput};

use crate::build::{BuiltPage, LayoutTree, TextLeaf};
use crate::{BoxRect, LayoutError, NodeGeometry, PageGeometry, Part, Size, TextRun};

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
                Some(leaf) => Some(remeasure(leaf, part_bounds.width, measurer)?),
                None => None,
            };
            parts.push(Part {
                name: part.name,
                bounds: part_bounds,
                text,
            });
        }
        nodes.push(NodeGeometry {
            pointer: record.pointer.clone(),
            tag: record.tag,
            kind: record.kind,
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
    Ok(PageGeometry { canvas, nodes })
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
    let max_width_px = match (known.width, available.width) {
        (Some(width), _) | (None, AvailableSpace::Definite(width)) => {
            Some(width.max(0.0) + WRAP_EPSILON_PX)
        }
        (None, AvailableSpace::MinContent) => Some(0.0),
        (None, AvailableSpace::MaxContent) => None,
    };
    let style = leaf.style_name.text_style().style;
    match measurer.measure(&leaf.text, &style, max_width_px) {
        Ok(metrics) => taffy::Size {
            width: known.width.unwrap_or(metrics.width_px),
            height: known.height.unwrap_or(metrics.height_px),
        },
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
    final_width: f32,
    measurer: &mut dyn TextMeasurer,
) -> Result<TextRun, LayoutError> {
    let style = leaf.style_name.text_style().style;
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
