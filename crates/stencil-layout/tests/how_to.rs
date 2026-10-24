//! A HowTo's items (section 14.4 rule 21): rows of equal columns, a heading row with the
//! number, title and command line, the text above or beside the terminal, and what
//! text-fits-box reports when a line set on one line does not fit.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{layout, node, page_from, part};
use serde_json::{Value, json};
use stencil_layout::checks::text_fits_box;
use stencil_layout::{NodeTag, PageGeometry, PartName};

const HOW_TO: &str = "/body/0/children/0";

fn item(title: &str, terminal: Value) -> Value {
    json!({
        "title": title,
        "commands": ["crew run", { "words": "then" }, "crew check"],
        "text": "Starts the work and checks that it ran.",
        "terminal": terminal
    })
}

fn short_terminal() -> Value {
    json!([
        { "kind": "command", "text": "crew run w-1" },
        { "kind": "output", "text": "w-1 queued" },
        { "kind": "output", "indent": 2, "text": "on the night shift" },
        { "kind": "comment", "text": "the crew picks it up" },
        { "kind": "ok", "text": "w-1 done" }
    ])
}

/// A sheet whose one View holds `children`, at `width`.
fn sheet_document(width: u32, children: Value) -> Value {
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": width,
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "grammar": "gcp",
        "body": [{
            "tag": "View",
            "title": "How-tos",
            "children": children
        }, {
            "tag": "View",
            "title": "System",
            "children": [{
                "tag": "Row",
                "children": [
                    { "tag": "Item", "id": "api", "kind": "product", "title": "API" },
                    { "tag": "Item", "id": "worker", "kind": "product", "title": "Worker" }
                ]
            }]
        }],
        "legend": [{ "line": "solid", "text": "request path" }],
        "links": [{ "from": "api", "to": "worker", "line": "solid" }]
    })
}

fn how_to(items: Vec<Value>) -> Value {
    json!({ "tag": "HowTo", "items": items })
}

fn items_of(geometry: &PageGeometry) -> Vec<&stencil_layout::NodeGeometry> {
    geometry
        .nodes
        .iter()
        .filter(|node| node.tag == NodeTag::HowToItem)
        .collect()
}

fn inside(outer: &stencil_layout::BoxRect, inner: &stencil_layout::BoxRect) -> bool {
    inner.x >= outer.x - 0.01
        && inner.y >= outer.y - 0.01
        && inner.right() <= outer.right() + 0.01
        && inner.bottom() <= outer.bottom() + 0.01
}

fn text_of(node: &stencil_layout::NodeGeometry, name: PartName) -> String {
    part(node, name).text.as_ref().unwrap().text.clone()
}

/// Text-fits-box defects of `geometry` whose message names `part`.
fn defects_naming(geometry: &PageGeometry, part: &str) -> Vec<String> {
    text_fits_box(geometry)
        .defects
        .into_iter()
        .map(|defect| format!("{} {}", defect.pointer, defect.message))
        .filter(|message| message.contains(part))
        .collect()
}

/// Four items that fit three to a row lie two and two, in equal columns, numbered after
/// their view; each is a geometry node after its HowTo.
#[test]
fn four_how_tos_that_fit_three_to_a_row_lie_two_and_two() {
    // 60 characters at the fixed metrics' 5.75 px: about 380 px of terminal, three to
    // 1200 px.
    let terminal = json!([{ "kind": "output", "text": "o".repeat(60) }]);
    let items: Vec<Value> = (1..=4)
        .map(|n| item(&format!("Run job {n}"), terminal.clone()))
        .collect();
    let page = page_from(sheet_document(1200, json!([how_to(items)])));
    let geometry = layout(&page);
    let how_to = node(&geometry, HOW_TO);
    assert!(how_to.parts.is_empty(), "a HowTo draws no part of its own");
    let drawn = items_of(&geometry);
    assert_eq!(drawn.len(), 4);
    let order: Vec<usize> = geometry
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node.tag, NodeTag::HowTo | NodeTag::HowToItem))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        order
            .windows(2)
            .filter(|pair| pair[1] != pair[0] + 1)
            .count(),
        0
    );
    assert_eq!(drawn[0].bounds.y, drawn[1].bounds.y);
    assert_eq!(drawn[2].bounds.y, drawn[3].bounds.y);
    assert!(drawn[2].bounds.y > drawn[0].bounds.bottom());
    assert_eq!(drawn[0].bounds.x, drawn[2].bounds.x);
    for item in &drawn {
        assert!((item.bounds.width - drawn[0].bounds.width).abs() < 0.01);
    }
    let numbers: Vec<String> = drawn
        .iter()
        .map(|item| text_of(item, PartName::HowToNumber))
        .collect();
    assert_eq!(numbers, ["2.1", "2.2", "2.3", "2.4"]);
    assert_eq!(text_fits_box(&geometry).defects, Vec::new());
}

/// A second HowTo in the same view goes on numbering, and one in a later view starts
/// again from 1.
#[test]
fn how_tos_number_through_their_view() {
    let mut document = sheet_document(
        1200,
        json!([
            how_to(vec![item("First", short_terminal())]),
            how_to(vec![item("Second", short_terminal())])
        ]),
    );
    document["body"][1]["children"]
        .as_array_mut()
        .unwrap()
        .push(how_to(vec![item("Third", short_terminal())]));
    let geometry = layout(&page_from(document));
    let numbers: Vec<String> = items_of(&geometry)
        .iter()
        .map(|item| text_of(item, PartName::HowToNumber))
        .collect();
    assert_eq!(numbers, ["2.1", "2.2", "3.1"]);
}

/// The command line reads left to right as chip, words, chip on one row, right of the
/// title, each chip's text inside its chip; a prompt stands on the command, comment and ok
/// lines only, every line starts in one column and an indented line further in.
#[test]
fn the_heading_and_terminal_lie_as_drawn() {
    let page = page_from(sheet_document(
        1200,
        json!([how_to(vec![item("Run work", short_terminal())])]),
    ));
    let geometry = layout(&page);
    let item = items_of(&geometry)[0];
    let parts = |name: PartName| -> Vec<&stencil_layout::Part> {
        item.parts.iter().filter(|part| part.name == name).collect()
    };
    let chips = parts(PartName::CommandChip);
    let texts = parts(PartName::CommandText);
    let words = parts(PartName::CommandWord);
    assert_eq!((chips.len(), texts.len(), words.len()), (2, 2, 1));
    let title = part(item, PartName::HowToTitle);
    assert!(chips[0].bounds.x >= title.bounds.right());
    assert!(words[0].bounds.x >= chips[0].bounds.right());
    assert!(chips[1].bounds.x >= words[0].bounds.right());
    for (chip, text) in chips.iter().zip(&texts) {
        assert!(
            inside(&chip.bounds, &text.bounds),
            "chip text leaves its chip"
        );
    }
    let rule = part(item, PartName::HowToRule);
    let terminal = part(item, PartName::Terminal);
    let text = part(item, PartName::HowToText);
    assert!(rule.bounds.y >= title.bounds.bottom());
    assert!(text.bounds.y >= rule.bounds.bottom());
    assert!(terminal.bounds.y >= text.bounds.bottom());

    let prompts = parts(PartName::TerminalPrompt);
    let lines = parts(PartName::TerminalLine);
    assert_eq!((prompts.len(), lines.len()), (3, 5));
    let prompt_texts: Vec<&str> = prompts
        .iter()
        .map(|prompt| prompt.text.as_ref().unwrap().text.as_str())
        .collect();
    assert_eq!(prompt_texts, ["$", "#", "\u{2713}"]);
    assert_eq!(prompts[0].bounds.y, lines[0].bounds.y);
    assert_eq!(prompts[1].bounds.y, lines[3].bounds.y);
    assert_eq!(prompts[2].bounds.y, lines[4].bounds.y);
    let column = lines[0].bounds.x;
    for (index, line) in lines.iter().enumerate() {
        assert!(inside(&terminal.bounds, &line.bounds), "line {index}");
        if index != 2 {
            assert!((line.bounds.x - column).abs() < 0.01, "line {index}");
        }
    }
    assert!(
        lines[2].bounds.x > column + 1.0,
        "an indented line starts further in"
    );
    for prompt in &prompts {
        assert!(
            prompt.bounds.right() <= column,
            "a prompt ends before the text column"
        );
    }
}

/// With beside, the terminal stands at its widest line and the text wraps right of it.
#[test]
fn beside_sets_the_text_right_of_a_terminal_at_its_widest_line() {
    let mut beside = item("Run work", short_terminal());
    beside["beside"] = json!(true);
    let geometry = layout(&page_from(sheet_document(
        1200,
        json!([how_to(vec![beside])]),
    )));
    let item = items_of(&geometry)[0];
    let terminal = part(item, PartName::Terminal);
    let text = part(item, PartName::HowToText);
    assert!(text.bounds.x >= terminal.bounds.right());
    assert!(text.bounds.y < terminal.bounds.bottom());
    let widest = item
        .parts
        .iter()
        .filter(|part| part.name == PartName::TerminalLine)
        .map(|part| part.bounds.right())
        .fold(0.0_f32, f32::max);
    assert!(
        terminal.bounds.right() - widest < 12.0,
        "the terminal hugs its lines"
    );
    assert_eq!(text_fits_box(&geometry).defects, Vec::new());
}

/// A terminal line wider than its column is drawn past the box it is given, which
/// text-fits-box reports at the item; a column as wide as the sheet allows still cannot
/// hold it, so no layout hides the line by wrapping it.
#[test]
fn a_terminal_line_wider_than_its_column_is_a_text_fits_box_defect() {
    let long = "x".repeat(380);
    let terminal = json!([{ "kind": "output", "text": long }]);
    let geometry = layout(&page_from(sheet_document(
        900,
        json!([how_to(vec![item("Run work", terminal)])]),
    )));
    let drawn = items_of(&geometry)[0];
    let line = part(drawn, PartName::TerminalLine);
    assert_eq!(line.text.as_ref().unwrap().metrics.line_count, 1);
    let defects = defects_naming(&geometry, "terminal_line");
    assert_eq!(defects.len(), 1, "{defects:?}");
    assert!(
        defects[0].starts_with("/body/0/children/0/items/0 "),
        "{defects:?}"
    );

    let fits = layout(&page_from(sheet_document(
        900,
        json!([how_to(vec![item("Run work", short_terminal())])]),
    )));
    assert_eq!(defects_naming(&fits, "terminal_line"), Vec::<String>::new());
}

/// A command line too wide to stand beside the title is drawn on one line past the item,
/// which text-fits-box reports.
#[test]
fn a_command_line_wider_than_its_item_is_a_text_fits_box_defect() {
    let mut wide = item("Run work", short_terminal());
    wide["commands"] = json!(["x".repeat(300), "y".repeat(300)]);
    let geometry = layout(&page_from(sheet_document(900, json!([how_to(vec![wide])]))));
    let defects = defects_naming(&geometry, "command_text");
    assert!(!defects.is_empty());
    assert!(
        defects
            .iter()
            .all(|defect| defect.contains("extends outside the node box")),
        "{defects:?}"
    );
}
