#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use stencil_layout::{NodeGeometry, PageGeometry, Part, PartName, layout_page};
use stencil_model::Page;
use stencil_model::pointer::NodePointer;
use stencil_model::text::FixedMetricsMeasurer;

pub const G7_JSON: &str = include_str!("../../../../examples/g7.json");
pub const STRESS_JSON: &str = include_str!("../../../../examples/stress-dense.json");

pub fn g7_page() -> Page {
    serde_json::from_str(G7_JSON).expect("g7.json parses as a Page")
}

pub fn page_from(document: Value) -> Page {
    serde_json::from_value(document).expect("test document parses as a Page")
}

/// A customer page at the given width whose body is `body` and whose legend is empty.
pub fn page_with_body(width: u32, body: Value) -> Page {
    page_from(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": width,
        "canvas": "customer",
        "body": body,
        "legend": []
    }))
}

pub fn layout(page: &Page) -> PageGeometry {
    layout_page(page, &mut FixedMetricsMeasurer::default()).expect("layout succeeds")
}

pub fn pointer(text: &str) -> NodePointer {
    let mut result = NodePointer::root();
    for token in text.split('/').skip(1) {
        result = match token.parse::<usize>() {
            Ok(index) => result.index(index),
            Err(_) => result.child(token),
        };
    }
    assert_eq!(result.as_str(), text);
    result
}

pub fn node<'a>(geometry: &'a PageGeometry, pointer_text: &str) -> &'a NodeGeometry {
    geometry
        .node(&pointer(pointer_text))
        .unwrap_or_else(|| panic!("no geometry node {pointer_text}"))
}

pub fn part(node: &NodeGeometry, name: PartName) -> &Part {
    node.part(name)
        .unwrap_or_else(|| panic!("{} has no part {:?}", node.pointer, name))
}

pub fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 0.01,
        "{what}: expected {expected}, got {actual}"
    );
}
