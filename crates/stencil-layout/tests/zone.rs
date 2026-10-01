#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, page_with_body, part};
use serde_json::json;
use stencil_layout::PartName;

fn zone_page(kind: &str) -> stencil_layout::PageGeometry {
    zone_page_tinted(kind, None)
}

fn zone_page_tinted(kind: &str, tint: Option<u8>) -> stencil_layout::PageGeometry {
    layout(&page_with_body(
        1280,
        json!([{
            "tag": "Box", "kind": kind, "tint": tint, "label": "Zone label",
            "children": [{ "tag": "Item", "kind": "product", "title": "Inside" }, { "tag": "Fact", "text": "Second" }]
        }]),
    ))
}

#[test]
fn region_first_child_sits_under_the_label_band() {
    let geometry = zone_page_tinted("region", Some(1));
    let zone = node(&geometry, "/body/0");
    let first = node(&geometry, "/body/0/children/0");
    assert_close(
        first.bounds.y,
        zone.bounds.y + 1.5 + 12.0 + 14.4 + 8.0,
        "region-a first child y",
    );
    assert_close(
        first.bounds.x,
        zone.bounds.x + 1.5 + 12.0,
        "region-a first child x",
    );
    let label = part(zone, PartName::Label);
    assert_close(label.bounds.height, 14.4, "zone_label line height");
    assert_eq!(label.text.as_ref().unwrap().style.size_px, 12.0);
}

#[test]
fn perimeter_first_child_sits_under_the_label_band() {
    let geometry = zone_page("perimeter");
    let zone = node(&geometry, "/body/0");
    let first = node(&geometry, "/body/0/children/0");
    assert_close(
        first.bounds.y,
        zone.bounds.y + 2.5 + 12.0 + 15.6 + 8.0,
        "perimeter first child y",
    );
    let label = part(zone, PartName::Label).text.as_ref().unwrap();
    assert_eq!(label.color, "#B06000");
    assert_eq!(label.style.size_px, 13.0);
}

#[test]
fn zone_children_are_separated_by_8() {
    let geometry = zone_page("subnet");
    let first = node(&geometry, "/body/0/children/0");
    let second = node(&geometry, "/body/0/children/1");
    assert_close(
        second.bounds.y - first.bounds.bottom(),
        8.0,
        "gap between zone children",
    );
}

#[test]
fn zone_content_box_is_border_box_minus_border_and_padding() {
    for (kind, inset) in [
        ("vpc", 12.0),
        ("k8s", 12.0),
        ("optional", 14.0),
        ("project", 13.5),
    ] {
        let geometry = zone_page(kind);
        let zone = node(&geometry, "/body/0");
        assert_close(zone.content.x, zone.bounds.x + inset, kind);
        assert_close(zone.content.width, zone.bounds.width - 2.0 * inset, kind);
    }
}

#[test]
fn gcp_bar_is_35_6_tall_and_children_start_inside_the_body_padding() {
    let geometry = zone_page("gcp");
    let zone = node(&geometry, "/body/0");
    let bar = part(zone, PartName::Bar);
    let body = part(zone, PartName::Body);
    assert_close(bar.bounds.height, 35.6, "bar height");
    assert_close(bar.bounds.y, zone.bounds.y + 3.0, "bar under the border");
    let label = part(zone, PartName::Label);
    assert_close(
        label.bounds.x,
        bar.bounds.x + 16.0,
        "label inside bar padding",
    );
    assert_close(
        label.bounds.y,
        bar.bounds.y + 7.0,
        "label inside bar padding",
    );
    let label_run = label.text.as_ref().unwrap();
    assert_eq!(label_run.color, "#FFFFFF");
    let first = node(&geometry, "/body/0/children/0");
    assert_close(
        first.bounds.y,
        body.bounds.y + 16.0,
        "first child under body top padding",
    );
    assert_close(
        first.bounds.x,
        body.bounds.x + 14.0,
        "first child inside body left padding",
    );
    assert_close(
        zone.content.x,
        body.bounds.x + 14.0,
        "content is the body content box",
    );
    assert_close(
        zone.content.y,
        body.bounds.y + 16.0,
        "content is the body content box",
    );
    assert_close(
        zone.content.width,
        body.bounds.width - 28.0,
        "content width",
    );
    assert_close(
        zone.content.bottom(),
        body.bounds.bottom() - 14.0,
        "content bottom",
    );
}
