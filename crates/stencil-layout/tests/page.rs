#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_from, page_with_body, part};
use serde_json::json;
use stencil_layout::PartName;

fn one_pipe_body() -> serde_json::Value {
    json!([{ "tag": "Pipe", "dir": "h", "kind": "blue", "label": "link" }])
}

#[test]
fn root_is_page_width_plus_40_and_title_starts_at_the_padding() {
    let geometry = layout(&common::g7_page());
    let root = node(&geometry, "");
    assert_eq!(root.bounds.width, 1320.0);
    assert_eq!(geometry.canvas.width, 1320.0);
    assert_eq!(node(&geometry, "/title").bounds.x, 20.0);
    assert_eq!(node(&geometry, "/title").bounds.width, 1280.0);
}

#[test]
fn kicker_sits_6_px_above_title() {
    let geometry = layout(&common::g7_page());
    let kicker = node(&geometry, "/kicker");
    let title = node(&geometry, "/title");
    assert_close(
        title.bounds.y - kicker.bounds.bottom(),
        6.0,
        "kicker to title",
    );
}

#[test]
fn canvas_height_is_last_root_child_bottom_plus_20() {
    let geometry = layout(&common::g7_page());
    let foot = node(&geometry, "/foot");
    assert_close(
        geometry.canvas.height,
        foot.bounds.bottom() + 20.0,
        "canvas height",
    );

    let without_foot = layout(&page_with_body(1280, one_pipe_body()));
    let body = node(&without_foot, "/body");
    assert!(without_foot.node(&common::pointer("/legend")).is_none());
    assert!(without_foot.node(&common::pointer("/foot")).is_none());
    assert_close(
        without_foot.canvas.height,
        body.bounds.bottom() + 20.0,
        "canvas height without legend and foot",
    );
}

#[test]
fn page_width_sets_the_canvas_width() {
    let geometry = layout(&page_with_body(640, one_pipe_body()));
    assert_eq!(geometry.canvas.width, 680.0);
    assert_eq!(node(&geometry, "/body").bounds.width, 640.0);
}

#[test]
fn long_kicker_wraps_inside_the_kicker() {
    let long_kicker = vec!["segment"; 40].join(" ");
    let page = page_from(json!({
        "title": "Title",
        "kicker": long_kicker,
        "lede": "Lede",
        "width": 640,
        "canvas": "internal",
        "body": one_pipe_body(),
        "legend": []
    }));
    let geometry = layout(&page);
    let kicker = node(&geometry, "/kicker");
    let text = part(kicker, PartName::Text);
    let run = text.text.as_ref().unwrap();
    assert!(run.metrics.line_count > 1, "kicker wraps");
    assert!(text.bounds.x >= kicker.bounds.x);
    assert!(text.bounds.right() <= kicker.bounds.right() + 0.01);
    assert!(text.bounds.bottom() <= kicker.bounds.bottom() + 0.01);
    assert!(run.metrics.width_px <= text.bounds.width + 0.01);
    assert_eq!(run.text, long_kicker.to_uppercase());
}

#[test]
fn badge_names_the_canvas_uppercased() {
    let geometry = layout(&common::g7_page());
    let kicker = node(&geometry, "/kicker");
    let badge = part(kicker, PartName::Badge);
    let badge_text = part(kicker, PartName::BadgeText);
    let run = badge_text.text.as_ref().unwrap();
    assert_eq!(run.text, "CUSTOMER");
    assert_eq!(run.color, "#174EA6");
    assert_close(badge.bounds.height, 16.0, "badge height, 2 + 12 + 2");
    assert_close(
        badge_text.bounds.x - badge.bounds.x,
        7.0,
        "badge left padding",
    );
}
