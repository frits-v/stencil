#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use stencil_layout::{NodeGeometry, PageGeometry, Part, PartName, layout_page};
use stencil_model::pointer::NodePointer;
use stencil_model::text::FixedMetricsMeasurer;
use stencil_model::{Grammar, Page, builtin_grammar};

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

pub fn gcp() -> Grammar {
    builtin_grammar("gcp")
        .expect("gcp is a built-in grammar")
        .expect("the gcp grammar is valid")
}

pub fn plain() -> Grammar {
    builtin_grammar("plain")
        .expect("plain is a built-in grammar")
        .expect("the plain grammar is valid")
}

/// The gcp grammar with `page` added to every kind's parents, so a layout test can place
/// any container kind at the top level. Nesting is vet's concern and is tested there.
pub fn gcp_anywhere() -> Grammar {
    let mut grammar = gcp();
    for container in &mut grammar.containers {
        if !container.parents.iter().any(|parent| parent == "page") {
            container.parents.push("page".to_string());
        }
    }
    grammar
}

/// Lays `page` out under the grammar it names: plain, or gcp with every kind allowed at
/// the top level.
pub fn layout(page: &Page) -> PageGeometry {
    let grammar = match page.grammar.as_deref() {
        Some("plain") => plain(),
        _ => gcp_anywhere(),
    };
    layout_page(page, &grammar, &mut FixedMetricsMeasurer::default()).expect("layout succeeds")
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
