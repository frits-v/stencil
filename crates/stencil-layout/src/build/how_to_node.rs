//! A HowTo's items (section 14.4 rule 21): a heading row with the number, the title and the
//! command line, a rule, then the text and the terminal.

use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{CommandRun, HowTo, HowToItem, SheetList};
use taffy::prelude::{AlignItems, Dimension, Display, NodeId, Style, auto, fr, length, minmax};

use super::sheet::FIRST_VIEW_NUMBER;
use super::{
    Builder, NodeRecord, PartRecord, Placement, TextSpec, apply_flex_placement, base_style,
    flex_column, flex_row, margins, sides,
};
use crate::how_to::{
    CHIP_BORDER_PX, CHIP_PADDING_PX, COMMAND_GAP_PX, HOW_TO_BESIDE_GAP_PX,
    HOW_TO_BESIDE_TEXT_MIN_PX, HOW_TO_COLUMN_GAP_PX, HOW_TO_HEADING_GAP_PX, HOW_TO_ROW_GAP_PX,
    HOW_TO_RULE_ABOVE_PX, HOW_TO_RULE_BELOW_PX, HOW_TO_RULE_PX, HOW_TO_TEXT_GAP_PX,
    TERMINAL_BORDER_PX, TERMINAL_PADDING_PX, how_to_number,
};
use crate::{LayoutError, NodeTag, PartName, TextAlign};

/// A box that may end narrower than its content, so a run that does not fit is drawn past
/// the box's edge and text-fits-box reports it.
fn shrinkable(style: Style) -> Style {
    Style {
        flex_shrink: 1.0,
        min_size: taffy::Size {
            width: length(0.0),
            height: auto(),
        },
        ..style
    }
}

impl Builder<'_> {
    pub(super) fn add_how_to(
        &mut self,
        how_to: &HowTo,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let columns = self.how_to_plan.columns(&pointer).max(1);
        let tracks: Vec<_> = (0..columns).map(|_| minmax(length(0.0), fr(1.0))).collect();
        let mut style = Style {
            display: Display::Grid,
            grid_template_columns: tracks,
            gap: taffy::Size {
                width: length(HOW_TO_COLUMN_GAP_PX),
                height: length(HOW_TO_ROW_GAP_PX),
            },
            ..base_style()
        };
        apply_flex_placement(&mut style, placement);
        let node = self.container(style, parent_container, &pointer)?;
        let views = self
            .records
            .iter()
            .filter(|record| record.tag == NodeTag::View)
            .count();
        let view_number = FIRST_VIEW_NUMBER + views.saturating_sub(1);
        let since_view = self
            .records
            .iter()
            .rposition(|record| record.tag == NodeTag::View)
            .map_or(0, |view| view + 1);
        let earlier_items = self
            .records
            .iter()
            .skip(since_view)
            .filter(|record| record.tag == NodeTag::HowToItem)
            .count();
        let items = SheetList::HowToItems;
        let mut item_records = Vec::new();
        for (index, item) in how_to.items.iter().enumerate().take(items.max() + 1) {
            let number = how_to_number(view_number, earlier_items + index + 1);
            let item_pointer = items.pointer_in(&pointer).index(index);
            item_records.push(self.add_how_to_item(item, &number, node, item_pointer)?);
        }
        let record = Self::record(pointer, NodeTag::HowTo, None, Some(parent_index), node);
        let how_to_index = self.push_record(record);
        for mut item_record in item_records {
            item_record.parent = Some(how_to_index);
            self.push_record(item_record);
        }
        Ok(())
    }

    /// Returns the item's record, whose parent the caller sets once the HowTo is pushed.
    fn add_how_to_item(
        &mut self,
        item: &HowToItem,
        number: &str,
        grid: NodeId,
        pointer: NodePointer,
    ) -> Result<NodeRecord, LayoutError> {
        let node = self.container(
            Style {
                min_size: taffy::Size {
                    width: length(0.0),
                    height: auto(),
                },
                ..flex_column(AlignItems::STRETCH)
            },
            grid,
            &pointer,
        )?;
        let mut parts = self.how_to_heading(item, number, node, &pointer)?;
        let rule = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: Dimension::auto(),
                    height: length(HOW_TO_RULE_PX),
                },
                margin: margins(HOW_TO_RULE_ABOVE_PX, 0.0, HOW_TO_RULE_BELOW_PX, 0.0),
                ..base_style()
            },
            node,
            &pointer,
        )?;
        parts.push(PartRecord {
            name: PartName::HowToRule,
            taffy_node: rule,
            text_leaf: None,
        });
        let beside = item.beside && item.text.is_some() && !item.terminal.is_empty();
        let body = self.container(
            if beside {
                Style {
                    gap: taffy::Size {
                        width: length(HOW_TO_BESIDE_GAP_PX),
                        height: length(0.0),
                    },
                    ..flex_row(AlignItems::START)
                }
            } else {
                Style {
                    gap: taffy::Size {
                        width: length(0.0),
                        height: length(HOW_TO_TEXT_GAP_PX),
                    },
                    ..flex_column(AlignItems::STRETCH)
                }
            },
            node,
            &pointer,
        )?;
        let text_part =
            |builder: &mut Self, style: Style| -> Result<Option<PartRecord>, LayoutError> {
                let Some(text) = &item.text else {
                    return Ok(None);
                };
                let (leaf, leaf_index) = builder.text_leaf(
                    style,
                    Some(body),
                    TextSpec {
                        text,
                        style_name: TextStyleName::BlockBody,
                        line: None,
                        align: TextAlign::Start,
                        source: pointer.child("text"),
                    },
                )?;
                Ok(Some(PartRecord {
                    name: PartName::HowToText,
                    taffy_node: leaf,
                    text_leaf: Some(leaf_index),
                }))
            };
        if beside {
            parts.extend(self.add_terminal(item, body, false, &pointer)?);
            parts.extend(text_part(
                self,
                Style {
                    flex_grow: 1.0,
                    flex_basis: length(0.0),
                    min_size: taffy::Size {
                        width: length(HOW_TO_BESIDE_TEXT_MIN_PX),
                        height: auto(),
                    },
                    ..base_style()
                },
            )?);
        } else {
            parts.extend(text_part(self, base_style())?);
            parts.extend(self.add_terminal(item, body, true, &pointer)?);
        }
        let mut record = Self::record(pointer, NodeTag::HowToItem, None, None, node);
        record.parts = parts;
        Ok(record)
    }

    /// The number, the title wrapping in the width left, and the command line, unwrapped.
    fn how_to_heading(
        &mut self,
        item: &HowToItem,
        number: &str,
        node: NodeId,
        pointer: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let heading = self.container(
            Style {
                gap: taffy::Size {
                    width: length(HOW_TO_HEADING_GAP_PX),
                    height: length(0.0),
                },
                ..flex_row(AlignItems::CENTER)
            },
            node,
            pointer,
        )?;
        let (number_node, number_leaf) = self.text_leaf(
            base_style(),
            Some(heading),
            TextSpec {
                text: number,
                style_name: TextStyleName::SheetLabel,
                line: None,
                align: TextAlign::Start,
                source: pointer.clone(),
            },
        )?;
        let (title, title_leaf) = self.text_leaf(
            Style {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: length(0.0),
                ..base_style()
            },
            Some(heading),
            TextSpec {
                text: &item.title,
                style_name: TextStyleName::HowToTitle,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("title"),
            },
        )?;
        let mut parts = vec![
            PartRecord {
                name: PartName::HowToNumber,
                taffy_node: number_node,
                text_leaf: Some(number_leaf),
            },
            PartRecord {
                name: PartName::HowToTitle,
                taffy_node: title,
                text_leaf: Some(title_leaf),
            },
        ];
        let runs = SheetList::CommandRuns;
        if item.commands.is_empty() {
            return Ok(parts);
        }
        let line = self.container(
            Style {
                gap: taffy::Size {
                    width: length(COMMAND_GAP_PX),
                    height: length(0.0),
                },
                ..flex_row(AlignItems::CENTER)
            },
            heading,
            pointer,
        )?;
        let commands_pointer = runs.pointer_in(pointer);
        for (index, run) in item.commands.iter().enumerate().take(runs.max() + 1) {
            let run_pointer = commands_pointer.index(index);
            match run {
                CommandRun::Chip(text) => {
                    let (vertical, horizontal) = CHIP_PADDING_PX;
                    let chip = self.container(
                        Style {
                            border: sides(
                                CHIP_BORDER_PX,
                                CHIP_BORDER_PX,
                                CHIP_BORDER_PX,
                                CHIP_BORDER_PX,
                            ),
                            padding: sides(vertical, horizontal, vertical, horizontal),
                            ..flex_row(AlignItems::CENTER)
                        },
                        line,
                        pointer,
                    )?;
                    let (leaf, leaf_index) = self.unwrapped_leaf(
                        base_style(),
                        chip,
                        TextSpec {
                            text,
                            style_name: TextStyleName::TerminalText,
                            line: None,
                            align: TextAlign::Start,
                            source: run_pointer,
                        },
                    )?;
                    parts.push(PartRecord {
                        name: PartName::CommandChip,
                        taffy_node: chip,
                        text_leaf: None,
                    });
                    parts.push(PartRecord {
                        name: PartName::CommandText,
                        taffy_node: leaf,
                        text_leaf: Some(leaf_index),
                    });
                }
                CommandRun::Words(words) => {
                    let (leaf, leaf_index) = self.unwrapped_leaf(
                        base_style(),
                        line,
                        TextSpec {
                            text: &words.words,
                            style_name: TextStyleName::ViewNoteSmall,
                            line: None,
                            align: TextAlign::Start,
                            source: run_pointer.child("words"),
                        },
                    )?;
                    parts.push(PartRecord {
                        name: PartName::CommandWord,
                        taffy_node: leaf,
                        text_leaf: Some(leaf_index),
                    });
                }
            }
        }
        Ok(parts)
    }

    /// The terminal block: one row per line, the prompt in its column and the line after
    /// its indent. Stretched to the item's width under its text, or at its widest line
    /// beside the text.
    fn add_terminal(
        &mut self,
        item: &HowToItem,
        body: NodeId,
        stretched: bool,
        pointer: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        if item.terminal.is_empty() {
            return Ok(Vec::new());
        }
        let (top, side, bottom) = TERMINAL_PADDING_PX;
        let mut style = Style {
            border: sides(
                TERMINAL_BORDER_PX,
                TERMINAL_BORDER_PX,
                TERMINAL_BORDER_PX,
                TERMINAL_BORDER_PX,
            ),
            padding: sides(top, side, bottom, side),
            ..flex_column(AlignItems::STRETCH)
        };
        if stretched {
            style.min_size = taffy::Size {
                width: length(0.0),
                height: auto(),
            };
        }
        let terminal = self.container(style, body, pointer)?;
        let mut parts = vec![PartRecord {
            name: PartName::Terminal,
            taffy_node: terminal,
            text_leaf: None,
        }];
        let metrics = self.how_to_plan.terminal;
        let lines = SheetList::TerminalLines;
        for (index, line) in item.terminal.iter().enumerate().take(lines.max() + 1) {
            let line_pointer = lines.pointer_in(pointer).index(index);
            let row = self.container(
                shrinkable(flex_row(AlignItems::START)),
                terminal,
                &line_pointer,
            )?;
            let prompt_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(metrics.prompt_column_px),
                        height: auto(),
                    },
                    ..flex_row(AlignItems::START)
                },
                row,
                &line_pointer,
            )?;
            if let Some(prompt) = line.kind.prompt() {
                let (leaf, leaf_index) = self.unwrapped_leaf(
                    base_style(),
                    prompt_cell,
                    TextSpec {
                        text: prompt,
                        style_name: TextStyleName::TerminalText,
                        line: None,
                        align: TextAlign::Start,
                        source: line_pointer.child("kind"),
                    },
                )?;
                parts.push(PartRecord {
                    name: PartName::TerminalPrompt,
                    taffy_node: leaf,
                    text_leaf: Some(leaf_index),
                });
            }
            let (leaf, leaf_index) = self.unwrapped_leaf(
                shrinkable(Style {
                    margin: margins(0.0, 0.0, 0.0, f32::from(line.indent) * metrics.indent_px),
                    ..base_style()
                }),
                row,
                TextSpec {
                    text: &line.text,
                    style_name: TextStyleName::TerminalText,
                    line: None,
                    align: TextAlign::Start,
                    source: line_pointer.child("text"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::TerminalLine,
                taffy_node: leaf,
                text_leaf: Some(leaf_index),
            });
        }
        Ok(parts)
    }

    /// A text leaf set on one line whatever width its box ends with.
    fn unwrapped_leaf(
        &mut self,
        style: Style,
        parent: NodeId,
        spec: TextSpec<'_>,
    ) -> Result<(NodeId, usize), LayoutError> {
        let (leaf, index) = self.text_leaf(style, Some(parent), spec)?;
        if let Some(text_leaf) = self.text_leaves.get_mut(index) {
            text_leaf.wraps = false;
        }
        Ok((leaf, index))
    }
}
