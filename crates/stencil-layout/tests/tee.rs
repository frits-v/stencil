#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::{Value, json};
use stencil_layout::PartName;

fn tee(first_arm: &str, second_arm: &str) -> Value {
    json!({
        "tag": "Tee", "line": "deny", "hub": "egress",
        "arms": [
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": first_arm },
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 2, "label": second_arm }
        ]
    })
}

fn tee_in_col() -> stencil_layout::PageGeometry {
    layout(&page_with_body(
        1280,
        json!([{ "tag": "Col", "children": [tee("allowed", "reply")] }]),
    ))
}

#[test]
fn spine_spans_the_grid_height_minus_36() {
    let geometry = tee_in_col();
    let tee_node = node(&geometry, "/body/0/children/0");
    let spine = part(tee_node, PartName::Spine);
    assert_close(
        spine.bounds.height,
        tee_node.bounds.height - 36.0,
        "spine height",
    );
    assert_close(spine.bounds.y, tee_node.bounds.y + 18.0, "spine top margin");
    assert_eq!(spine.bounds.width, 2.0);
    assert_close(
        spine.bounds.x + 1.0,
        tee_node.bounds.x + 7.0,
        "spine centered in the 14 px column",
    );
}

#[test]
fn hub_is_centered_in_the_middle_row() {
    let geometry = tee_in_col();
    let tee_node = node(&geometry, "/body/0/children/0");
    let hub = part(tee_node, PartName::Hub);
    let upper = node(&geometry, "/body/0/children/0/arms/0");
    let lower = node(&geometry, "/body/0/children/0/arms/1");
    assert_close(
        hub.bounds.x + hub.bounds.width / 2.0,
        tee_node.bounds.x + tee_node.bounds.width / 2.0,
        "hub center x",
    );
    assert!(hub.bounds.y >= upper.bounds.bottom() - 0.01);
    assert!(hub.bounds.bottom() <= lower.bounds.y + 0.01);
    assert_close(hub.bounds.height, 30.6, "hub is a one-line tag");
}

#[test]
fn arms_occupy_rows_1_and_3_in_column_2() {
    let geometry = tee_in_col();
    let tee_node = node(&geometry, "/body/0/children/0");
    let hub = part(tee_node, PartName::Hub);
    for arm_pointer in ["/body/0/children/0/arms/0", "/body/0/children/0/arms/1"] {
        let arm = node(&geometry, arm_pointer);
        assert_close(arm.bounds.x, tee_node.bounds.x + 14.0, "arm x");
        assert_close(
            arm.bounds.right(),
            tee_node.bounds.right(),
            "arm stretches across column 2",
        );
    }
    let upper = node(&geometry, "/body/0/children/0/arms/0");
    let lower = node(&geometry, "/body/0/children/0/arms/1");
    assert!(upper.bounds.bottom() <= hub.bounds.y + 0.01);
    assert!(lower.bounds.y >= hub.bounds.bottom() - 0.01);
    assert_close(upper.bounds.y, tee_node.bounds.y, "upper arm at the top");
    assert_close(
        lower.bounds.bottom(),
        tee_node.bounds.bottom(),
        "lower arm at the bottom",
    );
}

#[test]
fn tee_is_at_least_118_wide() {
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Row", "children": [
            { "tag": "Item", "kind": "product", "title": "Left" }, tee("a", "b"), { "tag": "Item", "kind": "product", "title": "Right" }
        ]}]),
    ));
    let tee_node = node(&geometry, "/body/0/children/1");
    assert_close(tee_node.bounds.width, 118.0, "minimum width");
    assert_close(
        tee_node.bounds.height,
        node(&geometry, "/body/0").bounds.height,
        "stretches in a Row",
    );
}

#[test]
fn tee_in_a_col_stretches_across() {
    let geometry = tee_in_col();
    let column = node(&geometry, "/body/0");
    let tee_node = node(&geometry, "/body/0/children/0");
    assert_close(tee_node.bounds.width, column.bounds.width, "tee width");
}
