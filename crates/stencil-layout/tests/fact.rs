#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::json;
use stencil_layout::PartName;
use stencil_layout::checks::child_inside_container;

#[test]
fn long_fact_wraps_and_stays_inside_its_parent() {
    let text = vec!["word"; 60].join(" ");
    let page = page_with_body(
        640,
        json!([{ "tag": "Col", "children": [{ "tag": "Fact", "text": text }] }]),
    );
    let geometry = layout(&page);
    let column = node(&geometry, "/body/0");
    let fact = node(&geometry, "/body/0/children/0");
    let text_part = part(fact, PartName::Text);
    let run = text_part.text.as_ref().unwrap();
    assert!(run.metrics.line_count > 1, "fact wraps");
    assert!(
        fact.bounds.x >= column.content.x && fact.bounds.right() <= column.content.right() + 0.01
    );
    assert_close(
        text_part.bounds.width,
        640.0 - 16.0,
        "text part is the content box",
    );
    assert_close(text_part.bounds.x, fact.content.x, "text part x");
    assert_close(
        text_part.bounds.height,
        fact.content.height,
        "text part height",
    );
    assert!(child_inside_container(&geometry).passed());
}
