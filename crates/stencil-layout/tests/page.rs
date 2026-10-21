#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_from, page_with_body, part};
use serde_json::json;
use stencil_layout::PartName;

fn one_pipe_body() -> serde_json::Value {
    json!([{ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "link" }])
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
        "badge": "Internal",
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

#[test]
fn a_badge_prints_its_own_text_uppercase() {
    let page = page_from(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "badge": "Design review",
        "body": one_pipe_body(),
        "legend": []
    }));
    let geometry = layout(&page);
    let run = part(node(&geometry, "/kicker"), PartName::BadgeText)
        .text
        .clone()
        .unwrap();
    assert_eq!(run.text, "DESIGN REVIEW");
    assert_eq!(run.color, "#174EA6");
}

#[test]
fn without_a_badge_the_kicker_starts_where_the_badge_did() {
    let document = |badge: Option<&str>| {
        let mut document = json!({
            "title": "Title",
            "kicker": "Kicker",
            "lede": "Lede",
            "body": one_pipe_body(),
            "legend": []
        });
        if let Some(badge) = badge {
            document["badge"] = json!(badge);
        }
        document
    };
    let badged = layout(&page_from(document(Some("Draft"))));
    let plain = layout(&page_from(document(None)));
    let kicker = node(&plain, "/kicker");
    assert!(kicker.part(PartName::Badge).is_none());
    assert!(kicker.part(PartName::BadgeText).is_none());
    let badge = part(node(&badged, "/kicker"), PartName::Badge);
    assert_close(
        part(kicker, PartName::Text).bounds.x,
        badge.bounds.x,
        "kicker text x without a badge",
    );
    assert_close(
        kicker.bounds.y,
        node(&badged, "/kicker").bounds.y,
        "kicker row y",
    );
}

#[test]
fn note_kind_selects_its_text_style() {
    let notes: Vec<serde_json::Value> = ["kicker", "h1", "lede", "legend", "foot"]
        .iter()
        .map(|kind| json!({ "tag": "Note", "kind": kind, "text": "Note text" }))
        .collect();
    let geometry = layout(&page_with_body(
        1280,
        json!([{ "tag": "Col", "children": notes }]),
    ));
    let expected = [
        ("NOTE TEXT", 11.0, 13.2, "#1A73E8"),
        ("Note text", 20.0, 24.0, "#202124"),
        ("Note text", 13.0, 15.6, "#5F6368"),
        ("Note text", 12.0, 14.4, "#5F6368"),
        ("Note text", 11.0, 13.2, "#5F6368"),
    ];
    for (index, (text, size, line_height, color)) in expected.into_iter().enumerate() {
        let note = node(&geometry, &format!("/body/0/children/{index}"));
        let run = part(note, PartName::Text).text.clone().unwrap();
        assert_eq!(run.text, text, "note {index}");
        assert_eq!(run.style.size_px, size, "note {index}");
        assert_close(note.bounds.height, line_height, "note height");
        assert_eq!(run.color, color, "note {index}");
    }
}

/// Section 14.4 rules 1 and 3: a sheet pads its root 50 px on every side, so a 1280 px
/// page has a 1380 px canvas; the frame's border sits 12 px in and its rule 32 px in, and
/// the title block starts at the padding.
#[test]
fn a_sheet_pads_its_root_50_px_and_draws_its_frame_inside_the_padding() {
    let page = page_from(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "body": [{
            "tag": "View",
            "title": "System",
            "children": [{ "tag": "Pipe", "dir": "h", "line": "solid", "label": "link" }]
        }],
        "legend": [{ "line": "solid", "text": "request path" }]
    }));
    let geometry = layout(&page);
    assert_eq!(geometry.canvas.width, 1380.0);
    let block = node(&geometry, "/sheet");
    assert_eq!(block.bounds.x, 50.0);
    assert_eq!(block.bounds.y, 50.0);
    assert_eq!(block.bounds.width, 1280.0);
    let body = node(&geometry, "/body");
    assert_close(
        geometry.canvas.height,
        body.bounds.bottom() + 50.0,
        "canvas height",
    );
    let root = node(&geometry, "");
    let border = part(root, PartName::Border).bounds;
    assert_eq!((border.x, border.y), (12.0, 12.0));
    assert_close(border.right(), 1380.0 - 12.0, "border right");
    let rule = part(root, PartName::RefRule).bounds;
    assert_eq!((rule.x, rule.y), (32.0, 32.0));
    let grid = part(root, PartName::Grid).bounds;
    assert_eq!((grid.x, grid.y), (33.0, 33.0));
    assert_close(grid.bottom(), geometry.canvas.height - 33.0, "grid bottom");
    let ticks = root
        .parts
        .iter()
        .filter(|part| part.name == PartName::RefTick)
        .count();
    assert_eq!(ticks, 2 * 7 + 2 * 5);
}
