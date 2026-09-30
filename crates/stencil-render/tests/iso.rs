#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! The section 12 isometric projection: point map, solids, billboards, the SVG, shading,
//! the iso-labels-clear check and the measured JSON.

mod common;

use serde_json::{Value, json};
use stencil_layout::{BoxRect, PageGeometry, Size, TextRun};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::pointer::NodePointer;
use stencil_model::{Page, Theme};
use stencil_render::iso::{
    Billboard, BillboardRole, ISO_MARGIN_PX, IsoScene, ScreenPoint, Solid, SolidShape,
    iso_labels_clear, project_page, project_point,
};
use stencil_render::palette::{self, Face, Palette, shade};
use stencil_render::{DeviceScale, measured_json, render_png, render_svg};

const HERO_JSON: &str = include_str!("../../../examples/hero-iso.json");
const THEMES: [Theme; 3] = [Theme::Center, Theme::Dusk, Theme::Wire];
const ZERO: ScreenPoint = ScreenPoint { x: 0.0, y: 0.0 };

fn hero_page() -> Page {
    serde_json::from_str(HERO_JSON).unwrap()
}

fn hero_document() -> Value {
    serde_json::from_str(HERO_JSON).unwrap()
}

fn index_of(geometry: &PageGeometry, pointer: &str) -> usize {
    geometry
        .nodes
        .iter()
        .position(|node| node.pointer.as_str() == pointer)
        .unwrap_or_else(|| panic!("no node {pointer}"))
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 0.01,
        "{what}: {actual} is not within 0.01 of {expected}"
    );
}

fn card(fn_text: &str, icon: &str) -> Value {
    json!({ "tag": "Pcard", "icon": icon, "fn": fn_text })
}

fn iso_page(body: Value) -> Page {
    let mut document = common::page_document(body, json!([]));
    document["projection"] = json!("iso");
    serde_json::from_value(document).unwrap()
}

#[test]
fn the_point_map_matches_section_12_2() {
    let across = project_point(100.0, 0.0, 0.0, ZERO);
    assert_close(across.x, 86.60254, "x of (100, 0, 0)");
    assert_close(across.y, 50.0, "y of (100, 0, 0)");
    let down = project_point(0.0, 100.0, 0.0, ZERO);
    assert_close(down.x, -86.60254, "x of (0, 100, 0)");
    assert_close(down.y, 50.0, "y of (0, 100, 0)");
    let up = project_point(0.0, 0.0, 18.0, ZERO);
    assert_eq!(up, ScreenPoint { x: 0.0, y: -18.0 });
}

#[test]
fn a_one_zone_page_is_centered_across_and_starts_at_the_body_top() {
    let page = iso_page(json!([
        { "tag": "Zone", "kind": "region-a", "label": "Region", "children": [card("Router", "networking")] }
    ]));
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let body = geometry
        .node(&NodePointer::root().child("body"))
        .unwrap()
        .bounds;

    let mut xs: Vec<f32> = Vec::new();
    let mut ys: Vec<f32> = Vec::new();
    for solid in &scene.solids {
        xs.extend(solid.silhouette.iter().map(|point| point.x));
        ys.extend(solid.silhouette.iter().map(|point| point.y));
    }
    for billboard in &scene.billboards {
        xs.extend([billboard.screen.x, billboard.screen.right()]);
        ys.extend([billboard.screen.y, billboard.screen.bottom()]);
    }
    let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let min_y = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let max_y = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    assert!(min_x >= ISO_MARGIN_PX - 0.01, "{min_x}");
    assert_close(min_x, scene.canvas.width - max_x, "left and right margins");
    assert_close(min_y, body.y, "smallest y");
    let expected_width = geometry
        .canvas
        .width
        .max((max_x - min_x) + 2.0 * ISO_MARGIN_PX);
    assert_close(scene.canvas.width, expected_width, "canvas width");
    assert_close(
        scene.footer_shift,
        (max_y - min_y) - body.height,
        "footer shift",
    );
    assert_close(
        scene.canvas.height,
        geometry.canvas.height + scene.footer_shift,
        "canvas height",
    );
}

#[test]
fn solids_stack_by_zone_depth_and_rows_draw_none() {
    let page = iso_page(json!([
        {
            "tag": "Row",
            "children": [
                {
                    "tag": "Zone", "kind": "region-a", "label": "Outer",
                    "children": [
                        { "tag": "Zone", "kind": "subnet", "label": "Inner", "children": [card("Router", "networking")] }
                    ]
                },
                { "tag": "Col", "children": [ { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "hop" } ] }
            ]
        }
    ]));
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let solid_of = |pointer: &str| {
        let index = index_of(&geometry, pointer);
        scene.solids.iter().find(|solid| solid.node == index)
    };
    let outer = solid_of("/body/0/children/0").unwrap();
    assert_eq!(
        (outer.shape, outer.base_z, outer.height),
        (SolidShape::Slab, 0.0, 6.0)
    );
    let inner = solid_of("/body/0/children/0/children/0").unwrap();
    assert_eq!(
        (inner.shape, inner.base_z, inner.height),
        (SolidShape::Slab, 6.0, 6.0)
    );
    let card = solid_of("/body/0/children/0/children/0/children/0").unwrap();
    assert_eq!(
        (card.shape, card.base_z, card.height),
        (SolidShape::Block, 12.0, 18.0)
    );
    let pipe = solid_of("/body/0/children/1/children/0").unwrap();
    assert_eq!(
        (pipe.shape, pipe.base_z, pipe.height),
        (SolidShape::Surface, 0.0, 0.0)
    );
    assert!(solid_of("/body/0").is_none());
    assert!(solid_of("/body/0/children/1").is_none());
}

#[test]
fn a_vpc_zone_is_a_ring_on_its_parent_top_and_adds_no_height() {
    let page = iso_page(json!([
        { "tag": "Zone", "kind": "gcp", "label": "Google Cloud", "children": [
            { "tag": "Zone", "kind": "vpc", "label": "Shared VPC", "children": [
                card("Warehouse", "bigquery") ] } ] }
    ]));
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let solid_of = |pointer: &str| {
        let index = index_of(&geometry, pointer);
        scene
            .solids
            .iter()
            .find(|solid| solid.node == index)
            .unwrap()
    };
    let ring = solid_of("/body/0/children/0");
    assert_eq!(
        (ring.shape, ring.base_z, ring.height),
        (SolidShape::Slab, 6.0, 0.0)
    );
    assert!(!ring.opaque);
    let card = solid_of("/body/0/children/0/children/0");
    assert_eq!((card.base_z, card.height), (6.0, 18.0));
    assert_eq!(ring.slab_edges().len(), 4);
}

fn two_zone_link_page(to_side: &str) -> Page {
    let mut document = common::page_document(
        json!([
            {
                "tag": "Row", "gap": 32,
                "children": [
                    { "tag": "Zone", "kind": "region-a", "label": "A", "children": [
                        { "tag": "Pcard", "id": "left", "fn": "Left" } ] },
                    { "tag": "Zone", "kind": "region-b", "label": "B", "children": [
                        { "tag": "Pcard", "id": "right", "fn": "Right" } ] }
                ]
            }
        ]),
        json!([{ "kind": "blue", "text": "request path" }]),
    );
    document["links"] =
        json!([{ "from": "left", "to": "right", "kind": "blue", "to_side": to_side }]);
    document["projection"] = json!("iso");
    serde_json::from_value(document).unwrap()
}

#[test]
fn a_link_between_two_zones_steps_down_to_the_ground_and_back_up() {
    let page = two_zone_link_page("bottom");
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let path = &scene.link_paths[0];
    let heights: Vec<f32> = path.iter().map(|point| point.z).collect();
    assert_eq!(heights.first(), Some(&6.0));
    assert_eq!(heights.last(), Some(&6.0));
    assert!(heights.contains(&0.0), "{heights:?}");
    let risers = path
        .windows(2)
        .filter(|pair| pair[0].x == pair[1].x && pair[0].y == pair[1].y && pair[0].z != pair[1].z)
        .count();
    assert_eq!(risers, 2, "{path:?}");
    let route = &geometry.links[0];
    let end = route.points.last().unwrap();
    let drawn_end = path.last().unwrap();
    assert_close(drawn_end.x, end.x, "end x on a visible face");
    assert_close(drawn_end.y, end.y, "end y on a visible face");
}

#[test]
fn a_link_into_a_hidden_face_stops_where_it_meets_the_block_on_screen() {
    let page = two_zone_link_page("left");
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let right = geometry
        .node(
            &NodePointer::root()
                .child("body")
                .index(0)
                .child("children")
                .index(1)
                .child("children")
                .index(0),
        )
        .unwrap()
        .bounds;
    let drawn_end = scene.link_paths[0].last().unwrap();
    assert!(drawn_end.x < right.x - 1.0, "{drawn_end:?} {right:?}");
}

#[test]
fn every_hero_link_reaches_its_endpoints_over_the_cloud_floor() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let scene = project_page(&geometry).unwrap();
    assert_eq!(scene.link_paths.len(), 4);
    for path in &scene.link_paths[2..] {
        assert!(path.iter().all(|point| point.z == 6.0), "{path:?}");
    }
    for path in &scene.link_paths[..2] {
        assert_eq!(path.first().unwrap().z, 6.0);
        assert_eq!(path.last().unwrap().z, 6.0);
        assert!(path.iter().any(|point| point.z == 0.0), "{path:?}");
    }
}

#[test]
fn a_link_from_a_zone_to_its_own_child_rises_onto_that_zone_where_it_enters_it() {
    let mut document = common::page_document(
        json!([
            { "tag": "Zone", "kind": "gcp", "label": "Google Cloud", "children": [
                { "tag": "Zone", "kind": "region-a", "id": "reg", "label": "europe-west4",
                  "children": [
                    { "tag": "Pcard", "id": "wh", "fn": "Warehouse" } ] } ] }
        ]),
        json!([{ "kind": "blue", "text": "request path" }]),
    );
    document["links"] = json!([{ "from": "reg", "to": "wh", "kind": "blue" }]);
    document["projection"] = json!("iso");
    let page: Page = serde_json::from_value(document).unwrap();
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let path = &scene.link_paths[0];
    assert_eq!(path.last().unwrap().z, 12.0, "{path:?}");
    assert!(
        path.iter().all(|point| point.z == 6.0 || point.z == 12.0),
        "{path:?}"
    );
}

#[test]
fn project_page_asserts_the_root_and_the_body() {
    let page = hero_page();
    let mut geometry = common::layout_with_fixed_metrics(&page);
    let body = index_of(&geometry, "/body");
    geometry.nodes[body].pointer = NodePointer::root().child("bodies");
    assert!(project_page(&geometry).is_err());
    let mut geometry = common::layout_with_fixed_metrics(&page);
    geometry.nodes.remove(0);
    assert!(project_page(&geometry).is_err());
}

#[test]
fn flat_written_out_renders_the_same_bytes_as_absent() {
    for document in [
        common::G7_JSON,
        include_str!("../../../examples/hybrid-ai.json"),
        include_str!("../../../examples/network-hub-spoke.json"),
    ] {
        let absent: Page = serde_json::from_str(document).unwrap();
        let mut value: Value = serde_json::from_str(document).unwrap();
        value["projection"] = json!("flat");
        let written: Page = serde_json::from_value(value).unwrap();
        let geometry = common::layout_with_cosmic_text(&absent);
        assert_eq!(
            render_svg(&absent, &geometry).unwrap().svg,
            render_svg(&written, &geometry).unwrap().svg
        );
    }
}

#[test]
fn measured_json_without_a_scene_has_no_projection_key() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let flat = measured_json(&hero_document(), &geometry, None);
    let mut iso = measured_json(&hero_document(), &geometry, Some(&scene));
    let keys: Vec<&String> = flat.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["canvas", "document", "links", "nodes"]);
    iso.as_object_mut().unwrap().remove("projection").unwrap();
    assert_eq!(
        serde_json::to_vec_pretty(&flat).unwrap(),
        serde_json::to_vec_pretty(&iso).unwrap()
    );
}

fn element_attribute<'a>(
    node: resvg::usvg::roxmltree::Node<'a, 'a>,
    name: &str,
) -> Option<&'a str> {
    node.attribute(name)
}

#[test]
fn the_iso_svg_has_the_section_12_5_structure() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let iso = render_svg(&page, &geometry).unwrap();
    let mut flat_page = page.clone();
    flat_page.projection = stencil_model::Projection::Flat;
    let flat = render_svg(&flat_page, &geometry).unwrap();
    assert_eq!(iso.text_elements, flat.text_elements);
    render_png(&iso.svg, iso.text_elements, DeviceScale::new(1).unwrap()).unwrap();

    let document = common::parse_xml(&iso.svg);
    let data_ids: Vec<&str> = document
        .descendants()
        .filter_map(|node| element_attribute(node, "data-id"))
        .filter(|id| !id.starts_with("/links/"))
        .collect();
    let geometry_ids: Vec<&str> = geometry
        .nodes
        .iter()
        .map(|node| node.pointer.as_str())
        .collect();
    assert_eq!(data_ids, geometry_ids);
    let billboard_ids: Vec<&str> = document
        .descendants()
        .filter_map(|node| element_attribute(node, "data-billboard"))
        .collect();
    let scene_ids: Vec<&str> = scene
        .billboards
        .iter()
        .map(|billboard| billboard.owner.as_str())
        .collect();
    assert_eq!(billboard_ids, scene_ids);
    assert!(!iso.svg.contains("<marker"));
    assert!(!iso.svg.contains("<defs"));
    assert!(!iso.svg.contains("<circle"));
    let root = document.root_element();
    let last = root.children().rfind(|node| node.is_element()).unwrap();
    assert_eq!(last.attribute("data-layer"), Some("billboards"));
    let page_group = common::group(&document, "");
    assert_eq!(page_group.attribute("data-projection"), Some("iso"));

    let zone_groups: Vec<_> = document
        .descendants()
        .filter(|node| node.attribute("data-tag") == Some("Zone"))
        .collect();
    assert_eq!(zone_groups.len(), 3);
    for zone in zone_groups {
        let children: Vec<_> = zone.children().filter(|node| node.is_element()).collect();
        let first_polygon = children
            .iter()
            .position(|node| node.has_tag_name("polygon"));
        let first_group = children.iter().position(|node| node.has_tag_name("g"));
        match (first_polygon, first_group) {
            (Some(polygon), Some(group)) => assert!(polygon < group),
            other => panic!("zone {:?}: {other:?}", zone.attribute("data-id")),
        }
        assert!(
            !zone.children().any(|node| node.has_tag_name("text")),
            "a body group holds no text"
        );
    }
}

#[test]
fn pipe_dots_are_ellipses_on_the_floor() {
    let mut page: Page = serde_json::from_str(common::G7_JSON).unwrap();
    page.projection = stencil_model::Projection::Iso;
    let geometry = common::layout_with_cosmic_text(&page);
    let iso = render_svg(&page, &geometry).unwrap();
    let document = common::parse_xml(&iso.svg);
    let ellipses: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("ellipse"))
        .collect();
    assert!(!ellipses.is_empty());
    for ellipse in ellipses {
        assert_eq!(ellipse.attribute("rx"), Some("4.9"));
        assert_eq!(ellipse.attribute("ry"), Some("2.83"));
    }
}

#[test]
fn every_theme_renders_the_hero_to_png() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        render_png(&svg.svg, svg.text_elements, DeviceScale::new(1).unwrap()).unwrap();
    }
}

#[test]
fn shade_keeps_a_zero_step_and_rejects_malformed_colors() {
    assert_eq!(shade("#D2E3FC", 0).as_deref(), Some("#D2E3FC"));
    assert_eq!(shade("#12345", -8), None);
    assert_eq!(shade("red", -8), None);
    assert_eq!(shade("#GGGGGG", -8), None);
    assert_eq!(shade("#D2E3FC", -8).as_deref(), Some("#ACCBF9"));
}

#[test]
fn every_color_constant_in_the_palette_parses() {
    let source = include_str!("../src/palette.rs");
    let mut colors = 0;
    for line in source
        .lines()
        .filter(|line| line.contains("const ") && line.contains("&str = \""))
    {
        let start = line.find('"').unwrap() + 1;
        let end = start + line[start..].find('"').unwrap();
        let literal = &line[start..end];
        if literal.starts_with('#') {
            assert!(shade(literal, -8).is_some(), "{literal}");
            colors += 1;
        }
    }
    assert!(colors > 50, "found only {colors} colors");
    for reexported in [
        palette::BADGE_FILL_CUSTOMER,
        palette::BADGE_FILL_INTERNAL,
        palette::BADGE_TEXT_CUSTOMER,
        palette::BADGE_TEXT_INTERNAL,
        palette::TEXT_AMBER,
        palette::TEXT_BLUE,
        palette::TEXT_DARK,
        palette::TEXT_DENY,
        palette::TEXT_MUTED,
        palette::TEXT_WHITE,
    ] {
        assert!(shade(reexported, -8).is_some(), "{reexported}");
    }
}

#[test]
fn face_fills_follow_the_theme_steps() {
    let center = Palette::new(Theme::Center);
    assert_eq!(
        center.face_fill("#D2E3FC", Face::Top).as_deref(),
        Some("#D2E3FC")
    );
    assert_eq!(
        center.face_fill("#D2E3FC", Face::Left).as_deref(),
        Some("#ACCBF9")
    );
    assert_eq!(
        center.face_fill("#D2E3FC", Face::Right).as_deref(),
        Some("#85B3F7")
    );
    let dusk = Palette::new(Theme::Dusk);
    assert_eq!(
        dusk.face_fill("#14213A", Face::Left).as_deref(),
        Some("#0F182B")
    );
    assert_eq!(
        dusk.face_fill("#14213A", Face::Right).as_deref(),
        Some("#0A101C")
    );
    let wire = Palette::new(Theme::Wire);
    assert_eq!(wire.face_lightness_step(Face::Left), None);
    for face in [Face::Top, Face::Left, Face::Right] {
        assert_eq!(wire.face_fill("#D2E3FC", face).as_deref(), Some("#FFFFFF"));
    }
    assert_eq!(center.face_fill("blue", Face::Top), None);
}

#[test]
fn wire_faces_are_white_or_unfilled() {
    for document in [HERO_JSON, common::G7_JSON] {
        let mut page: Page = serde_json::from_str(document).unwrap();
        page.theme = Theme::Wire;
        page.projection = stencil_model::Projection::Iso;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let body = common::group(&parsed, "/body");
        let mut polygons = 0;
        for polygon in body
            .descendants()
            .filter(|node| node.has_tag_name("polygon"))
        {
            let fill = polygon.attribute("fill").unwrap();
            let is_arrowhead = polygon.attribute("stroke").is_none() && fill == "#222222";
            assert!(
                fill == "#FFFFFF" || fill == "none" || is_arrowhead,
                "fill {fill}"
            );
            polygons += 1;
        }
        assert!(polygons > 0);
    }
}

/// A gcp zone, whose opaque label chip is exempt from the slab-edge pairs, around two
/// cards.
fn two_card_zone(container: Value) -> Page {
    iso_page(json!([
        { "tag": "Zone", "kind": "gcp", "label": "Google Cloud", "children": [container] }
    ]))
}

#[test]
fn cards_side_by_side_leave_their_labels_clear() {
    let page = two_card_zone(json!({
        "tag": "Row", "gap": 32,
        "children": [card("Gateway", "cloud-run"), card("Warehouse", "bigquery")]
    }));
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 9);
    assert_eq!(report.outcome(), CheckOutcome::Passed, "{report:?}");
}

/// Cards 8 px apart leave the second card's label no clear spot on its own top face, so it
/// keeps its centered placement and the check reports it over the first block.
#[test]
fn stacked_cards_put_the_second_label_over_the_first_block() {
    let page = two_card_zone(json!({
        "tag": "Col", "gap": 8,
        "children": [card("Gateway", "cloud-run"), card("Warehouse", "bigquery")]
    }));
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 9);
    let second = "/body/0/children/0/children/1";
    let first = "/body/0/children/0/children/0";
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert_eq!(report.defects[0].pointer.as_str(), second);
    assert!(report.defects[0].message.starts_with("content billboard "));
    assert!(
        report.defects[0]
            .message
            .ends_with(&format!("covers block {first}"))
    );
}

#[test]
fn a_zone_edge_through_a_label_is_a_defect_and_one_under_a_chip_is_not() {
    let slab = Solid {
        node: 0,
        pointer: NodePointer::root().child("body").index(0),
        shape: SolidShape::Slab,
        base_z: 0.0,
        height: 0.0,
        silhouette: [
            ScreenPoint { x: 0.0, y: 5.0 },
            ScreenPoint { x: 100.0, y: 5.0 },
            ScreenPoint { x: 100.0, y: 5.0 },
            ScreenPoint { x: 100.0, y: 50.0 },
            ScreenPoint { x: 0.0, y: 50.0 },
            ScreenPoint { x: 0.0, y: 50.0 },
        ],
        opaque: false,
    };
    let label = billboard_at("label", 20.0);
    let mut chip = billboard_at("chip", 60.0);
    chip.opaque = true;
    let mut scene = scene_with(vec![label, chip]);
    scene.solids = vec![slab];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 2);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0/label");
    assert_eq!(
        report.defects[0].message,
        "content billboard 20.00,0.00 10.00x10.00 is crossed by an edge of slab /body/0"
    );
}

#[test]
fn an_edge_hidden_behind_a_later_block_does_not_cross_a_label() {
    let edge_row = [
        ScreenPoint { x: 0.0, y: 5.0 },
        ScreenPoint { x: 100.0, y: 5.0 },
        ScreenPoint { x: 100.0, y: 5.0 },
        ScreenPoint { x: 100.0, y: 50.0 },
        ScreenPoint { x: 0.0, y: 50.0 },
        ScreenPoint { x: 0.0, y: 50.0 },
    ];
    let slab = Solid {
        node: 0,
        pointer: NodePointer::root().child("body").index(0),
        shape: SolidShape::Slab,
        base_z: 0.0,
        height: 0.0,
        silhouette: edge_row,
        opaque: false,
    };
    let block = Solid {
        node: 1,
        pointer: NodePointer::root().child("body").index(1),
        shape: SolidShape::Block,
        base_z: 0.0,
        height: 18.0,
        silhouette: [
            ScreenPoint { x: 15.0, y: 0.0 },
            ScreenPoint { x: 35.0, y: 0.0 },
            ScreenPoint { x: 35.0, y: 4.0 },
            ScreenPoint { x: 35.0, y: 8.0 },
            ScreenPoint { x: 15.0, y: 8.0 },
            ScreenPoint { x: 15.0, y: 4.0 },
        ],
        opaque: true,
    };
    let mut label = billboard_at("label", 20.0);
    label.node = Some(1);
    label.marks = vec![BoxRect {
        x: 20.0,
        y: 4.0,
        width: 10.0,
        height: 2.0,
    }];
    let mut scene = scene_with(vec![label]);
    scene.solids = vec![slab, block];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 1);
    assert!(report.passed(), "{report:?}");
}

#[test]
fn a_flat_render_is_not_applicable() {
    let report = iso_labels_clear(None);
    assert_eq!(report.check, CheckName::IsoLabelsClear);
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
    assert_eq!(report.not_applicable, Some("projection is flat"));
}

fn billboard_at(owner: &str, x: f32) -> Billboard {
    let screen = BoxRect {
        x,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };
    Billboard {
        owner: NodePointer::root().child("body").index(0).child(owner),
        node: None,
        role: BillboardRole::Content,
        flat: screen,
        z: 0.0,
        screen,
        opaque: false,
        marks: vec![screen],
    }
}

fn scene_with(billboards: Vec<Billboard>) -> IsoScene {
    IsoScene {
        canvas: Size {
            width: 100.0,
            height: 100.0,
        },
        offset: ZERO,
        footer_shift: 0.0,
        solids: Vec::new(),
        billboards,
        link_paths: Vec::new(),
    }
}

#[test]
fn billboards_overlapping_by_two_hundredths_are_a_defect_and_touching_ones_are_not() {
    let overlapping = scene_with(vec![billboard_at("a", 0.0), billboard_at("b", 9.98)]);
    let report = iso_labels_clear(Some(&overlapping));
    assert_eq!(report.examined, 1);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0/b");
    assert_eq!(
        report.defects[0].message,
        "content billboard 9.98,0.00 10.00x10.00 overlaps content billboard /body/0/a"
    );

    let touching = scene_with(vec![billboard_at("a", 0.0), billboard_at("b", 10.0)]);
    let report = iso_labels_clear(Some(&touching));
    assert_eq!(report.examined, 1);
    assert!(report.passed());
}

#[test]
fn one_billboard_and_no_block_examines_nothing_and_fails() {
    let report = iso_labels_clear(Some(&scene_with(vec![billboard_at("a", 0.0)])));
    assert_eq!(report.examined, 0);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn the_hero_measured_json_is_theme_independent_and_keeps_the_flat_nodes() {
    let mut measured_outputs = Vec::new();
    let mut svg_outputs = Vec::new();
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let scene = project_page(&geometry).unwrap();
        let iso = measured_json(&hero_document(), &geometry, Some(&scene));
        let flat = measured_json(&hero_document(), &geometry, None);
        assert_eq!(
            serde_json::to_vec(&iso["nodes"]).unwrap(),
            serde_json::to_vec(&flat["nodes"]).unwrap()
        );
        assert_eq!(iso["projection"]["billboards"].as_array().unwrap().len(), 9);
        assert_eq!(iso["projection"]["kind"], "iso");
        for billboard in iso["projection"]["billboards"].as_array().unwrap() {
            let id = billboard["id"].as_str().unwrap();
            assert!(hero_document().pointer(id).is_some(), "{id}");
        }
        measured_outputs.push(serde_json::to_vec_pretty(&iso).unwrap());
        svg_outputs.push(render_svg(&page, &geometry).unwrap().svg);
    }
    assert_eq!(measured_outputs[0], measured_outputs[1]);
    assert_eq!(measured_outputs[0], measured_outputs[2]);
    assert_ne!(svg_outputs[0], svg_outputs[1]);
    assert_ne!(svg_outputs[0], svg_outputs[2]);
    assert_ne!(svg_outputs[1], svg_outputs[2]);
}

/// The first element of a billboard group, and its text elements.
fn billboard_parts<'a>(
    parsed: &'a resvg::usvg::roxmltree::Document<'a>,
    owner: &str,
) -> (
    resvg::usvg::roxmltree::Node<'a, 'a>,
    Vec<resvg::usvg::roxmltree::Node<'a, 'a>>,
) {
    let group = parsed
        .descendants()
        .find(|node| node.attribute("data-billboard") == Some(owner))
        .unwrap();
    let first = group.children().find(|node| node.is_element()).unwrap();
    let texts = group
        .descendants()
        .filter(|node| node.has_tag_name("text"))
        .collect();
    (first, texts)
}

#[test]
fn chipless_billboard_text_sits_on_a_plate_of_the_surface_under_it() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let palette = Palette::for_projection(theme, stencil_model::Projection::Iso);
        let on_prem_top = palette
            .slab_faces(stencil_model::ZoneKind::OnpremA, 0)
            .unwrap()
            .top
            .unwrap();
        let card_top = palette
            .block_faces(Some(palette.card().fill), palette.card().border)
            .unwrap()
            .top
            .unwrap();
        let gcp_top = palette
            .slab_faces(stencil_model::ZoneKind::Gcp, 0)
            .unwrap()
            .top
            .unwrap();
        for (owner, ground) in [
            ("/body/0/children/0", on_prem_top.as_str()),
            ("/body/0/children/0/children/0", card_top.as_str()),
            ("/body/0/children/1/children/0", gcp_top.as_str()),
        ] {
            let (first, texts) = billboard_parts(&parsed, owner);
            assert!(first.has_tag_name("rect"), "{theme:?} {owner}");
            assert_eq!(first.attribute("fill"), Some(ground), "{theme:?} {owner}");
            assert!(!texts.is_empty());
            assert!(texts.iter().all(|text| text.attribute("stroke").is_none()));
        }
        let (chip, _) = billboard_parts(&parsed, "/body/0/children/1");
        assert_eq!(chip.attribute("fill"), Some(palette.gcp_label_chip().fill));
    }
}

#[test]
fn zone_labels_are_drawn_in_the_primary_ink() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let primary = Palette::new(theme).text_ink(
            stencil_model::text::TextStyleName::CardFunction,
            page.canvas,
            None,
        );
        let (_, texts) = billboard_parts(&parsed, "/body/0/children/0");
        assert_eq!(texts[0].attribute("fill"), Some(primary), "{theme:?}");
    }
}

#[test]
fn a_wire_fact_block_is_outlined_although_its_flat_box_has_no_border() {
    let mut page = iso_page(json!([
        { "tag": "Zone", "kind": "region-a", "label": "Region", "children": [
            { "tag": "Fact", "text": "BGP peering" } ] }
    ]));
    page.theme = Theme::Wire;
    let geometry = common::layout_with_fixed_metrics(&page);
    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let fact = common::group(&parsed, "/body/0/children/0");
    let faces: Vec<_> = common::children_named(fact, "polygon");
    assert_eq!(faces.len(), 3);
    for face in faces {
        assert_eq!(face.attribute("stroke"), Some("#222222"));
        assert_eq!(face.attribute("stroke-width"), Some("1.25"));
    }

    page.theme = Theme::Center;
    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let fact = common::group(&parsed, "/body/0/children/0");
    for face in common::children_named(fact, "polygon") {
        assert_eq!(face.attribute("stroke"), None);
    }
}

/// The blocks, Tee hub and spine, Callout accent and Frame diagonals take paths the hero
/// does not; every example renders under iso with the flat text count.
#[test]
fn every_example_renders_under_iso_in_every_theme() {
    for document in [
        include_str!("../../../examples/onepager.json"),
        include_str!("../../../examples/stress-dense.json"),
        include_str!("../../../examples/hybrid-ai.json"),
        include_str!("../../../examples/network-hub-spoke.json"),
    ] {
        // Geometry is theme-independent (section 11.1), so one layout serves every theme.
        let geometry = common::layout_with_cosmic_text(&serde_json::from_str(document).unwrap());
        for theme in THEMES {
            let mut page: Page = serde_json::from_str(document).unwrap();
            page.theme = theme;
            let flat = render_svg(&page, &geometry).unwrap();
            page.projection = stencil_model::Projection::Iso;
            let iso = render_svg(&page, &geometry).unwrap();
            assert_eq!(iso.text_elements, flat.text_elements);
            render_png(&iso.svg, iso.text_elements, DeviceScale::new(1).unwrap()).unwrap();
        }
    }
}

/// Relative luminance of `#RRGGBB`, for ordering surfaces from dark to light.
fn luminance(color: &str) -> f64 {
    let channel = |start: usize| {
        let value = f64::from(u8::from_str_radix(&color[start..start + 2], 16).unwrap()) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

#[test]
fn dusk_surfaces_get_lighter_from_the_page_to_the_floor_to_the_blocks() {
    let dusk = Palette::for_projection(Theme::Dusk, stencil_model::Projection::Iso);
    let page = luminance(dusk.page_background());
    let floor = dusk.slab_faces(stencil_model::ZoneKind::Gcp, 0).unwrap();
    let on_prem = dusk
        .slab_faces(stencil_model::ZoneKind::OnpremA, 0)
        .unwrap();
    let region = dusk
        .slab_faces(stencil_model::ZoneKind::RegionA, 1)
        .unwrap();
    let card = dusk.card();
    let block = dusk.block_faces(Some(card.fill), card.border).unwrap();
    let floor_top = luminance(floor.top.as_deref().unwrap());
    let block_top = luminance(block.top.as_deref().unwrap());
    assert!(floor_top > page);
    assert!(luminance(on_prem.top.as_deref().unwrap()) > page);
    assert!(luminance(region.top.as_deref().unwrap()) > luminance(on_prem.top.as_deref().unwrap()));
    assert!(block_top > floor_top);
    assert!(block_top > luminance(region.top.as_deref().unwrap()));
    assert!(luminance(block.left.as_deref().unwrap()) < block_top);
    assert!(luminance(block.right.as_deref().unwrap()) < luminance(block.left.as_deref().unwrap()));
    assert!(on_prem.top_stroke.is_some());
}

#[test]
fn the_gcp_slab_carries_the_brand_on_its_sides_and_a_thin_outline_on_top() {
    for theme in [Theme::Center, Theme::Dusk] {
        let palette = Palette::for_projection(theme, stencil_model::Projection::Iso);
        let faces = palette.slab_faces(stencil_model::ZoneKind::Gcp, 0).unwrap();
        assert_eq!(faces.left.as_deref(), Some(palette::GCP_BORDER));
        assert_eq!(
            faces.right.as_deref(),
            shade(palette::GCP_BORDER, -8).as_deref()
        );
        assert_eq!(faces.side_stroke, None);
        let outline = faces.top_stroke.unwrap();
        assert_eq!(outline.width_px, palette::ISO_GCP_OUTLINE_PX);
        assert!(
            outline.width_px
                < palette
                    .wire_style(stencil_model::PipeKind::Blue)
                    .stroke
                    .width_px
        );
    }
}

#[test]
fn a_dashed_zone_border_is_drawn_once_on_the_top_face() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let ring = common::group(&parsed, "/body/0/children/1/children/0");
        let faces = common::children_named(ring, "polygon");
        assert_eq!(faces.len(), 1, "{theme:?}");
        assert!(faces[0].attribute("stroke-dasharray").is_some());
        assert_eq!(faces[0].attribute("fill"), Some("none"));
        for polygon in parsed
            .descendants()
            .filter(|node| node.has_tag_name("polygon") && node.attribute("stroke").is_some())
        {
            assert_eq!(polygon.attribute("stroke-linejoin"), Some("round"));
        }
    }
}

#[test]
fn every_theme_draws_a_ringed_icon_chip_under_iso_and_center_flat_draws_none() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let group = parsed
            .descendants()
            .find(|node| node.attribute("data-billboard") == Some("/body/0/children/0/children/0"))
            .unwrap();
        let children: Vec<_> = group.children().filter(|node| node.is_element()).collect();
        let image = children
            .iter()
            .position(|node| node.has_tag_name("image"))
            .unwrap();
        let chip = children[image - 1];
        assert!(chip.has_tag_name("rect"));
        let expected =
            Palette::for_projection(theme, stencil_model::Projection::Iso).iso_icon_chip();
        assert_eq!(chip.attribute("fill"), Some(expected.fill), "{theme:?}");
        assert_eq!(
            chip.attribute("stroke"),
            expected.border.map(|border| border.color),
            "{theme:?}"
        );
    }
    let mut flat = hero_page();
    flat.projection = stencil_model::Projection::Flat;
    let geometry = common::layout_with_cosmic_text(&flat);
    let svg = render_svg(&flat, &geometry).unwrap();
    assert!(!svg.svg.contains(r#"fill-opacity"#));
}

#[test]
fn iso_links_are_heavier_than_zone_edges_and_the_wire_legend_names_the_line() {
    let expected_width = [(Theme::Center, "3"), (Theme::Dusk, "3"), (Theme::Wire, "2")];
    for (theme, width) in expected_width {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let link = common::group(&parsed, "/links/0");
        let path = common::children_named(link, "path")[0];
        assert_eq!(path.attribute("stroke-width"), Some(width), "{theme:?}");
        let swatch = common::children_named(common::group(&parsed, "/legend/0"), "line")[0];
        assert_eq!(swatch.attribute("stroke-width"), Some(width), "{theme:?}");
        let labels: Vec<&str> = (0..3)
            .map(|index| {
                let entry = common::group(&parsed, &format!("/legend/{index}"));
                common::children_named(entry, "text")[0].text().unwrap()
            })
            .collect();
        match theme {
            Theme::Wire => assert_eq!(labels, ["Solid line", "Dashed line", "Thin line"]),
            Theme::Center | Theme::Dusk => {
                assert_eq!(labels, ["Solid blue", "Dashed blue", "Solid gray"]);
            }
        }
    }
}

#[test]
fn a_relabeled_wire_legend_keeps_the_gap_before_its_description() {
    let mut page = hero_page();
    page.theme = Theme::Wire;
    let geometry = common::layout_with_cosmic_text(&page);
    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let mut flat = page.clone();
    flat.projection = stencil_model::Projection::Flat;
    let flat_svg = render_svg(&flat, &geometry).unwrap();
    let flat_parsed = common::parse_xml(&flat_svg.svg);
    let gap = |document: &resvg::usvg::roxmltree::Document<'_>, run: &TextRun| {
        let entry = common::group(document, "/legend/2");
        let texts = common::children_named(entry, "text");
        let label_x: f32 = texts[0].attribute("x").unwrap().parse().unwrap();
        let text_x: f32 = texts[1].attribute("x").unwrap().parse().unwrap();
        text_x - label_x - run.metrics.width_px
    };
    let label_run = |label: &str| {
        let mut measurer = stencil_text::CosmicTextMeasurer::new().unwrap();
        let style = geometry
            .nodes
            .iter()
            .find(|node| node.pointer.as_str() == "/legend/2")
            .unwrap()
            .part(stencil_layout::PartName::LegendLabel)
            .unwrap()
            .text
            .clone()
            .unwrap();
        let metrics =
            stencil_model::text::TextMeasurer::measure(&mut measurer, label, &style.style, None)
                .unwrap();
        TextRun { metrics, ..style }
    };
    let relabeled_gap = gap(&parsed, &label_run("Thin line"));
    let flat_gap = gap(&flat_parsed, &label_run("Solid gray"));
    assert_close(relabeled_gap, flat_gap, "gap after the legend label");
}
