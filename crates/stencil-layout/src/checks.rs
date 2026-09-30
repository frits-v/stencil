//! The geometry checks of section 6 and the two link checks of section 11.2.

use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::pointer::NodePointer;

use crate::route::{link_obstacles, segment_enters};
use crate::{BoxRect, GEOMETRY_EPSILON_PX, PageGeometry, Part, RouteStatus};

/// Why the link checks do not apply to a page without links.
const NO_LINKS: &str = "page has no links";

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
        if metrics.width_px > part.bounds.width + GEOMETRY_EPSILON_PX
            || metrics.height_px > part.bounds.height + GEOMETRY_EPSILON_PX
        {
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
            if overlap_both_axes(&tag, &node.bounds) {
                defects.push(Defect {
                    pointer: pointer.clone(),
                    message: format!(
                        "tag {} overlaps {} box {}",
                        describe(&tag),
                        node.pointer,
                        describe(&node.bounds)
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
