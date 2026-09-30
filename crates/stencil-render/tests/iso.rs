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
use stencil_layout::{BoxRect, PageGeometry, Size};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::pointer::NodePointer;
use stencil_model::{Page, Theme};
use stencil_render::iso::{
    Billboard, BillboardRole, ISO_MARGIN_PX, IsoScene, ScreenPoint, SolidShape, iso_labels_clear,
    project_page, project_point,
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
fn a_one_zone_page_starts_at_the_margin_and_the_body_top() {
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
    // The body's own ground corners are part of the extent too.
    for (x, y) in [
        (body.x, body.y),
        (body.right(), body.y),
        (body.x, body.bottom()),
        (body.right(), body.bottom()),
    ] {
        let corner = project_point(x, y, 0.0, scene.offset);
        xs.push(corner.x);
        ys.push(corner.y);
    }
    let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let min_y = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let max_y = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let solid_or_billboard_min_x = scene
        .solids
        .iter()
        .flat_map(|solid| solid.silhouette.iter().map(|point| point.x))
        .chain(scene.billboards.iter().map(|billboard| billboard.screen.x))
        .fold(f32::INFINITY, f32::min);
    assert_close(solid_or_billboard_min_x, ISO_MARGIN_PX, "smallest x");
    assert_close(min_x, ISO_MARGIN_PX, "smallest extent x");
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
fn a_link_lies_on_the_slab_top_of_the_innermost_zone_holding_both_ends() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let scene = project_page(&geometry).unwrap();
    assert_eq!(scene.link_planes, vec![18.0]);

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
    document["links"] = json!([{ "from": "left", "to": "right", "kind": "blue" }]);
    document["projection"] = json!("iso");
    let page: Page = serde_json::from_value(document).unwrap();
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    assert_eq!(scene.link_planes, vec![0.0]);
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
    let ellipses: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("ellipse"))
        .collect();
    assert!(!ellipses.is_empty());
    for ellipse in ellipses {
        assert_eq!(ellipse.attribute("rx"), Some("4.9"));
        assert_eq!(ellipse.attribute("ry"), Some("2.83"));
    }
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

fn two_card_zone(container: Value) -> Page {
    iso_page(json!([
        { "tag": "Zone", "kind": "region-a", "label": "Region", "children": [container] }
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
    assert_eq!(report.examined, 7);
    assert_eq!(report.outcome(), CheckOutcome::Passed, "{report:?}");
}

#[test]
fn stacked_cards_put_the_second_label_over_the_first() {
    let page = two_card_zone(json!({
        "tag": "Col", "gap": 8,
        "children": [card("Gateway", "cloud-run"), card("Warehouse", "bigquery")]
    }));
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 7);
    let second = "/body/0/children/0/children/1";
    let first = "/body/0/children/0/children/0";
    assert_eq!(report.defects.len(), 2, "{report:?}");
    assert!(
        report
            .defects
            .iter()
            .all(|defect| defect.pointer.as_str() == second)
    );
    assert!(report.defects[0].message.starts_with("content billboard "));
    assert!(
        report.defects[0]
            .message
            .ends_with(&format!("overlaps content billboard {first}"))
    );
    assert!(
        report.defects[1]
            .message
            .ends_with(&format!("covers block {first}"))
    );
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
        link_planes: Vec::new(),
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

#[test]
fn chipless_billboard_text_carries_a_page_background_halo() {
    for theme in THEMES {
        let mut page = hero_page();
        page.theme = theme;
        let geometry = common::layout_with_cosmic_text(&page);
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let background = Palette::new(theme).page_background();
        let billboard = |owner: &str| {
            parsed
                .descendants()
                .find(|node| node.attribute("data-billboard") == Some(owner))
                .unwrap()
        };
        let texts = |owner: &str| -> Vec<(Option<&str>, Option<&str>)> {
            billboard(owner)
                .descendants()
                .filter(|node| node.has_tag_name("text"))
                .map(|node| (node.attribute("stroke"), node.attribute("paint-order")))
                .collect()
        };
        for owner in ["/body/0/children/0", "/body/0/children/0/children/0"] {
            let runs = texts(owner);
            assert!(!runs.is_empty());
            assert!(
                runs.iter()
                    .all(|run| *run == (Some(background), Some("stroke"))),
                "{owner}: {runs:?}"
            );
        }
        for owner in ["/body/0/children/2", "/body/0/children/1/children/0"] {
            let runs = texts(owner);
            assert!(!runs.is_empty());
            assert!(
                runs.iter().all(|run| *run == (None, None)),
                "{owner}: {runs:?}"
            );
        }
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
