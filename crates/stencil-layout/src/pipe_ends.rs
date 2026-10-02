//! Section 12.3 rule 5, under iso: a pipe end reaches the box it lands on, or the solid of
//! the target it names, and a Tee's spine stands on its left neighbour's edge, so no tube
//! or band stops on bare floor a Row gap short of what it joins.

use stencil_model::{Node, NodeRef, Page, PipeDir, Projection, body_nodes};

use crate::checks::{children_by_parent, landing_box, pipe_ends};
use crate::compute::targeted_pipes;
use crate::route::attach_box;
use crate::{BoxRect, NodeTag, PageGeometry, PartName};

/// One dot or spine part to move: the node, the part, and the coordinate on the run axis
/// its center reaches.
struct Reach {
    node: usize,
    part: PartName,
    dir: PipeDir,
    to: f32,
}

pub(crate) fn extend_pipe_ends(page: &Page, geometry: &mut PageGeometry) {
    if page.projection != Projection::Iso {
        return;
    }
    let mut pipes: Vec<(String, PipeDir)> = Vec::new();
    for entry in body_nodes(page) {
        match entry.node {
            NodeRef::Node(Node::Pipe(pipe)) => pipes.push((entry.pointer.to_string(), pipe.dir)),
            NodeRef::TeeArm(arm) => pipes.push((entry.pointer.to_string(), arm.dir)),
            NodeRef::Node(_) => {}
        }
    }
    let children = children_by_parent(geometry);
    let targeted = targeted_pipes(page, geometry);
    let mut reaches: Vec<Reach> = Vec::new();
    for (pointer, dir) in &pipes {
        let Some(pipe) = geometry
            .nodes
            .iter()
            .position(|node| node.pointer.as_str() == pointer)
        else {
            continue;
        };
        let targets = targeted
            .iter()
            .find(|targeted| targeted.pointer.as_str() == pointer);
        let from_target = targets.and_then(|targets| targets.from.as_ref()?.node);
        let to_target = targets.and_then(|targets| targets.to.as_ref()?.node);
        let ends = pipe_ends(geometry, &children, pipe, *dir);
        let (before_side, after_side) = match dir {
            PipeDir::Horizontal => ("left", "right"),
            PipeDir::Vertical => ("above", "below"),
        };
        for (side, part, target) in [
            (before_side, PartName::DotStart, from_target),
            (after_side, PartName::DotEnd, to_target),
        ] {
            // An untargeted end reaches the nearest node across the pipe's center in the
            // neighbour's subtree, not the neighbour's outer box: a Col or an item with
            // wide floor text would leave the tube short of the solid.
            let landing: Option<BoxRect> = match target {
                Some(target) => geometry.nodes.get(target).map(attach_box),
                None => ends.iter().find(|end| end.side == side).and_then(|end| {
                    let center = match dir {
                        PipeDir::Horizontal => geometry
                            .nodes
                            .get(pipe)
                            .map(|node| node.bounds.y + node.bounds.height / 2.0),
                        PipeDir::Vertical => geometry
                            .nodes
                            .get(pipe)
                            .map(|node| node.bounds.x + node.bounds.width / 2.0),
                    }?;
                    let pipe_before = part == PartName::DotEnd;
                    landing_box(
                        geometry,
                        &children,
                        end.neighbor,
                        (*dir, center, pipe_before),
                    )
                    .map(|(_, attach)| attach)
                }),
            };
            let Some(bounds) = landing else {
                continue;
            };
            let to = match (dir, part) {
                (PipeDir::Horizontal, PartName::DotStart) => bounds.right(),
                (PipeDir::Horizontal, _) => bounds.x,
                (PipeDir::Vertical, PartName::DotStart) => bounds.bottom(),
                (PipeDir::Vertical, _) => bounds.y,
            };
            reaches.push(Reach {
                node: pipe,
                part,
                dir: *dir,
                to,
            });
        }
    }
    for (index, node) in geometry.nodes.iter().enumerate() {
        if node.tag != NodeTag::Tee {
            continue;
        }
        let left = pipe_ends(geometry, &children, index, PipeDir::Horizontal)
            .into_iter()
            .find(|end| end.side == "left")
            .and_then(|end| geometry.nodes.get(end.neighbor))
            .map(|neighbor| neighbor.bounds.right());
        if let Some(to) = left {
            reaches.push(Reach {
                node: index,
                part: PartName::Spine,
                dir: PipeDir::Horizontal,
                to,
            });
        }
    }
    for reach in reaches {
        let Some(part) = geometry
            .nodes
            .get_mut(reach.node)
            .and_then(|node| node.parts.iter_mut().find(|part| part.name == reach.part))
        else {
            continue;
        };
        let bounds = &mut part.bounds;
        // An end only ever moves outward, never back into its own slot.
        let outward = |center: f32| match reach.part {
            PartName::DotEnd => reach.to > center,
            _ => reach.to < center,
        };
        match reach.dir {
            PipeDir::Horizontal => {
                if outward(bounds.x + bounds.width / 2.0) {
                    bounds.x = reach.to - bounds.width / 2.0;
                }
            }
            PipeDir::Vertical => {
                if outward(bounds.y + bounds.height / 2.0) {
                    bounds.y = reach.to - bounds.height / 2.0;
                }
            }
        }
    }
}
