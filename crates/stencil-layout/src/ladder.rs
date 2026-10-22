//! A detail ladder's sizes and the title column all its rungs share (section 14.4 rule 20).

use std::collections::BTreeMap;

use stencil_model::marks::MarkPlace;
use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, TextStyleName};
use stencil_model::{Ladder, Node, NodeRef, Page, PageMarks, SheetList, body_nodes};

use crate::LayoutError;
use crate::lead_run::measure_lead_run;
use crate::marks::{MARK_GAP_PX, glyphs_width, numbered};
use crate::styles::text_style_for;

pub(crate) const LADDER_BORDER_PX: f32 = 1.0;
/// Top, sides and bottom.
pub(crate) const LADDER_PADDING_PX: (f32, f32, f32) = (10.0, 12.0, 11.0);
pub(crate) const LADDER_HEADING_GAP_PX: f32 = 10.0;
pub(crate) const LADDER_TITLE_GAP_PX: f32 = 8.0;
pub(crate) const LADDER_TAG_PX: f32 = 20.0;
/// The column the rails stand in, and the two rails' centres inside it.
pub(crate) const RAIL_COLUMN_PX: f32 = 34.0;
pub(crate) const RAIL_LEFT_X_PX: f32 = 6.0;
pub(crate) const RAIL_RIGHT_X_PX: f32 = 28.0;
pub(crate) const RAIL_STROKE_PX: f32 = 1.5;
/// The escalation arrow's centre in the rail column.
pub(crate) const ESCALATION_X_PX: f32 = 17.0;
pub(crate) const RAIL_GAP_PX: f32 = 10.0;
pub(crate) const RUNG_GAP_PX: f32 = 5.0;
pub(crate) const RUNG_MIN_HEIGHT_PX: f32 = 40.0;
/// Top and bottom, then the sides.
pub(crate) const RUNG_PADDING_PX: (f32, f32) = (6.0, 10.0);
pub(crate) const RUNG_COLUMN_GAP_PX: f32 = 12.0;
/// The shared title column fits the widest title, who and marks on one line, within these.
pub(crate) const RUNG_TITLE_MIN_PX: f32 = 158.0;
pub(crate) const RUNG_TITLE_MAX_PX: f32 = 280.0;

#[derive(Debug, Clone, Default)]
pub(crate) struct LadderPlan {
    title_widths: BTreeMap<String, f32>,
}

impl LadderPlan {
    pub(crate) fn title_width(&self, pointer: &NodePointer) -> f32 {
        self.title_widths
            .get(pointer.as_str())
            .copied()
            .unwrap_or(RUNG_TITLE_MIN_PX)
    }
}

pub(crate) fn rung_title_text(index: usize, title: &str) -> String {
    format!("{} \u{b7} {title}", index + 1)
}

pub(crate) fn plan_ladders(
    page: &Page,
    measurer: &mut dyn TextMeasurer,
) -> Result<LadderPlan, LayoutError> {
    let mut plan = LadderPlan::default();
    if page.sheet.is_none() {
        return Ok(plan);
    }
    let page_marks = PageMarks::of(page);
    for entry in body_nodes(page) {
        let NodeRef::Node(Node::Ladder(ladder)) = entry.node else {
            continue;
        };
        let width =
            title_width(ladder, &entry.pointer, &page_marks, measurer).map_err(|source| {
                LayoutError::Measure {
                    pointer: entry.pointer.clone(),
                    source,
                }
            })?;
        plan.title_widths
            .insert(entry.pointer.as_str().to_string(), width);
    }
    Ok(plan)
}

fn title_width(
    ladder: &Ladder,
    pointer: &NodePointer,
    page_marks: &PageMarks<'_>,
    measurer: &mut dyn TextMeasurer,
) -> Result<f32, MeasureError> {
    let lead_style = text_style_for(TextStyleName::RungTitle, false);
    let who_style = text_style_for(TextStyleName::RungWho, false);
    let mut widest: f32 = 0.0;
    for (index, rung) in ladder
        .rungs
        .iter()
        .enumerate()
        .take(SheetList::Rungs.max() + 1)
    {
        let lead = rung_title_text(index, &rung.title).to_uppercase();
        let text_width = match &rung.who {
            Some(who) => {
                measure_lead_run(measurer, &lead, &lead_style, who, &who_style, None)?.width_px
            }
            None => measurer.measure(&lead, &lead_style, None)?.width_px,
        };
        let marks = PageMarks::rung_marks(ladder, pointer, index);
        let trailing: Vec<_> = marks
            .iter()
            .filter(|mark| mark.place == MarkPlace::Trailing)
            .collect();
        let glyphs = numbered(page_marks, &trailing);
        let marks_width = if glyphs.is_empty() {
            0.0
        } else {
            MARK_GAP_PX + glyphs_width(&glyphs)
        };
        widest = widest.max(text_width + marks_width);
    }
    // One px of slack keeps a title measured at its own width on one line.
    Ok((widest.ceil() + 1.0).clamp(RUNG_TITLE_MIN_PX, RUNG_TITLE_MAX_PX))
}
