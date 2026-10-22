//! A Phases node and its Steps (section 14.4 rules 11 to 14), placed at the offsets
//! `plan_phases` fixed: every part and Step is an absolutely positioned child of the node.

use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{Node, Phases, SheetList, Step, StepLook, VetRule, Violation};
use taffy::prelude::{
    AlignItems, LengthPercentageAuto, NodeId, Position, Rect, Style, auto, length,
};

use super::{
    Builder, PartRecord, Placement, TextSpec, apply_flex_placement, base_style, flex_column,
};
use crate::phases::{
    LANE_BAR_PX, LANE_LABEL_LEFT_PX, LANE_NAME_TOP_PX, LANE_TEXT_GAP_PX, PHASE_HEAD_CENTER_PX,
    PHASE_HEAD_RADIUS_PX, PHASE_SEPARATOR_TOP_PX, PHASE_TITLE_LEFT_PX, PhasesLayout,
    STACK_OFFSET_PX, STEP_TITLE_GAP_PX, StepPlan, step_title_style,
};
use crate::{LayoutError, NodeTag, PartName, TextAlign};

/// A rule between two lanes and a phase separator are hairlines.
const HAIRLINE_PX: f32 = 1.0;

fn at(x: f32, y: f32) -> Rect<LengthPercentageAuto> {
    Rect {
        left: LengthPercentageAuto::length(x),
        top: LengthPercentageAuto::length(y),
        right: LengthPercentageAuto::auto(),
        bottom: LengthPercentageAuto::auto(),
    }
}

/// A box of a fixed size at (x, y) in its parent.
fn placed(x: f32, y: f32, width: f32, height: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: at(x, y),
        size: taffy::Size {
            width: length(width),
            height: length(height),
        },
        ..base_style()
    }
}

/// A text run at (x, y) in its parent, wrapping at `width`.
fn placed_text(x: f32, y: f32, width: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: at(x, y),
        size: taffy::Size {
            width: length(width),
            height: auto(),
        },
        ..base_style()
    }
}

impl Builder<'_> {
    fn placed_part(
        &mut self,
        parent: NodeId,
        name: PartName,
        style: Style,
        pointer: &NodePointer,
    ) -> Result<PartRecord, LayoutError> {
        let node = self.plain_leaf(style, parent, pointer)?;
        Ok(PartRecord {
            name,
            taffy_node: node,
            text_leaf: None,
        })
    }

    fn placed_text_part(
        &mut self,
        parent: NodeId,
        name: PartName,
        style: Style,
        (text, style_name, align): (&str, TextStyleName, TextAlign),
        source: NodePointer,
    ) -> Result<PartRecord, LayoutError> {
        let (node, leaf) = self.text_leaf(
            style,
            Some(parent),
            TextSpec {
                text,
                style_name,
                line: None,
                align,
                source,
            },
        )?;
        Ok(PartRecord {
            name,
            taffy_node: node,
            text_leaf: Some(leaf),
        })
    }

    pub(super) fn add_phases(
        &mut self,
        phases: &Phases,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let Some(layout) = self.phases_plan.node(&pointer).cloned() else {
            return Err(LayoutError::Taffy {
                pointer,
                message: "a Phases node was laid out without its plan".to_string(),
            });
        };
        let mut style = Style {
            size: taffy::Size {
                width: length(layout.width),
                height: length(layout.height),
            },
            flex_shrink: 0.0,
            ..flex_column(AlignItems::START)
        };
        apply_flex_placement(&mut style, placement);
        let node = self.container(style, parent_container, &pointer)?;
        let parts = self.phases_parts(phases, &layout, node, &pointer)?;
        let mut record = Self::record(
            pointer.clone(),
            NodeTag::Phases,
            None,
            Some(parent_index),
            node,
        );
        record.parts = parts;
        let index = self.push_record(record);
        let children_pointer = pointer.child("children");
        for (child_index, child) in phases
            .children
            .iter()
            .enumerate()
            .take(SheetList::Steps.max() + 1)
        {
            let child_pointer = children_pointer.index(child_index);
            let Node::Step(step) = child else {
                return Err(LayoutError::Invalid(vec![Violation {
                    message: "a Phases node holds Steps only".to_string(),
                    pointer: child_pointer,
                    rule: VetRule::PhasesChildNotStep,
                }]));
            };
            let plan = layout.steps.get(child_index).cloned();
            if let Some(plan) = plan {
                self.add_step(step, &plan, child_pointer, (index, node))?;
            }
        }
        Ok(())
    }

    /// The lanes, the phase heads and the separators, in drawing order: bands and bars under
    /// the rules, then the names, captions and heads.
    fn phases_parts(
        &mut self,
        phases: &Phases,
        layout: &PhasesLayout,
        node: NodeId,
        pointer: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let mut parts = Vec::new();
        let width = layout.width;
        let lanes_pointer = pointer.child("lanes");
        for lane in &layout.lanes {
            parts.push(self.placed_part(
                node,
                PartName::LaneBand,
                placed(0.0, lane.y, width, lane.height),
                pointer,
            )?);
            parts.push(self.placed_part(
                node,
                PartName::LaneBar,
                placed(0.0, lane.y, LANE_BAR_PX, lane.height),
                pointer,
            )?);
        }
        for lane in layout.lanes.iter().skip(1) {
            parts.push(self.placed_part(
                node,
                PartName::LaneRule,
                placed(0.0, lane.y, width, HAIRLINE_PX),
                pointer,
            )?);
        }
        for (index, (lane, id)) in layout.lanes.iter().zip(&phases.lanes).enumerate() {
            let Some(actor) = self.page.actors.iter().find(|actor| &actor.id == id) else {
                continue;
            };
            let name_top = lane.y + LANE_NAME_TOP_PX;
            parts.push(self.placed_text_part(
                node,
                PartName::LaneName,
                placed_text(LANE_LABEL_LEFT_PX, name_top, lane.label_width),
                (&actor.name, TextStyleName::LaneName, TextAlign::Start),
                lanes_pointer.index(index),
            )?);
            if let Some(text) = &actor.text {
                let name_height = TextStyleName::LaneName.text_style().style.line_height_px;
                parts.push(self.placed_text_part(
                    node,
                    PartName::LaneText,
                    placed_text(
                        LANE_LABEL_LEFT_PX,
                        name_top + name_height + LANE_TEXT_GAP_PX,
                        lane.label_width,
                    ),
                    (text, TextStyleName::StepText, TextAlign::Start),
                    lanes_pointer.index(index),
                )?);
            }
        }
        let phases_pointer = pointer.child("phases");
        let (center_x, center_y) = PHASE_HEAD_CENTER_PX;
        let diameter = 2.0 * PHASE_HEAD_RADIUS_PX;
        for (index, (phase, plan)) in phases.phases.iter().zip(&layout.phases).enumerate() {
            if index > 0 {
                parts.push(self.placed_part(
                    node,
                    PartName::PhaseSeparator,
                    placed(
                        plan.x,
                        PHASE_SEPARATOR_TOP_PX,
                        HAIRLINE_PX,
                        layout.height - PHASE_SEPARATOR_TOP_PX,
                    ),
                    pointer,
                )?);
            }
            let phase_pointer = phases_pointer.index(index);
            parts.push(self.placed_part(
                node,
                PartName::PhaseHead,
                placed(
                    plan.x + center_x - PHASE_HEAD_RADIUS_PX,
                    center_y - PHASE_HEAD_RADIUS_PX,
                    diameter,
                    diameter,
                ),
                pointer,
            )?);
            let numeral_line = TextStyleName::ViewNumeral.text_style().style.line_height_px;
            let numeral = (index + 1).to_string();
            parts.push(self.placed_text_part(
                node,
                PartName::PhaseNumeral,
                placed_text(
                    plan.x + center_x - PHASE_HEAD_RADIUS_PX,
                    center_y - numeral_line / 2.0,
                    diameter,
                ),
                (&numeral, TextStyleName::ViewNumeral, TextAlign::Center),
                phase_pointer.clone(),
            )?);
            let title_line = TextStyleName::SheetLabel.text_style().style.line_height_px;
            parts.push(self.placed_text_part(
                node,
                PartName::PhaseTitle,
                placed_text(
                    plan.x + PHASE_TITLE_LEFT_PX,
                    center_y - title_line / 2.0,
                    plan.width - PHASE_TITLE_LEFT_PX,
                ),
                (&phase.title, TextStyleName::SheetLabel, TextAlign::Start),
                phase_pointer.child("title"),
            )?);
        }
        Ok(parts)
    }

    /// A Step at its planned box: the look's boxes, then its title and lines in a column.
    fn add_step(
        &mut self,
        step: &Step,
        plan: &StepPlan,
        pointer: NodePointer,
        (phases_index, phases_node): (usize, NodeId),
    ) -> Result<(), LayoutError> {
        let bounds = plan.bounds;
        let step_node = self.container(
            Style {
                position: Position::Absolute,
                inset: at(bounds.x, bounds.y),
                size: taffy::Size {
                    width: length(bounds.width),
                    height: length(bounds.height),
                },
                ..flex_column(AlignItems::START)
            },
            phases_node,
            &pointer,
        )?;
        let mut parts = Vec::new();
        match step.look {
            StepLook::Stack => {
                let (offset_x, offset_y) = STACK_OFFSET_PX;
                let width = bounds.width - offset_x;
                let height = bounds.height - offset_y;
                for (x, y) in [(offset_x, 0.0), (offset_x / 2.0, offset_y / 2.0)] {
                    parts.push(self.placed_part(
                        step_node,
                        PartName::StackBack,
                        placed(x, y, width, height),
                        &pointer,
                    )?);
                }
                parts.push(self.placed_part(
                    step_node,
                    PartName::Front,
                    placed(0.0, offset_y, width, height),
                    &pointer,
                )?);
            }
            StepLook::Box | StepLook::Chip | StepLook::Note | StepLook::Planned => {
                parts.push(self.placed_part(
                    step_node,
                    PartName::Front,
                    placed(0.0, 0.0, bounds.width, bounds.height),
                    &pointer,
                )?);
            }
            StepLook::Text => {}
        }
        let column = self.container(
            Style {
                position: Position::Absolute,
                inset: at(plan.text_x - bounds.x, plan.text_y - bounds.y),
                size: taffy::Size {
                    width: length(plan.text_width),
                    height: auto(),
                },
                ..flex_column(AlignItems::STRETCH)
            },
            step_node,
            &pointer,
        )?;
        let (title, title_leaf) = self.text_leaf(
            base_style(),
            Some(column),
            TextSpec {
                text: &step.title,
                style_name: step_title_style(step.look),
                line: None,
                align: TextAlign::Start,
                source: pointer.child("title"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::Text,
            taffy_node: title,
            text_leaf: Some(title_leaf),
        });
        let lines_pointer = pointer.child("lines");
        for (index, line) in step
            .lines
            .iter()
            .enumerate()
            .take(SheetList::StepLines.max() + 1)
        {
            let top = if index == 0 { STEP_TITLE_GAP_PX } else { 0.0 };
            let (leaf, leaf_index) = self.text_leaf(
                Style {
                    margin: Rect {
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::length(0.0),
                        top: LengthPercentageAuto::length(top),
                        bottom: LengthPercentageAuto::length(0.0),
                    },
                    ..base_style()
                },
                Some(column),
                TextSpec {
                    text: line,
                    style_name: TextStyleName::StepText,
                    line: None,
                    align: TextAlign::Start,
                    source: lines_pointer.index(index),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::StepLine,
                taffy_node: leaf,
                text_leaf: Some(leaf_index),
            });
        }
        let mut record = Self::record(pointer, NodeTag::Step, None, Some(phases_index), step_node);
        record.tint = self.actors.tint(&step.lane);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }
}
