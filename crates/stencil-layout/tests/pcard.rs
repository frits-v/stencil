#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::{Value, json};
use stencil_layout::PartName;
use stencil_model::text::{FixedMetricsMeasurer, TextMeasurer};

fn single_card(card: Value) -> stencil_layout::PageGeometry {
    layout(&page_with_body(1280, json!([card])))
}

#[test]
fn icon_is_28_by_28() {
    let geometry =
        single_card(json!({ "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster" }));
    let icon = part(node(&geometry, "/body/0"), PartName::Icon);
    assert_eq!((icon.bounds.width, icon.bounds.height), (28.0, 28.0));
}

#[test]
fn card_with_icon_fn_and_pn_is_46_tall() {
    let geometry = single_card(
        json!({ "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster", "subtitle": "GKE" }),
    );
    assert_close(
        node(&geometry, "/body/0").bounds.height,
        46.0,
        "6 + 15.6 + 1 + 14.4 + 6 + 3",
    );
}

#[test]
fn card_with_icon_and_fn_only_is_raised_to_44() {
    let geometry =
        single_card(json!({ "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster" }));
    assert_close(
        node(&geometry, "/body/0").bounds.height,
        44.0,
        "43 raised to the minimum",
    );
}

#[test]
fn fact_adds_its_box_height_plus_4() {
    let plain = single_card(
        json!({ "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster", "subtitle": "GKE" }),
    );
    let with_fact = single_card(json!({
        "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster", "subtitle": "GKE", "facts": [{ "text": "Three zones" }]
    }));
    let card = node(&with_fact, "/body/0");
    let fact_box = part(card, PartName::FactBox);
    assert_close(fact_box.bounds.height, 24.2, "4 + 16.2 + 4");
    assert_close(
        card.bounds.height,
        node(&plain, "/body/0").bounds.height + fact_box.bounds.height + 4.0,
        "card with fact",
    );
}

#[test]
fn ask_is_prefixed_and_boxed() {
    let geometry = single_card(
        json!({ "tag": "Item", "kind": "product", "title": "Cluster", "facts": [{ "text": "confirm zones", "source": "ask" }] }),
    );
    let card = node(&geometry, "/body/0");
    let ask = part(card, PartName::Ask);
    let run = ask.text.as_ref().unwrap();
    assert_eq!(run.text, "Ask: confirm zones");
    assert_eq!(run.color, "#B06000");
    let ask_box = part(card, PartName::AskBox);
    assert_close(ask.bounds.x - ask_box.bounds.x, 8.0, "ask box left padding");
}

#[test]
fn text_starts_at_left_padding_without_icon() {
    let geometry = single_card(
        json!({ "tag": "Item", "kind": "product", "title": "Cluster", "subtitle": "GKE" }),
    );
    let card = node(&geometry, "/body/0");
    let text = part(card, PartName::Text);
    assert_close(
        text.bounds.x,
        card.bounds.x + 1.5 + 10.0,
        "border plus left padding",
    );
}

#[test]
fn text_column_takes_remaining_width_and_the_fact_wraps() {
    // 84 scalars at 6 px is 504 px of max-content, wider than the 239 px column.
    let fact = ["abcdefghij"; 8].join(" ") + " abc";
    let page = page_with_body(
        640,
        json!([{
            "tag": "Row",
            "gap": 40,
            "children": [
                { "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster", "facts": [{ "text": fact }] },
                { "tag": "Item", "kind": "product", "title": "Other" }
            ]
        }]),
    );
    let geometry = layout(&page);
    let card = node(&geometry, "/body/0/children/0");
    assert_close(card.bounds.width, 300.0, "card width");
    let text = part(card, PartName::Text);
    assert_close(
        text.bounds.width,
        300.0 - 3.0 - 20.0 - 28.0 - 10.0,
        "text column width",
    );
    let fact_run = part(card, PartName::Fact).text.as_ref().unwrap();
    let max_content = FixedMetricsMeasurer::default()
        .measure(&fact, &fact_run.style, None)
        .unwrap();
    assert!(max_content.width_px >= 500.0);
    assert!(fact_run.metrics.line_count > 1, "fact wraps");
}
