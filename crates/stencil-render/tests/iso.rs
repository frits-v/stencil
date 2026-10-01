#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! The section 12 isometric projection: point map, solids, billboards, the SVG, shading,
//! the iso-labels-clear check and the measured JSON.

mod common;

use common::{THEMES, project_page, project_zoomed, render_svg};
use serde_json::{Value, json};
use stencil_layout::{BoxRect, PageGeometry, Size, TextRun};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::grammar::Role;
use stencil_model::pointer::NodePointer;
use stencil_model::{Page, Projection, Theme};
use stencil_render::iso::{
    Billboard, BillboardRole, ISO_MARGIN_PX, ISO_ZOOM_MAX, IsoPoint, IsoScene, ScreenPoint, Solid,
    SolidShape, iso_labels_clear, iso_links_clear, project_point, zoomed_geometry,
};
use stencil_render::palette::{Face, LineUse, Palette, ZoneTab, shade};
use stencil_render::{DeviceScale, measured_json, render_png};

const HERO_JSON: &str = include_str!("../../../examples/hero-iso.json");
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
    json!({ "tag": "Item", "kind": "product", "icon": icon, "title": fn_text })
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
        { "tag": "Box", "kind": "region", "tint": 1, "label": "Region", "children": [card("Router", "networking")] }
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
                    "tag": "Box", "kind": "region", "tint": 1, "label": "Outer",
                    "children": [
                        { "tag": "Box", "kind": "subnet", "label": "Inner", "children": [card("Router", "networking")] }
                    ]
                },
                { "tag": "Col", "children": [ { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "hop" } ] }
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
        { "tag": "Box", "kind": "gcp", "label": "Google Cloud", "children": [
            { "tag": "Box", "kind": "vpc", "label": "Shared VPC", "children": [
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
                    { "tag": "Box", "kind": "region", "tint": 1, "label": "A", "children": [
                        { "tag": "Item", "kind": "product", "id": "left", "title": "Left" } ] },
                    { "tag": "Box", "kind": "region", "tint": 2, "label": "B", "children": [
                        { "tag": "Item", "kind": "product", "id": "right", "title": "Right" } ] }
                ]
            }
        ]),
        json!([{ "line": "solid", "tint": 1, "text": "request path" }]),
    );
    document["links"] =
        json!([{ "from": "left", "to": "right", "line": "solid", "tint": 1, "to_side": to_side }]);
    document["projection"] = json!("iso");
    serde_json::from_value(document).unwrap()
}

#[test]
fn a_link_between_two_zones_steps_down_to_the_ground_and_back_up() {
    let page = two_zone_link_page("bottom");
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let path = &scene.link_paths[0];
    let heights: Vec<f32> = path.iter().map(|point| point.z).collect();
    assert_eq!(heights.first(), Some(&(6.0 * zoom)));
    assert_eq!(heights.last(), Some(&(6.0 * zoom)));
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
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
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
    let floor = 6.0 * scene.zoom;
    assert_eq!(scene.link_paths.len(), 4);
    for path in &scene.link_paths[2..] {
        assert!(path.iter().all(|point| point.z == floor), "{path:?}");
    }
    for path in &scene.link_paths[..2] {
        assert_eq!(path.first().unwrap().z, floor);
        assert_eq!(path.last().unwrap().z, floor);
        assert!(path.iter().any(|point| point.z == 0.0), "{path:?}");
    }
}

#[test]
fn a_link_from_a_zone_to_its_own_child_rises_onto_that_zone_where_it_enters_it() {
    let mut document = common::page_document(
        json!([
            { "tag": "Box", "kind": "gcp", "label": "Google Cloud", "children": [
                { "tag": "Box", "kind": "region", "tint": 1, "id": "reg", "label": "europe-west4",
                  "children": [
                    { "tag": "Item", "kind": "product", "id": "wh", "title": "Warehouse" } ] } ] }
        ]),
        json!([{ "line": "solid", "tint": 1, "text": "request path" }]),
    );
    document["links"] = json!([{ "from": "reg", "to": "wh", "line": "solid", "tint": 1 }]);
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
    assert_eq!(zone_groups.len(), 4);
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
        page.theme = Some(theme.to_string());
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
fn every_color_of_every_builtin_theme_shades() {
    fn collect<'a>(value: &'a Value, colors: &mut Vec<&'a str>) {
        match value {
            Value::String(text) if text.starts_with('#') => colors.push(text),
            Value::Array(items) => items.iter().for_each(|item| collect(item, colors)),
            Value::Object(fields) => fields.values().for_each(|field| collect(field, colors)),
            _ => {}
        }
    }
    for name in THEMES {
        let value = serde_json::to_value(common::theme(name)).unwrap();
        let mut colors = Vec::new();
        collect(&value, &mut colors);
        assert!(
            colors.len() > 50,
            "{name}: found only {} colors",
            colors.len()
        );
        for color in colors {
            assert!(shade(color, -8).is_some(), "{name} {color}");
        }
    }
}

#[test]
fn face_fills_follow_the_theme_steps() {
    let center_theme = common::theme("center");
    let center = Palette::new(&center_theme, Projection::Iso);
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
    let dusk_theme = common::theme("dusk");
    let dusk = Palette::new(&dusk_theme, Projection::Iso);
    assert_eq!(dusk.face_lightness_step(Face::Left), -4);
    assert_eq!(dusk.face_lightness_step(Face::Right), -8);
    assert_eq!(
        dusk.face_fill("#14213A", Face::Left).as_deref(),
        Some("#0F182B")
    );
    assert_eq!(
        dusk.face_fill("#14213A", Face::Right).as_deref(),
        Some("#0A101C")
    );
    let wire_theme = common::theme("wire");
    let wire = Palette::new(&wire_theme, Projection::Iso);
    for face in [Face::Top, Face::Left, Face::Right] {
        assert_eq!(wire.face_lightness_step(face), 0);
        assert_eq!(wire.face_fill("#FFFFFF", face).as_deref(), Some("#FFFFFF"));
    }
    assert_eq!(center.face_fill("blue", Face::Top), None);
}

#[test]
fn wire_faces_are_white_or_unfilled() {
    for document in [HERO_JSON, common::G7_JSON] {
        let mut page: Page = serde_json::from_str(document).unwrap();
        page.theme = Some("wire".to_string());
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
        { "tag": "Box", "kind": "gcp", "label": "Google Cloud", "children": [container] }
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
/// keeps its first placement and the check reports it over the first block.
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
    assert!(
        report
            .defects
            .iter()
            .any(|defect| defect.pointer.as_str() == second
                && defect.message.starts_with("content billboard ")
                && defect.message.ends_with(&format!("covers block {first}"))),
        "{report:?}"
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
        footprint: NO_FOOTPRINT,
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
        footprint: NO_FOOTPRINT,
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
        footprint: NO_FOOTPRINT,
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
        contained: false,
    }
}

/// A flat footprint far from every hand-built scene's links.
const NO_FOOTPRINT: BoxRect = BoxRect {
    x: -1000.0,
    y: -1000.0,
    width: 1.0,
    height: 1.0,
};

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
        link_kinds: Vec::new(),
        zoom: 1.0,
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
        page.theme = Some(theme.to_string());
        let geometry = common::layout_with_cosmic_text(&page);
        let scene = project_page(&geometry).unwrap();
        let iso = measured_json(&hero_document(), &geometry, Some(&scene));
        let flat = measured_json(&hero_document(), &geometry, None);
        assert_eq!(
            serde_json::to_vec(&iso["nodes"]).unwrap(),
            serde_json::to_vec(&flat["nodes"]).unwrap()
        );
        assert_eq!(
            iso["projection"]["billboards"].as_array().unwrap().len(),
            10
        );
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

const HERO_ON_PREM: &str = "/body/0/children/0/children/0";
const HERO_ROUTER: &str = "/body/0/children/0/children/0/children/0";
const HERO_GCP: &str = "/body/0/children/1";
const HERO_VPC: &str = "/body/0/children/1/children/0/children/0";
const HERO_APIS: &str = "/body/0/children/1/children/0/children/1";

#[test]
fn card_text_sits_on_a_plate_on_a_light_page_and_on_none_on_a_dark_one() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = Some(theme.to_string());
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let theme_data = common::theme(theme);
        let palette = Palette::new(&theme_data, Projection::Iso);
        let card_top = palette
            .block_faces(Some(palette.card().fill), palette.card().border)
            .unwrap()
            .top
            .unwrap();
        let (first, texts) = billboard_parts(&parsed, HERO_ROUTER);
        assert!(!texts.is_empty());
        assert!(texts.iter().all(|text| text.attribute("stroke").is_none()));
        let plated =
            first.has_tag_name("rect") && first.attribute("fill") == Some(card_top.as_str());
        let dark = ["dusk", "clear-dark"].contains(&theme);
        assert_eq!(plated, !dark, "{theme:?}");
        assert_eq!(palette.iso_text_plates(), !dark);
    }
}

#[test]
fn zone_tabs_are_filled_at_the_top_level_and_outlined_when_nested() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = Some(theme.to_string());
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let theme_data = common::theme(theme);
        let palette = Palette::new(&theme_data, Projection::Iso);
        for (owner, kind) in [
            (HERO_ON_PREM, common::zone("onprem", Some(1))),
            (HERO_GCP, common::zone("gcp", None)),
        ] {
            let ZoneTab::Filled { fill, ink } = palette.iso_zone_tab(kind.0, kind.1, false) else {
                panic!("a top-level tab is filled");
            };
            let (tab, texts) = billboard_parts(&parsed, owner);
            assert!(tab.has_tag_name("rect"), "{theme:?} {owner}");
            assert_eq!(tab.attribute("fill"), Some(fill), "{theme:?} {owner}");
            assert!(tab.attribute("stroke").is_none(), "{theme:?} {owner}");
            assert_eq!(texts[0].attribute("fill"), Some(ink), "{theme:?} {owner}");
        }
        let gcp_top = palette
            .slab_faces(common::zone("gcp", None).0, common::zone("gcp", None).1)
            .unwrap()
            .top
            .unwrap();
        let ZoneTab::Outline { border, ink } =
            palette.iso_zone_tab(common::zone("vpc", None).0, None, true)
        else {
            panic!("a nested tab is an outline");
        };
        let (tab, texts) = billboard_parts(&parsed, HERO_VPC);
        assert_eq!(tab.attribute("fill"), Some(gcp_top.as_str()), "{theme:?}");
        assert_eq!(tab.attribute("stroke"), Some(border.color), "{theme:?}");
        assert_eq!(texts[0].attribute("fill"), Some(ink), "{theme:?}");
    }
}

#[test]
fn every_zone_tab_hangs_from_its_zone_back_corner() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    for owner in [HERO_ON_PREM, HERO_GCP, HERO_VPC, HERO_APIS] {
        let solid = scene
            .solids
            .iter()
            .find(|solid| solid.pointer.as_str() == owner)
            .unwrap();
        let tab = scene
            .billboards
            .iter()
            .find(|billboard| billboard.owner.as_str() == owner)
            .unwrap();
        let back = solid.silhouette[0];
        assert!(
            (tab.screen.x - back.x).abs() < 1e-3,
            "{owner} {tab:?} {back:?}"
        );
        let middle = tab.screen.y + tab.screen.height / 2.0;
        assert!((middle - back.y).abs() < 1e-3, "{owner} {tab:?} {back:?}");
        assert!(tab.opaque);
    }
}

#[test]
fn a_wire_fact_block_is_outlined_although_its_flat_box_has_no_border() {
    let mut page = iso_page(json!([
        { "tag": "Box", "kind": "region", "tint": 1, "label": "Region", "children": [
            { "tag": "Fact", "text": "BGP peering" } ] }
    ]));
    page.theme = Some("wire".to_string());
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

    page.theme = Some("center".to_string());
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
            page.theme = Some(theme.to_string());
            let flat = render_svg(&page, &geometry).unwrap();
            page.projection = stencil_model::Projection::Iso;
            let iso = render_svg(&page, &geometry).unwrap();
            assert_eq!(iso.text_elements, flat.text_elements);
            render_png(&iso.svg, iso.text_elements, DeviceScale::new(1).unwrap()).unwrap();
        }
    }
}

/// CIELAB L* of `#RRGGBB`.
fn lightness(color: &str) -> f64 {
    stencil_model::theme::lightness(&stencil_model::theme::Color(color.to_string())).unwrap()
}

/// Section 13.5's widened ladder: page, frame body, tone fills, card, fact box. Tint fills
/// stand outside it, so a slab or block on a tinted floor is not compared with that floor.
#[test]
fn dark_iso_surfaces_climb_the_ladder_and_every_side_is_darker_than_its_top() {
    for name in ["dusk", "clear-dark"] {
        let theme = common::theme(name);
        let palette = Palette::new(&theme, Projection::Iso);
        let mut page = common::with_theme(hero_page(), name);
        page.projection = Projection::Iso;
        let geometry = common::layout_with_cosmic_text(&page);
        // The top each node stands on, and whether that top is a ladder rung (the page or an
        // untinted Box) rather than a tint fill.
        let mut grounds: Vec<(String, bool)> = Vec::with_capacity(geometry.nodes.len());
        let mut slabs = 0;
        let mut blocks = 0;
        for node in &geometry.nodes {
            let ground = node.parent.map_or_else(
                || (theme.page.as_str().to_string(), true),
                |parent| grounds[parent].clone(),
            );
            let mut own = ground.clone();
            let context = format!("{name} {}", node.pointer);
            if let Some(look) = node.container {
                let faces = palette.slab_faces(look, node.tint).unwrap();
                if let Some(top) = faces.top.as_deref() {
                    if ground.1 {
                        assert!(lightness(top) > lightness(&ground.0), "{context} top");
                    }
                    let left = faces.left.as_deref().unwrap();
                    let right = faces.right.as_deref().unwrap();
                    if look.role != Role::Frame {
                        assert!(lightness(left) < lightness(top), "{context} left");
                    }
                    assert!(lightness(right) < lightness(left), "{context} right");
                    let tinted = look.role == Role::Group && node.tint.is_some();
                    own = (top.to_string(), !tinted);
                    slabs += 1;
                }
            } else if node.tag == stencil_layout::NodeTag::Pcard {
                let card = palette.card();
                let faces = palette.block_faces(Some(card.fill), card.border).unwrap();
                let top = faces.top.as_deref().unwrap();
                assert!(
                    lightness(top) > lightness(theme.frame.body_fill.as_str()),
                    "{context} item over the frame body"
                );
                for (_, tone) in theme.tones.all() {
                    if let Some(fill) = &tone.fill {
                        assert!(lightness(top) > lightness(fill.as_str()), "{context}");
                    }
                }
                let left = faces.left.as_deref().unwrap();
                let right = faces.right.as_deref().unwrap();
                assert!(lightness(left) < lightness(top), "{context} left");
                assert!(lightness(right) < lightness(left), "{context} right");
                blocks += 1;
            }
            grounds.push(own);
        }
        assert!(slabs >= 2, "{name}: {slabs} slabs");
        assert!(blocks >= 4, "{name}: {blocks} blocks");
        assert!(lightness(theme.fact.fill.as_str()) > lightness(theme.card.fill.as_str()));
    }
}

#[test]
fn the_frame_slab_carries_the_frame_sides_and_a_thin_outline_on_top() {
    for name in THEMES {
        let theme = common::theme(name);
        let palette = Palette::new(&theme, Projection::Iso);
        let faces = palette
            .slab_faces(common::zone("gcp", None).0, common::zone("gcp", None).1)
            .unwrap();
        match &theme.iso.frame_sides {
            Some(sides) => {
                assert_eq!(faces.left.as_deref(), Some(sides.left.as_str()), "{name}");
                assert_eq!(faces.right.as_deref(), Some(sides.right.as_str()), "{name}");
                assert_eq!(sides.left, theme.frame.border, "{name}");
                assert_eq!(
                    Some(sides.right.as_str()),
                    shade(theme.frame.border.as_str(), -12).as_deref(),
                    "{name}"
                );
                assert_eq!(faces.side_stroke, None, "{name}");
            }
            None => {
                assert_eq!(faces.left, faces.top, "{name}: faces 0 shade nothing");
                assert!(faces.side_stroke.is_some(), "{name}");
            }
        }
        let outline = faces.top_stroke.unwrap();
        assert_eq!(outline.width_px, theme.iso.frame_outline, "{name}");
        assert!(
            outline.width_px
                < palette
                    .wire_style(LineUse::new(stencil_model::Line::Solid, Some(1)))
                    .stroke
                    .width_px
        );
    }
    let center = common::theme("center");
    let sides = center.iso.frame_sides.as_ref().unwrap();
    assert_eq!(sides.left.as_str(), "#1A73E8");
    assert_eq!(sides.right.as_str(), "#1257B3");
}

#[test]
fn a_dashed_zone_border_is_drawn_once_on_the_top_face() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = Some(theme.to_string());
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let ring = common::group(&parsed, HERO_VPC);
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
        page.theme = Some(theme.to_string());
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let group = parsed
            .descendants()
            .find(|node| node.attribute("data-billboard") == Some(HERO_ROUTER))
            .unwrap();
        let children: Vec<_> = group.children().filter(|node| node.is_element()).collect();
        let image = children
            .iter()
            .position(|node| node.has_tag_name("image"))
            .unwrap();
        let chip = children[image - 1];
        assert!(chip.has_tag_name("rect"));
        let theme_data = common::theme(theme);
        let expected = Palette::new(&theme_data, Projection::Iso).iso_icon_chip();
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
fn iso_link_weights_rank_primary_failover_and_service_and_the_wire_legend_names_the_line() {
    assert_eq!(
        widths_of(&common::theme("center")),
        ["3.75".to_string(), "2.5".to_string(), "2".to_string()]
    );
    assert_eq!(
        widths_of(&common::theme("wire")),
        ["3.75".to_string(), "2.5".to_string(), "1.25".to_string()]
    );
    for theme in THEMES {
        let theme_data = common::theme(theme);
        let widths = widths_of(&theme_data);
        let mut page = hero_page();
        page.theme = Some(theme.to_string());
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        for (index, width) in widths.iter().enumerate() {
            let link = common::group(&parsed, &format!("/links/{index}"));
            let path = common::children_named(link, "path")[0];
            assert_eq!(
                path.attribute("stroke-width"),
                Some(width.as_str()),
                "{theme:?}"
            );
            let entry = common::group(&parsed, &format!("/legend/{index}"));
            let swatch = common::children_named(entry, "line")[0];
            assert_eq!(
                swatch.attribute("stroke-width"),
                Some(width.as_str()),
                "{theme:?}"
            );
        }
        let labels: Vec<&str> = (0..3)
            .map(|index| {
                let entry = common::group(&parsed, &format!("/legend/{index}"));
                common::children_named(entry, "text")[0].text().unwrap()
            })
            .collect();
        if theme == "wire" {
            assert_eq!(labels, ["Solid line", "Dashed line", "Thin line"]);
        } else {
            assert_eq!(
                labels,
                ["Solid blue", "Dashed blue", "Solid gray"],
                "{theme}"
            );
        }
    }
}

/// The hero's three link widths under a theme: solid slot 1, dash, gray.
fn widths_of(theme: &Theme) -> [String; 3] {
    let widths = &theme.iso.widths;
    [widths.primary, widths.secondary, widths.gray]
        .map(|width| stencil_render::format_number(width).to_string())
}

#[test]
fn a_relabeled_wire_legend_keeps_the_gap_before_its_description() {
    let mut page = hero_page();
    page.theme = Some("wire".to_string());
    let geometry = common::layout_with_cosmic_text(&page);
    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let mut flat = page.clone();
    flat.projection = stencil_model::Projection::Flat;
    flat.theme = None;
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

#[test]
fn a_narrow_body_is_zoomed_to_the_cap_with_heights_and_text_sizes_kept_in_proportion() {
    let page = iso_page(json!([
        { "tag": "Row", "grow": [0], "children": [
            { "tag": "Box", "kind": "region", "tint": 1, "label": "Region", "children": [
                card("Gateway", "cloud-run") ] } ] }
    ]));
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    assert_eq!(scene.zoom, ISO_ZOOM_MAX);
    let card_pointer = "/body/0/children/0/children/0";
    let block = scene
        .solids
        .iter()
        .find(|solid| solid.pointer.as_str() == card_pointer)
        .unwrap();
    assert_close(block.height, 18.0 * ISO_ZOOM_MAX, "block height");
    let flat_card = &geometry.nodes[index_of(&geometry, card_pointer)];
    assert_close(
        block.footprint.width,
        flat_card.bounds.width * ISO_ZOOM_MAX,
        "footprint width",
    );
    let billboard = scene
        .billboards
        .iter()
        .find(|billboard| billboard.owner.as_str() == card_pointer)
        .unwrap();
    let icon = flat_card.part(stencil_layout::PartName::Icon).unwrap();
    assert!(
        billboard.screen.width < flat_card.bounds.width,
        "{billboard:?}"
    );
    assert!(billboard.screen.width >= icon.bounds.width);
    let (zoomed, _) = zoomed_geometry(&geometry).unwrap();
    let zoomed_icon = zoomed.nodes[index_of(&zoomed, card_pointer)]
        .part(stencil_layout::PartName::Icon)
        .unwrap()
        .bounds;
    let top = project_point(
        block.footprint.x + block.footprint.width / 2.0,
        block.footprint.y + block.footprint.height / 2.0,
        block.base_z + block.height,
        scene.offset,
    );
    let icon_x = billboard.screen.x + zoomed_icon.x - billboard.flat.x + zoomed_icon.width / 2.0;
    let icon_y = billboard.screen.y + zoomed_icon.y - billboard.flat.y + zoomed_icon.height / 2.0;
    assert_close(icon_x, top.x, "icon center x on the top face center");
    assert_close(icon_y, top.y, "icon center y on the top face center");
}

#[test]
fn a_wide_body_is_not_zoomed() {
    let page = two_zone_link_page("bottom");
    let mut document = serde_json::to_value(&page).unwrap();
    document["width"] = json!(640);
    let page: Page = serde_json::from_value(document).unwrap();
    let geometry = common::layout_with_fixed_metrics(&page);
    let (zoomed, zoom) = zoomed_geometry(&geometry).unwrap();
    if zoom == 1.0 {
        assert_eq!(zoomed, geometry);
    }
    assert!((1.0..=ISO_ZOOM_MAX).contains(&zoom));
}

#[test]
fn a_card_icon_stands_on_its_block_top_center_with_the_name_under_it() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let (zoomed, zoom) = zoomed_geometry(&geometry).unwrap();
    let scene = project_zoomed(&zoomed, zoom).unwrap();
    for owner in [
        "/body/0/children/1/children/0/children/0/children/0/children/0/children/0",
        "/body/0/children/1/children/0/children/0/children/0/children/1/children/0",
        "/body/0/children/1/children/0/children/1/children/0",
    ] {
        let node = &zoomed.nodes[index_of(&zoomed, owner)];
        let block = scene
            .solids
            .iter()
            .find(|solid| solid.pointer.as_str() == owner)
            .unwrap();
        let billboard = scene
            .billboards
            .iter()
            .find(|billboard| billboard.owner.as_str() == owner)
            .unwrap();
        let icon = node.part(stencil_layout::PartName::Icon).unwrap().bounds;
        let name = node
            .part(stencil_layout::PartName::FunctionName)
            .unwrap()
            .bounds;
        assert!(name.y > icon.bottom(), "{owner}");
        let icon_center = (
            billboard.screen.x + icon.x - billboard.flat.x + icon.width / 2.0,
            billboard.screen.y + icon.y - billboard.flat.y + icon.height / 2.0,
        );
        let top_z = block.base_z + block.height;
        let corner = |x: f32, y: f32| project_point(x, y, top_z, scene.offset);
        let face = block.footprint;
        let polygon = [
            corner(face.x, face.y),
            corner(face.right(), face.y),
            corner(face.right(), face.bottom()),
            corner(face.x, face.bottom()),
        ];
        let inside = (0..4).all(|index| {
            let start = polygon[index];
            let end = polygon[(index + 1) % 4];
            (end.x - start.x) * (icon_center.1 - start.y)
                - (end.y - start.y) * (icon_center.0 - start.x)
                >= 0.0
        });
        assert!(
            inside,
            "{owner}: icon {icon_center:?} off the top face {polygon:?}"
        );
    }
}

#[test]
fn the_hero_primary_is_one_straight_leg_with_its_tag_on_it() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    assert!(geometry.links[0].points.len() > 2);
    let scene = project_page(&geometry).unwrap();
    let primary = &scene.link_paths[0];
    let first = primary.first().unwrap();
    assert!(
        primary.iter().all(|point| (point.y - first.y).abs() < 1e-3),
        "{primary:?}"
    );
    let tag = scene
        .billboards
        .iter()
        .find(|billboard| billboard.owner.as_str() == "/links/0")
        .unwrap();
    let center = ScreenPoint {
        x: tag.screen.x + tag.screen.width / 2.0,
        y: tag.screen.y + tag.screen.height / 2.0,
    };
    let on_path = primary.windows(2).any(|pair| {
        let start = project_point(pair[0].x, pair[0].y, pair[0].z, scene.offset);
        let end = project_point(pair[1].x, pair[1].y, pair[1].z, scene.offset);
        let cross =
            (end.x - start.x) * (center.y - start.y) - (end.y - start.y) * (center.x - start.x);
        let length = (end.x - start.x).hypot(end.y - start.y);
        length > 0.0
            && (cross / length).abs() < 0.5
            && center.x >= start.x.min(end.x) - 0.5
            && center.x <= start.x.max(end.x) + 0.5
    });
    assert!(on_path, "{tag:?} {primary:?}");
}

#[test]
fn the_hero_links_clear_every_zone_edge_and_end_on_long_legs() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let scene = project_page(&geometry).unwrap();
    let report = iso_links_clear(Some(&scene));
    assert_eq!(report.check, CheckName::IsoLinksClear);
    assert_eq!(report.examined, 6);
    assert!(report.passed(), "{report:?}");
    assert_eq!(
        iso_links_clear(None).not_applicable,
        Some("projection is flat")
    );
}

fn link_scene(path: Vec<IsoPoint>, zone: BoxRect) -> IsoScene {
    let mut scene = scene_with(Vec::new());
    scene.solids = vec![Solid {
        node: 0,
        pointer: NodePointer::root().child("body").index(0),
        shape: SolidShape::Slab,
        base_z: 0.0,
        height: 6.0,
        silhouette: [ZERO; 6],
        opaque: true,
        footprint: zone,
    }];
    scene.link_paths = vec![path];
    scene.link_kinds = vec![(stencil_model::Line::Solid, Some(1))];
    scene
}

fn flat_point(x: f32, y: f32) -> IsoPoint {
    IsoPoint { x, y, z: 0.0 }
}

#[test]
fn a_leg_beside_a_zone_edge_a_leg_that_turns_back_and_a_short_last_leg_are_defects() {
    let zone = BoxRect {
        x: 100.0,
        y: 0.0,
        width: 200.0,
        height: 200.0,
    };
    let beside = link_scene(
        vec![
            flat_point(0.0, 50.0),
            flat_point(90.0, 50.0),
            flat_point(90.0, 150.0),
            flat_point(150.0, 150.0),
        ],
        zone,
    );
    let report = iso_links_clear(Some(&beside));
    assert_eq!(report.examined, 3);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert_eq!(report.defects[0].pointer.as_str(), "/links/0");
    assert_eq!(
        report.defects[0].message,
        "leg 1 runs 10.00 px beside an edge of zone /body/0, closer than 24"
    );

    let hairpin = link_scene(
        vec![
            flat_point(0.0, 250.0),
            flat_point(400.0, 250.0),
            flat_point(360.0, 250.0),
        ],
        zone,
    );
    let report = iso_links_clear(Some(&hairpin));
    let messages: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.message.as_str())
        .collect();
    assert_eq!(messages, ["leg 1 turns back on the leg before it"]);

    let short = link_scene(vec![flat_point(0.0, 250.0), flat_point(30.0, 250.0)], zone);
    let report = iso_links_clear(Some(&short));
    assert_eq!(
        report.defects[0].message,
        "last leg is 30.00 px, shorter than two arrowheads (36.00 px)"
    );
}

#[test]
fn text_that_leaves_its_block_is_a_defect() {
    let block = Solid {
        node: 0,
        pointer: NodePointer::root().child("body").index(0),
        shape: SolidShape::Block,
        base_z: 0.0,
        height: 18.0,
        silhouette: [
            ScreenPoint { x: 0.0, y: 0.0 },
            ScreenPoint { x: 40.0, y: 0.0 },
            ScreenPoint { x: 40.0, y: 20.0 },
            ScreenPoint { x: 40.0, y: 40.0 },
            ScreenPoint { x: 0.0, y: 40.0 },
            ScreenPoint { x: 0.0, y: 20.0 },
        ],
        opaque: true,
        footprint: NO_FOOTPRINT,
    };
    let mut inside = billboard_at("text", 5.0);
    inside.node = Some(0);
    inside.contained = true;
    inside.marks = vec![BoxRect {
        x: 5.0,
        y: 5.0,
        width: 20.0,
        height: 10.0,
    }];
    let mut outside = inside.clone();
    outside.marks[0].width = 50.0;
    for (billboard, defects) in [(inside, 0), (outside, 1)] {
        let mut scene = scene_with(vec![billboard]);
        scene.solids = vec![block.clone()];
        let report = iso_labels_clear(Some(&scene));
        assert_eq!(report.defects.len(), defects, "{report:?}");
        if defects == 1 {
            assert!(report.defects[0].message.ends_with("leaves its block"));
        }
    }
}

#[test]
fn a_dashed_link_skips_its_risers_and_a_solid_one_draws_them() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let svg = render_svg(&hero, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let data = |index: usize| {
        let link = common::group(&parsed, &format!("/links/{index}"));
        common::children_named(link, "path")[0]
            .attribute("d")
            .unwrap()
            .to_string()
    };
    assert!(data(1).matches(" M ").count() >= 2, "{}", data(1));
    assert_eq!(data(0).matches(" M ").count(), 0, "{}", data(0));
    let heads = |index: usize| {
        let link = common::group(&parsed, &format!("/links/{index}"));
        common::children_named(link, "polygon").len()
    };
    assert_eq!(heads(0), 1);
    assert_eq!(heads(1), 1);
}

#[test]
fn the_wire_vpc_ring_is_dotted_light_gray_and_solid_slab_outlines_are_heavier() {
    let wire = common::theme("wire");
    let palette = Palette::new(&wire, Projection::Iso);
    let ring = palette
        .slab_faces(common::zone("vpc", None).0, common::zone("vpc", None).1)
        .unwrap()
        .top_stroke
        .unwrap();
    assert_eq!(ring.line, stencil_render::palette::LineStyle::Dotted);
    assert_eq!(ring.color, "#999999");
    assert_eq!(ring.width_px, 1.5);
    let on_prem = palette
        .slab_faces(
            common::zone("onprem", Some(1)).0,
            common::zone("onprem", Some(1)).1,
        )
        .unwrap();
    assert_eq!(on_prem.top_stroke.unwrap().width_px, wire.iso.edge_width);
    assert_eq!(on_prem.side_stroke.unwrap().width_px, wire.iso.edge_width);
    let flat = Palette::new(&wire, Projection::Flat).zone_style(common::zone("vpc", None).0, None);
    assert_eq!(
        flat.border.unwrap().line,
        stencil_render::palette::LineStyle::Dashed
    );
}

#[test]
fn a_thicker_slab_raises_every_slab_and_grows_the_canvas() {
    let page = hero_page();
    let geometry = common::layout_with_fixed_metrics(&page);
    let six = stencil_render::iso::project_page(
        &geometry,
        &stencil_render::iso::SolidInputs::new(&geometry, 6.0),
    )
    .unwrap();
    let eight = stencil_render::iso::project_page(
        &geometry,
        &stencil_render::iso::SolidInputs::new(&geometry, 8.0),
    )
    .unwrap();
    let (_, zoom) = zoomed_geometry(&geometry).unwrap();
    let slabs = |scene: &IsoScene| -> Vec<f32> {
        scene
            .solids
            .iter()
            .filter(|solid| solid.shape == SolidShape::Slab && solid.height > 0.0)
            .map(|solid| solid.height)
            .collect()
    };
    assert!(!slabs(&six).is_empty());
    for height in slabs(&six) {
        assert_close(height, 6.0 * zoom, "slab at 6");
    }
    for height in slabs(&eight) {
        assert_close(height, 8.0 * zoom, "slab at 8");
    }
    assert!(eight.canvas.height > six.canvas.height);
}

#[test]
fn the_vpc_is_a_ring_under_every_theme() {
    for name in THEMES {
        let page = common::with_theme(hero_page(), name);
        let geometry = common::layout_with_fixed_metrics(&page);
        let inputs = common::solid_inputs(&page, &geometry);
        let vpc = index_of(&geometry, HERO_VPC);
        assert!(inputs.rings[vpc], "{name}");
        assert_eq!(
            inputs.rings.iter().filter(|ring| **ring).count(),
            1,
            "{name}"
        );
        let scene = stencil_render::iso::project_page(&geometry, &inputs).unwrap();
        let solid = scene.solids.iter().find(|solid| solid.node == vpc).unwrap();
        assert_eq!(solid.height, 0.0, "{name}");
    }
}
