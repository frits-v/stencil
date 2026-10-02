//! The geometry checks of section 6, the two link checks of section 11.2 and print-fit of
//! section 13.10.

use std::collections::HashMap;

use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;
use stencil_model::{Node, NodeRef, Page, PipeDir, body_nodes};

use crate::compute::{PipeTarget, move_extent, targeted_pipes};
use crate::route::{link_obstacles, segment_enters};
use crate::{BoxRect, GEOMETRY_EPSILON_PX, NodeTag, PageGeometry, Part, PartName, RouteStatus};

/// Why print-fit does not apply when no `--print-width` was given.
const NO_PRINT_WIDTH: &str = "no print width";

/// The smallest type size a printed figure may carry, in points.
pub const PRINT_TYPE_MIN_PT: f32 = 8.0;
/// Tolerance of print-fit, in points.
pub const PRINT_EPSILON_PT: f32 = 0.01;
const POINTS_PER_INCH: f32 = 72.0;

pub const PRINT_WIDTH_MIN_INCHES: f32 = 0.5;
pub const PRINT_WIDTH_MAX_INCHES: f32 = 200.0;

/// The width a figure prints at, in inches: finite and from PRINT_WIDTH_MIN_INCHES to
/// PRINT_WIDTH_MAX_INCHES (section 13.10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrintWidth(f32);

#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[error(
    "print width {0} is not a number of inches from {PRINT_WIDTH_MIN_INCHES} to {PRINT_WIDTH_MAX_INCHES}"
)]
pub struct PrintWidthError(pub f32);

impl PrintWidth {
    pub fn new(inches: f32) -> Result<Self, PrintWidthError> {
        if inches.is_finite() && (PRINT_WIDTH_MIN_INCHES..=PRINT_WIDTH_MAX_INCHES).contains(&inches)
        {
            Ok(PrintWidth(inches))
        } else {
            Err(PrintWidthError(inches))
        }
    }

    pub fn inches(self) -> f32 {
        self.0
    }
}

/// Each text run of every node part and every link tag, counted as text-fits-box counts
/// them: a run whose size would print below 8 pt when the drawn canvas, `canvas_width_px`
/// wide, prints `print_width` wide is a defect at the node that owns it, or at `/links/<i>`.
/// Not applicable without a print width.
pub fn print_fit(
    geometry: &PageGeometry,
    canvas_width_px: f32,
    print_width: Option<PrintWidth>,
) -> CheckReport {
    let Some(print_width) = print_width else {
        return CheckReport::not_applicable(CheckName::PrintFit, NO_PRINT_WIDTH);
    };
    let inches = print_width.inches();
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    let mut examine = |owner: &NodePointer, parts: &[Part]| {
        for part in parts {
            let Some(run) = &part.text else {
                continue;
            };
            examined += 1;
            let size_px = run.style.size_px;
            let points = size_px * POINTS_PER_INCH * inches / canvas_width_px;
            if points + PRINT_EPSILON_PT < PRINT_TYPE_MIN_PT {
                defects.push(Defect {
                    pointer: owner.clone(),
                    message: format!(
                        "{} {:?} prints at {points:.2} pt, below {PRINT_TYPE_MIN_PT:.0} pt ({size_px:.2} px on a {canvas_width_px:.2} px canvas at {inches:.2} in)",
                        part.name.as_str(),
                        run.text,
                    ),
                });
            }
        }
    };
    for node in &geometry.nodes {
        examine(&node.pointer, &node.parts);
    }
    let links_pointer = NodePointer::root().child("links");
    for route in &geometry.links {
        if route.tag.is_some() {
            examine(&links_pointer.index(route.index), &route.parts);
        }
    }
    CheckReport {
        check: CheckName::PrintFit,
        examined,
        defects,
        not_applicable: None,
    }
}

/// Why the link checks do not apply to a page without links.
const NO_LINKS: &str = "page has no links";
/// Why pipes-land does not apply to a page without a Pipe or Tee arm.
const NO_PIPES: &str = "page has no pipes";
/// Why pipes-land does not apply when no pipe sits beside a Row or Col sibling.
const NO_PIPE_NEIGHBORS: &str = "no pipe has a neighbor";

/// Each (parent, child) pair, the root excluded as a child: the child's border box must stay
/// inside the parent's content box.
pub fn child_inside_container(geometry: &PageGeometry) -> CheckReport {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for child in &geometry.nodes {
        let Some(parent_index) = child.parent else {
            continue;
        };
        examined += 1;
        let Some(parent) = geometry.nodes.get(parent_index) else {
            defects.push(Defect {
                pointer: child.pointer.clone(),
                message: format!("parent index {parent_index} is not a geometry node"),
            });
            continue;
        };
        if extends_outside(&child.bounds, &parent.content) {
            defects.push(Defect {
                pointer: child.pointer.clone(),
                message: format!(
                    "box {} extends outside the content box {} of {}",
                    describe(&child.bounds),
                    describe(&parent.content),
                    parent.pointer
                ),
            });
        }
    }
    CheckReport {
        check: CheckName::ChildInsideContainer,
        examined,
        defects,
        not_applicable: None,
    }
}

/// Each unordered pair of nodes with the same parent: the border boxes may touch but not
/// overlap by more than the epsilon in both axes. The defect sits on the later sibling. A node
/// whose parent index is not a geometry node is examined and reported, as in
/// child-inside-container.
pub fn siblings_do_not_overlap(geometry: &PageGeometry) -> CheckReport {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    let mut children_by_parent: Vec<Vec<usize>> = vec![Vec::new(); geometry.nodes.len()];
    for (index, node) in geometry.nodes.iter().enumerate() {
        let Some(parent_index) = node.parent else {
            continue;
        };
        match children_by_parent.get_mut(parent_index) {
            Some(siblings) => siblings.push(index),
            None => {
                examined += 1;
                defects.push(Defect {
                    pointer: node.pointer.clone(),
                    message: format!("parent index {parent_index} is not a geometry node"),
                });
            }
        }
    }

    for siblings in &children_by_parent {
        for (position, &earlier_index) in siblings.iter().enumerate() {
            for &later_index in siblings.iter().skip(position + 1) {
                let (Some(earlier), Some(later)) = (
                    geometry.nodes.get(earlier_index),
                    geometry.nodes.get(later_index),
                ) else {
                    continue;
                };
                examined += 1;
                let overlap_width = earlier.bounds.right().min(later.bounds.right())
                    - earlier.bounds.x.max(later.bounds.x);
                let overlap_height = earlier.bounds.bottom().min(later.bounds.bottom())
                    - earlier.bounds.y.max(later.bounds.y);
                if overlap_width > GEOMETRY_EPSILON_PX && overlap_height > GEOMETRY_EPSILON_PX {
                    defects.push(Defect {
                        pointer: later.pointer.clone(),
                        message: format!(
                            "box {} overlaps {} box {} by {:.2}x{:.2}",
                            describe(&later.bounds),
                            earlier.pointer,
                            describe(&earlier.bounds),
                            overlap_width,
                            overlap_height
                        ),
                    });
                }
            }
        }
    }
    CheckReport {
        check: CheckName::SiblingsDoNotOverlap,
        examined,
        defects,
        not_applicable: None,
    }
}

/// Each text part of every node and every link tag: the measured run must fit the part box,
/// and the part box must stay inside its node's border box, or for a link inside its tag.
pub fn text_fits_box(geometry: &PageGeometry) -> CheckReport {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for node in &geometry.nodes {
        examine_text_parts(
            &node.pointer,
            &node.parts,
            &node.bounds,
            &mut examined,
            &mut defects,
        );
    }
    let links_pointer = NodePointer::root().child("links");
    for route in &geometry.links {
        if let Some(tag) = route.tag {
            examine_text_parts(
                &links_pointer.index(route.index),
                &route.parts,
                &tag,
                &mut examined,
                &mut defects,
            );
        }
    }
    CheckReport {
        check: CheckName::TextFitsBox,
        examined,
        defects,
        not_applicable: None,
    }
}

fn examine_text_parts(
    owner: &NodePointer,
    parts: &[Part],
    owner_bounds: &BoxRect,
    examined: &mut u64,
    defects: &mut Vec<Defect>,
) {
    for part in parts {
        let Some(run) = &part.text else {
            continue;
        };
        *examined += 1;
        let metrics = &run.metrics;
        // A run that reads along y under iso is laid out as a strip with its width and
        // height swapped (section 12.4), so it fits when it fits the box turned.
        let fits = |width: f32, height: f32| {
            metrics.width_px <= width + GEOMETRY_EPSILON_PX
                && metrics.height_px <= height + GEOMETRY_EPSILON_PX
        };
        let turned_strip =
            part.bounds.height > part.bounds.width && fits(part.bounds.height, part.bounds.width);
        if !fits(part.bounds.width, part.bounds.height) && !turned_strip {
            defects.push(Defect {
                pointer: owner.clone(),
                message: format!(
                    "{} {:?} measured {:.2}x{:.2} in box {:.2}x{:.2}",
                    part.name.as_str(),
                    run.text,
                    metrics.width_px,
                    metrics.height_px,
                    part.bounds.width,
                    part.bounds.height
                ),
            });
        }
        if extends_outside(&part.bounds, owner_bounds) {
            defects.push(Defect {
                pointer: owner.clone(),
                message: format!(
                    "{} box {} extends outside the node box {}",
                    part.name.as_str(),
                    describe(&part.bounds),
                    describe(owner_bounds)
                ),
            });
        }
    }
}

/// One link examined per route; a route that fell back to the L shape is a defect at
/// `/links/<i>`. Not applicable on a page without links.
pub fn links_routed(geometry: &PageGeometry) -> CheckReport {
    if geometry.links.is_empty() {
        return CheckReport::not_applicable(CheckName::LinksRouted, NO_LINKS);
    }
    let links_pointer = NodePointer::root().child("links");
    let mut defects = Vec::new();
    for route in &geometry.links {
        if route.status == RouteStatus::Fallback {
            defects.push(Defect {
                pointer: links_pointer.index(route.index),
                message: format!(
                    "no route from {} to {} avoids every obstacle; drawn as a fallback L",
                    node_pointer_text(geometry, route.from_node),
                    node_pointer_text(geometry, route.to_node)
                ),
            });
        }
    }
    CheckReport {
        check: CheckName::LinksRouted,
        examined: count_as_u64(geometry.links.len()),
        defects,
        not_applicable: None,
    }
}

/// Each (segment, obstacle) pair of every link, with the obstacles the router avoided for
/// that link, plus each (tag, node) pair for every node that is not an ancestor of an
/// endpoint. A segment entering an obstacle's interior or a tag overlapping such a node by
/// more than the epsilon on both axes is a defect at `/links/<i>`. Not applicable on a page
/// without links.
pub fn links_avoid_boxes(geometry: &PageGeometry) -> CheckReport {
    if geometry.links.is_empty() {
        return CheckReport::not_applicable(CheckName::LinksAvoidBoxes, NO_LINKS);
    }
    let links_pointer = NodePointer::root().child("links");
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for route in &geometry.links {
        let pointer = links_pointer.index(route.index);
        let obstacles = link_obstacles(geometry, route.from_node, route.to_node);
        for (segment_index, segment) in route.points.windows(2).enumerate() {
            let (Some(&start), Some(&end)) = (segment.first(), segment.get(1)) else {
                continue;
            };
            for obstacle in &obstacles {
                examined += 1;
                if segment_enters(start, end, &obstacle.bounds) {
                    defects.push(Defect {
                        pointer: pointer.clone(),
                        message: format!(
                            "segment {segment_index} from ({:.2}, {:.2}) to ({:.2}, {:.2}) enters {} box {}",
                            start.x,
                            start.y,
                            end.x,
                            end.y,
                            node_pointer_text(geometry, obstacle.node),
                            describe(&obstacle.bounds)
                        ),
                    });
                }
            }
        }
        let Some(tag) = route.tag else {
            continue;
        };
        let ancestors = endpoint_ancestors(geometry, route.from_node, route.to_node);
        for (index, node) in geometry.nodes.iter().enumerate() {
            if ancestors.get(index).copied().unwrap_or(true) {
                continue;
            }
            examined += 1;
            // An item standing on a footprint under iso is its footprint and its floor
            // text; the room between them is open floor (section 12.3).
            let boxes: Vec<BoxRect> =
                match (node.part(PartName::Footprint), node.part(PartName::Text)) {
                    (Some(footprint), Some(text)) => vec![footprint.bounds, text.bounds],
                    _ => vec![node.bounds],
                };
            if let Some(hit) = boxes.iter().find(|bounds| overlap_both_axes(&tag, bounds)) {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!(
                        "tag {} overlaps {} box {}",
                        describe(&tag),
                        node.pointer,
                        describe(hit)
                    ),
                });
            }
        }
    }
    CheckReport {
        check: CheckName::LinksAvoidBoxes,
        examined,
        defects,
        not_applicable: None,
    }
}

/// One pipe end facing a neighbor: the pipe, the side, and the Row or Col child on that side.
struct PipeEnd {
    pipe: usize,
    dir: PipeDir,
    side: &'static str,
    neighbor: usize,
}

/// One examined pipe end: a neighbor end of section 6, or an end aimed at a named target.
enum ExaminedEnd<'a> {
    Neighbor(PipeEnd),
    Target {
        pipe: usize,
        dir: PipeDir,
        end: TargetEnd,
        target: &'a PipeTarget,
    },
}

/// Which end of a pipe a target names: `from` the left (h) or upper (v) end, `to` the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetEnd {
    From,
    To,
}

impl TargetEnd {
    fn as_str(self) -> &'static str {
        match self {
            TargetEnd::From => "from",
            TargetEnd::To => "to",
        }
    }
}

/// The defect message of a pipe end aimed at a named target (section 13.8 rule 6), or None
/// when the end lands: the pipe's center on the move axis lies within the target's extent,
/// the epsilon included, and the target lies on the end's side along the run axis.
fn target_end_defect(
    pipe: &crate::NodeGeometry,
    dir: PipeDir,
    end: TargetEnd,
    target: &crate::NodeGeometry,
) -> Option<String> {
    let (center, axis) = match dir {
        PipeDir::Horizontal => (pipe.bounds.y + pipe.bounds.height / 2.0, "y"),
        PipeDir::Vertical => (pipe.bounds.x + pipe.bounds.width / 2.0, "x"),
    };
    let (start, finish) = move_extent(&target.bounds, dir);
    if center < start - GEOMETRY_EPSILON_PX || center > finish + GEOMETRY_EPSILON_PX {
        return Some(format!(
            "{} target {} has no box across the pipe's center {axis} {center:.2}",
            end.as_str(),
            target.pointer
        ));
    }
    let (pipe_run_center, target_run_center) = match dir {
        PipeDir::Horizontal => (
            pipe.bounds.x + pipe.bounds.width / 2.0,
            target.bounds.x + target.bounds.width / 2.0,
        ),
        PipeDir::Vertical => (
            pipe.bounds.y + pipe.bounds.height / 2.0,
            target.bounds.y + target.bounds.height / 2.0,
        ),
    };
    let on_its_side = match end {
        TargetEnd::From => target_run_center < pipe_run_center,
        TargetEnd::To => target_run_center > pipe_run_center,
    };
    if on_its_side {
        return None;
    }
    let wrong_side = match (dir, end) {
        (PipeDir::Horizontal, TargetEnd::From) => "on the right of",
        (PipeDir::Horizontal, TargetEnd::To) => "on the left of",
        (PipeDir::Vertical, TargetEnd::From) => "below",
        (PipeDir::Vertical, TargetEnd::To) => "above",
    };
    Some(format!(
        "{} target {} lies {wrong_side} the pipe",
        end.as_str(),
        target.pointer
    ))
}

/// Each pipe end that faces a neighbor (section 6). A Pipe h, Tee arms included, looks along
/// the Row that holds its nearest ancestor-or-self Row child: that child's siblings directly
/// left and right are its neighbors. A Pipe v does the same along a Col, above and below. On
/// each side the pipe's center on the cross axis must fall within the extent of some node in
/// the neighbor's subtree that is not a Row or Col. A Pipe that names a target examines one
/// end per target in place of the neighbor on that side (section 13.8 rule 6). A defect sits
/// on the pipe and names the side and the neighbor or target. Not applicable on a page
/// without pipes, or when no pipe has a neighbor or a target.
pub fn pipes_land(page: &Page, geometry: &PageGeometry) -> CheckReport {
    let mut pipes: Vec<(NodePointer, PipeDir)> = Vec::new();
    for entry in body_nodes(page) {
        match entry.node {
            NodeRef::Node(Node::Pipe(pipe)) => pipes.push((entry.pointer, pipe.dir)),
            NodeRef::TeeArm(arm) => pipes.push((entry.pointer, arm.dir)),
            NodeRef::Node(
                Node::Row(_)
                | Node::Col(_)
                | Node::Lanes(_)
                | Node::Box(_)
                | Node::Item(_)
                | Node::Fact(_)
                | Node::Note(_)
                | Node::Tee(_)
                | Node::Text(_)
                | Node::Callout(_)
                | Node::Frame(_),
            ) => {}
        }
    }
    if pipes.is_empty() {
        return CheckReport::not_applicable(CheckName::PipesLand, NO_PIPES);
    }

    let index_by_pointer: HashMap<&str, usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.pointer.as_str(), index))
        .collect();
    let children_by_parent = children_by_parent(geometry);

    let targeted = targeted_pipes(page, geometry);
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    let mut ends: Vec<ExaminedEnd<'_>> = Vec::new();
    for (pointer, dir) in &pipes {
        let Some(&pipe) = index_by_pointer.get(pointer.as_str()) else {
            examined += 1;
            defects.push(Defect {
                pointer: pointer.clone(),
                message: "pipe is not a geometry node".to_string(),
            });
            continue;
        };
        let targets = targeted
            .iter()
            .find(|targeted| targeted.pointer == *pointer);
        let from = targets.and_then(|targets| targets.from.as_ref());
        let to = targets.and_then(|targets| targets.to.as_ref());
        let (before_side, after_side) = match dir {
            PipeDir::Horizontal => ("left", "right"),
            PipeDir::Vertical => ("above", "below"),
        };
        let mut neighbors = pipe_ends(geometry, &children_by_parent, pipe, *dir);
        let before = neighbors.iter().position(|end| end.side == before_side);
        let before = before.map(|position| neighbors.remove(position));
        let after = neighbors.into_iter().find(|end| end.side == after_side);
        for (target, end, neighbor) in [(from, TargetEnd::From, before), (to, TargetEnd::To, after)]
        {
            match (target, neighbor) {
                (Some(target), _) => ends.push(ExaminedEnd::Target {
                    pipe,
                    dir: *dir,
                    end,
                    target,
                }),
                (None, Some(neighbor)) => ends.push(ExaminedEnd::Neighbor(neighbor)),
                (None, None) => {}
            }
        }
    }
    if ends.is_empty() && defects.is_empty() {
        return CheckReport::not_applicable(CheckName::PipesLand, NO_PIPE_NEIGHBORS);
    }

    for examined_end in &ends {
        match examined_end {
            ExaminedEnd::Target {
                pipe: pipe_index,
                dir,
                end,
                target,
            } => {
                let Some(pipe) = geometry.nodes.get(*pipe_index) else {
                    continue;
                };
                examined += 1;
                let message = match target.node.and_then(|index| geometry.nodes.get(index)) {
                    Some(target_node) => target_end_defect(pipe, *dir, *end, target_node),
                    None => Some(format!(
                        "{} target \"{}\" is not a geometry node",
                        end.as_str(),
                        target.id
                    )),
                };
                if let Some(message) = message {
                    defects.push(Defect {
                        pointer: pipe.pointer.clone(),
                        message,
                    });
                }
            }
            ExaminedEnd::Neighbor(end) => {
                let (Some(pipe), Some(neighbor)) = (
                    geometry.nodes.get(end.pipe),
                    geometry.nodes.get(end.neighbor),
                ) else {
                    continue;
                };
                examined += 1;
                let (center, axis) = match end.dir {
                    PipeDir::Horizontal => (pipe.bounds.y + pipe.bounds.height / 2.0, "y"),
                    PipeDir::Vertical => (pipe.bounds.x + pipe.bounds.width / 2.0, "x"),
                };
                if !subtree_spans(geometry, &children_by_parent, end.neighbor, end.dir, center) {
                    defects.push(Defect {
                        pointer: pipe.pointer.clone(),
                        message: format!(
                            "{} neighbor {} has no box across the pipe's center {axis} {center:.2}",
                            end.side, neighbor.pointer
                        ),
                    });
                }
            }
        }
    }
    CheckReport {
        check: CheckName::PipesLand,
        examined,
        defects,
        not_applicable: None,
    }
}

/// Indices of each node's children, in geometry order.
fn children_by_parent(geometry: &PageGeometry) -> Vec<Vec<usize>> {
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); geometry.nodes.len()];
    for (index, node) in geometry.nodes.iter().enumerate() {
        if let Some(siblings) = node.parent.and_then(|parent| children.get_mut(parent)) {
            siblings.push(index);
        }
    }
    children
}

/// The neighbors of the pipe's nearest ancestor-or-self whose parent is a Row (for h) or a
/// Col (for v), with the side each one is on. Empty when there is no such ancestor or it has
/// no siblings.
fn pipe_ends(
    geometry: &PageGeometry,
    children_by_parent: &[Vec<usize>],
    pipe: usize,
    dir: PipeDir,
) -> Vec<PipeEnd> {
    // A Lanes node holds its heads in a row, so an h pipe among the heads looks along them
    // as along a Row's children.
    let (containers, before_side, after_side) = match dir {
        PipeDir::Horizontal => ([NodeTag::Row, NodeTag::Lanes], "left", "right"),
        PipeDir::Vertical => ([NodeTag::Col, NodeTag::Col], "above", "below"),
    };
    let mut current = pipe;
    for _ in 0..geometry.nodes.len() {
        let Some(parent) = geometry.nodes.get(current).and_then(|node| node.parent) else {
            return Vec::new();
        };
        let Some(parent_node) = geometry.nodes.get(parent) else {
            return Vec::new();
        };
        if !containers.contains(&parent_node.tag) {
            current = parent;
            continue;
        }
        let siblings = children_by_parent
            .get(parent)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let Some(position) = siblings.iter().position(|&sibling| sibling == current) else {
            return Vec::new();
        };
        let mut ends = Vec::new();
        let before = position
            .checked_sub(1)
            .and_then(|index| siblings.get(index));
        if let Some(&neighbor) = before {
            ends.push(PipeEnd {
                pipe,
                dir,
                side: before_side,
                neighbor,
            });
        }
        if let Some(&neighbor) = siblings.get(position + 1) {
            ends.push(PipeEnd {
                pipe,
                dir,
                side: after_side,
                neighbor,
            });
        }
        return ends;
    }
    Vec::new()
}

/// True when some node in the subtree rooted at `root`, `root` included, that is not a Row,
/// Col or Lanes spans `center` on the pipe's cross axis: y for a Pipe h, x for a Pipe v.
fn subtree_spans(
    geometry: &PageGeometry,
    children_by_parent: &[Vec<usize>],
    root: usize,
    dir: PipeDir,
    center: f32,
) -> bool {
    let mut pending = vec![root];
    for _ in 0..geometry.nodes.len() {
        let Some(index) = pending.pop() else {
            return false;
        };
        let Some(node) = geometry.nodes.get(index) else {
            continue;
        };
        let (start, end) = match dir {
            PipeDir::Horizontal => (node.bounds.y, node.bounds.bottom()),
            PipeDir::Vertical => (node.bounds.x, node.bounds.right()),
        };
        if pipe_lands_on(node.tag)
            && center >= start - GEOMETRY_EPSILON_PX
            && center <= end + GEOMETRY_EPSILON_PX
        {
            return true;
        }
        if let Some(children) = children_by_parent.get(index) {
            pending.extend(children.iter().copied());
        }
    }
    false
}

/// A pipe points at a box it can land on. A Row, Col or Lanes only arranges its children,
/// so it never counts; the page-level tags never occur inside a body subtree.
fn pipe_lands_on(tag: NodeTag) -> bool {
    match tag {
        NodeTag::Zone
        | NodeTag::Pcard
        | NodeTag::Fact
        | NodeTag::Note
        | NodeTag::Pipe
        | NodeTag::Tee
        | NodeTag::Text
        | NodeTag::Callout
        | NodeTag::Frame => true,
        NodeTag::Row
        | NodeTag::Col
        | NodeTag::Lanes
        | NodeTag::Page
        | NodeTag::Kicker
        | NodeTag::Title
        | NodeTag::Lede
        | NodeTag::Body
        | NodeTag::Legend
        | NodeTag::LegendEntry
        | NodeTag::Foot => false,
    }
}

/// Flags every strict ancestor of either endpoint.
fn endpoint_ancestors(geometry: &PageGeometry, from: usize, to: usize) -> Vec<bool> {
    let mut ancestors = vec![false; geometry.nodes.len()];
    for endpoint in [from, to] {
        let mut current = geometry.nodes.get(endpoint).and_then(|node| node.parent);
        for _ in 0..geometry.nodes.len() {
            let Some(index) = current else {
                break;
            };
            if let Some(flag) = ancestors.get_mut(index) {
                *flag = true;
            }
            current = geometry.nodes.get(index).and_then(|node| node.parent);
        }
    }
    ancestors
}

fn overlap_both_axes(first: &BoxRect, second: &BoxRect) -> bool {
    let overlap_width = first.right().min(second.right()) - first.x.max(second.x);
    let overlap_height = first.bottom().min(second.bottom()) - first.y.max(second.y);
    overlap_width > GEOMETRY_EPSILON_PX && overlap_height > GEOMETRY_EPSILON_PX
}

fn node_pointer_text(geometry: &PageGeometry, index: usize) -> String {
    geometry.nodes.get(index).map_or_else(
        || format!("geometry node {index}"),
        |node| node.pointer.to_string(),
    )
}

fn count_as_u64(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

fn extends_outside(inner: &BoxRect, outer: &BoxRect) -> bool {
    inner.x < outer.x - GEOMETRY_EPSILON_PX
        || inner.y < outer.y - GEOMETRY_EPSILON_PX
        || inner.right() > outer.right() + GEOMETRY_EPSILON_PX
        || inner.bottom() > outer.bottom() + GEOMETRY_EPSILON_PX
}

fn describe(bounds: &BoxRect) -> String {
    format!(
        "{:.2}x{:.2} at ({:.2}, {:.2})",
        bounds.width, bounds.height, bounds.x, bounds.y
    )
}
