//! The sizes of mark glyphs (section 14.4 rule 22), shared by the builder, the Phases
//! plan, link tags and `marks-clear`, and the where-used text filled in after layout
//! (rule 19).

use stencil_model::marks::{CarrierMark, PageMarks};
use stencil_model::text::{TextMeasurer, TextStyleName, WRAP_EPSILON_PX};
use stencil_model::{MarkKind, Page, SHEET_COLUMNS, SHEET_ROW_LETTERS};

use crate::styles::{text_color, text_style_for};
use crate::units::{DescriptionUnits, description_units, where_used};
use crate::{BoxRect, Part, TextAlign, TextRun};
use crate::{LayoutError, NodeTag, PageGeometry, PartName};

/// A balloon is a circle 18 across.
pub const BALLOON_PX: f32 = 18.0;
/// A flag numbered 1 to 9 is a triangle 20 wide and 17 tall, apex up.
pub const FLAG_PX: (f32, f32) = (20.0, 17.0);
/// A flag numbered from 10 is 24 wide and 21 tall, so two digits stay inside it.
pub const WIDE_FLAG_PX: (f32, f32) = (24.0, 21.0);
/// A diamond is a square 20 across, stood on its point.
pub const DIAMOND_PX: f32 = 20.0;
/// Between two glyphs of one carrier, and between a carrier's text and its first glyph.
pub const MARK_GAP_PX: f32 = 4.0;
/// A flag's numeral sits this far above the flag's base.
pub const FLAG_NUMERAL_LIFT_PX: f32 = 1.5;
/// A Step line that carries marks is at least this tall, room for a glyph.
pub const MARKED_LINE_PX: f32 = 22.0;
/// The line before a sheet's where-used cells under a note or an attention item.
pub const WHERE_USED_PREFIX: &str = "marked at ";
/// The where-used text of a key that no glyph names.
pub const WHERE_USED_NONE: &str = "none";

/// The width and height of the glyph a mark of `kind` numbered `number` draws.
pub fn glyph_size(kind: MarkKind, number: usize) -> (f32, f32) {
    match kind {
        MarkKind::Part => (BALLOON_PX, BALLOON_PX),
        MarkKind::Note if number >= 10 => WIDE_FLAG_PX,
        MarkKind::Note => FLAG_PX,
        MarkKind::Attention => (DIAMOND_PX, DIAMOND_PX),
    }
}

/// The style a glyph's numeral is set in.
pub fn numeral_style(kind: MarkKind) -> TextStyleName {
    match kind {
        MarkKind::Part => TextStyleName::MarkNumeral,
        MarkKind::Note => TextStyleName::FlagNumeral,
        MarkKind::Attention => TextStyleName::MarkNumeralSmall,
    }
}

/// The part name of a carrier glyph of `kind`.
pub fn glyph_part(kind: MarkKind) -> PartName {
    match kind {
        MarkKind::Part => PartName::Balloon,
        MarkKind::Note => PartName::Flag,
        MarkKind::Attention => PartName::Diamond,
    }
}

/// The kind of mark a glyph part draws, for carrier glyphs, list glyphs and samples read
/// by their shape.
pub fn glyph_kind(name: PartName) -> Option<MarkKind> {
    match name {
        PartName::Balloon | PartName::PartBalloon => Some(MarkKind::Part),
        PartName::Flag | PartName::NoteFlag => Some(MarkKind::Note),
        PartName::Diamond | PartName::AttentionDiamond => Some(MarkKind::Attention),
        _ => None,
    }
}

/// The kind and numeral of each mark, the numeral its key's position plus one. A mark vet
/// would refuse draws 0.
pub fn numbered(page_marks: &PageMarks<'_>, marks: &[&CarrierMark]) -> Vec<(MarkKind, usize)> {
    marks
        .iter()
        .map(|mark| (mark.mark.kind(), page_marks.number(&mark.mark).unwrap_or(0)))
        .collect()
}

/// The width a row of glyphs takes, gaps between them included; 0 for none.
pub fn glyphs_width(glyphs: &[(MarkKind, usize)]) -> f32 {
    let widths: f32 = glyphs
        .iter()
        .map(|(kind, number)| glyph_size(*kind, *number).0)
        .sum();
    let gaps = MARK_GAP_PX * glyphs.len().saturating_sub(1) as f32;
    widths + gaps
}

/// The height a row of glyphs takes; 0 for none.
pub fn glyphs_height(glyphs: &[(MarkKind, usize)]) -> f32 {
    glyphs
        .iter()
        .map(|(kind, number)| glyph_size(*kind, *number).1)
        .fold(0.0, f32::max)
}

/// The kinds the legend strip samples, in the order part, note, attention: each kind some
/// carrier on the page draws.
pub fn sampled_kinds(page: &Page) -> Vec<MarkKind> {
    let page_marks = PageMarks::of(page);
    let sites = page_marks.all(page);
    MarkKind::ALL
        .into_iter()
        .filter(|kind| sites.iter().any(|site| site.mark.kind() == *kind))
        .collect()
}

/// Every cell name a reference can print, `A1` to `F8`.
fn cell_names() -> Vec<String> {
    SHEET_ROW_LETTERS
        .iter()
        .flat_map(|letter| (1..=SHEET_COLUMNS).map(move |column| format!("{letter}{column}")))
        .collect()
}

/// The widest cell name in the where-used style, which layout reserves where-used text
/// with before the glyphs are placed.
pub(crate) fn widest_cell_name(measurer: &mut dyn TextMeasurer) -> Result<String, LayoutError> {
    let style = text_style_for(TextStyleName::StepText, false);
    let mut widest = (0.0_f32, String::new());
    for name in cell_names() {
        let width = measurer
            .measure(&name, &style, None)
            .map_err(|source| LayoutError::Measure {
                pointer: stencil_model::NodePointer::root().child("sheet"),
                source,
            })?
            .width_px;
        if width > widest.0 {
            widest = (width, name);
        }
    }
    Ok(widest.1)
}

/// The placeholder a where-used box is measured with: `sites` copies of the widest cell
/// name, at least one, after `prefix`.
pub(crate) fn where_used_placeholder(prefix: &str, widest: &str, sites: usize) -> String {
    let names = vec![widest; sites.max(1)];
    format!("{prefix}{}", names.join(" "))
}

/// The where-used text of a key: its distinct cells after `prefix`, or `none`.
pub fn where_used_text(prefix: &str, cells: &[crate::SheetZone]) -> String {
    if cells.is_empty() {
        format!("{prefix}{WHERE_USED_NONE}")
    } else {
        let names: Vec<String> = cells.iter().map(ToString::to_string).collect();
        format!("{prefix}{}", names.join(" "))
    }
}

/// The where-used part of each key list, its prefix, and the node that holds it.
pub const WHERE_USED_PARTS: [(MarkKind, PartName, NodeTag, &str); 3] = [
    (
        MarkKind::Attention,
        PartName::AttentionZones,
        NodeTag::TitleCell,
        WHERE_USED_PREFIX,
    ),
    (MarkKind::Part, PartName::PartZones, NodeTag::PartsList, ""),
    (
        MarkKind::Note,
        PartName::NoteZones,
        NodeTag::GeneralNotes,
        WHERE_USED_PREFIX,
    ),
];

/// Writes each where-used text from the placed glyphs and measures it into the box
/// reserved for it (section 14.4 rule 19). Runs after routing, since a link's tag carries
/// glyphs too; it moves nothing a glyph's position depends on.
pub(crate) fn fill_where_used(
    page: &Page,
    geometry: &mut PageGeometry,
    measurer: &mut dyn TextMeasurer,
) -> Result<(), LayoutError> {
    let units: DescriptionUnits = description_units(page, geometry);
    let page_marks = PageMarks::of(page);
    for (kind, part_name, owner_tag, prefix) in WHERE_USED_PARTS {
        let owner_pointer = match kind {
            MarkKind::Attention => Some(
                stencil_model::NodePointer::root()
                    .child("sheet")
                    .child("attention"),
            ),
            MarkKind::Part => page_marks.parts_pointer().cloned(),
            MarkKind::Note => page_marks.notes_pointer().cloned(),
        };
        let Some(owner_pointer) = owner_pointer else {
            continue;
        };
        let Some(owner) = geometry
            .nodes
            .iter_mut()
            .find(|node| node.pointer == owner_pointer && node.tag == owner_tag)
        else {
            continue;
        };
        for (position, part) in owner
            .parts
            .iter_mut()
            .filter(|part| part.name == part_name)
            .enumerate()
        {
            let Some(key) = page_marks.key_pointer(kind, position) else {
                break;
            };
            let Some(run) = part.text.as_mut() else {
                continue;
            };
            let text = where_used_text(prefix, &where_used(&units, &key));
            run.metrics = measurer
                .measure(&text, &run.style, Some(part.bounds.width + WRAP_EPSILON_PX))
                .map_err(|source| LayoutError::Measure {
                    pointer: key.clone(),
                    source,
                })?;
            run.text = text;
        }
    }
    Ok(())
}

/// A glyph and its numeral placed with the glyph's top-left corner at (`x`, `y`), as the
/// builder places them: the numeral centred across, in the middle of a balloon or a
/// diamond and `FLAG_NUMERAL_LIFT_PX` above a flag's base.
pub(crate) fn placed_glyph(
    (glyph_name, numeral_name): (PartName, PartName),
    (kind, number): (MarkKind, usize),
    (x, y): (f32, f32),
    measurer: &mut dyn TextMeasurer,
    source: &stencil_model::NodePointer,
) -> Result<[Part; 2], LayoutError> {
    let (width, height) = glyph_size(kind, number);
    let style_name = numeral_style(kind);
    let style = text_style_for(style_name, false);
    let text = number.to_string();
    let metrics = measurer
        .measure(&text, &style, None)
        .map_err(|error| LayoutError::Measure {
            pointer: source.clone(),
            source: error,
        })?;
    let numeral_y = match kind {
        MarkKind::Note => y + height - FLAG_NUMERAL_LIFT_PX - metrics.height_px,
        MarkKind::Part | MarkKind::Attention => y + (height - metrics.height_px) / 2.0,
    };
    let numeral = BoxRect {
        x: x + (width - metrics.width_px) / 2.0,
        y: numeral_y,
        width: metrics.width_px,
        height: metrics.height_px,
    };
    Ok([
        Part {
            name: glyph_name,
            bounds: BoxRect {
                x,
                y,
                width,
                height,
            },
            text: None,
        },
        Part {
            name: numeral_name,
            bounds: numeral,
            text: Some(TextRun {
                text,
                style,
                color: text_color(style_name, None),
                align: TextAlign::Center,
                metrics,
                first_line_indent_px: 0.0,
            }),
        },
    ])
}

/// Each where-used text the drawing prints that differs from the cells of its key's
/// glyphs: (key, drawn text, the text the glyphs give).
pub fn where_used_mismatches(
    page: &Page,
    geometry: &PageGeometry,
    units: &DescriptionUnits,
) -> Vec<(stencil_model::NodePointer, String, String)> {
    let page_marks = PageMarks::of(page);
    let mut mismatches = Vec::new();
    for (kind, part_name, owner_tag, prefix) in WHERE_USED_PARTS {
        let owner_pointer = match kind {
            MarkKind::Attention => Some(
                stencil_model::NodePointer::root()
                    .child("sheet")
                    .child("attention"),
            ),
            MarkKind::Part => page_marks.parts_pointer().cloned(),
            MarkKind::Note => page_marks.notes_pointer().cloned(),
        };
        let Some(owner) = owner_pointer.and_then(|pointer| {
            geometry
                .nodes
                .iter()
                .find(|node| node.pointer == pointer && node.tag == owner_tag)
        }) else {
            continue;
        };
        for (position, part) in owner
            .parts
            .iter()
            .filter(|part| part.name == part_name)
            .enumerate()
        {
            let Some(key) = page_marks.key_pointer(kind, position) else {
                break;
            };
            let expected = where_used_text(prefix, &where_used(units, &key));
            let drawn = part
                .text
                .as_ref()
                .map(|run| run.text.clone())
                .unwrap_or_default();
            if drawn != expected {
                mismatches.push((key, drawn, expected));
            }
        }
    }
    mismatches
}
