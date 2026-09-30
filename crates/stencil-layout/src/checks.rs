//! The three geometry checks of section 6.

use stencil_model::checks::{CheckName, CheckReport, Defect};

use crate::{BoxRect, GEOMETRY_EPSILON_PX, PageGeometry};

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
    }
}

/// Each unordered pair of nodes with the same parent: the border boxes may touch but not
/// overlap by more than the epsilon in both axes. The defect sits on the later sibling.
pub fn siblings_do_not_overlap(geometry: &PageGeometry) -> CheckReport {
    let mut children_by_parent: Vec<Vec<usize>> = vec![Vec::new(); geometry.nodes.len()];
    for (index, node) in geometry.nodes.iter().enumerate() {
        if let Some(siblings) = node
            .parent
            .and_then(|parent_index| children_by_parent.get_mut(parent_index))
        {
            siblings.push(index);
        }
    }

    let mut examined: u64 = 0;
    let mut defects = Vec::new();
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
    }
}

/// Each text part: the measured run must fit the part box, and the part box must stay inside
/// its node's border box.
pub fn text_fits_box(geometry: &PageGeometry) -> CheckReport {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for node in &geometry.nodes {
        for part in &node.parts {
            let Some(run) = &part.text else {
                continue;
            };
            examined += 1;
            let metrics = &run.metrics;
            if metrics.width_px > part.bounds.width + GEOMETRY_EPSILON_PX
                || metrics.height_px > part.bounds.height + GEOMETRY_EPSILON_PX
            {
                defects.push(Defect {
                    pointer: node.pointer.clone(),
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
            if extends_outside(&part.bounds, &node.bounds) {
                defects.push(Defect {
                    pointer: node.pointer.clone(),
                    message: format!(
                        "{} box {} extends outside the node box {}",
                        part.name.as_str(),
                        describe(&part.bounds),
                        describe(&node.bounds)
                    ),
                });
            }
        }
    }
    CheckReport {
        check: CheckName::TextFitsBox,
        examined,
        defects,
    }
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
