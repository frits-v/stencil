//! Marks on their carriers, the keyed lists' where-used text and `marks-clear` (section
//! 14.4 rules 7, 17 to 19 and 22, section 14.6).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{layout, page_from};
use serde_json::{Value, json};
use stencil_layout::checks::{marks_clear, text_fits_box};
use stencil_layout::marks::{WHERE_USED_PARTS, where_used_text};
use stencil_layout::{NodeGeometry, PageGeometry, PartName, SheetZone, sheet_frame, sheet_zone_at};
use stencil_model::marks::PageMarks;
use stencil_model::{MarkKind, Page, PagePoint, body_nodes};

const ONEPAGER: &str = include_str!("../../../examples/onepager.json");

fn onepager() -> (Page, PageGeometry) {
    let page: Page = serde_json::from_str(ONEPAGER).unwrap();
    let geometry = layout(&page);
    (page, geometry)
}

/// A sheet whose Item and Step each carry a corner mark: the Item keyed to the parts list,
/// the Step to the attention item.
fn corner_document() -> Value {
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "actors": [{ "id": "crew", "name": "Crew", "tint": 2 }],
        "grammar": "gcp",
        "body": [{
            "tag": "View",
            "title": "System",
            "children": [
                {
                    "tag": "Row",
                    "gap": 40,
                    "children": [
                        { "tag": "Item", "id": "api", "kind": "product", "title": "API",
                          "actor": "crew", "corner_mark": { "part": "cache" } },
                        { "tag": "Item", "id": "worker", "kind": "product", "title": "Worker",
                          "actor": "crew" }
                    ]
                },
                {
                    "tag": "Phases",
                    "lanes": ["crew"],
                    "phases": [{ "id": "run", "title": "Run" }],
                    "children": [{
                        "tag": "Step", "lane": "crew", "phase": "run", "title": "Work",
                        "corner_mark": { "attention": "first" }
                    }]
                },
                {
                    "tag": "PartsList",
                    "title": "Parts",
                    "parts": [{ "id": "cache", "name": "Cache", "why": "Keeps reads fast." }]
                }
            ]
        }],
        "legend": [{ "line": "solid", "by_actor": true, "text": "request path" }],
        "links": [{ "from": "api", "to": "worker", "line": "solid" }]
    })
}

/// The glyph parts of a node in drawn order.
fn glyphs(node: &NodeGeometry) -> Vec<usize> {
    node.parts
        .iter()
        .enumerate()
        .filter(|(_, part)| part.name.is_mark_glyph())
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn marks_on_onepager_pass_text_fits_box_and_marks_clear() {
    let (page, geometry) = onepager();
    let fits = text_fits_box(&geometry);
    assert_eq!(fits.defects, Vec::new());
    let numerals = geometry
        .nodes
        .iter()
        .flat_map(|node| node.parts.iter())
        .chain(geometry.links.iter().flat_map(|route| route.parts.iter()))
        .filter(|part| part.name == PartName::MarkNumeral)
        .count();
    assert!(numerals > 20, "{numerals} carrier numerals");
    let clear = marks_clear(&page, &geometry);
    assert_eq!(clear.defects, Vec::new());
    let sites = PageMarks::of(&page).all(&page).len();
    let parts = |name: PartName| {
        geometry
            .nodes
            .iter()
            .flat_map(|node| node.parts.iter())
            .chain(geometry.links.iter().flat_map(|route| route.parts.iter()))
            .filter(|part| part.name == name)
            .count()
    };
    let arrowheads: usize = geometry
        .links
        .iter()
        .map(|route| match route.arrow {
            stencil_model::Arrow::None => 0,
            stencil_model::Arrow::Start | stencil_model::Arrow::End => 1,
            stencil_model::Arrow::Both => 2,
        })
        .sum();
    // Six step badges on links and the strip's sample, the queue job's valve, and an end
    // feature per arrowhead and rail dot.
    assert_eq!(parts(PartName::StepBadge), 7);
    assert_eq!(parts(PartName::ValveNear), 1);
    let line_marks = parts(PartName::StepBadge)
        + parts(PartName::ValveNear)
        + parts(PartName::Hop)
        + arrowheads
        + parts(PartName::RailDot);
    // Every carrier glyph, every list glyph (3 attention items, 9 parts, 4 notes), a
    // sample per kind, the marks of the lines, and the detail frame and its tag.
    assert_eq!(
        clear.examined,
        (sites + 3 + 9 + 4 + 3 + line_marks + 2) as u64
    );
}

#[test]
fn a_corner_mark_passes_text_fits_box_and_marks_clear() {
    let page = page_from(corner_document());
    let geometry = layout(&page);
    assert_eq!(text_fits_box(&geometry).defects, Vec::new());
    let report = marks_clear(&page, &geometry);
    assert_eq!(report.defects, Vec::new());
    let item = geometry
        .node(&common::pointer("/body/0/children/0/children/0"))
        .unwrap();
    let glyph = item.part(PartName::Balloon).unwrap().bounds;
    let center = (glyph.x + glyph.width / 2.0, glyph.y + glyph.height / 2.0);
    common::assert_close(center.0, item.bounds.right(), "corner x");
    common::assert_close(center.1, item.bounds.y, "corner y");
}

/// The corner sheet with its one part named and described as given.
fn corner_parts_list(name: &str, what: &str) -> PageGeometry {
    let mut document = corner_document();
    document["body"][0]["children"][2]["parts"][0]["name"] = json!(name);
    document["body"][0]["children"][2]["parts"][0]["what"] = json!(what);
    layout(&page_from(document))
}

/// The name cell is 136 px wide and its boxes stop there, so a name or what that is one
/// word wider than the cell overflows its box and runs into the why column: text-fits-box
/// reports it at the list. The same words with spaces wrap inside the cell.
#[test]
fn a_part_name_of_one_word_wider_than_its_column_is_a_text_fits_box_defect() {
    let name = "trigger_evaluation_plan_store_for_operators";
    let what = "postgres_with_a_logical_replication_slot";
    let geometry = corner_parts_list(name, what);
    let fits = text_fits_box(&geometry);
    let found: Vec<(&str, &str)> = fits
        .defects
        .iter()
        .map(|defect| {
            let part = defect.message.split(' ').next().unwrap_or_default();
            (defect.pointer.as_str(), part)
        })
        .collect();
    assert_eq!(
        found,
        [
            ("/body/0/children/2", "part_title"),
            ("/body/0/children/2", "part_what")
        ],
        "{:?}",
        fits.defects
    );
    let list = geometry
        .node(&common::pointer("/body/0/children/2"))
        .unwrap();
    let title = list.part(PartName::PartTitle).unwrap().bounds;
    let why_rule = list.part(PartName::PartWhyRule).unwrap().bounds;
    assert!(
        title.right() <= why_rule.x,
        "{title:?} crosses {why_rule:?}"
    );

    let spaced = corner_parts_list(&name.replace('_', " "), &what.replace('_', " "));
    assert_eq!(text_fits_box(&spaced).defects, Vec::new());
}

/// Every where-used text is the distinct cells of the glyph centres of its key, read here
/// from the drawn glyphs and the marks of each carrier, sorted by row and column.
#[test]
fn the_drawn_zones_are_the_cells_of_the_glyph_centres() {
    let (page, geometry) = onepager();
    let frame = sheet_frame(&geometry).unwrap();
    let page_marks = PageMarks::of(&page);
    let mut cells: BTreeMap<(MarkKind, usize), BTreeSet<SheetZone>> = BTreeMap::new();
    let mut carriers = Vec::new();
    for index in 0..page.sheet.as_ref().unwrap().problem.points.len() {
        let pointer = stencil_model::SheetList::ProblemPoints
            .pointer()
            .index(index);
        carriers.push((pointer, page_marks.point_marks(&page, index)));
    }
    for entry in body_nodes(&page) {
        carriers.push((entry.pointer.clone(), page_marks.node_marks(&entry)));
        if let stencil_model::NodeRef::Node(stencil_model::Node::Ladder(ladder)) = entry.node {
            for index in 0..ladder.rungs.len() {
                let pointer = stencil_model::SheetList::Rungs
                    .pointer_in(&entry.pointer)
                    .index(index);
                carriers.push((
                    pointer,
                    PageMarks::rung_marks(ladder, &entry.pointer, index),
                ));
            }
        }
    }
    let mut sites = 0;
    for (pointer, marks) in carriers {
        let Some(node) = geometry.node(&pointer) else {
            continue;
        };
        for (mark, part) in marks.iter().zip(glyphs(node)) {
            let bounds = node.parts[part].bounds;
            let center = PagePoint {
                x: bounds.x + bounds.width / 2.0,
                y: bounds.y + bounds.height / 2.0,
            };
            let key = (mark.mark.kind(), page_marks.position(&mark.mark).unwrap());
            cells
                .entry(key)
                .or_default()
                .insert(sheet_zone_at(center, frame));
            sites += 1;
        }
    }
    for route in &geometry.links {
        let marks = PageMarks::link_marks(&page.links[route.index], route.index);
        let route_glyphs = route.parts.iter().filter(|part| part.name.is_mark_glyph());
        for (mark, part) in marks.iter().zip(route_glyphs) {
            let center = PagePoint {
                x: part.bounds.x + part.bounds.width / 2.0,
                y: part.bounds.y + part.bounds.height / 2.0,
            };
            let key = (mark.mark.kind(), page_marks.position(&mark.mark).unwrap());
            cells
                .entry(key)
                .or_default()
                .insert(sheet_zone_at(center, frame));
            sites += 1;
        }
    }
    assert_eq!(sites, page_marks.all(&page).len());
    let mut examined = 0;
    for (kind, part_name, _, prefix) in WHERE_USED_PARTS {
        let owner: &NodeGeometry = geometry
            .nodes
            .iter()
            .find(|node| node.parts.iter().any(|part| part.name == part_name))
            .unwrap();
        for (position, part) in owner
            .parts
            .iter()
            .filter(|part| part.name == part_name)
            .enumerate()
        {
            let zones: Vec<SheetZone> = cells
                .get(&(kind, position))
                .map(|zones| zones.iter().copied().collect())
                .unwrap_or_default();
            assert_eq!(
                part.text.as_ref().unwrap().text,
                where_used_text(prefix, &zones),
                "{} {position}",
                part_name.as_str()
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 3 + 9 + 4);
}

/// (pointer, message) of a marks-clear report.
fn defects(page: &Page, geometry: &PageGeometry) -> Vec<(String, String)> {
    marks_clear(page, geometry)
        .defects
        .into_iter()
        .map(|defect| (defect.pointer.as_str().to_string(), defect.message))
        .collect()
}

/// The first problem point with marks, and the index of its first glyph part.
fn first_marked_point(geometry: &mut PageGeometry) -> (&mut NodeGeometry, usize) {
    let node = geometry
        .nodes
        .iter_mut()
        .find(|node| node.parts.iter().any(|part| part.name.is_mark_glyph()))
        .unwrap();
    let glyph = node
        .parts
        .iter()
        .position(|part| part.name.is_mark_glyph())
        .unwrap();
    (node, glyph)
}

#[test]
fn a_glyph_outside_its_carrier_or_over_its_text_is_a_defect() {
    let (page, mut geometry) = onepager();
    let (node, glyph) = first_marked_point(&mut geometry);
    let pointer = node.pointer.as_str().to_string();
    node.parts[glyph].bounds.x = node.bounds.right() + 10.0;
    let found = defects(&page, &geometry);
    assert!(
        found
            .iter()
            .any(|(at, message)| *at == pointer && message.contains("extends outside its carrier")),
        "{found:?}"
    );

    let (page, mut geometry) = onepager();
    let (node, glyph) = first_marked_point(&mut geometry);
    let text = node
        .parts
        .iter()
        .find(|part| part.name == PartName::Text)
        .unwrap()
        .bounds;
    node.parts[glyph].bounds.x = text.x;
    node.parts[glyph].bounds.y = text.y;
    let found = defects(&page, &geometry);
    assert!(
        found
            .iter()
            .any(|(_, message)| message.contains("overlaps text")),
        "{found:?}"
    );
}

#[test]
fn a_numeral_off_its_glyph_or_reading_another_number_is_a_defect() {
    let (page, mut geometry) = onepager();
    let (node, glyph) = first_marked_point(&mut geometry);
    let numeral = node
        .parts
        .iter()
        .position(|part| part.name == PartName::MarkNumeral)
        .unwrap();
    node.parts[numeral].bounds.y = node.parts[glyph].bounds.bottom() + 4.0;
    let found = defects(&page, &geometry);
    assert!(
        found
            .iter()
            .any(|(_, message)| message.contains("numeral box")),
        "{found:?}"
    );

    let (page, mut geometry) = onepager();
    let (node, _) = first_marked_point(&mut geometry);
    let numeral = node
        .parts
        .iter_mut()
        .find(|part| part.name == PartName::MarkNumeral)
        .unwrap();
    numeral.text.as_mut().unwrap().text = "9".to_string();
    let found = defects(&page, &geometry);
    assert!(
        found
            .iter()
            .any(|(_, message)| message.contains("reads \"9\", its key is number")),
        "{found:?}"
    );
}

#[test]
fn a_corner_glyph_off_its_corner_is_a_defect() {
    let page = page_from(corner_document());
    let mut geometry = layout(&page);
    let pointer = common::pointer("/body/0/children/0/children/0");
    let item = geometry
        .nodes
        .iter_mut()
        .find(|node| node.pointer == pointer)
        .unwrap();
    let right = item.bounds.right();
    let balloon = item
        .parts
        .iter_mut()
        .find(|part| part.name == PartName::Balloon)
        .unwrap();
    balloon.bounds.x = right - balloon.bounds.width - 30.0;
    let found = defects(&page, &geometry);
    assert!(
        found.iter().any(|(at, message)| at == pointer.as_str()
            && message.contains("is not centred on the top-right corner")),
        "{found:?}"
    );
}

#[test]
fn a_list_numeral_that_skips_a_number_is_a_defect_and_a_page_without_marks_is_not_applicable() {
    let (page, mut geometry) = onepager();
    let list = geometry
        .nodes
        .iter_mut()
        .find(|node| {
            node.parts
                .iter()
                .any(|part| part.name == PartName::PartNumeral)
        })
        .unwrap();
    let pointer = list.pointer.clone();
    let numeral = list
        .parts
        .iter_mut()
        .filter(|part| part.name == PartName::PartNumeral)
        .nth(1)
        .unwrap();
    numeral.text.as_mut().unwrap().text = "3".to_string();
    let found = defects(&page, &geometry);
    assert_eq!(
        found,
        [(
            pointer.as_str().to_string(),
            "part_balloon 1 reads \"3\", its key is number 2".to_string()
        )]
    );

    let page = common::g7_page();
    let report = marks_clear(&page, &layout(&page));
    assert_eq!(
        report.not_applicable,
        Some("page has no marks, step badges, valves, hops or detail frames")
    );
}
