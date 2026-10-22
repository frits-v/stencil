//! The flat outlines of an Item, the assembly tab, rails and step badges on a design sheet
//! (section 14.4 rules 23 and 25 to 27).

use stencil_model::marks::{CarrierMark, MarkPlace};
use stencil_model::pointer::NodePointer;
use stencil_model::text::TextStyleName;
use stencil_model::{Item, MarkKind, Outline, Pipe, PipeForm, SheetList, line_key, line_tint};
use taffy::prelude::{
    AlignItems, AlignSelf, JustifyContent, LengthPercentageAuto, NodeId, Position, Rect, Style,
    auto, length,
};

use super::{
    Builder, NodeRecord, PartRecord, Placement, TextSpec, apply_flex_placement, base_style,
    fixed_size, flex_column, flex_row, margins, sides,
};
use crate::{LayoutError, NodeTag, PartName, TextAlign};

/// The half height of a store's lid ellipse; the lid and the bottom arc each take this
/// much of the outline (section 14.4 rule 25).
pub const STORE_LID_RY_PX: f32 = 10.0;
/// A store's padding: under the lid, at each side, and over the bottom arc.
const STORE_PADDING_PX: (f32, f32, f32) =
    (2.0 * STORE_LID_RY_PX + 6.0, 14.0, STORE_LID_RY_PX + 8.0);
/// The compartments of a store: a row is this tall, rows and boxes this far apart, at most
/// this many boxes a row, and each box pads its text this much at each side.
const COMPARTMENT_HEIGHT_PX: f32 = 26.0;
const COMPARTMENT_GAP_PX: f32 = 5.0;
const COMPARTMENTS_PER_ROW: usize = 3;
const COMPARTMENT_PADDING_PX: f32 = 8.0;
const COMPARTMENTS_TOP_PX: f32 = 8.0;
/// A person outline: the head's diameter, the shoulders' width and height, the gap between
/// them and the gap to the title, and the padding at each side of the text.
const PERSON_HEAD_PX: f32 = 30.0;
const PERSON_SHOULDERS_PX: (f32, f32) = (60.0, 33.0);
const PERSON_NECK_PX: f32 = 7.0;
const PERSON_TEXT_GAP_PX: f32 = 5.0;
const PERSON_PADDING_PX: f32 = 9.5;
/// A stack outline reserves this much right and above its front for the two cards behind
/// it, each offset half of it.
pub const STACK_OFFSET_PX: (f32, f32) = (8.0, 6.0);
/// An Item card's border and padding, as `add_item` lays a card out.
const CARD_BORDER_PX: f32 = 1.5;
const CARD_PADDING_PX: (f32, f32) = (6.0, 10.0);
/// The assembly tab: the drawn border starts this far under the Box's top, the tab is this
/// far in from the left and pads its text this much across and down.
pub const TAB_DROP_PX: f32 = 8.0;
const TAB_INSET_PX: f32 = 10.0;
const TAB_PADDING_PX: (f32, f32) = (1.4, 6.0);
/// A step badge: a square 18 across with a radius of 3, 6 before the label (section 14.4
/// rule 23).
pub const STEP_BADGE_PX: f32 = 18.0;
pub const STEP_BADGE_RADIUS_PX: f32 = 3.0;
pub const STEP_BADGE_GAP_PX: f32 = 6.0;
/// A rail's column: the strip the bar stands in, the gap to its tag, and the shortest rail.
pub const RAIL_STRIP_PX: f32 = 12.0;
const RAIL_TAG_GAP_PX: f32 = 6.0;
const RAIL_MIN_HEIGHT_PX: f32 = 80.0;

/// A pipe or link tag's padding and border, as `add_pipe` lays a tag out.
const TAG_PADDING_PX: (f32, f32) = (6.0, 8.0);
const TAG_BORDER_PX: f32 = 1.5;

/// How many compartments each row of a store holds: rows of at most three, the earlier rows
/// taking the extra (4 is 2 + 2, 5 is 3 + 2).
pub fn compartment_rows(count: usize) -> Vec<usize> {
    if count == 0 {
        return Vec::new();
    }
    let rows = count.div_ceil(COMPARTMENTS_PER_ROW);
    (0..rows)
        .map(|row| count / rows + usize::from(row < count % rows))
        .collect()
}

/// An absolutely placed child at these insets from its parent's padding box.
fn inset_style(top: f32, right: f32, bottom: f32, left: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            left: LengthPercentageAuto::length(left),
            right: LengthPercentageAuto::length(right),
            top: LengthPercentageAuto::length(top),
            bottom: LengthPercentageAuto::length(bottom),
        },
        ..base_style()
    }
}

impl Builder<'_> {
    /// A text leaf that never wraps: its min-content width is its whole run.
    fn unwrapped_text_leaf(
        &mut self,
        style: Style,
        parent: NodeId,
        spec: TextSpec<'_>,
    ) -> Result<(NodeId, usize), LayoutError> {
        let (leaf, index) = self.text_leaf(style, Some(parent), spec)?;
        if let Some(text_leaf) = self.text_leaves.get_mut(index) {
            text_leaf.min_line_px = Some(f32::INFINITY);
        }
        Ok((leaf, index))
    }

    /// An Item drawn as a store, a person or a stack (section 14.4 rule 25).
    pub(super) fn add_outlined_item(
        &mut self,
        item: &Item,
        pointer: NodePointer,
        (parent_index, parent_container): (usize, NodeId),
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let marks = std::mem::take(&mut self.carrier_marks);
        let mut parts = Vec::new();
        let mut entry_glyphs = Vec::new();
        let (card, glyphs) = match item.outline {
            Outline::Store => self.add_store(
                item,
                &marks,
                &pointer,
                (parent_container, placement),
                (&mut parts, &mut entry_glyphs),
            )?,
            Outline::Person => self.add_person(
                item,
                &marks,
                &pointer,
                (parent_container, placement),
                &mut parts,
            )?,
            // `add_item` lays a card out itself and never sends one here.
            Outline::Stack | Outline::Card => self.add_stack(
                item,
                &marks,
                &pointer,
                (parent_container, placement),
                &mut parts,
            )?,
        };
        // Glyphs follow in drawn order: the title row's, the corner's, the compartments'.
        parts.extend(glyphs);
        parts.extend(self.add_corner_mark((card, 0.0), &marks, &pointer)?);
        parts.extend(entry_glyphs);
        let mut record = Self::record(pointer, NodeTag::Pcard, None, Some(parent_index), card);
        record.tint = self.actors.slot(item.actor.as_deref());
        record.shape = Some(stencil_model::products::item_shape(
            self.grammar.item(&item.kind),
            item,
        ));
        record.parts = parts;
        record.outline = Some(item.outline);
        self.push_record(record);
        Ok(())
    }

    /// The icon, title, subtitle and title-row glyphs of an outlined Item in `column`.
    fn add_item_text(
        &mut self,
        item: &Item,
        marks: &[CarrierMark],
        (row_parent, align): (NodeId, AlignItems),
        pointer: &NodePointer,
        parts: &mut Vec<PartRecord>,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let card_row = self.container(
            Style {
                gap: taffy::Size {
                    width: length(10.0),
                    height: length(0.0),
                },
                justify_content: Some(JustifyContent::CENTER),
                ..flex_row(AlignItems::CENTER)
            },
            row_parent,
            pointer,
        )?;
        if item.icon.is_some() {
            let icon = self.plain_leaf(
                Style {
                    size: fixed_size(28.0, 28.0),
                    ..base_style()
                },
                card_row,
                pointer,
            )?;
            parts.push(PartRecord {
                name: PartName::Icon,
                taffy_node: icon,
                text_leaf: None,
            });
        }
        let column = self.container(flex_column(align), card_row, pointer)?;
        parts.push(PartRecord {
            name: PartName::Text,
            taffy_node: column,
            text_leaf: None,
        });
        let title_row = self.marked_text_row(column, marks, MarkPlace::Trailing, pointer)?;
        let (function_name, function_leaf) = self.text_leaf(
            match title_row {
                Some(_) => Self::marked_text_style(base_style()),
                None => base_style(),
            },
            Some(title_row.unwrap_or(column)),
            TextSpec {
                text: &item.title,
                style_name: TextStyleName::CardFunction,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("title"),
            },
        )?;
        parts.push(PartRecord {
            name: PartName::FunctionName,
            taffy_node: function_name,
            text_leaf: Some(function_leaf),
        });
        let glyphs = match title_row {
            Some(row) => self.add_mark_row(row, marks, MarkPlace::Trailing, pointer)?,
            None => Vec::new(),
        };
        if let Some(subtitle) = &item.subtitle {
            let (product, product_leaf) = self.text_leaf(
                Style {
                    margin: margins(1.0, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                Some(column),
                TextSpec {
                    text: subtitle,
                    style_name: TextStyleName::CardProduct,
                    line: None,
                    align: TextAlign::Start,
                    source: pointer.child("subtitle"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::ProductName,
                taffy_node: product,
                text_leaf: Some(product_leaf),
            });
        }
        Ok(glyphs)
    }

    /// A store: a cylinder whose front is the node box, the card's text under the lid and
    /// the compartments in balanced rows under it.
    fn add_store(
        &mut self,
        item: &Item,
        marks: &[CarrierMark],
        pointer: &NodePointer,
        (parent_container, placement): (NodeId, Placement),
        (parts, entry_glyphs): (&mut Vec<PartRecord>, &mut Vec<PartRecord>),
    ) -> Result<(NodeId, Vec<PartRecord>), LayoutError> {
        let (top, side, bottom) = STORE_PADDING_PX;
        let mut style = Style {
            padding: sides(top, side, bottom, side),
            ..flex_column(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let store = self.container(style, parent_container, pointer)?;
        let front = self.plain_leaf(inset_style(0.0, 0.0, 0.0, 0.0), store, pointer)?;
        parts.push(PartRecord {
            name: PartName::Front,
            taffy_node: front,
            text_leaf: None,
        });
        let glyphs =
            self.add_item_text(item, marks, (store, AlignItems::CENTER), pointer, parts)?;
        let compartments: Vec<_> = item
            .compartments
            .iter()
            .take(SheetList::Compartments.max())
            .collect();
        if !compartments.is_empty() {
            let block = self.container(
                Style {
                    gap: taffy::Size {
                        width: length(0.0),
                        height: length(COMPARTMENT_GAP_PX),
                    },
                    margin: margins(COMPARTMENTS_TOP_PX, 0.0, 0.0, 0.0),
                    ..flex_column(AlignItems::STRETCH)
                },
                store,
                pointer,
            )?;
            let compartments_pointer = pointer.child("compartments");
            let mut index = 0;
            for count in compartment_rows(compartments.len()) {
                let row = self.container(
                    Style {
                        gap: taffy::Size {
                            width: length(COMPARTMENT_GAP_PX),
                            height: length(0.0),
                        },
                        ..flex_row(AlignItems::STRETCH)
                    },
                    block,
                    pointer,
                )?;
                for _ in 0..count {
                    let Some(compartment) = compartments.get(index) else {
                        break;
                    };
                    let entry_pointer = compartments_pointer.index(index);
                    let boxed = self.container(
                        Style {
                            flex_grow: 1.0,
                            flex_basis: length(0.0),
                            size: taffy::Size {
                                width: auto(),
                                height: length(COMPARTMENT_HEIGHT_PX),
                            },
                            padding: sides(
                                0.0,
                                COMPARTMENT_PADDING_PX,
                                0.0,
                                COMPARTMENT_PADDING_PX,
                            ),
                            ..flex_row(AlignItems::CENTER)
                        },
                        row,
                        pointer,
                    )?;
                    parts.push(PartRecord {
                        name: PartName::Compartment,
                        taffy_node: boxed,
                        text_leaf: None,
                    });
                    let text_source = if compartment.is_marked() {
                        entry_pointer.child("text")
                    } else {
                        entry_pointer.clone()
                    };
                    let (text, text_leaf) = self.unwrapped_text_leaf(
                        Style {
                            flex_grow: 1.0,
                            ..base_style()
                        },
                        boxed,
                        TextSpec {
                            text: compartment.text(),
                            style_name: TextStyleName::SheetLabel,
                            line: None,
                            align: TextAlign::Start,
                            source: text_source,
                        },
                    )?;
                    parts.push(PartRecord {
                        name: PartName::CompartmentText,
                        taffy_node: text,
                        text_leaf: Some(text_leaf),
                    });
                    entry_glyphs.extend(self.add_mark_row(
                        boxed,
                        marks,
                        MarkPlace::Compartment(index),
                        pointer,
                    )?);
                    index += 1;
                }
            }
        }
        Ok((store, glyphs))
    }

    /// A person: a head and shoulders with the title and subtitle centred under them.
    fn add_person(
        &mut self,
        item: &Item,
        marks: &[CarrierMark],
        pointer: &NodePointer,
        (parent_container, placement): (NodeId, Placement),
        parts: &mut Vec<PartRecord>,
    ) -> Result<(NodeId, Vec<PartRecord>), LayoutError> {
        let mut style = Style {
            padding: sides(0.0, PERSON_PADDING_PX, 0.0, PERSON_PADDING_PX),
            ..flex_column(AlignItems::CENTER)
        };
        apply_flex_placement(&mut style, placement);
        let person = self.container(style, parent_container, pointer)?;
        let head = self.plain_leaf(
            Style {
                size: fixed_size(PERSON_HEAD_PX, PERSON_HEAD_PX),
                margin: margins(1.0, 0.0, 0.0, 0.0),
                ..base_style()
            },
            person,
            pointer,
        )?;
        let shoulders = self.plain_leaf(
            Style {
                size: fixed_size(PERSON_SHOULDERS_PX.0, PERSON_SHOULDERS_PX.1),
                margin: margins(PERSON_NECK_PX, 0.0, PERSON_TEXT_GAP_PX, 0.0),
                ..base_style()
            },
            person,
            pointer,
        )?;
        parts.push(PartRecord {
            name: PartName::Head,
            taffy_node: head,
            text_leaf: None,
        });
        parts.push(PartRecord {
            name: PartName::Shoulders,
            taffy_node: shoulders,
            text_leaf: None,
        });
        let glyphs =
            self.add_item_text(item, marks, (person, AlignItems::CENTER), pointer, parts)?;
        Ok((person, glyphs))
    }

    /// A stack: the card of section 2.5 drawn as its front, with two cards behind it offset
    /// up and right inside the node's box.
    fn add_stack(
        &mut self,
        item: &Item,
        marks: &[CarrierMark],
        pointer: &NodePointer,
        (parent_container, placement): (NodeId, Placement),
        parts: &mut Vec<PartRecord>,
    ) -> Result<(NodeId, Vec<PartRecord>), LayoutError> {
        let (offset_x, offset_y) = STACK_OFFSET_PX;
        let (pad_y, pad_x) = CARD_PADDING_PX;
        let mut style = Style {
            gap: taffy::Size {
                width: length(10.0),
                height: length(0.0),
            },
            padding: sides(
                offset_y + CARD_BORDER_PX + pad_y,
                offset_x + CARD_BORDER_PX + pad_x,
                CARD_BORDER_PX + pad_y,
                CARD_BORDER_PX + pad_x,
            ),
            min_size: taffy::Size {
                width: auto(),
                height: length(44.0 + offset_y),
            },
            ..flex_row(AlignItems::CENTER)
        };
        apply_flex_placement(&mut style, placement);
        let stack = self.container(style, parent_container, pointer)?;
        for (name, insets) in [
            (PartName::StackBack, (0.0, 0.0, offset_y, offset_x)),
            (
                PartName::StackBack,
                (
                    offset_y / 2.0,
                    offset_x / 2.0,
                    offset_y / 2.0,
                    offset_x / 2.0,
                ),
            ),
            (PartName::Front, (offset_y, offset_x, 0.0, 0.0)),
        ] {
            let (top, right, bottom, left) = insets;
            let node = self.plain_leaf(inset_style(top, right, bottom, left), stack, pointer)?;
            parts.push(PartRecord {
                name,
                taffy_node: node,
                text_leaf: None,
            });
        }
        let glyphs =
            self.add_item_text(item, marks, (stack, AlignItems::STRETCH), pointer, parts)?;
        Ok((stack, glyphs))
    }

    /// The tab of an assembly Box (section 14.4 rule 26): a box 10 in from the left
    /// straddling the drawn border, which starts `TAB_DROP_PX` under the Box's top, holding
    /// the label and its glyphs. Returns the parts in drawn order: the border, the tab, the
    /// label and the glyphs.
    pub(super) fn add_tab(
        &mut self,
        (zone_node, padding): (NodeId, f32),
        label: &str,
        marks: &[CarrierMark],
        pointer: &NodePointer,
    ) -> Result<Vec<PartRecord>, LayoutError> {
        let border =
            self.plain_leaf(inset_style(TAB_DROP_PX, 0.0, 0.0, 0.0), zone_node, pointer)?;
        // The tab stands in the Box's flow, so the Box is at least as wide as its name; its
        // top is the Box's top and the outline starts `TAB_DROP_PX` under it.
        let tab = self.container(
            Style {
                align_self: Some(AlignSelf::START),
                margin: margins(0.0, 0.0, 0.0, TAB_INSET_PX - padding),
                padding: sides(
                    TAB_PADDING_PX.0,
                    TAB_PADDING_PX.1,
                    TAB_PADDING_PX.0,
                    TAB_PADDING_PX.1,
                ),
                ..flex_row(AlignItems::CENTER)
            },
            zone_node,
            pointer,
        )?;
        let (label_node, label_leaf) = self.unwrapped_text_leaf(
            base_style(),
            tab,
            TextSpec {
                text: label,
                style_name: TextStyleName::SheetLabel,
                line: None,
                align: TextAlign::Start,
                source: pointer.child("label"),
            },
        )?;
        let mut parts = vec![
            PartRecord {
                name: PartName::TabBorder,
                taffy_node: border,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::Tab,
                taffy_node: tab,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::Label,
                taffy_node: label_node,
                text_leaf: Some(label_leaf),
            },
        ];
        parts.extend(self.add_mark_row(tab, marks, MarkPlace::Trailing, pointer)?);
        Ok(parts)
    }

    /// The padding a tab Box keeps above its tab: none, since the tab's top is the Box's.
    pub(super) fn tab_padding_top(_padding: f32) -> f32 {
        0.0
    }

    /// A step badge at the left of a tag: an 18 px square holding its numeral.
    pub(super) fn add_step_badge(
        &mut self,
        parent: NodeId,
        step: u16,
        source: &NodePointer,
    ) -> Result<[PartRecord; 2], LayoutError> {
        let badge = self.container(
            Style {
                size: fixed_size(STEP_BADGE_PX, STEP_BADGE_PX),
                margin: margins(0.0, STEP_BADGE_GAP_PX, 0.0, 0.0),
                justify_content: Some(JustifyContent::CENTER),
                ..flex_column(AlignItems::CENTER)
            },
            parent,
            source,
        )?;
        let numeral_text = step.to_string();
        let (numeral, numeral_leaf) = self.text_leaf(
            base_style(),
            Some(badge),
            TextSpec {
                text: &numeral_text,
                style_name: crate::marks::numeral_style(MarkKind::Part),
                line: None,
                align: TextAlign::Center,
                source: source.child("step"),
            },
        )?;
        Ok([
            PartRecord {
                name: PartName::StepBadge,
                taffy_node: badge,
                text_leaf: None,
            },
            PartRecord {
                name: PartName::StepNumeral,
                taffy_node: numeral,
                text_leaf: Some(numeral_leaf),
            },
        ])
    }

    /// A rail (section 14.4 rule 27): a column of a Row as tall as the Row, its tag at the
    /// bottom and the bar in a strip at its right, which links attach to.
    pub(super) fn add_rail(
        &mut self,
        pipe: &Pipe,
        pointer: NodePointer,
        parent_index: usize,
        parent_container: NodeId,
        placement: Placement,
    ) -> Result<(), LayoutError> {
        let marks = std::mem::take(&mut self.carrier_marks);
        let mut style = Style {
            min_size: taffy::Size {
                width: auto(),
                height: length(RAIL_MIN_HEIGHT_PX),
            },
            align_self: Some(AlignSelf::STRETCH),
            ..flex_row(AlignItems::STRETCH)
        };
        apply_flex_placement(&mut style, placement);
        let rail_node = self.container(style, parent_container, &pointer)?;
        let tag_column = self.container(
            Style {
                justify_content: Some(JustifyContent::END),
                margin: margins(0.0, RAIL_TAG_GAP_PX, 0.0, 0.0),
                ..flex_column(AlignItems::END)
            },
            rail_node,
            &pointer,
        )?;
        let tag = self.container(
            Style {
                padding: sides(
                    TAG_PADDING_PX.0,
                    TAG_PADDING_PX.1,
                    TAG_PADDING_PX.0,
                    TAG_PADDING_PX.1,
                ),
                border: sides(TAG_BORDER_PX, TAG_BORDER_PX, TAG_BORDER_PX, TAG_BORDER_PX),
                ..flex_row(AlignItems::CENTER)
            },
            tag_column,
            &pointer,
        )?;
        let mut parts = Vec::new();
        let mut badge = Vec::new();
        if let Some(step) = pipe.step {
            badge.extend(self.add_step_badge(tag, step, &pointer)?);
        }
        let text_column = self.container(flex_column(AlignItems::END), tag, &pointer)?;
        let (label, label_leaf) = self.unwrapped_text_leaf(
            base_style(),
            text_column,
            TextSpec {
                text: &pipe.label,
                style_name: TextStyleName::TagLabel,
                line: Some(pipe.line),
                align: TextAlign::Center,
                source: pointer.child("label"),
            },
        )?;
        let strip = self.plain_leaf(
            Style {
                size: taffy::Size {
                    width: length(RAIL_STRIP_PX),
                    height: auto(),
                },
                ..base_style()
            },
            rail_node,
            &pointer,
        )?;
        parts.push(PartRecord {
            name: PartName::Rail,
            taffy_node: strip,
            text_leaf: None,
        });
        parts.push(PartRecord {
            name: PartName::Tag,
            taffy_node: tag,
            text_leaf: None,
        });
        parts.extend(badge);
        parts.push(PartRecord {
            name: PartName::TagLabel,
            taffy_node: label,
            text_leaf: Some(label_leaf),
        });
        if let Some(sub) = &pipe.sub {
            let (sub_node, sub_leaf) = self.unwrapped_text_leaf(
                Style {
                    margin: margins(2.0, 0.0, 0.0, 0.0),
                    ..base_style()
                },
                text_column,
                TextSpec {
                    text: sub,
                    style_name: TextStyleName::TagSub,
                    line: Some(pipe.line),
                    align: TextAlign::Center,
                    source: pointer.child("sub"),
                },
            )?;
            parts.push(PartRecord {
                name: PartName::TagSub,
                taffy_node: sub_node,
                text_leaf: Some(sub_leaf),
            });
        }
        parts.extend(self.add_mark_row(tag, &marks, MarkPlace::Trailing, &pointer)?);
        let tint = self.actors.slot(self.actors.pipe_actor(pipe)).or(pipe.tint);
        let mut record: NodeRecord = Self::record(
            pointer,
            NodeTag::Pipe,
            Some(line_key(pipe.line, tint).to_string()),
            Some(parent_index),
            rail_node,
        );
        record.tint = line_tint(pipe.line, tint);
        record.parts = parts;
        record.pipe_form = Some(PipeForm::Rail);
        self.push_record(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::compartment_rows;

    #[test]
    fn compartments_fill_rows_of_three_with_the_extra_in_the_earlier_rows() {
        assert_eq!(compartment_rows(0), Vec::<usize>::new());
        assert_eq!(compartment_rows(1), vec![1]);
        assert_eq!(compartment_rows(3), vec![3]);
        assert_eq!(compartment_rows(4), vec![2, 2]);
        assert_eq!(compartment_rows(5), vec![3, 2]);
        assert_eq!(compartment_rows(6), vec![3, 3]);
    }
}
