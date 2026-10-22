//! The detail ladder (section 14.4 rule 20), rung 1 at the bottom.

use stencil_model::marks::MarkPlace;
use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{Ladder, PageMarks, Rung, SheetList};
use taffy::prelude::{
    AlignItems, JustifyContent, LengthPercentageAuto, NodeId, Position, Rect, Style, length,
};

use super::sheet::RunStyles;
use super::{
    Builder, PartRecord, Placement, TextSpec, apply_flex_placement, base_style, fixed_size,
    flex_column, flex_row, margins, sides,
};
use crate::ladder::{
    ESCALATION_X_PX, LADDER_BORDER_PX, LADDER_HEADING_GAP_PX, LADDER_PADDING_PX, LADDER_TAG_PX,
    LADDER_TITLE_GAP_PX, RAIL_COLUMN_PX, RAIL_GAP_PX, RAIL_LEFT_X_PX, RAIL_RIGHT_X_PX,
    RAIL_STROKE_PX, RUNG_COLUMN_GAP_PX, RUNG_GAP_PX, RUNG_MIN_HEIGHT_PX, RUNG_PADDING_PX,
    rung_title_text,
};
use crate::{ARROWHEAD_WIDTH_PX, LayoutError, NodeTag, PartName, TextAlign};

const RUNG_RUN: RunStyles = RunStyles {
    lead: TextStyleName::RungTitle,
    body: TextStyleName::RungWho,
    lead_field: "title",
    body_field: "who",
};

/// An absolute box `left` into its parent and `width` wide, from its top to its bottom.
fn upright(left: f32, width: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            left: length(left),
            right: LengthPercentageAuto::auto(),
            top: length(0.0),
            bottom: length(0.0),
        },
        size: taffy::Size {
            width: length(width),
            height: taffy::prelude::auto(),
        },
        ..base_style()
    }
}

impl Builder<'_> {
    pub(super) fn add_ladder(
        &mut self,
        ladder: &Ladder,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let letter_index = self
            .records
            .iter()
            .filter(|record| record.tag == NodeTag::Ladder)
            .count();
        let (top, side, bottom) = LADDER_PADDING_PX;
        let mut style = Style {
            border: sides(
                LADDER_BORDER_PX,
                LADDER_BORDER_PX,
                LADDER_BORDER_PX,
                LADDER_BORDER_PX,
            ),
            padding: sides(top, side, bottom, side),
            gap: taffy::Size {
                width: length(0.0),
                height: length(LADDER_HEADING_GAP_PX),
            },
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let node = self.container(style, parent_container, &pointer)?;
        let mut parts = self.ladder_heading(ladder, node, letter_index, &pointer)?;

        let body = self.container(
            Style {
                flex_grow: 1.0,
                gap: taffy::Size {
                    width: length(0.0),
                    height: length(RUNG_GAP_PX),
                },
                ..flex_column(AlignItems::STRETCH)
            },
            node,
            &pointer,
        )?;
        for (name, left, width) in [
            (
                PartName::RailLeft,
                RAIL_LEFT_X_PX - RAIL_STROKE_PX / 2.0,
                RAIL_STROKE_PX,
            ),
            (
                PartName::RailRight,
                RAIL_RIGHT_X_PX - RAIL_STROKE_PX / 2.0,
                RAIL_STROKE_PX,
            ),
            (
                PartName::EscalationArrow,
                ESCALATION_X_PX - ARROWHEAD_WIDTH_PX / 2.0,
                ARROWHEAD_WIDTH_PX,
            ),
        ] {
            let leaf = self.plain_leaf(upright(left, width), body, &pointer)?;
            parts.push(PartRecord {
                name,
                taffy_node: leaf,
                text_leaf: None,
            });
        }

        let rungs = SheetList::Rungs;
        let count = ladder.rungs.len().min(rungs.max() + 1);
        let mut ticks: Vec<Option<PartRecord>> = (0..count).map(|_| None).collect();
        let mut rung_records = Vec::with_capacity(count);
        for index in (0..count).rev() {
            let Some(rung) = ladder.rungs.get(index) else {
                continue;
            };
            let rung_pointer = rungs.pointer_in(&pointer).index(index);
            let row = self.container(
                Style {
                    flex_grow: 1.0,
                    gap: taffy::Size {
                        width: length(RAIL_GAP_PX),
                        height: length(0.0),
                    },
                    ..flex_row(AlignItems::STRETCH)
                },
                body,
                &rung_pointer,
            )?;
            let rail_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(RAIL_COLUMN_PX),
                        height: taffy::prelude::auto(),
                    },
                    flex_shrink: 0.0,
                    justify_content: Some(JustifyContent::CENTER),
                    ..flex_column(AlignItems::START)
                },
                row,
                &rung_pointer,
            )?;
            let tick = self.plain_leaf(
                Style {
                    size: fixed_size(RAIL_RIGHT_X_PX - RAIL_LEFT_X_PX, RAIL_STROKE_PX),
                    margin: margins(0.0, 0.0, 0.0, RAIL_LEFT_X_PX),
                    ..base_style()
                },
                rail_cell,
                &rung_pointer,
            )?;
            if let Some(slot) = ticks.get_mut(index) {
                *slot = Some(PartRecord {
                    name: PartName::RungTick,
                    taffy_node: tick,
                    text_leaf: None,
                });
            }
            rung_records.push(self.add_rung((ladder, rung, index), row, &pointer, rung_pointer)?);
        }
        parts.extend(ticks.into_iter().flatten());
        let mut record = Self::record(pointer, NodeTag::Ladder, None, Some(parent_index), node);
        record.parts = parts;
        let ladder_index = self.push_record(record);
        rung_records.reverse();
        for mut rung_record in rung_records {
            rung_record.parent = Some(ladder_index);
            self.push_record(rung_record);
        }
        Ok(())
    }

    fn ladder_heading(
        &mut self,
        ladder: &Ladder,
        node: NodeId,
        letter_index: usize,
        pointer: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let heading = self.container(
            Style {
                gap: taffy::Size {
                    width: length(LADDER_TITLE_GAP_PX),
                    height: length(0.0),
                },
                ..flex_row(AlignItems::CENTER)
            },
            node,
            pointer,
        )?;
        let tag = self.container(
            Style {
                size: fixed_size(LADDER_TAG_PX, LADDER_TAG_PX),
                flex_shrink: 0.0,
                justify_content: Some(JustifyContent::CENTER),
                ..flex_column(AlignItems::CENTER)
            },
            heading,
            pointer,
        )?;
        let letter = detail_letter(letter_index);
        let (letter_node, letter_leaf) = self.text_leaf(
            base_style(),
            Some(tag),
            TextSpec {
                text: &letter,
                style_name: TextStyleName::ViewNumeral,
                line: None,
                align: TextAlign::Center,
                source: pointer.clone(),
            },
        )?;
        let (title, title_leaf) = self.text_leaf(
            base_style(),
            Some(heading),
            TextSpec {
                text: &ladder.title,
                style_name: TextStyleName::ListTitle,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("title"),
            },
        )?;
        let mut parts = vec![
            PartRecord {
                name: PartName::DetailTag,
                taffy_node: tag,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::DetailLetter,
                taffy_node: letter_node,
                text_leaf: Some(letter_leaf),
            },
            PartRecord {
                name: PartName::Title,
                taffy_node: title,
                text_leaf: Some(title_leaf),
            },
        ];
        if let Some(note) = &ladder.note {
            let (leaf, leaf_index) = self.text_leaf(
                Style {
                    flex_shrink: 1.0,
                    ..base_style()
                },
                Some(heading),
                TextSpec {
                    text: note,
                    style_name: TextStyleName::ViewNoteSmall,
                    line: None,
                    align: TextAlign::Start,
                    source: pointer.child("note"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::Note,
                taffy_node: leaf,
                text_leaf: Some(leaf_index),
            });
        }
        Ok(parts)
    }

    /// Returns the rung's record, whose parent the caller sets once the Ladder is pushed.
    fn add_rung(
        &mut self,
        (ladder, rung, index): (&Ladder, &Rung, usize),
        row: NodeId,
        ladder_pointer: &NodePointer,
        pointer: NodePointer,
    ) -> Result<super::NodeRecord, LayoutError> {
        let (vertical, horizontal) = RUNG_PADDING_PX;
        let node = self.container(
            Style {
                flex_grow: 1.0,
                flex_basis: length(0.0),
                min_size: taffy::Size {
                    width: taffy::prelude::auto(),
                    height: length(RUNG_MIN_HEIGHT_PX),
                },
                padding: sides(vertical, horizontal, vertical, horizontal),
                gap: taffy::Size {
                    width: length(RUNG_COLUMN_GAP_PX),
                    height: length(0.0),
                },
                ..flex_row(AlignItems::CENTER)
            },
            row,
            &pointer,
        )?;
        let title_width = self.ladder_plan.title_width(ladder_pointer);
        let title_cell = self.container(
            Style {
                size: taffy::Size {
                    width: length(title_width),
                    height: taffy::prelude::auto(),
                },
                flex_shrink: 0.0,
                ..flex_row(AlignItems::START)
            },
            node,
            &pointer,
        )?;
        let title = rung_title_text(index, &rung.title);
        let title_style = Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: length(0.0),
            ..base_style()
        };
        let mut parts = vec![PartRecord {
            name: PartName::Front,
            taffy_node: node,
            text_leaf: None,
        }];
        match &rung.who {
            Some(who) => parts.extend(self.lead_run(
                title_cell,
                title_style,
                &RUNG_RUN,
                [(PartName::RungTitle, &title), (PartName::RungWho, who)],
                &pointer,
            )?),
            None => {
                let (leaf, leaf_index) = self.text_leaf(
                    title_style,
                    Some(title_cell),
                    TextSpec {
                        text: &title,
                        style_name: TextStyleName::RungTitle,
                        line: None,
                        align: TextAlign::Start,
                        source: pointer.child("title"),
                    },
                )?;
                parts.push(PartRecord {
                    name: PartName::RungTitle,
                    taffy_node: leaf,
                    text_leaf: Some(leaf_index),
                });
            }
        }
        let marks = PageMarks::rung_marks(ladder, ladder_pointer, index);
        parts.extend(self.add_mark_row(title_cell, &marks, MarkPlace::Trailing, &pointer)?);
        let (text, text_leaf) = self.text_leaf(
            Style {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: length(0.0),
                ..base_style()
            },
            Some(node),
            TextSpec {
                text: &rung.text,
                style_name: TextStyleName::BlockBody,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("text"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::RungText,
            taffy_node: text,
            text_leaf: Some(text_leaf),
        });
        let mut record = Self::record(pointer, NodeTag::Rung, None, None, node);
        record.tint = rung
            .actor
            .as_deref()
            .and_then(|actor| self.actors.tint(actor));
        record.parts = parts;
        Ok(record)
    }
}

pub(crate) fn detail_letter(index: usize) -> String {
    let offset = u8::try_from(index).unwrap_or(u8::MAX).min(25);
    char::from(b'A' + offset).to_string()
}
