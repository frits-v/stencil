//! Mark glyphs on their carriers, the parts list and the general notes (section 14.4
//! rules 17, 18 and 22).

use stencil_model::marks::{CarrierMark, MarkPlace};
use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{GeneralNotes, MarkKind, PartsList, SheetList};
use taffy::prelude::{
    AlignItems, AlignSelf, JustifyContent, NodeId, Position, Rect, Style, length,
};

use super::{
    Builder, PartRecord, Placement, TextSpec, apply_flex_placement, base_style, fixed_size,
    flex_column, flex_row, margins, sides,
};
use crate::marks::{
    FLAG_NUMERAL_LIFT_PX, MARK_GAP_PX, WHERE_USED_PREFIX, glyph_part, glyph_size, numbered,
    numeral_style, where_used_placeholder,
};
use crate::{LayoutError, NodeTag, PartName, TextAlign};

/// The parts list's border, as light as the title block's.
const LIST_BORDER_PX: f32 = 1.5;
const LIST_TITLE_PADDING_PX: (f32, f32) = (8.0, 12.0);
const LIST_RULE_PX: f32 = 1.0;
const LIST_ROW_PADDING_PX: (f32, f32) = (6.0, 8.0);
/// The columns of a parts list row: the balloon, the part and what it is, and the zones;
/// why it exists takes the rest.
const PART_ITEM_COLUMN_PX: f32 = 40.0;
const PART_NAME_COLUMN_PX: f32 = 136.0;
const PART_ZONES_COLUMN_PX: f32 = 64.0;
const PART_WHY_GAP_PX: f32 = 8.0;
const PART_PLANNED_GAP_PX: f32 = 4.0;
const NOTES_BORDER_PX: f32 = 1.0;
const NOTES_PADDING_PX: (f32, f32, f32) = (10.0, 12.0, 11.0);
const NOTES_TITLE_GAP_PX: f32 = 12.0;
const NOTE_FLAG_COLUMN_PX: f32 = 26.0;
const NOTE_GAP_PX: f32 = 8.0;
const NOTE_FOOT_GAP_PX: f32 = 8.0;
const NOTE_LINE_GAP_PX: f32 = 1.0;

impl Builder<'_> {
    /// One glyph and its numeral: a box of the glyph's size holding the numeral centred
    /// across, in its middle for a balloon or a diamond and near the base of a flag.
    pub(super) fn add_glyph(
        &mut self,
        parent: NodeId,
        (glyph_name, numeral_name): (PartName, PartName),
        (kind, number): (MarkKind, usize),
        style: Style,
        source: &NodePointer,
    ) -> Result<[PartRecord; 2], LayoutError> {
        let (width, height) = glyph_size(kind, number);
        let (justify, bottom) = match kind {
            MarkKind::Note => (JustifyContent::END, FLAG_NUMERAL_LIFT_PX),
            MarkKind::Part | MarkKind::Attention => (JustifyContent::CENTER, 0.0),
        };
        let glyph = self.container(
            Style {
                size: fixed_size(width, height),
                justify_content: Some(justify),
                padding: sides(0.0, 0.0, bottom, 0.0),
                ..style
            },
            parent,
            source,
        )?;
        let numeral_text = number.to_string();
        let (numeral, numeral_leaf) = self.text_leaf(
            base_style(),
            Some(glyph),
            TextSpec {
                text: &numeral_text,
                style_name: numeral_style(kind),
                line: None,
                align: TextAlign::Center,
                source: source.clone(),
            },
        )?;
        Ok([
            PartRecord {
                name: glyph_name,
                taffy_node: glyph,
                text_leaf: None,
            },
            PartRecord {
                name: numeral_name,
                taffy_node: numeral,
                text_leaf: Some(numeral_leaf),
            },
        ])
    }

    /// A row of glyphs, 4 apart, for the marks `place` selects; None when there are none.
    pub(super) fn add_mark_row(
        &mut self,
        parent: NodeId,
        marks: &[CarrierMark],
        place: MarkPlace,
        source: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let selected: Vec<&CarrierMark> = marks.iter().filter(|mark| mark.place == place).collect();
        if selected.is_empty() {
            return Ok(Vec::new());
        }
        let row = self.container(
            Style {
                gap: taffy::Size {
                    width: length(MARK_GAP_PX),
                    height: length(0.0),
                },
                margin: margins(0.0, 0.0, 0.0, MARK_GAP_PX),
                ..flex_row(AlignItems::CENTER)
            },
            parent,
            source,
        )?;
        let mut parts = Vec::new();
        for glyph in numbered(&self.page_marks, &selected) {
            parts.extend(self.add_glyph(
                row,
                (glyph_part(glyph.0), PartName::MarkNumeral),
                glyph,
                flex_column(AlignItems::CENTER),
                source,
            )?);
        }
        Ok(parts)
    }

    /// A carrier's corner glyph, centred on the top-right corner of `carrier`'s border box.
    /// An absolute child is placed from the padding box, inside a border of `border` px.
    pub(super) fn add_corner_mark(
        &mut self,
        (carrier, border): (NodeId, f32),
        marks: &[CarrierMark],
        source: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let Some(corner) = marks.iter().find(|mark| mark.place == MarkPlace::Corner) else {
            return Ok(Vec::new());
        };
        let glyph = numbered(&self.page_marks, &[corner])
            .first()
            .copied()
            .unwrap_or((corner.mark.kind(), 0));
        let (width, height) = glyph_size(glyph.0, glyph.1);
        let style = Style {
            position: Position::Absolute,
            inset: Rect {
                left: taffy::prelude::LengthPercentageAuto::auto(),
                top: length(-height / 2.0 - border),
                right: length(-width / 2.0 - border),
                bottom: taffy::prelude::LengthPercentageAuto::auto(),
            },
            ..flex_column(AlignItems::CENTER)
        };
        Ok(self
            .add_glyph(
                carrier,
                (glyph_part(glyph.0), PartName::MarkNumeral),
                glyph,
                style,
                source,
            )?
            .into())
    }

    /// Wraps a text leaf in a row with the trailing glyphs after it: the text grows and
    /// wraps at the width the glyphs leave. Returns the row and the glyph parts; without
    /// trailing marks the text stays where it is and the row is `parent`.
    pub(super) fn marked_text_row(
        &mut self,
        parent: NodeId,
        marks: &[CarrierMark],
        place: MarkPlace,
        source: &NodePointer,
    ) -> Result<Option<NodeId>, LayoutError> {
        if !marks.iter().any(|mark| mark.place == place) {
            return Ok(None);
        }
        let row = self.container(flex_row(AlignItems::START), parent, source)?;
        Ok(Some(row))
    }

    /// The style a text leaf takes in a marked row: it grows into the row and wraps.
    pub(super) fn marked_text_style(mut style: Style) -> Style {
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style
    }

    /// The number of glyphs that name the key at `position` in the list of `kind`, from the
    /// document: the boxes their where-used text needs.
    fn sites_of(&self, kind: MarkKind, position: usize) -> usize {
        self.page_marks
            .all(self.page)
            .iter()
            .filter(|site| {
                site.mark.kind() == kind && self.page_marks.position(&site.mark) == Some(position)
            })
            .count()
    }

    /// A where-used text part reserved for the key's sites, to be filled after layout.
    pub(super) fn where_used_part(
        &mut self,
        parent: NodeId,
        name: PartName,
        (kind, position, prefix): (MarkKind, usize, &str),
        style: Style,
        source: NodePointer,
    ) -> Result<PartRecord, LayoutError> {
        let placeholder =
            where_used_placeholder(prefix, &self.widest_cell, self.sites_of(kind, position));
        let (leaf, leaf_index) = self.text_leaf(
            style,
            Some(parent),
            TextSpec {
                text: &placeholder,
                style_name: TextStyleName::StepText,
                line: None,
                align: TextAlign::Start,
                source,
            },
        )?;
        Ok(PartRecord {
            name,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        })
    }

    /// The parts list (section 14.4 rule 17): a title row, a rule, a header row, then a row
    /// per part with its balloon, its name and what it is, why it exists and its zones.
    pub(super) fn add_parts_list(
        &mut self,
        list: &PartsList,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let mut style = Style {
            border: sides(
                LIST_BORDER_PX,
                LIST_BORDER_PX,
                LIST_BORDER_PX,
                LIST_BORDER_PX,
            ),
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let node = self.container(style, parent_container, &pointer)?;
        let mut parts = self.list_title_row(
            node,
            &list.title,
            list.note.as_deref(),
            JustifyContent::SPACE_BETWEEN,
            &pointer,
        )?;
        let rule = self.list_rule(node, &pointer)?;
        parts.push(PartRecord {
            name: PartName::TitleRule,
            taffy_node: rule,
            text_leaf: None,
        });
        let (vertical, horizontal) = LIST_ROW_PADDING_PX;
        let header = self.container(
            Style {
                padding: sides(vertical, horizontal, 2.0, horizontal),
                ..flex_row(AlignItems::START)
            },
            node,
            &pointer,
        )?;
        let columns = [
            ("Item", Some(PART_ITEM_COLUMN_PX)),
            ("Part \u{b7} what it is", Some(PART_NAME_COLUMN_PX)),
            ("Why it exists", None),
            ("Zones", Some(PART_ZONES_COLUMN_PX)),
        ];
        for (index, (text, width)) in columns.into_iter().enumerate() {
            let leaf_style = match width {
                Some(width) => Style {
                    size: taffy::Size {
                        width: length(width),
                        height: taffy::prelude::auto(),
                    },
                    ..base_style()
                },
                None => Style {
                    flex_grow: 1.0,
                    flex_basis: length(0.0),
                    margin: margins(0.0, 0.0, 0.0, 1.0 + PART_WHY_GAP_PX),
                    ..base_style()
                },
            };
            let (leaf, leaf_index) = self.text_leaf(
                leaf_style,
                Some(header),
                TextSpec {
                    text,
                    style_name: TextStyleName::SheetLabel,
                    line: None,
                    align: TextAlign::Start,
                    source: pointer.clone(),
                },
            )?;
            debug_assert!(index < 4);
            parts.push(PartRecord {
                name: PartName::HeaderCell,
                taffy_node: leaf,
                text_leaf: Some(leaf_index),
            });
        }
        let parts_pointer = SheetList::Parts.pointer_in(&pointer);
        for (index, part) in list
            .parts
            .iter()
            .enumerate()
            .take(SheetList::Parts.max() + 1)
        {
            let part_pointer = parts_pointer.index(index);
            let rule = self.list_rule(node, &part_pointer)?;
            parts.push(PartRecord {
                name: PartName::RowRule,
                taffy_node: rule,
                text_leaf: None,
            });
            let row = self.container(
                Style {
                    padding: sides(vertical, horizontal, vertical, horizontal),
                    ..flex_row(AlignItems::STRETCH)
                },
                node,
                &part_pointer,
            )?;
            let item_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(PART_ITEM_COLUMN_PX),
                        height: taffy::prelude::auto(),
                    },
                    ..flex_column(AlignItems::START)
                },
                row,
                &part_pointer,
            )?;
            parts.extend(self.add_glyph(
                item_cell,
                (PartName::PartBalloon, PartName::PartNumeral),
                (MarkKind::Part, index + 1),
                flex_column(AlignItems::CENTER),
                &part_pointer,
            )?);
            let name_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(PART_NAME_COLUMN_PX),
                        height: taffy::prelude::auto(),
                    },
                    ..flex_column(AlignItems::START)
                },
                row,
                &part_pointer,
            )?;
            let name = self.page_marks.part_name(part);
            let name_source = match part.node {
                Some(_) => part_pointer.child("node"),
                None => part_pointer.child("name"),
            };
            parts.push(self.list_text(
                name_cell,
                PartName::PartTitle,
                (&name, TextStyleName::LeadBody),
                0.0,
                name_source.clone(),
            )?);
            if let Some(what) = self.page_marks.part_what(part) {
                let what_source = match part.node {
                    Some(_) => name_source.clone(),
                    None => part_pointer.child("what"),
                };
                parts.push(self.list_text(
                    name_cell,
                    PartName::PartWhat,
                    (&what, TextStyleName::StepText),
                    0.0,
                    what_source,
                )?);
            }
            if self.page_marks.part_planned(part) {
                parts.push(self.list_text(
                    name_cell,
                    PartName::PartPlanned,
                    ("planned", TextStyleName::SheetLabel),
                    PART_PLANNED_GAP_PX,
                    part_pointer.child("planned"),
                )?);
            }
            let why_rule = self.plain_leaf(
                Style {
                    size: taffy::Size {
                        width: length(1.0),
                        height: taffy::prelude::auto(),
                    },
                    align_self: Some(AlignSelf::STRETCH),
                    ..base_style()
                },
                row,
                &part_pointer,
            )?;
            parts.push(PartRecord {
                name: PartName::PartWhyRule,
                taffy_node: why_rule,
                text_leaf: None,
            });
            let why_cell = self.container(
                Style {
                    flex_grow: 1.0,
                    flex_basis: length(0.0),
                    padding: sides(0.0, PART_WHY_GAP_PX, 0.0, PART_WHY_GAP_PX),
                    ..flex_column(AlignItems::STRETCH)
                },
                row,
                &part_pointer,
            )?;
            parts.push(self.list_text(
                why_cell,
                PartName::PartWhy,
                (&part.why, TextStyleName::StepText),
                0.0,
                part_pointer.child("why"),
            )?);
            let zones_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(PART_ZONES_COLUMN_PX),
                        height: taffy::prelude::auto(),
                    },
                    ..flex_column(AlignItems::STRETCH)
                },
                row,
                &part_pointer,
            )?;
            parts.push(self.where_used_part(
                zones_cell,
                PartName::PartZones,
                (MarkKind::Part, index, ""),
                base_style(),
                part_pointer.clone(),
            )?);
        }
        let mut record = Self::record(pointer, NodeTag::PartsList, None, Some(parent_index), node);
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// The general notes (section 14.4 rule 18): a title row, then per note its flag, its
    /// text, its detail and its zones, then the foot under a dashed rule.
    pub(super) fn add_general_notes(
        &mut self,
        notes: &GeneralNotes,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let (top, side, bottom) = NOTES_PADDING_PX;
        let mut style = Style {
            border: sides(
                NOTES_BORDER_PX,
                NOTES_BORDER_PX,
                NOTES_BORDER_PX,
                NOTES_BORDER_PX,
            ),
            padding: sides(top, side, bottom, side),
            gap: taffy::Size {
                width: length(0.0),
                height: length(NOTE_GAP_PX),
            },
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let node = self.container(style, parent_container, &pointer)?;
        let title_row = self.container(
            Style {
                gap: taffy::Size {
                    width: length(NOTES_TITLE_GAP_PX),
                    height: length(0.0),
                },
                ..flex_row(AlignItems::CENTER)
            },
            node,
            &pointer,
        )?;
        let mut parts = vec![self.list_text(
            title_row,
            PartName::Title,
            (&notes.title, TextStyleName::ListTitle),
            0.0,
            pointer.child("title"),
        )?];
        if let Some(note) = &notes.note {
            parts.push(self.list_text(
                title_row,
                PartName::Note,
                (note, TextStyleName::ViewNoteSmall),
                0.0,
                pointer.child("note"),
            )?);
        }
        let notes_pointer = SheetList::GeneralNotes.pointer_in(&pointer);
        for (index, note) in notes
            .notes
            .iter()
            .enumerate()
            .take(SheetList::GeneralNotes.max() + 1)
        {
            let note_pointer = notes_pointer.index(index);
            let row = self.container(flex_row(AlignItems::START), node, &note_pointer)?;
            let flag_cell = self.container(
                Style {
                    size: taffy::Size {
                        width: length(NOTE_FLAG_COLUMN_PX),
                        height: taffy::prelude::auto(),
                    },
                    ..flex_column(AlignItems::START)
                },
                row,
                &note_pointer,
            )?;
            parts.extend(self.add_glyph(
                flag_cell,
                (PartName::NoteFlag, PartName::NoteNumeral),
                (MarkKind::Note, index + 1),
                flex_column(AlignItems::CENTER),
                &note_pointer,
            )?);
            let column = self.container(
                Style {
                    flex_grow: 1.0,
                    flex_basis: taffy::prelude::auto(),
                    flex_shrink: 1.0,
                    ..flex_column(AlignItems::STRETCH)
                },
                row,
                &note_pointer,
            )?;
            parts.push(self.list_text(
                column,
                PartName::NoteText,
                (&note.text, TextStyleName::BlockBody),
                0.0,
                note_pointer.child("text"),
            )?);
            if let Some(detail) = &note.detail {
                parts.push(self.list_text(
                    column,
                    PartName::NoteDetail,
                    (detail, TextStyleName::StepText),
                    NOTE_LINE_GAP_PX,
                    note_pointer.child("detail"),
                )?);
            }
            parts.push(self.where_used_part(
                column,
                PartName::NoteZones,
                (MarkKind::Note, index, WHERE_USED_PREFIX),
                Style {
                    margin: margins(NOTE_LINE_GAP_PX, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                note_pointer.clone(),
            )?);
        }
        if let Some(foot) = &notes.foot {
            let rule = self.plain_leaf(
                Style {
                    size: taffy::Size {
                        width: taffy::prelude::auto(),
                        height: length(LIST_RULE_PX),
                    },
                    margin: margins(NOTE_FOOT_GAP_PX - NOTE_GAP_PX, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                node,
                &pointer,
            )?;
            parts.push(PartRecord {
                name: PartName::FootRule,
                taffy_node: rule,
                text_leaf: None,
            });
            parts.push(self.list_text(
                node,
                PartName::Foot,
                (foot, TextStyleName::ViewNoteSmall),
                0.0,
                pointer.child("foot"),
            )?);
        }
        let mut record = Self::record(
            pointer,
            NodeTag::GeneralNotes,
            None,
            Some(parent_index),
            node,
        );
        record.parts = parts;
        self.push_record(record);
        Ok(())
    }

    /// The title row of a keyed list: the title in `list_title` and the note beside it.
    fn list_title_row(
        &mut self,
        node: NodeId,
        title: &str,
        note: Option<&str>,
        justify: JustifyContent,
        pointer: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let (vertical, horizontal) = LIST_TITLE_PADDING_PX;
        let row = self.container(
            Style {
                padding: sides(vertical, horizontal, vertical, horizontal),
                gap: taffy::Size {
                    width: length(NOTES_TITLE_GAP_PX),
                    height: length(0.0),
                },
                justify_content: Some(justify),
                ..flex_row(AlignItems::CENTER)
            },
            node,
            pointer,
        )?;
        let mut parts = vec![self.list_text(
            row,
            PartName::Title,
            (title, TextStyleName::ListTitle),
            0.0,
            pointer.child("title"),
        )?];
        if let Some(note) = note {
            let (leaf, leaf_index) = self.text_leaf(
                Style {
                    flex_shrink: 1.0,
                    ..base_style()
                },
                Some(row),
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

    fn list_rule(&mut self, node: NodeId, pointer: &NodePointer) -> Result<NodeId, LayoutError> {
        self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: taffy::prelude::auto(),
                    height: length(LIST_RULE_PX),
                },
                ..base_style()
            },
            node,
            pointer,
        )
    }

    /// A text part of a keyed list with a top margin.
    fn list_text(
        &mut self,
        parent: NodeId,
        name: PartName,
        (text, style_name): (&str, TextStyleName),
        top: f32,
        source: NodePointer,
    ) -> Result<PartRecord, LayoutError> {
        let (leaf, leaf_index) = self.text_leaf(
            Style {
                margin: margins(top, 0.0, 0.0, 0.0),
                ..base_style()
            },
            Some(parent),
            TextSpec {
                text,
                style_name,
                line: None,
                align: TextAlign::Start,
                source,
            },
        )?;
        Ok(PartRecord {
            name,
            taffy_node: leaf,
            text_leaf: Some(leaf_index),
        })
    }
}
