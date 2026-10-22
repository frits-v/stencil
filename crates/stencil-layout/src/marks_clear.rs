//! `marks-clear` (section 14.6): every mark glyph on a carrier, every glyph of a keyed list
//! and every legend sample lies where it belongs, clear of text and of other glyphs, with
//! its numeral inside its outline reading its key's position.

use stencil_model::checks::{CheckName, CheckReport, Defect};
use stencil_model::marks::{CarrierMark, MarkPlace, PageMarks};
use stencil_model::pointer::NodePointer;
use stencil_model::{LINKS_MAX, MarkKind, Page, SheetList, body_nodes};

use crate::marks::glyph_kind;
use crate::{BoxRect, GEOMETRY_EPSILON_PX, NodeTag, PageGeometry, Part, PartName};

/// Why the check does not apply on a page that draws no glyph.
const NO_MARKS: &str = "page has no marks, step badges, valves, hops or detail frames";

/// A corner glyph's centre may lie this far from its carrier's corner.
const CORNER_TOLERANCE_PX: f32 = 0.5;

/// Every glyph's place, clearance and numeral.
pub fn marks_clear(page: &Page, geometry: &PageGeometry) -> CheckReport {
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    if page.sheet.is_none() {
        return CheckReport::not_applicable(CheckName::MarksClear, NO_MARKS);
    }
    let page_marks = PageMarks::of(page);

    let mut carriers: Vec<(NodePointer, Vec<CarrierMark>, usize)> = Vec::new();
    let points = SheetList::ProblemPoints;
    let point_count = page
        .sheet
        .as_ref()
        .map_or(0, |sheet| sheet.problem.points.len().min(points.max() + 1));
    for index in 0..point_count {
        let pointer = points.pointer().index(index);
        if let Some(node) = geometry
            .nodes
            .iter()
            .position(|node| node.pointer == pointer)
        {
            carriers.push((pointer, page_marks.point_marks(page, index), node));
        }
    }
    for entry in body_nodes(page) {
        let marks = page_marks.node_marks(&entry);
        if marks.is_empty() {
            continue;
        }
        if let Some(node) = geometry
            .nodes
            .iter()
            .position(|node| node.pointer == entry.pointer)
        {
            carriers.push((entry.pointer.clone(), marks, node));
        }
    }
    for (pointer, marks, index) in carriers {
        let Some(node) = geometry.nodes.get(index) else {
            continue;
        };
        let ancestors = ancestors_of(geometry, index);
        let carrier = Carrier {
            pointer: &pointer,
            bounds: node.bounds,
            parts: &node.parts,
        };
        examine_carrier(
            &carrier,
            &marks,
            &page_marks,
            (geometry, &ancestors, index),
            (&mut examined, &mut defects),
        );
    }
    let links = NodePointer::root().child("links");
    for route in geometry.links.iter().take(LINKS_MAX + 1) {
        let Some(link) = page.links.get(route.index) else {
            continue;
        };
        let marks = PageMarks::link_marks(link, route.index);
        if marks.is_empty() {
            continue;
        }
        let pointer = links.index(route.index);
        let carrier = Carrier {
            pointer: &pointer,
            bounds: route.tag.unwrap_or(BoxRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }),
            parts: &route.parts,
        };
        examine_carrier(
            &carrier,
            &marks,
            &page_marks,
            (geometry, &[], usize::MAX),
            (&mut examined, &mut defects),
        );
    }

    for node in &geometry.nodes {
        let lists: &[(PartName, PartName, Option<MarkKind>)] = match node.tag {
            NodeTag::TitleCell => &[(PartName::AttentionDiamond, PartName::AttentionNumeral, None)],
            NodeTag::PartsList => &[(PartName::PartBalloon, PartName::PartNumeral, None)],
            NodeTag::GeneralNotes => &[(PartName::NoteFlag, PartName::NoteNumeral, None)],
            NodeTag::Legend => &[(PartName::MarkSample, PartName::MarkSampleNumeral, None)],
            _ => continue,
        };
        for (glyph_name, numeral_name, _) in lists {
            let glyphs: Vec<&Part> = node
                .parts
                .iter()
                .filter(|part| part.name == *glyph_name)
                .collect();
            let numerals: Vec<&Part> = node
                .parts
                .iter()
                .filter(|part| part.name == *numeral_name)
                .collect();
            let samples = node.tag == NodeTag::Legend;
            for (position, glyph) in glyphs.iter().enumerate() {
                examined += 1;
                let label = format!("{} {}", glyph_name.as_str(), position);
                if extends_outside(&glyph.bounds, &node.bounds) {
                    defects.push(Defect {
                        pointer: node.pointer.clone(),
                        message: format!(
                            "{label} box {} extends outside the node box {}",
                            describe(&glyph.bounds),
                            describe(&node.bounds)
                        ),
                    });
                }
                let kind = if samples {
                    sample_kind(node, position)
                } else {
                    glyph_kind(*glyph_name)
                };
                let expected = if samples { 1 } else { position + 1 };
                examine_numeral(
                    (&node.pointer, &label),
                    glyph,
                    numerals.get(position).copied(),
                    (kind, expected),
                    &mut defects,
                );
            }
        }
    }

    let (line_marks, line_defects) = crate::sheet_checks::line_mark_defects(page, geometry);
    examined += line_marks;
    defects.extend(line_defects);

    if examined == 0 {
        return CheckReport::not_applicable(CheckName::MarksClear, NO_MARKS);
    }
    CheckReport {
        check: CheckName::MarksClear,
        examined,
        defects,
        not_applicable: None,
    }
}

/// A node or link tag that carries glyphs.
struct Carrier<'a> {
    pointer: &'a NodePointer,
    bounds: BoxRect,
    parts: &'a [Part],
}

/// The k-th sample of the legend strip shows the k-th kind the page draws; its shape is
/// read back from its size.
fn sample_kind(node: &crate::NodeGeometry, position: usize) -> Option<MarkKind> {
    let sample = node
        .parts
        .iter()
        .filter(|part| part.name == PartName::MarkSample)
        .nth(position)?;
    let (width, height) = (sample.bounds.width, sample.bounds.height);
    MarkKind::ALL.into_iter().find(|kind| {
        let (glyph_width, glyph_height) = crate::marks::glyph_size(*kind, 1);
        (glyph_width - width).abs() < GEOMETRY_EPSILON_PX
            && (glyph_height - height).abs() < GEOMETRY_EPSILON_PX
    })
}

fn ancestors_of(geometry: &PageGeometry, index: usize) -> Vec<usize> {
    let mut ancestors = Vec::new();
    let mut current = geometry.nodes.get(index).and_then(|node| node.parent);
    for _ in 0..geometry.nodes.len() {
        let Some(parent) = current else {
            break;
        };
        ancestors.push(parent);
        current = geometry.nodes.get(parent).and_then(|node| node.parent);
    }
    ancestors
}

/// The glyphs of one carrier, paired with its marks in drawn order.
fn examine_carrier(
    carrier: &Carrier<'_>,
    marks: &[CarrierMark],
    page_marks: &PageMarks<'_>,
    (geometry, ancestors, own): (&PageGeometry, &[usize], usize),
    (examined, defects): (&mut u64, &mut Vec<Defect>),
) {
    let glyphs: Vec<&Part> = carrier
        .parts
        .iter()
        .filter(|part| part.name.is_mark_glyph())
        .collect();
    let numerals: Vec<&Part> = carrier
        .parts
        .iter()
        .filter(|part| part.name == PartName::MarkNumeral)
        .collect();
    let texts: Vec<&Part> = carrier
        .parts
        .iter()
        .filter(|part| part.text.is_some() && part.name != PartName::MarkNumeral)
        .collect();
    for (position, (mark, glyph)) in marks.iter().zip(&glyphs).enumerate() {
        *examined += 1;
        let label = format!("{} {}", glyph.name.as_str(), mark.pointer);
        let bounds = glyph.bounds;
        if mark.place == MarkPlace::Corner {
            let center_x = bounds.x + bounds.width / 2.0;
            let center_y = bounds.y + bounds.height / 2.0;
            if (center_x - carrier.bounds.right()).abs() > CORNER_TOLERANCE_PX
                || (center_y - carrier.bounds.y).abs() > CORNER_TOLERANCE_PX
            {
                defects.push(Defect {
                    pointer: carrier.pointer.clone(),
                    message: format!(
                        "{label} box {} is not centred on the top-right corner of {}",
                        describe(&bounds),
                        describe(&carrier.bounds)
                    ),
                });
            }
            for (index, other) in geometry.nodes.iter().enumerate() {
                if index == own || ancestors.contains(&index) || other.parts.is_empty() {
                    continue;
                }
                if overlap(&bounds, &other.bounds) {
                    defects.push(Defect {
                        pointer: carrier.pointer.clone(),
                        message: format!(
                            "{label} box {} overlaps {}",
                            describe(&bounds),
                            other.pointer
                        ),
                    });
                }
            }
        } else if extends_outside(&bounds, &carrier.bounds) {
            defects.push(Defect {
                pointer: carrier.pointer.clone(),
                message: format!(
                    "{label} box {} extends outside its carrier {}",
                    describe(&bounds),
                    describe(&carrier.bounds)
                ),
            });
        }
        for text in &texts {
            if overlap(&bounds, &text.bounds) {
                defects.push(Defect {
                    pointer: carrier.pointer.clone(),
                    message: format!(
                        "{label} box {} overlaps {} {}",
                        describe(&bounds),
                        text.name.as_str(),
                        describe(&text.bounds)
                    ),
                });
            }
        }
        for other in glyphs.iter().skip(position + 1) {
            if overlap(&bounds, &other.bounds) {
                defects.push(Defect {
                    pointer: carrier.pointer.clone(),
                    message: format!(
                        "{label} box {} overlaps glyph {}",
                        describe(&bounds),
                        describe(&other.bounds)
                    ),
                });
            }
        }
        let expected = page_marks.number(&mark.mark).unwrap_or(0);
        examine_numeral(
            (carrier.pointer, &label),
            glyph,
            numerals.get(position).copied(),
            (Some(mark.mark.kind()), expected),
            defects,
        );
    }
}

/// A numeral reads `expected` and every corner of its box lies inside its glyph's outline.
fn examine_numeral(
    (pointer, label): (&NodePointer, &str),
    glyph: &Part,
    numeral: Option<&Part>,
    (kind, expected): (Option<MarkKind>, usize),
    defects: &mut Vec<Defect>,
) {
    let Some(numeral) = numeral else {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!("{label} draws no numeral"),
        });
        return;
    };
    let text = numeral
        .text
        .as_ref()
        .map(|run| run.text.as_str())
        .unwrap_or_default();
    if text != expected.to_string() {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!("{label} reads {text:?}, its key is number {expected}"),
        });
    }
    let Some(kind) = kind else {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!("{label} box {} is no glyph's size", describe(&glyph.bounds)),
        });
        return;
    };
    let run_box = match &numeral.text {
        Some(run) => BoxRect {
            x: numeral.bounds.x + (numeral.bounds.width - run.metrics.width_px) / 2.0,
            y: numeral.bounds.y,
            width: run.metrics.width_px,
            height: run.metrics.height_px,
        },
        None => numeral.bounds,
    };
    let corners = [
        (run_box.x, run_box.y),
        (run_box.right(), run_box.y),
        (run_box.x, run_box.bottom()),
        (run_box.right(), run_box.bottom()),
    ];
    if corners
        .iter()
        .any(|corner| !inside_outline(kind, &glyph.bounds, *corner))
    {
        defects.push(Defect {
            pointer: pointer.clone(),
            message: format!(
                "{label} numeral box {} leaves its {} {}",
                describe(&run_box),
                kind.glyph(),
                describe(&glyph.bounds)
            ),
        });
    }
}

/// True when `point` lies inside the outline a glyph of `kind` draws in `bounds`: a circle,
/// a triangle with its apex up, or a square on its point.
pub(crate) fn inside_outline(kind: MarkKind, bounds: &BoxRect, (x, y): (f32, f32)) -> bool {
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;
    let half_width = bounds.width / 2.0;
    let half_height = bounds.height / 2.0;
    let epsilon = GEOMETRY_EPSILON_PX;
    match kind {
        MarkKind::Part => {
            let radius = half_width.min(half_height);
            (x - center_x).hypot(y - center_y) <= radius + epsilon
        }
        MarkKind::Note => {
            let depth = y - bounds.y;
            depth >= -epsilon
                && y <= bounds.bottom() + epsilon
                && (x - center_x).abs() <= half_width * depth / bounds.height + epsilon
        }
        MarkKind::Attention => {
            (x - center_x).abs() / half_width + (y - center_y).abs() / half_height <= 1.0 + epsilon
        }
    }
}

fn overlap(first: &BoxRect, second: &BoxRect) -> bool {
    let width = first.right().min(second.right()) - first.x.max(second.x);
    let height = first.bottom().min(second.bottom()) - first.y.max(second.y);
    width > GEOMETRY_EPSILON_PX && height > GEOMETRY_EPSILON_PX
}

fn extends_outside(inner: &BoxRect, outer: &BoxRect) -> bool {
    inner.x < outer.x - GEOMETRY_EPSILON_PX
        || inner.y < outer.y - GEOMETRY_EPSILON_PX
        || inner.right() > outer.right() + GEOMETRY_EPSILON_PX
        || inner.bottom() > outer.bottom() + GEOMETRY_EPSILON_PX
}

fn describe(bounds: &BoxRect) -> String {
    format!(
        "{:.2}x{:.2} at ({:.2}, {:.2})",
        bounds.width, bounds.height, bounds.x, bounds.y
    )
}
