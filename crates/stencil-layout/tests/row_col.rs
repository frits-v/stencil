#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body};
use serde_json::{Value, json};
use stencil_layout::checks::child_inside_container;

fn card(function_name: &str) -> Value {
    json!({ "tag": "Item", "kind": "product", "title": function_name })
}

fn one_card_zone(label: &str) -> Value {
    json!({ "tag": "Box", "kind": "region", "tint": 1, "label": label, "children": [card("Inside")] })
}

fn card_stack(count: usize) -> Value {
    json!({ "tag": "Col", "children": (0..count).map(|index| card(&format!("Card {index}"))).collect::<Vec<_>>() })
}

#[test]
fn row_without_grow_gives_two_cards_equal_widths() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [card("Short"), card("A much longer function name")] }]),
    ));
    let first = node(&geometry, "/body/0/children/0");
    let second = node(&geometry, "/body/0/children/1");
    assert_close(first.bounds.width, (1280.0 - 8.0) / 2.0, "first card");
    assert_close(second.bounds.width, first.bounds.width, "second card");
}

/// A zone holding one 44 px card: 3 border + 24 padding + 14.4 label + 8 gap + 44.
const ONE_CARD_ZONE_HEIGHT: f32 = 93.4;

#[test]
fn col_without_grow_keeps_zone_children_at_content_height() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [
                { "tag": "Col", "children": [one_card_zone("Upper"), one_card_zone("Lower")] },
                card_stack(6)
            ]
        }]),
    ));
    let row = node(&geometry, "/body/0");
    assert_close(
        row.bounds.height,
        6.0 * 44.0 + 5.0 * 8.0,
        "row height from the card stack",
    );
    let upper = node(&geometry, "/body/0/children/0/children/0");
    let lower = node(&geometry, "/body/0/children/0/children/1");
    assert_close(upper.bounds.height, ONE_CARD_ZONE_HEIGHT, "upper zone");
    assert_close(lower.bounds.height, ONE_CARD_ZONE_HEIGHT, "lower zone");
    assert_close(lower.bounds.y - upper.bounds.bottom(), 8.0, "gap");
}

#[test]
fn col_with_grow_splits_the_row_height_equally() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [
                { "tag": "Col", "grow": [1, 1], "children": [one_card_zone("Upper"), one_card_zone("Lower")] },
                card_stack(6)
            ]
        }]),
    ));
    let row_height = 6.0 * 44.0 + 5.0 * 8.0;
    let upper = node(&geometry, "/body/0/children/0/children/0");
    let lower = node(&geometry, "/body/0/children/0/children/1");
    assert_close(upper.bounds.height, (row_height - 8.0) / 2.0, "upper zone");
    assert_close(lower.bounds.height, upper.bounds.height, "lower zone");
    assert_close(
        lower.bounds.bottom(),
        node(&geometry, "/body/0").bounds.bottom(),
        "fills the row",
    );
}

#[test]
fn pipe_in_a_row_keeps_its_max_content_width() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [card("Left"), { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "VLAN 1" }, card("Right")]
        }]),
    ));
    let pipe = node(&geometry, "/body/0/children/1");
    let label_width = 6.0 * 6.0;
    assert_close(
        pipe.bounds.width,
        8.0 + 14.0 + (label_width + 16.0 + 3.0) + 14.0 + 8.0,
        "pipe width",
    );
    let left = node(&geometry, "/body/0/children/0");
    let right = node(&geometry, "/body/0/children/2");
    assert_close(left.bounds.width, right.bounds.width, "equal card columns");
    assert_close(
        left.bounds.width + pipe.bounds.width + right.bounds.width + 16.0,
        1280.0,
        "row filled",
    );
}

#[test]
fn grow_0_0_1_gives_all_free_width_to_the_third_child() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "grow": [0, 0, 1], "children": [card("One"), card("Two"), card("Three")] }]),
    ));
    let max_content_card = 3.0 + 20.0 + 3.0 * 6.5;
    let first = node(&geometry, "/body/0/children/0");
    let second = node(&geometry, "/body/0/children/1");
    let third = node(&geometry, "/body/0/children/2");
    assert_close(first.bounds.width, max_content_card, "first");
    assert_close(second.bounds.width, max_content_card, "second");
    assert_close(
        third.bounds.width,
        1280.0 - 2.0 * max_content_card - 16.0,
        "third",
    );
}

#[test]
fn justify_center_centers_two_pipes_in_a_taller_col() {
    let pipe = |label: &str| json!({ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": label });
    let geometry = layout(&page_with_body(
        1280,
        json!([{
            "tag": "Row",
            "children": [
                { "tag": "Col", "gap": 12, "justify": "center", "children": [pipe("one"), pipe("two")] },
                card_stack(6)
            ]
        }]),
    ));
    let column = node(&geometry, "/body/0/children/0");
    let first = node(&geometry, "/body/0/children/0/children/0");
    let second = node(&geometry, "/body/0/children/0/children/1");
    assert_close(second.bounds.y - first.bounds.bottom(), 12.0, "gap");
    let space_above = first.bounds.y - column.bounds.y;
    let space_below = column.bounds.bottom() - second.bounds.bottom();
    assert!(space_above > 100.0);
    assert_close(space_above, space_below, "centered");
}

#[test]
fn justify_end_and_space_between_place_the_stack() {
    let pipe = |label: &str| json!({ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": label });
    for (justify, first_at_top, last_at_bottom) in [
        ("end", false, true),
        ("space-between", true, true),
        ("start", true, false),
    ] {
        let geometry = layout(&page_with_body(
            1280,
            json!([{
                "tag": "Row",
                "children": [
                    { "tag": "Col", "justify": justify, "children": [pipe("one"), pipe("two")] },
                    card_stack(6)
                ]
            }]),
        ));
        let column = node(&geometry, "/body/0/children/0");
        let first = node(&geometry, "/body/0/children/0/children/0");
        let second = node(&geometry, "/body/0/children/0/children/1");
        assert_eq!(
            (first.bounds.y - column.bounds.y).abs() < 0.01,
            first_at_top,
            "{justify}"
        );
        assert_eq!(
            (column.bounds.bottom() - second.bounds.bottom()).abs() < 0.01,
            last_at_bottom,
            "{justify}"
        );
    }
}

#[test]
fn row_and_col_gap_default_to_8() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [card_stack(2), card("Right")] }]),
    ));
    let left = node(&geometry, "/body/0/children/0");
    let right = node(&geometry, "/body/0/children/1");
    assert_close(right.bounds.x - left.bounds.right(), 8.0, "row gap");
    let upper = node(&geometry, "/body/0/children/0/children/0");
    let lower = node(&geometry, "/body/0/children/0/children/1");
    assert_close(lower.bounds.y - upper.bounds.bottom(), 8.0, "col gap");
}

/// Section 2.1: body nodes do not shrink. Two weight-0 cards whose max-content widths add up
/// to more than the Row keep those widths, and the second one overflows the Row.
#[test]
fn weight_zero_cards_wider_than_the_row_keep_max_content_and_overflow() {
    let long_name = "A function name long enough to need most of the page width";
    let row =
        json!([{ "tag": "Row", "grow": [0, 0], "children": [card(long_name), card(long_name)] }]);
    let wide = layout(&page_with_body(1280, row.clone()));
    let max_content = node(&wide, "/body/0/children/0").bounds.width;
    assert!(2.0 * max_content + 8.0 < 1280.0, "both cards fit at 1280");
    assert!(child_inside_container(&wide).passed());

    let narrow_width = 640;
    assert!(2.0 * max_content + 8.0 > narrow_width as f32);
    let narrow = layout(&page_with_body(narrow_width, row));
    let first = node(&narrow, "/body/0/children/0");
    let second = node(&narrow, "/body/0/children/1");
    assert_close(first.bounds.width, max_content, "first card");
    assert_close(second.bounds.width, max_content, "second card");

    let inside = child_inside_container(&narrow);
    let pointers: Vec<&str> = inside
        .defects
        .iter()
        .map(|defect| defect.pointer.as_str())
        .collect();
    assert_eq!(pointers, ["/body/0/children/1"]);
}
