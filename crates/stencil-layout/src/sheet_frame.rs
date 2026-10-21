//! The frame of a design sheet (section 14.4 rule 3): a border 12 px in from the canvas
//! edge, an inner rule 32 px in, and between them a band cut into 8 columns and 6 rows
//! whose numbers and letters name the grid references of section 14.1 rule 13.

use stencil_model::text::{TextMeasurer, TextStyleName, WRAP_EPSILON_PX};
use stencil_model::{SHEET_COLUMNS, SHEET_ROW_LETTERS, SHEET_ROWS};

use crate::styles::{text_color, text_style_for};
use crate::{BoxRect, LayoutError, PageGeometry, Part, PartName, TextAlign, TextRun};

const BORDER_INSET_PX: f32 = 12.0;
const BORDER_PX: f32 = 2.0;
const RULE_INSET_PX: f32 = 32.0;
const RULE_PX: f32 = 1.0;

fn inset(bounds: BoxRect, by: f32) -> BoxRect {
    BoxRect {
        x: bounds.x + by,
        y: bounds.y + by,
        width: bounds.width - 2.0 * by,
        height: bounds.height - 2.0 * by,
    }
}

/// Appends the frame parts to the page root: `Border`, `RefRule`, `Grid`, a `RefTick` at
/// each inner cell edge in the band, then the column numbers along the top and bottom and
/// the row letters down the left and right.
pub(crate) fn add_sheet_frame(
    geometry: &mut PageGeometry,
    measurer: &mut dyn TextMeasurer,
) -> Result<(), LayoutError> {
    let canvas = BoxRect {
        x: 0.0,
        y: 0.0,
        width: geometry.canvas.width,
        height: geometry.canvas.height,
    };
    let border = inset(canvas, BORDER_INSET_PX);
    let rule = inset(canvas, RULE_INSET_PX);
    let grid = inset(rule, RULE_PX);
    let band = RULE_INSET_PX - BORDER_INSET_PX - BORDER_PX;
    let band_near = BORDER_INSET_PX + BORDER_PX;
    let band_far_x = canvas.width - RULE_INSET_PX;
    let band_far_y = canvas.height - RULE_INSET_PX;
    let column_width = grid.width / f32::from(SHEET_COLUMNS);
    let row_height = grid.height / f32::from(SHEET_ROWS);

    let mut parts = vec![
        shape(PartName::Border, border),
        shape(PartName::RefRule, rule),
        shape(PartName::Grid, grid),
    ];
    for column in 1..SHEET_COLUMNS {
        let x = grid.x + f32::from(column) * column_width;
        for y in [band_near, band_far_y] {
            parts.push(shape(
                PartName::RefTick,
                BoxRect {
                    x,
                    y,
                    width: 0.0,
                    height: band,
                },
            ));
        }
    }
    for row in 1..SHEET_ROWS {
        let y = grid.y + f32::from(row) * row_height;
        for x in [band_near, band_far_x] {
            parts.push(shape(
                PartName::RefTick,
                BoxRect {
                    x,
                    y,
                    width: band,
                    height: 0.0,
                },
            ));
        }
    }

    let style = text_style_for(TextStyleName::SheetLabel, false);
    let label_height = style.line_height_px;
    let mut label = |name: PartName, text: String, cell: BoxRect| -> Result<Part, LayoutError> {
        let metrics = measurer
            .measure(&text, &style, Some(cell.width + WRAP_EPSILON_PX))
            .map_err(|source| LayoutError::Measure {
                pointer: stencil_model::NodePointer::root(),
                source,
            })?;
        Ok(Part {
            name,
            bounds: BoxRect {
                x: cell.x,
                y: cell.y + (cell.height - label_height) / 2.0,
                width: cell.width,
                height: label_height,
            },
            text: Some(TextRun {
                text,
                style,
                color: text_color(TextStyleName::SheetLabel, None),
                align: TextAlign::Center,
                metrics,
                first_line_indent_px: 0.0,
            }),
        })
    };
    for (name, y) in [
        (PartName::RefColumn, band_near),
        (PartName::RefColumnFoot, band_far_y),
    ] {
        for column in 0..SHEET_COLUMNS {
            let cell = BoxRect {
                x: grid.x + f32::from(column) * column_width,
                y,
                width: column_width,
                height: band,
            };
            parts.push(label(name, (column + 1).to_string(), cell)?);
        }
    }
    for (name, x) in [
        (PartName::RefRow, band_near),
        (PartName::RefRowRight, band_far_x),
    ] {
        for (row, letter) in SHEET_ROW_LETTERS.iter().enumerate() {
            let cell = BoxRect {
                x,
                y: grid.y + row as f32 * row_height,
                width: band,
                height: row_height,
            };
            parts.push(label(name, letter.to_string(), cell)?);
        }
    }
    if let Some(page) = geometry.nodes.first_mut() {
        page.parts.extend(parts);
    }
    Ok(())
}

fn shape(name: PartName, bounds: BoxRect) -> Part {
    Part {
        name,
        bounds,
        text: None,
    }
}
