//! A HowTo's sizes and how many of its items share a row (section 14.4 rule 21).

use std::collections::BTreeMap;

use stencil_model::pointer::NodePointer;
use stencil_model::text::{MeasureError, TextMeasurer, TextStyleName};
use stencil_model::{
    CommandRun, HowTo, HowToItem, Node, NodeRef, Page, SHEET_ROOT_PADDING_PX, SheetList,
    TerminalKind, body_nodes, canvas_width,
};

use crate::LayoutError;
use crate::styles::text_style_for;

/// Between two items of a row, and between two rows.
pub(crate) const HOW_TO_COLUMN_GAP_PX: f32 = 24.0;
pub(crate) const HOW_TO_ROW_GAP_PX: f32 = 20.0;
/// Between the number, the title and the command line of the heading row.
pub(crate) const HOW_TO_HEADING_GAP_PX: f32 = 8.0;
/// Between two runs of the command line.
pub(crate) const COMMAND_GAP_PX: f32 = 4.0;
/// A chip's padding, top and bottom then the sides, and its border.
pub(crate) const CHIP_PADDING_PX: (f32, f32) = (2.0, 4.0);
pub(crate) const CHIP_BORDER_PX: f32 = 1.0;
/// The rule under the heading row, and the space above and below it.
pub(crate) const HOW_TO_RULE_PX: f32 = 1.0;
pub(crate) const HOW_TO_RULE_ABOVE_PX: f32 = 6.0;
pub(crate) const HOW_TO_RULE_BELOW_PX: f32 = 8.0;
/// Between the text and the terminal under it, and beside it.
pub(crate) const HOW_TO_TEXT_GAP_PX: f32 = 6.0;
pub(crate) const HOW_TO_BESIDE_GAP_PX: f32 = 12.0;
/// The narrowest the text beside a terminal may wrap.
pub(crate) const HOW_TO_BESIDE_TEXT_MIN_PX: f32 = 160.0;
/// The terminal's padding, top, sides and bottom, and its border.
pub(crate) const TERMINAL_PADDING_PX: (f32, f32, f32) = (6.0, 10.0, 7.0);
pub(crate) const TERMINAL_BORDER_PX: f32 = 1.0;

/// The column every terminal line starts after, and the width of one indent level.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TerminalMetrics {
    /// The widest prompt in whole advances, plus one advance for the space after it.
    pub prompt_column_px: f32,
    /// Two advances.
    pub indent_px: f32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HowToPlan {
    columns: BTreeMap<String, usize>,
    pub terminal: TerminalMetrics,
}

impl HowToPlan {
    /// Items per row of the HowTo at `pointer`.
    pub(crate) fn columns(&self, pointer: &NodePointer) -> usize {
        self.columns.get(pointer.as_str()).copied().unwrap_or(1)
    }
}

/// The item number drawn before a how-to's title: the view's number and the item's place
/// among the view's how-tos.
pub(crate) fn how_to_number(view: usize, item: usize) -> String {
    format!("{view}.{item}")
}

pub(crate) fn plan_how_tos(
    page: &Page,
    measurer: &mut dyn TextMeasurer,
) -> Result<HowToPlan, LayoutError> {
    let mut plan = HowToPlan::default();
    if page.sheet.is_none() {
        return Ok(plan);
    }
    let entries: Vec<_> = body_nodes(page)
        .into_iter()
        .filter(|entry| matches!(entry.node, NodeRef::Node(Node::HowTo(_))))
        .collect();
    let Some(first) = entries.first() else {
        return Ok(plan);
    };
    plan.terminal = terminal_metrics(measurer).map_err(|source| LayoutError::Measure {
        pointer: first.pointer.clone(),
        source,
    })?;
    let content_width = canvas_width(page) - 2.0 * SHEET_ROOT_PADDING_PX;
    for entry in &entries {
        let NodeRef::Node(Node::HowTo(how_to)) = entry.node else {
            continue;
        };
        let widest = widest_item(how_to, plan.terminal, measurer).map_err(|source| {
            LayoutError::Measure {
                pointer: entry.pointer.clone(),
                source,
            }
        })?;
        let count = how_to.items.len().min(SheetList::HowToItems.max() + 1);
        plan.columns.insert(
            entry.pointer.as_str().to_string(),
            columns_for(count, widest, content_width),
        );
    }
    Ok(plan)
}

/// The most items a row holds when each is at least `widest` wide, then balanced so rows
/// differ by at most one item: four items that fit three to a row lie two and two.
pub(crate) fn columns_for(count: usize, widest: f32, content_width: f32) -> usize {
    if count == 0 {
        return 1;
    }
    let fits = |columns: usize| {
        let gaps = HOW_TO_COLUMN_GAP_PX * (columns as f32 - 1.0);
        widest * columns as f32 + gaps <= content_width
    };
    let most = (1..=count)
        .rev()
        .find(|columns| fits(*columns))
        .unwrap_or(1);
    let rows = count.div_ceil(most);
    count.div_ceil(rows)
}

/// The terminal face is monospace, so one advance is measured on a single character and
/// the prompt column and the indent step are whole advances: an indented line starts on a
/// column of the line above it.
fn terminal_metrics(measurer: &mut dyn TextMeasurer) -> Result<TerminalMetrics, MeasureError> {
    let style = text_style_for(TextStyleName::TerminalText, false);
    let mut width = |text: &str| -> Result<f32, MeasureError> {
        Ok(measurer.measure(text, &style, None)?.width_px)
    };
    let advance = width("x")?;
    let mut prompt_advances: f32 = 0.0;
    for kind in TerminalKind::ALL {
        if let Some(text) = kind.prompt() {
            prompt_advances = prompt_advances.max((width(text)? / advance).ceil());
        }
    }
    Ok(TerminalMetrics {
        prompt_column_px: (prompt_advances + 1.0) * advance,
        indent_px: 2.0 * advance,
    })
}

/// The narrowest an item can be with its heading row, command line and terminal lines on
/// their own lines, unwrapped.
fn widest_item(
    how_to: &HowTo,
    terminal: TerminalMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Result<f32, MeasureError> {
    let mut widest: f32 = 0.0;
    for (index, item) in how_to
        .items
        .iter()
        .enumerate()
        .take(SheetList::HowToItems.max() + 1)
    {
        widest = widest.max(item_min_width(item, index, terminal, measurer)?);
    }
    Ok(widest)
}

fn item_min_width(
    item: &HowToItem,
    index: usize,
    terminal: TerminalMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Result<f32, MeasureError> {
    let number_style = text_style_for(TextStyleName::SheetLabel, false);
    let title_style = text_style_for(TextStyleName::HowToTitle, false);
    // Two digits a side cover every view and item number a sheet draws.
    let number = measurer
        .measure(&how_to_number(10, index + 1), &number_style, None)?
        .width_px;
    let title = measurer
        .measure(&item.title.to_uppercase(), &title_style, Some(0.0))?
        .width_px;
    let commands = command_line_width(&item.commands, measurer)?;
    let mut heading = number + HOW_TO_HEADING_GAP_PX + title;
    if commands > 0.0 {
        heading += HOW_TO_HEADING_GAP_PX + commands;
    }
    let block = terminal_width(item, terminal, measurer)?;
    let body = if item.beside {
        block + HOW_TO_BESIDE_GAP_PX + HOW_TO_BESIDE_TEXT_MIN_PX
    } else {
        block
    };
    Ok(heading.max(body))
}

pub(crate) fn command_line_width(
    commands: &[CommandRun],
    measurer: &mut dyn TextMeasurer,
) -> Result<f32, MeasureError> {
    let chip_style = text_style_for(TextStyleName::TerminalText, false);
    let words_style = text_style_for(TextStyleName::ViewNoteSmall, false);
    let mut width: f32 = 0.0;
    for (index, run) in commands
        .iter()
        .take(SheetList::CommandRuns.max() + 1)
        .enumerate()
    {
        if index > 0 {
            width += COMMAND_GAP_PX;
        }
        width += match run {
            CommandRun::Chip(text) => {
                measurer.measure(text, &chip_style, None)?.width_px
                    + 2.0 * (CHIP_PADDING_PX.1 + CHIP_BORDER_PX)
            }
            CommandRun::Words(words) => {
                measurer.measure(&words.words, &words_style, None)?.width_px
            }
        };
    }
    Ok(width)
}

/// The terminal's width at its widest line: the prompt column, the line's indent and its
/// text, inside the padding and border. Zero for an item without terminal lines.
pub(crate) fn terminal_width(
    item: &HowToItem,
    terminal: TerminalMetrics,
    measurer: &mut dyn TextMeasurer,
) -> Result<f32, MeasureError> {
    let style = text_style_for(TextStyleName::TerminalText, false);
    let mut widest: f32 = 0.0;
    for line in item
        .terminal
        .iter()
        .take(SheetList::TerminalLines.max() + 1)
    {
        let text = measurer.measure(&line.text, &style, None)?.width_px;
        widest = widest
            .max(terminal.prompt_column_px + f32::from(line.indent) * terminal.indent_px + text);
    }
    if widest == 0.0 {
        return Ok(0.0);
    }
    Ok(widest.ceil() + 2.0 * (TERMINAL_PADDING_PX.1 + TERMINAL_BORDER_PX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows hold as many items as fit and are then balanced: a lone item never trails a
    /// full row.
    #[test]
    fn columns_fill_the_width_then_balance_the_rows() {
        let gap = HOW_TO_COLUMN_GAP_PX;
        let cases = [
            (4, 300.0, 4.0 * 300.0 + 3.0 * gap, 4),
            (4, 300.0, 3.0 * 300.0 + 2.0 * gap, 2),
            (5, 300.0, 3.0 * 300.0 + 2.0 * gap, 3),
            (6, 300.0, 4.0 * 300.0 + 3.0 * gap, 3),
            (3, 900.0, 800.0, 1),
            (1, 100.0, 1000.0, 1),
            (0, 100.0, 1000.0, 1),
        ];
        for (count, widest, width, expected) in cases {
            assert_eq!(
                columns_for(count, widest, width),
                expected,
                "{count} items of {widest} in {width}"
            );
        }
        assert_eq!(cases.len(), 7);
    }
}
