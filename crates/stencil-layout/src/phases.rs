//! A Phases node (section 14.4 rules 11 to 14): a lane per actor top to bottom, phases left
//! to right, and Steps placed by lane, phase and column. `plan_phases` measures every Step
//! before the tree is built and fixes the column widths, lane heights and Step boxes;
//! `Builder::add_phases` places every part and Step at its planned offset in the node.

use std::collections::BTreeMap;

use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, TextStyleName, WRAP_EPSILON_PX};
use stencil_model::{
    Node, NodeRef, PHASE_COLUMNS_MAX, Page, Phases, SheetList, Step, StepLook, body_nodes,
};

use crate::LayoutError;
use crate::marks::{MARK_GAP_PX, MARKED_LINE_PX, glyphs_height, glyphs_width};
use crate::styles::text_style_for;
use stencil_model::{MarkKind, PageMarks};

/// Height of the row of phase heads above the first lane.
pub(crate) const HEAD_ROW_PX: f32 = 30.0;
/// Between two columns of one phase: room for a link between two Steps to leave one and
/// reach the other past its arrowhead's straight approach.
pub(crate) const COLUMN_GAP_PX: f32 = 48.0;
/// Inside a phase, before its first column and after its last, so two Steps on either side
/// of a phase boundary are one column gap apart.
pub(crate) const PHASE_PADDING_PX: f32 = COLUMN_GAP_PX / 2.0;
/// A column no single-column Step fills.
const EMPTY_COLUMN_PX: f32 = 12.0;
/// A Step's padding: top, left, right, bottom.
pub(crate) const STEP_PADDING_PX: (f32, f32, f32, f32) = (8.0, 9.0, 10.0, 8.0);
/// A stack look reserves this much inside its bounds for the two boxes behind its front.
pub(crate) const STACK_OFFSET_PX: (f32, f32) = (8.0, 6.0);
/// The widest a single column grows for its own Steps; wider text wraps.
const COLUMN_MAX_PX: f32 = 200.0;
/// Between a lane's top and its Steps, and between its Steps and its bottom: a detail frame
/// around a Step and its clearance from the lane's edge fit in it.
const LANE_TOP_PX: f32 = 22.0;
const LANE_BOTTOM_PX: f32 = 22.0;
/// Between two Steps of one lane whose columns overlap; after a text-look Step, 4.
const STACKING_GAP_PX: f32 = 20.0;
const STACKING_GAP_AFTER_TEXT_PX: f32 = 4.0;
/// The lane label column: 16 before the widest lane name and 14 after it, and never
/// narrower than a caption of a few words needs.
pub(crate) const LANE_LABEL_LEFT_PX: f32 = 16.0;
const LANE_LABEL_RIGHT_PX: f32 = 14.0;
const LANE_LABEL_MIN_PX: f32 = 120.0;
/// The lane name's top below the lane's top, and the caption's gap under the name.
pub(crate) const LANE_NAME_TOP_PX: f32 = 14.0;
pub(crate) const LANE_TEXT_GAP_PX: f32 = 3.0;
/// The bar at the left edge of a lane, in the actor's wire.
pub(crate) const LANE_BAR_PX: f32 = 4.0;
/// A phase head: a circle of radius 10 centred 18 after the phase's start and 15 below the
/// node's top, then the title 34 after the start.
pub(crate) const PHASE_HEAD_RADIUS_PX: f32 = 10.0;
pub(crate) const PHASE_HEAD_CENTER_PX: (f32, f32) = (18.0, 15.0);
pub(crate) const PHASE_TITLE_LEFT_PX: f32 = 34.0;
/// A phase separator starts this far below the node's top.
pub(crate) const PHASE_SEPARATOR_TOP_PX: f32 = 4.0;
/// Between a Step's title and its first line.
pub(crate) const STEP_TITLE_GAP_PX: f32 = 2.0;

/// An axis-aligned box relative to the Phases node's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Offset {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LanePlan {
    pub y: f32,
    pub height: f32,
    /// The width the lane name and caption wrap at.
    pub label_width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhasePlan {
    pub x: f32,
    pub width: f32,
    /// Each column's start and width, relative to the node.
    pub columns: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StepPlan {
    /// The Step's bounds.
    pub bounds: Offset,
    /// Where its title and lines start and the width they wrap at, relative to the node.
    pub text_x: f32,
    pub text_y: f32,
    pub text_width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhasesLayout {
    pub width: f32,
    pub height: f32,
    pub label_width: f32,
    pub lanes: Vec<LanePlan>,
    pub phases: Vec<PhasePlan>,
    /// One per child, in child order.
    pub steps: Vec<StepPlan>,
}

/// The plan of every Phases node on a page, by the node's pointer.
#[derive(Debug, Default)]
pub(crate) struct PhasesPlan {
    nodes: BTreeMap<String, PhasesLayout>,
}

impl PhasesPlan {
    pub(crate) fn node(&self, pointer: &NodePointer) -> Option<&PhasesLayout> {
        self.nodes.get(pointer.as_str())
    }
}

/// A Step's measured text: its title and lines at max-content and at min-content.
struct StepText {
    max_content: f32,
    min_content: f32,
}

/// The style a Step's title is set in: `step_text` for a text look, else `step_title`.
pub(crate) fn step_title_style(look: StepLook) -> TextStyleName {
    match look {
        StepLook::Text => TextStyleName::StepText,
        StepLook::Box | StepLook::Chip | StepLook::Note | StepLook::Stack | StepLook::Planned => {
            TextStyleName::StepTitle
        }
    }
}

/// The text a run draws: upper-cased when its style is.
pub(crate) fn drawn_text(text: &str, style: TextStyleName) -> String {
    if style.text_style().uppercase {
        text.to_uppercase()
    } else {
        text.to_string()
    }
}

fn measure_width(
    measurer: &mut dyn TextMeasurer,
    text: &str,
    style: TextStyleName,
    max_width_px: Option<f32>,
) -> Result<f32, MeasureError> {
    let drawn = drawn_text(text, style);
    Ok(measurer
        .measure(&drawn, &text_style_for(style, false), max_width_px)?
        .width_px)
}

fn measure_height(
    measurer: &mut dyn TextMeasurer,
    text: &str,
    style: TextStyleName,
    width_px: f32,
) -> Result<f32, MeasureError> {
    let drawn = drawn_text(text, style);
    Ok(measurer
        .measure(
            &drawn,
            &text_style_for(style, false),
            Some(width_px.max(0.0) + WRAP_EPSILON_PX),
        )?
        .height_px)
}

/// The width a Step's box adds around its text: the padding, and the stack offset.
fn step_frame_width(look: StepLook) -> f32 {
    let (_, left, right, _) = STEP_PADDING_PX;
    let stack = if look == StepLook::Stack {
        STACK_OFFSET_PX.0
    } else {
        0.0
    };
    left + right + stack
}

fn step_frame_height(look: StepLook) -> f32 {
    let (top, _, _, bottom) = STEP_PADDING_PX;
    let stack = if look == StepLook::Stack {
        STACK_OFFSET_PX.1
    } else {
        0.0
    };
    top + bottom + stack
}

/// The glyphs of a Step's title line and of each of its lines, numbered.
pub(crate) struct StepGlyphs {
    pub title: Vec<(MarkKind, usize)>,
    pub lines: Vec<Vec<(MarkKind, usize)>>,
}

pub(crate) fn step_glyphs(page_marks: &PageMarks<'_>, step: &Step) -> StepGlyphs {
    let number = |marks: &[stencil_model::Mark]| -> Vec<(MarkKind, usize)> {
        marks
            .iter()
            .take(SheetList::Marks.max() + 1)
            .map(|mark| (mark.kind(), page_marks.number(mark).unwrap_or(0)))
            .collect()
    };
    StepGlyphs {
        title: number(&step.marks),
        lines: step
            .lines
            .iter()
            .take(SheetList::StepLines.max() + 1)
            .map(|line| number(line.marks()))
            .collect(),
    }
}

/// The width a run of glyphs takes after its text: the gap and the glyphs, or nothing.
pub(crate) fn trailing_width(glyphs: &[(MarkKind, usize)]) -> f32 {
    if glyphs.is_empty() {
        0.0
    } else {
        MARK_GAP_PX + glyphs_width(glyphs)
    }
}

fn measure_step_text(
    measurer: &mut dyn TextMeasurer,
    step: &Step,
    glyphs: &StepGlyphs,
) -> Result<StepText, MeasureError> {
    let title_style = step_title_style(step.look);
    let title_extra = trailing_width(&glyphs.title);
    let mut max_content = measure_width(measurer, &step.title, title_style, None)? + title_extra;
    let mut min_content =
        measure_width(measurer, &step.title, title_style, Some(0.0))? + title_extra;
    for (index, line) in step
        .lines
        .iter()
        .take(SheetList::StepLines.max() + 1)
        .enumerate()
    {
        let extra = glyphs
            .lines
            .get(index)
            .map_or(0.0, |line| trailing_width(line));
        max_content = max_content
            .max(measure_width(measurer, line.text(), TextStyleName::StepText, None)? + extra);
        min_content = min_content
            .max(measure_width(measurer, line.text(), TextStyleName::StepText, Some(0.0))? + extra);
    }
    Ok(StepText {
        max_content,
        min_content,
    })
}

/// The height of a Step whose text wraps at `text_width`.
fn step_height(
    measurer: &mut dyn TextMeasurer,
    step: &Step,
    glyphs: &StepGlyphs,
    text_width: f32,
) -> Result<f32, MeasureError> {
    let mut height = measure_height(
        measurer,
        &step.title,
        step_title_style(step.look),
        text_width - trailing_width(&glyphs.title),
    )?
    .max(glyphs_height(&glyphs.title));
    let lines: Vec<&stencil_model::MarkedText> = step
        .lines
        .iter()
        .take(SheetList::StepLines.max() + 1)
        .collect();
    if !lines.is_empty() {
        height += STEP_TITLE_GAP_PX;
    }
    for (index, line) in lines.into_iter().enumerate() {
        let line_glyphs = glyphs.lines.get(index).map_or(&[][..], Vec::as_slice);
        let line_height = measure_height(
            measurer,
            line.text(),
            TextStyleName::StepText,
            text_width - trailing_width(line_glyphs),
        )?;
        height += if line_glyphs.is_empty() {
            line_height
        } else {
            line_height.max(MARKED_LINE_PX)
        };
    }
    Ok(height + step_frame_height(step.look))
}

/// The Steps of a Phases node with their child index, in child order. Vet keeps every child
/// a Step.
fn steps_of(phases: &Phases) -> Vec<(usize, &Step)> {
    phases
        .children
        .iter()
        .enumerate()
        .take(SheetList::Steps.max() + 1)
        .filter_map(|(index, child)| match child {
            Node::Step(step) => Some((index, step)),
            _ => None,
        })
        .collect()
}

/// The first and last column of a Step, from 0, kept inside the phase's columns.
fn step_columns(step: &Step) -> (usize, usize) {
    let first = usize::from(step.col.clamp(1, PHASE_COLUMNS_MAX)) - 1;
    let last = usize::from(
        (u16::from(step.col) + u16::from(step.span.max(1)) - 1)
            .clamp(1, u16::from(PHASE_COLUMNS_MAX)) as u8,
    ) - 1;
    (first, last.max(first))
}

/// Measures every Phases node of the page at the content width `width`.
pub(crate) fn plan_phases(
    page: &Page,
    width: f32,
    measurer: &mut dyn TextMeasurer,
) -> Result<PhasesPlan, LayoutError> {
    let mut plan = PhasesPlan::default();
    for entry in body_nodes(page) {
        let NodeRef::Node(Node::Phases(phases)) = entry.node else {
            continue;
        };
        let layout =
            plan_one(page, phases, width, measurer).map_err(|source| LayoutError::Measure {
                pointer: entry.pointer.clone(),
                source,
            })?;
        plan.nodes
            .insert(entry.pointer.as_str().to_string(), layout);
    }
    Ok(plan)
}

fn plan_one(
    page: &Page,
    phases: &Phases,
    width: f32,
    measurer: &mut dyn TextMeasurer,
) -> Result<PhasesLayout, MeasureError> {
    let lane_ids: Vec<&str> = phases
        .lanes
        .iter()
        .take(SheetList::Lanes.max() + 1)
        .map(String::as_str)
        .collect();
    let actor = |id: &str| page.actors.iter().find(|actor| actor.id == id);

    let mut widest_name: f32 = LANE_LABEL_MIN_PX;
    for id in &lane_ids {
        if let Some(actor) = actor(id) {
            widest_name = widest_name.max(measure_width(
                measurer,
                &actor.name,
                TextStyleName::LaneName,
                None,
            )?);
        }
    }
    let label_width = LANE_LABEL_LEFT_PX + widest_name + LANE_LABEL_RIGHT_PX;

    let steps = steps_of(phases);
    let phase_index = |step: &Step| {
        phases
            .phases
            .iter()
            .position(|phase| phase.id == step.phase)
            .unwrap_or(0)
    };
    let lane_index = |step: &Step| {
        lane_ids
            .iter()
            .position(|lane| *lane == step.lane)
            .unwrap_or(0)
    };
    let phase_count = phases.phases.len().min(SheetList::Phases.max() + 1);

    // Columns per phase, and the width each column needs for its single-column Steps.
    let mut columns: Vec<Vec<Option<f32>>> = vec![vec![None; 1]; phase_count];
    let mut minimums: Vec<Vec<f32>> = vec![vec![EMPTY_COLUMN_PX; 1]; phase_count];
    let page_marks = PageMarks::of(page);
    let glyphs: Vec<StepGlyphs> = steps
        .iter()
        .map(|(_, step)| step_glyphs(&page_marks, step))
        .collect();
    let mut texts = Vec::with_capacity(steps.len());
    for ((_, step), step_glyphs) in steps.iter().zip(&glyphs) {
        let text = measure_step_text(measurer, step, step_glyphs)?;
        let frame = step_frame_width(step.look);
        let (first, last) = step_columns(step);
        let phase = phase_index(step);
        if let (Some(phase_columns), Some(phase_minimums)) =
            (columns.get_mut(phase), minimums.get_mut(phase))
        {
            if phase_columns.len() <= last {
                phase_columns.resize(last + 1, None);
                phase_minimums.resize(last + 1, EMPTY_COLUMN_PX);
            }
            if first == last {
                let minimum = text.min_content + frame;
                let needed = (text.max_content + frame)
                    .clamp(minimum.min(COLUMN_MAX_PX), COLUMN_MAX_PX.max(minimum));
                if let Some(column) = phase_columns.get_mut(first) {
                    *column = Some(column.unwrap_or(0.0).max(needed));
                }
                if let Some(column) = phase_minimums.get_mut(first) {
                    *column = column.max(minimum);
                }
            }
        }
        texts.push(text);
    }
    let filled: Vec<Vec<bool>> = columns
        .iter()
        .map(|phase_columns| phase_columns.iter().map(Option::is_some).collect())
        .collect();
    let mut widths: Vec<Vec<f32>> = columns
        .iter()
        .map(|phase_columns| {
            phase_columns
                .iter()
                .map(|column| column.unwrap_or(EMPTY_COLUMN_PX))
                .collect()
        })
        .collect();
    let mut covered_by_span = filled.clone();

    // Spanning Steps widen the columns they cover by equal shares, smallest span first.
    let mut spanning: Vec<(usize, usize)> = steps
        .iter()
        .enumerate()
        .filter(|(_, (_, step))| step_columns(step).0 != step_columns(step).1)
        .map(|(position, (_, step))| {
            let (first, last) = step_columns(step);
            (last - first, position)
        })
        .collect();
    spanning.sort_unstable();
    for (_, position) in spanning {
        let (Some((_, step)), Some(text)) = (steps.get(position), texts.get(position)) else {
            continue;
        };
        let (first, last) = step_columns(step);
        let phase = phase_index(step);
        let Some(phase_widths) = widths.get_mut(phase) else {
            continue;
        };
        let frame = step_frame_width(step.look);
        let needed = (text.max_content + frame).min(COLUMN_MAX_PX.max(text.min_content + frame));
        let span_count = last - first + 1;
        let gaps = COLUMN_GAP_PX * (span_count - 1) as f32;
        let current: f32 = phase_widths
            .iter()
            .skip(first)
            .take(span_count)
            .sum::<f32>()
            + gaps;
        if current < needed {
            let share = (needed - current) / span_count as f32;
            for column in phase_widths.iter_mut().skip(first).take(span_count) {
                *column += share;
            }
        }
        if let Some(phase_covered) = covered_by_span.get_mut(phase) {
            for column in phase_covered.iter_mut().skip(first).take(span_count) {
                *column = true;
            }
        }
    }

    // A phase is at least as wide as its head: the circle, the gap and the title.
    let mut head_widths = Vec::with_capacity(phase_count);
    for phase in phases.phases.iter().take(phase_count) {
        let title = measure_width(measurer, &phase.title, TextStyleName::SheetLabel, None)?;
        head_widths.push(PHASE_TITLE_LEFT_PX + title + PHASE_PADDING_PX);
    }
    let phase_width = |phase_widths: &[f32]| -> f32 {
        let gaps = COLUMN_GAP_PX * phase_widths.len().saturating_sub(1) as f32;
        2.0 * PHASE_PADDING_PX + phase_widths.iter().sum::<f32>() + gaps
    };
    for (phase, phase_widths) in widths.iter_mut().enumerate() {
        let head = head_widths.get(phase).copied().unwrap_or(0.0);
        let current = phase_width(phase_widths);
        if current < head {
            let filled = covered_by_span.get(phase).map_or(&[][..], Vec::as_slice);
            spread(phase_widths, filled, head - current);
        }
    }

    // Slack up to the node's width is spread evenly over every filled column; a plan wider
    // than the node gives back what its filled columns hold above their min-content, each
    // in proportion to its share, so the widest text wraps first.
    let used: f32 = label_width + widths.iter().map(|phase| phase_width(phase)).sum::<f32>();
    let slack = width - used;
    if slack < 0.0 {
        let give: f32 = widths
            .iter()
            .zip(&minimums)
            .flat_map(|(phase, phase_minimums)| {
                phase
                    .iter()
                    .zip(phase_minimums)
                    .map(|(column, minimum)| (column - minimum).max(0.0))
            })
            .sum();
        if give > 0.0 {
            let fraction = (-slack / give).min(1.0);
            for (phase, phase_minimums) in widths.iter_mut().zip(&minimums) {
                for (column, minimum) in phase.iter_mut().zip(phase_minimums) {
                    *column -= (*column - minimum).max(0.0) * fraction;
                }
            }
        }
    } else if slack > 0.0 {
        let filled_count: usize = covered_by_span
            .iter()
            .map(|phase| phase.iter().filter(|filled| **filled).count())
            .sum();
        if filled_count > 0 {
            let share = slack / filled_count as f32;
            for (phase_widths, phase_covered) in widths.iter_mut().zip(&covered_by_span) {
                for (column, filled) in phase_widths.iter_mut().zip(phase_covered) {
                    if *filled {
                        *column += share;
                    }
                }
            }
        }
    }

    let mut phase_plans = Vec::with_capacity(phase_count);
    let mut x = label_width;
    for phase_widths in &widths {
        let mut column_x = x + PHASE_PADDING_PX;
        let mut phase_columns = Vec::with_capacity(phase_widths.len());
        for column in phase_widths {
            phase_columns.push((column_x, *column));
            column_x += column + COLUMN_GAP_PX;
        }
        let phase_width = phase_width(phase_widths);
        phase_plans.push(PhasePlan {
            x,
            width: phase_width,
            columns: phase_columns,
        });
        x += phase_width;
    }
    let node_width = x.max(width);

    // Step widths are known: measure each Step's natural height.
    let mut step_boxes: Vec<(f32, f32, f32)> = Vec::with_capacity(steps.len());
    for ((_, step), step_glyphs) in steps.iter().zip(&glyphs) {
        let (first, last) = step_columns(step);
        let phase = phase_plans.get(phase_index(step));
        let start = phase
            .and_then(|phase| phase.columns.get(first))
            .map_or(label_width, |(column_x, _)| *column_x);
        let end = phase
            .and_then(|phase| phase.columns.get(last))
            .map_or(start + EMPTY_COLUMN_PX, |(column_x, column)| {
                column_x + column
            });
        let step_width = end - start;
        let text_width = step_width - step_frame_width(step.look);
        let height = step_height(measurer, step, step_glyphs, text_width)?;
        step_boxes.push((start, step_width, height));
    }

    // Lanes: Steps stack where their columns overlap, in document order.
    let mut lanes = Vec::with_capacity(lane_ids.len());
    let mut step_y: Vec<f32> = vec![0.0; steps.len()];
    let mut y = HEAD_ROW_PX;
    let caption_width = label_width - LANE_LABEL_LEFT_PX - LANE_LABEL_RIGHT_PX;
    for (lane, id) in lane_ids.iter().enumerate() {
        let mut placed: Vec<(usize, f32, f32, f32)> = Vec::new();
        let mut content_bottom: f32 = 0.0;
        let mut stacked = false;
        for (position, (_, step)) in steps.iter().enumerate() {
            if lane_index(step) != lane {
                continue;
            }
            let Some(&(start, step_width, height)) = step_boxes.get(position) else {
                continue;
            };
            let mut top = 0.0_f32;
            for &(other, other_start, other_width, other_bottom) in &placed {
                let overlaps =
                    start < other_start + other_width && other_start < start + step_width;
                if overlaps {
                    stacked = true;
                    let after_text = steps
                        .get(other)
                        .is_some_and(|(_, other_step)| other_step.look == StepLook::Text);
                    let gap = if after_text {
                        STACKING_GAP_AFTER_TEXT_PX
                    } else {
                        STACKING_GAP_PX
                    };
                    top = top.max(other_bottom + gap);
                }
            }
            if let Some(slot) = step_y.get_mut(position) {
                *slot = top;
            }
            placed.push((position, start, step_width, top + height));
            content_bottom = content_bottom.max(top + height);
        }
        let mut label_bottom = LANE_NAME_TOP_PX;
        if let Some(actor) = actor(id) {
            label_bottom += measure_height(
                measurer,
                &actor.name,
                TextStyleName::LaneName,
                caption_width,
            )?;
            if let Some(text) = &actor.text {
                label_bottom += LANE_TEXT_GAP_PX
                    + measure_height(measurer, text, TextStyleName::StepText, caption_width)?;
            }
        }
        let height =
            (LANE_TOP_PX + content_bottom + LANE_BOTTOM_PX).max(label_bottom + LANE_BOTTOM_PX);
        // Without stacking every boxed Step of a lane takes the lane's full height, and a
        // chip or a text look sits at the middle, so the lane reads as one row.
        let row_height = height - LANE_TOP_PX - LANE_BOTTOM_PX;
        for &(position, _, _, bottom) in &placed {
            let Some((_, step)) = steps.get(position) else {
                continue;
            };
            let natural = bottom - step_y.get(position).copied().unwrap_or(0.0);
            if stacked {
                continue;
            }
            match step.look {
                StepLook::Box | StepLook::Note | StepLook::Stack | StepLook::Planned => {
                    if let Some(entry) = step_boxes.get_mut(position) {
                        entry.2 = row_height;
                    }
                }
                StepLook::Chip | StepLook::Text => {
                    if let Some(slot) = step_y.get_mut(position) {
                        *slot = (row_height - natural) / 2.0;
                    }
                }
            }
        }
        for &(position, _, _, _) in &placed {
            if let Some(slot) = step_y.get_mut(position) {
                *slot += y + LANE_TOP_PX;
            }
        }
        lanes.push(LanePlan {
            y,
            height,
            label_width: caption_width,
        });
        y += height;
    }

    let (padding_top, padding_left, _, _) = STEP_PADDING_PX;
    let mut step_plans = vec![
        StepPlan {
            bounds: Offset {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            text_x: 0.0,
            text_y: 0.0,
            text_width: 0.0,
        };
        phases.children.len().min(SheetList::Steps.max() + 1)
    ];
    for (position, (child, step)) in steps.iter().enumerate() {
        let (Some(&(start, step_width, height)), Some(&top)) =
            (step_boxes.get(position), step_y.get(position))
        else {
            continue;
        };
        let stack_y = if step.look == StepLook::Stack {
            STACK_OFFSET_PX.1
        } else {
            0.0
        };
        if let Some(plan) = step_plans.get_mut(*child) {
            *plan = StepPlan {
                bounds: Offset {
                    x: start,
                    y: top,
                    width: step_width,
                    height,
                },
                text_x: start + padding_left,
                text_y: top + stack_y + padding_top,
                text_width: step_width - step_frame_width(step.look),
            };
        }
    }

    Ok(PhasesLayout {
        width: node_width,
        height: y,
        label_width,
        lanes,
        phases: phase_plans,
        steps: step_plans,
    })
}

/// Adds `extra` to the columns of one phase in equal shares: to its filled columns when it
/// has any, else to all of them.
fn spread(widths: &mut [f32], filled: &[bool], extra: f32) {
    let filled_count = filled.iter().filter(|filled| **filled).count();
    if filled_count == 0 {
        let count = widths.len().max(1) as f32;
        for column in widths.iter_mut() {
            *column += extra / count;
        }
        return;
    }
    let share = extra / filled_count as f32;
    for (column, is_filled) in widths.iter_mut().zip(filled) {
        if *is_filled {
            *column += share;
        }
    }
}
