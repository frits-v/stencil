//! Detail frames around a ladder's Steps and their marks-clear defects (section 14.4 rule 16).

use std::collections::BTreeMap;

use stencil_model::checks::Defect;
use stencil_model::text::{TextMeasurer, TextStyleName, WRAP_EPSILON_PX};
use stencil_model::{Node, NodePointer, NodeRef, Page, PagePoint, SheetList, body_nodes};

use crate::build::detail_letter;
use crate::drawn::drawn_link;
use crate::route::segment_enters;
use crate::sheet_lines::BEND_AIR_PX;
use crate::styles::{text_color, text_style_for};
use crate::{
    ARROWHEAD_LENGTH_PX, BoxRect, GEOMETRY_EPSILON_PX, LayoutError, NodeTag, PageGeometry, Part,
    PartName, TextAlign, TextRun,
};

/// A line into a framed Step crosses the frame on its shaft, past its arrowhead.
pub const DETAIL_FRAME_GROW_PX: f32 = ARROWHEAD_LENGTH_PX + BEND_AIR_PX;
pub const DETAIL_FRAME_RADIUS_PX: f32 = 8.0;
pub const DETAIL_TAG_RADIUS_PX: f32 = 11.0;
/// The air a frame keeps from a parallel rule, border or leg, so both lines read.
pub const DETAIL_FRAME_CLEARANCE_PX: f32 = 6.0;

struct Framed {
    letter: String,
    phases: usize,
    steps: Vec<NodePointer>,
}

fn framed_steps(page: &Page, geometry: &PageGeometry) -> Vec<Framed> {
    let entries = body_nodes(page);
    let step_pointers: BTreeMap<&str, &NodePointer> = entries
        .iter()
        .filter_map(|entry| match entry.node {
            NodeRef::Node(Node::Step(step)) => step.id.as_deref().map(|id| (id, &entry.pointer)),
            _ => None,
        })
        .collect();
    let index_of: BTreeMap<&str, usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.pointer.as_str(), index))
        .collect();
    let mut framed = Vec::new();
    let ladders = entries
        .iter()
        .filter_map(|entry| match entry.node {
            NodeRef::Node(Node::Ladder(ladder)) => Some(ladder),
            _ => None,
        })
        .take(stencil_model::LADDERS_MAX);
    for (index, ladder) in ladders.enumerate() {
        let mut phases: Option<usize> = None;
        let mut steps = Vec::new();
        for id in ladder.around.iter().take(SheetList::Around.max() + 1) {
            let Some(pointer) = step_pointers.get(id.as_str()) else {
                continue;
            };
            let Some(parent) = index_of
                .get(pointer.as_str())
                .and_then(|node| geometry.nodes.get(*node))
                .and_then(|node| node.parent)
            else {
                continue;
            };
            if *phases.get_or_insert(parent) == parent {
                steps.push((*pointer).clone());
            }
        }
        if let Some(phases) = phases
            && geometry
                .nodes
                .get(phases)
                .is_some_and(|node| node.tag == NodeTag::Phases)
        {
            framed.push(Framed {
                letter: detail_letter(index),
                phases,
                steps,
            });
        }
    }
    framed
}

fn union(boxes: impl IntoIterator<Item = BoxRect>) -> Option<BoxRect> {
    boxes.into_iter().reduce(|first, second| {
        let x = first.x.min(second.x);
        let y = first.y.min(second.y);
        BoxRect {
            x,
            y,
            width: first.right().max(second.right()) - x,
            height: first.bottom().max(second.bottom()) - y,
        }
    })
}

fn grown(bounds: BoxRect, by: f32) -> BoxRect {
    BoxRect {
        x: bounds.x - by,
        y: bounds.y - by,
        width: bounds.width + 2.0 * by,
        height: bounds.height + 2.0 * by,
    }
}

pub(crate) fn add_detail_frames(
    page: &Page,
    geometry: &mut PageGeometry,
    measurer: &mut dyn TextMeasurer,
) -> Result<(), LayoutError> {
    let style = text_style_for(TextStyleName::ViewNumeral, false);
    for framed in framed_steps(page, geometry) {
        let steps = framed
            .steps
            .iter()
            .filter_map(|pointer| geometry.node(pointer).map(|node| node.bounds));
        let Some(steps_box) = union(steps) else {
            continue;
        };
        let Some(phases) = geometry.nodes.get_mut(framed.phases) else {
            continue;
        };
        let frame = grown(steps_box, DETAIL_FRAME_GROW_PX);
        let radius = DETAIL_TAG_RADIUS_PX;
        let center_x = frame
            .x
            .clamp(phases.bounds.x + radius, phases.bounds.right() - radius);
        let center_y = frame
            .bottom()
            .clamp(phases.bounds.y + radius, phases.bounds.bottom() - radius);
        let tag = BoxRect {
            x: center_x - radius,
            y: center_y - radius,
            width: 2.0 * radius,
            height: 2.0 * radius,
        };
        let metrics = measurer
            .measure(&framed.letter, &style, Some(tag.width + WRAP_EPSILON_PX))
            .map_err(|source| LayoutError::Measure {
                pointer: phases.pointer.clone(),
                source,
            })?;
        let letter = BoxRect {
            x: tag.x,
            y: center_y - metrics.height_px / 2.0,
            width: tag.width,
            height: metrics.height_px,
        };
        phases.parts.push(Part {
            name: PartName::DetailFrame,
            bounds: frame,
            text: None,
        });
        phases.parts.push(Part {
            name: PartName::DetailTag,
            bounds: tag,
            text: None,
        });
        phases.parts.push(Part {
            name: PartName::DetailLetter,
            bounds: letter,
            text: Some(TextRun {
                text: framed.letter,
                style,
                color: text_color(TextStyleName::ViewNumeral, None),
                align: TextAlign::Center,
                metrics,
                first_line_indent_px: 0.0,
            }),
        });
    }
    Ok(())
}

/// An axis-aligned segment: its fixed coordinate and its extent along the other axis.
#[derive(Debug, Clone, Copy)]
struct Rule {
    at: f32,
    from: f32,
    to: f32,
    horizontal: bool,
}

fn edges(bounds: &BoxRect) -> [Rule; 4] {
    [
        Rule {
            at: bounds.y,
            from: bounds.x,
            to: bounds.right(),
            horizontal: true,
        },
        Rule {
            at: bounds.bottom(),
            from: bounds.x,
            to: bounds.right(),
            horizontal: true,
        },
        Rule {
            at: bounds.x,
            from: bounds.y,
            to: bounds.bottom(),
            horizontal: false,
        },
        Rule {
            at: bounds.right(),
            from: bounds.y,
            to: bounds.bottom(),
            horizontal: false,
        },
    ]
}

fn part_rule(bounds: &BoxRect) -> Rule {
    if bounds.width >= bounds.height {
        Rule {
            at: bounds.y + bounds.height / 2.0,
            from: bounds.x,
            to: bounds.right(),
            horizontal: true,
        }
    } else {
        Rule {
            at: bounds.x + bounds.width / 2.0,
            from: bounds.y,
            to: bounds.bottom(),
            horizontal: false,
        }
    }
}

fn crowds(edge: &Rule, rule: &Rule) -> bool {
    edge.horizontal == rule.horizontal
        && (edge.at - rule.at).abs() < DETAIL_FRAME_CLEARANCE_PX - GEOMETRY_EPSILON_PX
        && edge.from.max(rule.from) < edge.to.min(rule.to) - GEOMETRY_EPSILON_PX
}

fn overlap(first: &BoxRect, second: &BoxRect) -> bool {
    let width = first.right().min(second.right()) - first.x.max(second.x);
    let height = first.bottom().min(second.bottom()) - first.y.max(second.y);
    width > GEOMETRY_EPSILON_PX && height > GEOMETRY_EPSILON_PX
}

fn contains(outer: &BoxRect, inner: &BoxRect) -> bool {
    inner.x >= outer.x - GEOMETRY_EPSILON_PX
        && inner.y >= outer.y - GEOMETRY_EPSILON_PX
        && inner.right() <= outer.right() + GEOMETRY_EPSILON_PX
        && inner.bottom() <= outer.bottom() + GEOMETRY_EPSILON_PX
}

/// The box meets the frame and lies neither inside its inner edge nor outside it.
fn outline_crosses(frame: &BoxRect, bounds: &BoxRect) -> bool {
    let air = BEND_AIR_PX / 2.0;
    overlap(&grown(*frame, air), bounds) && !contains(&grown(*frame, -air), bounds)
}

fn outline_distance(frame: &BoxRect, point: PagePoint) -> f32 {
    let outside_x = (frame.x - point.x).max(point.x - frame.right()).max(0.0);
    let outside_y = (frame.y - point.y).max(point.y - frame.bottom()).max(0.0);
    if outside_x > 0.0 || outside_y > 0.0 {
        return outside_x.hypot(outside_y);
    }
    (point.x - frame.x)
        .min(frame.right() - point.x)
        .min(point.y - frame.y)
        .min(frame.bottom() - point.y)
}

fn describe(bounds: &BoxRect) -> String {
    format!(
        "{:.2}x{:.2} at ({:.2}, {:.2})",
        bounds.width, bounds.height, bounds.x, bounds.y
    )
}

/// The defects of every detail frame and tag, and how many frames and tags it examined.
pub(crate) fn detail_frame_defects(page: &Page, geometry: &PageGeometry) -> (u64, Vec<Defect>) {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    let links_pointer = stencil_model::NodePointer::root().child("links");
    for framed in framed_steps(page, geometry) {
        let Some(phases) = geometry.nodes.get(framed.phases) else {
            continue;
        };
        let letters: Vec<&Part> = phases
            .parts
            .iter()
            .filter(|part| part.name == PartName::DetailLetter)
            .collect();
        let Some(position) = letters.iter().position(|part| {
            part.text
                .as_ref()
                .is_some_and(|run| run.text == framed.letter)
        }) else {
            defects.push(Defect {
                pointer: phases.pointer.clone(),
                message: format!("detail {} draws no frame", framed.letter),
            });
            continue;
        };
        let nth = |name: PartName| {
            phases
                .parts
                .iter()
                .filter(|part| part.name == name)
                .nth(position)
        };
        let (Some(frame_part), Some(tag_part)) =
            (nth(PartName::DetailFrame), nth(PartName::DetailTag))
        else {
            continue;
        };
        let frame = frame_part.bounds;
        let tag = tag_part.bounds;
        let label = format!("detail frame {}", framed.letter);
        let mut push = |message: String| {
            defects.push(Defect {
                pointer: phases.pointer.clone(),
                message,
            });
        };
        examined += 2;
        if !contains(&phases.bounds, &frame) {
            push(format!(
                "{label} {} extends outside the Phases node {}",
                describe(&frame),
                describe(&phases.bounds)
            ));
        }
        let mut rules: Vec<(String, Rule)> = Vec::new();
        for part in &phases.parts {
            match part.name {
                PartName::LaneRule | PartName::PhaseSeparator => {
                    rules.push((part.name.as_str().to_string(), part_rule(&part.bounds)));
                }
                PartName::LaneBand => {
                    let [top, bottom, _, _] = edges(&part.bounds);
                    rules.push(("lane_band".to_string(), top));
                    rules.push(("lane_band".to_string(), bottom));
                }
                _ => {}
            }
        }
        for route in &geometry.links {
            for leg in route.points.windows(2) {
                if let [a, b] = leg {
                    let rule = if (a.y - b.y).abs() <= GEOMETRY_EPSILON_PX {
                        Rule {
                            at: a.y,
                            from: a.x.min(b.x),
                            to: a.x.max(b.x),
                            horizontal: true,
                        }
                    } else {
                        Rule {
                            at: a.x,
                            from: a.y.min(b.y),
                            to: a.y.max(b.y),
                            horizontal: false,
                        }
                    };
                    rules.push((links_pointer.index(route.index).to_string(), rule));
                }
            }
        }
        for node in &geometry.nodes {
            if node.tag != NodeTag::Step || node.parent != Some(framed.phases) {
                continue;
            }
            let framed_step = framed.steps.contains(&node.pointer);
            if !framed_step && overlap(&frame, &node.bounds) {
                push(format!(
                    "{label} {} crosses {}, a Step it does not frame",
                    describe(&frame),
                    node.pointer
                ));
            }
            if overlap(&tag, &node.bounds) {
                push(format!(
                    "detail tag {} {} lies over {}",
                    framed.letter,
                    describe(&tag),
                    node.pointer
                ));
            }
            for edge in edges(&node.bounds) {
                rules.push((node.pointer.to_string(), edge));
            }
        }
        for edge in edges(&frame) {
            for (what, rule) in &rules {
                if crowds(&edge, rule) {
                    push(format!(
                        "{label} edge at {:.2} runs {:.2} px from {what}",
                        edge.at,
                        (edge.at - rule.at).abs()
                    ));
                }
            }
        }
        for part in &phases.parts {
            let label_part = matches!(
                part.name,
                PartName::LaneName
                    | PartName::LaneText
                    | PartName::PhaseHead
                    | PartName::PhaseTitle
            );
            if label_part && overlap(&tag, &part.bounds) {
                push(format!(
                    "detail tag {} {} lies over {} {}",
                    framed.letter,
                    describe(&tag),
                    part.name.as_str(),
                    describe(&part.bounds)
                ));
            }
        }
        for node in &geometry.nodes {
            for part in &node.parts {
                let covered = part.text.is_some() || part.name.is_mark_glyph();
                if covered
                    && part.name != PartName::DetailLetter
                    && outline_crosses(&frame, &part.bounds)
                {
                    push(format!(
                        "{label} {} passes through {} {} of {}",
                        describe(&frame),
                        part.name.as_str(),
                        describe(&part.bounds),
                        node.pointer
                    ));
                }
            }
        }
        for route in &geometry.links {
            let link = links_pointer.index(route.index);
            let mut boxes: Vec<(&str, BoxRect)> = Vec::new();
            if let Some(tag_box) = route.tag {
                boxes.push(("tag", tag_box));
            }
            boxes.extend(
                route
                    .parts
                    .iter()
                    .filter(|part| part.name != PartName::Tag)
                    .map(|part| (part.name.as_str(), part.bounds)),
            );
            let drawn = drawn_link(geometry, route);
            for arrow in [drawn.start_arrow, drawn.end_arrow].into_iter().flatten() {
                boxes.push((drawn.cap.name(), drawn.cap.bounds(arrow)));
            }
            for (what, bounds) in boxes {
                if outline_crosses(&frame, &bounds) {
                    push(format!(
                        "{label} {} passes through the {what} {} of {link}",
                        describe(&frame),
                        describe(&bounds)
                    ));
                }
            }
            for corner in crate::sheet_lines::bends(&route.points) {
                if outline_distance(&frame, *corner) < route.corner + BEND_AIR_PX / 2.0 {
                    push(format!(
                        "{label} {} passes {:.2} px from a bend of {link} at ({:.2}, {:.2})",
                        describe(&frame),
                        outline_distance(&frame, *corner),
                        corner.x,
                        corner.y
                    ));
                }
            }
            let crosses_tag = route
                .points
                .windows(2)
                .any(|leg| matches!(leg, [a, b] if segment_enters(*a, *b, &tag)));
            if crosses_tag {
                push(format!(
                    "detail tag {} {} lies over {link}",
                    framed.letter,
                    describe(&tag)
                ));
            }
        }
    }
    (examined, defects)
}
