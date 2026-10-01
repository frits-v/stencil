#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 13.11: the soft shadow under every opaque isometric block.

mod common;

use common::{project_page, render_svg, with_theme};
use serde_json::json;
use stencil_model::{Page, Projection};
use stencil_render::iso::SolidShape;
use stencil_render::{DeviceScale, render_png};

const HERO_JSON: &str = include_str!("../../../examples/hero-iso.json");
const FILTER: &str = r#"<filter id="stencil-shadow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="2.5"/></filter>"#;
const SHADOW_REFERENCE: &str = r#"filter="url(#stencil-shadow)""#;

fn hero_page() -> Page {
    serde_json::from_str(HERO_JSON).unwrap()
}

/// One Item and one Note on the page, under iso.
fn lone_item_page() -> Page {
    let document = common::page_document(
        json!([{ "tag": "Row", "gap": 64, "children": [
            { "tag": "Item", "kind": "product", "icon": "gke", "title": "Cluster" },
            { "tag": "Note", "kind": "legend", "text": "a note" }
        ] }]),
        json!([]),
    );
    let mut page: Page = serde_json::from_value(document).unwrap();
    page.projection = Projection::Iso;
    page
}

#[test]
fn center_draws_one_filter_and_one_shadow_first_in_each_opaque_block() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let svg = render_svg(&page, &geometry).unwrap().svg;
    assert_eq!(svg.matches("<defs>").count(), 1);
    assert_eq!(svg.matches(FILTER).count(), 1);
    let background_end = svg.find("/>\n").unwrap();
    assert!(
        svg[background_end..]
            .trim_start_matches("/>\n")
            .trim_start()
            .starts_with("<defs>"),
        "the defs follow the background rect"
    );

    let opaque_blocks: Vec<_> = scene
        .solids
        .iter()
        .filter(|solid| solid.shape == SolidShape::Block && solid.opaque)
        .collect();
    assert_eq!(opaque_blocks.len(), 4);
    assert_eq!(svg.matches(SHADOW_REFERENCE).count(), opaque_blocks.len());

    let document = common::parse_xml(&svg);
    for block in opaque_blocks {
        let group = common::group(&document, block.pointer.as_str());
        let first = group.children().find(|node| node.is_element()).unwrap();
        assert!(first.has_tag_name("polygon"), "{}", block.pointer);
        assert_eq!(first.attribute("fill"), Some("#202124"));
        assert_eq!(first.attribute("fill-opacity"), Some("0.2"));
        assert_eq!(first.attribute("filter"), Some("url(#stencil-shadow)"));
    }
    for zone in ["/body/0/children/0/children/0", "/body/0/children/1"] {
        let group = common::group(&document, zone);
        assert!(
            !common::children_named(group, "polygon")
                .iter()
                .any(|polygon| polygon.attribute("filter").is_some()),
            "slab {zone} draws no shadow"
        );
    }
}

#[test]
fn a_note_draws_no_shadow() {
    let page = lone_item_page();
    let geometry = common::layout_with_fixed_metrics(&page);
    let svg = render_svg(&page, &geometry).unwrap().svg;
    assert_eq!(svg.matches(SHADOW_REFERENCE).count(), 1);
    let document = common::parse_xml(&svg);
    let note = common::group(&document, "/body/0/children/1");
    assert!(
        !note
            .descendants()
            .any(|node| node.attribute("filter").is_some())
    );
}

#[test]
fn wire_and_flat_renders_draw_neither_filter_nor_shadow() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let wire = render_svg(&with_theme(page.clone(), "wire"), &geometry)
        .unwrap()
        .svg;
    assert!(!wire.contains("<defs"));
    assert!(!wire.contains("stencil-shadow"));
    assert!(!wire.contains("<marker"));

    for theme in common::THEMES {
        let mut flat = with_theme(page.clone(), theme);
        flat.projection = Projection::Flat;
        let svg = render_svg(&flat, &geometry).unwrap().svg;
        assert!(!svg.contains("stencil-shadow"), "{theme}");
    }
}

#[test]
fn every_theme_but_wire_shadows_the_hero() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let mut examined = 0;
    for theme in common::THEMES {
        let svg = render_svg(&with_theme(page.clone(), theme), &geometry)
            .unwrap()
            .svg;
        assert_eq!(
            svg.matches(SHADOW_REFERENCE).count(),
            if theme == "wire" { 0 } else { 4 },
            "{theme}"
        );
        examined += 1;
    }
    assert_eq!(examined, 6);
}

#[test]
fn two_renders_are_byte_identical() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let first = render_svg(&page, &geometry).unwrap();
    let second = render_svg(&page, &geometry).unwrap();
    assert_eq!(first.svg, second.svg);
    let scale = DeviceScale::new(1).unwrap();
    let first_png = render_png(&first.svg, first.text_elements, scale).unwrap();
    let second_png = render_png(&second.svg, second.text_elements, scale).unwrap();
    assert_eq!(first_png, second_png);
}

/// The shadow shows below the block's lower silhouette edges and has faded 20 px past them.
#[test]
fn the_shadow_grounds_a_lone_item_and_stays_within_20_px() {
    let page = lone_item_page();
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let block = scene
        .solids
        .iter()
        .find(|solid| solid.shape == SolidShape::Block && solid.opaque)
        .unwrap();
    let svg = render_svg(&page, &geometry).unwrap();
    let png = render_png(&svg.svg, svg.text_elements, DeviceScale::new(1).unwrap()).unwrap();
    let pixmap = common::decode_png(&png);
    let white_at = |x: f32, y: f32| {
        let pixel = pixmap
            .pixel(x.round() as u32, y.round() as u32)
            .unwrap_or_else(|| panic!("{x},{y} is off the canvas"));
        common::white_pixel(&pixel)
    };

    // Silhouette vertices 2, 3 and 4 (section 12.3 rule 3) run along the block's base:
    // right, front and left corner.
    let base = [
        block.silhouette[2],
        block.silhouette[3],
        block.silhouette[4],
    ];
    let mut shaded_below = 0;
    let mut clear_past = 0;
    for edge in base.windows(2) {
        for step in 1..20 {
            let fraction = step as f32 / 20.0;
            let x = edge[0].x + (edge[1].x - edge[0].x) * fraction;
            let y = edge[0].y + (edge[1].y - edge[0].y) * fraction;
            if (2..=5).any(|below| !white_at(x, y + below as f32)) {
                shaded_below += 1;
            }
            assert!(white_at(x, y + 20.0), "shadow reaches {x},{} ", y + 20.0);
            clear_past += 1;
        }
    }
    assert_eq!(clear_past, 38);
    assert!(
        shaded_below >= 30,
        "only {shaded_below} of 38 points are shaded"
    );
}
