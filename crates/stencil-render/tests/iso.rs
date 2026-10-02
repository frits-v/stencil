#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! The section 12 isometric projection: point map, solids, labels on their planes, the SVG,
//! shading, the iso-labels-clear check and the measured JSON.

mod common;

use common::{THEMES, project_page, project_zoomed, render_svg};
use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{
    BoxRect, ISO_TYPE_SCALE, ISO_ZONE_LABEL_SCALE, NodeTag, PageGeometry, PartName, Size, TextRun,
    turned_box, unturned_box,
};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::grammar::Role;
use stencil_model::pointer::NodePointer;
use stencil_model::{Line, Page, Projection, Shape, Theme};
use stencil_render::iso::{
    Axis, ISO_MARGIN_PX, ISO_TUBE_RADIUS_PX, ISO_ZOOM_MAX, IsoPoint, IsoScene, Label, LabelParts,
    PlaneMap, ScreenPoint, Solid, SolidShape, iso_labels_clear, iso_link_ends, iso_links_apart,
    iso_links_clear, link_tube_radius, project_point, zoomed_geometry,
};
use stencil_render::palette::{Face, LineUse, Palette, shade};
use stencil_render::{DeviceScale, format_number, measured_json, render_png};

const HERO_JSON: &str = include_str!("../../../examples/hero-iso.json");
const PLATFORM_JSON: &str = include_str!("../../../examples/platform-iso.json");
const PEOPLE_JSON: &str = include_str!("../../../examples/people-iso.json");
const ZERO: ScreenPoint = ScreenPoint { x: 0.0, y: 0.0 };

const HERO_ON_PREM: &str = "/body/0/children/0/children/0";
const HERO_ROUTER: &str = "/body/0/children/0/children/0/children/0";
const HERO_GCP: &str = "/body/0/children/1";
const HERO_VPC: &str = "/body/0/children/1/children/0/children/0";
const HERO_APIS: &str = "/body/0/children/1/children/0/children/1";
const HERO_GATEWAY: &str =
    "/body/0/children/1/children/0/children/0/children/0/children/0/children/0";
const HERO_WAREHOUSE: &str = "/body/0/children/1/children/0/children/1/children/0";
const HERO_ZONES: [&str; 4] = [HERO_ON_PREM, HERO_GCP, HERO_VPC, HERO_APIS];

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

/// `item` with its own `shape`, which outranks the shape of its icon's product row.
fn with_shape(mut item: Value, shape: &str) -> Value {
    item["shape"] = json!(shape);
    item
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
    for label in &scene.labels {
        xs.extend(label.corners.iter().map(|corner| corner.x));
        ys.extend(label.corners.iter().map(|corner| corner.y));
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
    // The networking row stands its item as a tile.
    let tile = solid_of("/body/0/children/0/children/0/children/0").unwrap();
    assert_eq!(
        (tile.shape, tile.base_z, tile.height),
        (SolidShape::Block, 12.0, 6.0)
    );
    // A pipe is a surface whose height is the tube's diameter, so its tag lies on the tube.
    let pipe = solid_of("/body/0/children/1/children/0").unwrap();
    assert_eq!(
        (pipe.shape, pipe.base_z, pipe.height),
        (SolidShape::Surface, 0.0, 2.0 * ISO_TUBE_RADIUS_PX)
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
    // The bigquery row stands its item as a cylinder, 36 high.
    let cylinder = solid_of("/body/0/children/0/children/0");
    assert_eq!((cylinder.base_z, cylinder.height), (6.0, 36.0));
    assert_eq!(ring.slab_edges().len(), 4);
}

fn two_zone_link_page(to_side: &str) -> Page {
    let mut document = common::page_document(
        json!([
            {
                "tag": "Row", "gap": 32,
                "children": [
                    { "tag": "Box", "kind": "region", "tint": 1, "label": "A", "children": [
                        { "tag": "Item", "kind": "product", "id": "left", "title": "Left", "shape": "card" } ] },
                    { "tag": "Box", "kind": "region", "tint": 2, "label": "B", "children": [
                        { "tag": "Item", "kind": "product", "id": "right", "title": "Right", "shape": "card" } ] }
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
fn a_link_between_two_zones_lies_on_the_zones_top_throughout() {
    let page = two_zone_link_page("bottom");
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let path = &scene.link_paths[0];
    // Section 12.3 rule 7: the path lies on the highest terrain it crosses, the two zones'
    // top, and floats over the ground between them instead of stepping down and up.
    assert!(path.len() >= 2, "{path:?}");
    assert!(path.iter().all(|point| point.z == 6.0 * zoom), "{path:?}");
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
    let service_call = &scene.link_paths[2];
    assert!(
        service_call.iter().all(|point| point.z == floor),
        "{service_call:?}"
    );
    // The warehouse stands on the apis slab, one slab thickness above the cloud floor, so
    // the last service call lies at that slab's top throughout (section 12.3 rule 7).
    let to_warehouse = &scene.link_paths[3];
    assert!(
        to_warehouse
            .iter()
            .all(|point| point.z == floor + 6.0 * scene.zoom),
        "{to_warehouse:?}"
    );
    // Both VLANs run between two slabs of the cloud floor's height and float over the
    // ground between them at that height.
    for path in &scene.link_paths[..2] {
        assert!(path.iter().all(|point| point.z == floor), "{path:?}");
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
    let plane_owners: Vec<&str> = document
        .descendants()
        .filter(|node| node.attribute("data-plane").is_some())
        .map(|plane| {
            plane
                .parent_element()
                .unwrap()
                .attribute("data-id")
                .unwrap()
        })
        .collect();
    let label_owners: Vec<&str> = scene
        .labels
        .iter()
        .map(|label| label.owner.as_str())
        .collect();
    assert_eq!(plane_owners, label_owners);
    assert!(!iso.svg.contains("data-billboard"));
    assert!(!iso.svg.contains("data-layer"));
    assert!(!iso.svg.contains("<marker"));
    assert_eq!(
        iso.svg.matches("<defs>").count(),
        1,
        "the block shadow filter only"
    );
    let root = document.root_element();
    let last = root.children().rfind(|node| node.is_element()).unwrap();
    assert_eq!(last.attribute("data-id"), Some("/links/3"));
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
        let plane = children
            .iter()
            .position(|node| node.attribute("data-plane").is_some());
        match (first_polygon, plane) {
            (Some(polygon), Some(plane)) => assert!(polygon < plane),
            other => panic!("zone {:?}: {other:?}", zone.attribute("data-id")),
        }
        assert!(
            !zone.children().any(|node| node.has_tag_name("text")),
            "a zone's text lies in its plane group"
        );
    }
}

#[test]
fn the_plane_group_of_each_hero_zone_carries_its_label_map_as_its_matrix() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    for owner in HERO_ZONES {
        let label = label_of(&scene, owner);
        let plane = plane_group(common::group(&parsed, owner)).unwrap();
        let PlaneMap { a, b, c, d, e, f } = label.map;
        for (written, expected) in matrix_of(plane).into_iter().zip([a, b, c, d, e, f]) {
            assert_close(written, expected, owner);
        }
        assert_eq!(
            plane.attribute("data-plane"),
            Some(format_number(label.z).to_string().as_str()),
            "{owner}"
        );
        assert_eq!(plane.attribute("data-axis"), Some("x"), "{owner}");
        let texts = common::children_named(plane, "text");
        assert_eq!(texts.len(), 1, "{owner}");
    }
}

/// Each node group holds its faces, then its plane groups (two for an item standing as a
/// shape: its icon on top and its text on the floor), then the groups of its children, so
/// every later solid is painted over the labels.
#[test]
fn a_plane_group_follows_its_node_faces_and_precedes_every_later_node_group() {
    for document in [HERO_JSON, common::G7_JSON] {
        let mut page: Page = serde_json::from_str(document).unwrap();
        page.projection = Projection::Iso;
        let geometry = common::layout_with_cosmic_text(&page);
        let scene = project_page(&geometry).unwrap();
        let svg = render_svg(&page, &geometry).unwrap();
        let parsed = common::parse_xml(&svg.svg);
        let mut planes = 0;
        for group in parsed
            .descendants()
            .filter(|node| node.has_tag_name("g") && node.attribute("data-id").is_some())
        {
            let owned = plane_groups(group);
            let (Some(first), Some(last)) = (owned.first(), owned.last()) else {
                continue;
            };
            planes += owned.len();
            let owner = group.attribute("data-id").unwrap();
            let children: Vec<_> = group.children().filter(|node| node.is_element()).collect();
            let start = children.iter().position(|child| child == first).unwrap();
            let end = children.iter().position(|child| child == last).unwrap();
            assert!(start > 0, "{owner}: the plane groups come after the solid");
            assert_eq!(end - start + 1, owned.len(), "{owner}: plane groups apart");
            for (index, child) in children.iter().enumerate() {
                let is_group = child.has_tag_name("g");
                if index < start {
                    assert!(!is_group, "{owner}: a group before the plane groups");
                } else if index > end {
                    assert!(
                        is_group && child.attribute("data-id").is_some(),
                        "{owner}: {child:?} after the plane groups"
                    );
                }
            }
        }
        assert_eq!(planes, scene.labels.len());
        assert!(planes >= 10, "{planes}");
    }
}

#[test]
fn a_zone_label_lies_on_its_slab_top_and_a_card_content_on_its_block_top() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let floor = 6.0 * scene.zoom;
    for (owner, shape, top) in [
        (HERO_ON_PREM, SolidShape::Slab, floor),
        (HERO_GCP, SolidShape::Slab, floor),
        (HERO_VPC, SolidShape::Slab, floor),
        (HERO_APIS, SolidShape::Slab, 2.0 * floor),
        (HERO_GATEWAY, SolidShape::Block, floor + 18.0 * scene.zoom),
    ] {
        let solid = solid_of(&scene, owner);
        let label = label_of(&scene, owner);
        assert_eq!(solid.shape, shape, "{owner}");
        assert_close(solid.base_z + solid.height, top, owner);
        assert_close(label.z, top, owner);
        let plane = plane_group(common::group(&parsed, owner)).unwrap();
        assert_eq!(
            plane.attribute("data-plane"),
            Some(format_number(top).to_string().as_str()),
            "{owner}"
        );
    }
}

/// A Col of two regions joined by a vertical pipe whose tag carries a label and a sub.
fn vertical_pipe_page() -> Page {
    iso_page(json!([
        { "tag": "Col", "children": [
            { "tag": "Box", "kind": "region", "tint": 1, "label": "Region A",
              "children": [card("Primary", "cloud-run")] },
            { "tag": "Pipe", "dir": "v", "line": "dash", "label": "failover replication",
              "sub": "async" },
            { "tag": "Box", "kind": "region", "tint": 2, "label": "Region B",
              "children": [card("Standby", "cloud-run")] } ] }
    ]))
}

#[test]
fn a_vertical_pipe_tag_reads_along_y_and_is_drawn_in_its_unturned_box() {
    let page = vertical_pipe_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let pipe_pointer = "/body/0/children/1";
    let pipe = &geometry.nodes[index_of(&geometry, pipe_pointer)];
    let label = label_of(&scene, pipe_pointer);
    assert_eq!(label.axis, Axis::Y);
    assert!(label.opaque);

    let members: Vec<BoxRect> = [PartName::Tag, PartName::TagLabel, PartName::TagSub]
        .into_iter()
        .map(|name| pipe.part(name).unwrap().bounds)
        .collect();
    let union = members
        .iter()
        .copied()
        .reduce(|first, second| {
            let left = first.x.min(second.x);
            let top = first.y.min(second.y);
            BoxRect {
                x: left,
                y: top,
                width: first.right().max(second.right()) - left,
                height: first.bottom().max(second.bottom()) - top,
            }
        })
        .unwrap();
    let pivot = center_of(union);
    let strip = pipe.part(PartName::TagLabel).unwrap();
    let run = strip.text.as_ref().unwrap();
    assert!(strip.bounds.height > strip.bounds.width, "{strip:?}");
    assert!(run.metrics.width_px > strip.bounds.width, "{strip:?}");
    for strip in &members {
        let local = unturned_box(*strip, pivot);
        assert_eq!((local.width, local.height), (strip.height, strip.width));
        let turned = turned_box(local, pivot);
        for (actual, expected) in [
            (turned.x, strip.x),
            (turned.y, strip.y),
            (turned.width, strip.width),
            (turned.height, strip.height),
        ] {
            assert_close(actual, expected, "turned_box of the unturned strip");
        }
    }

    let svg = render_svg(&page, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let plane = plane_group(common::group(&parsed, pipe_pointer)).unwrap();
    assert_eq!(plane.attribute("data-axis"), Some("y"));
    let PlaneMap { a, b, c, d, e, f } = label.map;
    for (written, expected) in matrix_of(plane).into_iter().zip([a, b, c, d, e, f]) {
        assert_close(written, expected, "pipe tag matrix");
    }
    let pill = common::children_named(plane, "rect")[0];
    let number = |name: &str| -> f32 { pill.attribute(name).unwrap().parse().unwrap() };
    let inset = pill
        .attribute("stroke-width")
        .map_or(0.0, |width| width.parse::<f32>().unwrap() / 2.0);
    let local_tag = unturned_box(members[0], pivot);
    assert!(local_tag.width > local_tag.height, "{local_tag:?}");
    assert_close(number("x"), local_tag.x + inset, "pill x");
    assert_close(number("y"), local_tag.y + inset, "pill y");
    assert_close(number("width"), local_tag.width - 2.0 * inset, "pill width");
    assert_close(
        number("height"),
        local_tag.height - 2.0 * inset,
        "pill height",
    );
    let lines: u32 = [PartName::TagLabel, PartName::TagSub]
        .into_iter()
        .map(|name| {
            pipe.part(name)
                .unwrap()
                .text
                .as_ref()
                .unwrap()
                .metrics
                .line_count
        })
        .sum();
    assert_eq!(common::children_named(plane, "text").len() as u32, lines);
}

#[test]
fn iso_layout_grows_a_card_title_by_the_type_scale_and_keeps_the_page_title_flat() {
    let iso = iso_page(json!([
        { "tag": "Box", "kind": "region", "tint": 1, "label": "Region",
          "children": [card("Gateway", "cloud-run")] }
    ]));
    let mut flat = iso.clone();
    flat.projection = Projection::Flat;
    let iso_geometry = common::layout_with_fixed_metrics(&iso);
    let flat_geometry = common::layout_with_fixed_metrics(&flat);
    let style = |geometry: &PageGeometry, pointer: &str, name: PartName| {
        geometry.nodes[index_of(geometry, pointer)]
            .part(name)
            .unwrap()
            .text
            .as_ref()
            .unwrap()
            .style
    };
    let card_pointer = "/body/0/children/0";
    let iso_card = style(&iso_geometry, card_pointer, PartName::FunctionName);
    let flat_card = style(&flat_geometry, card_pointer, PartName::FunctionName);
    assert_close(
        iso_card.size_px,
        flat_card.size_px * ISO_TYPE_SCALE,
        "card title size",
    );
    assert_close(
        iso_card.line_height_px,
        flat_card.line_height_px * ISO_TYPE_SCALE,
        "card title line height",
    );
    let iso_zone = style(&iso_geometry, "/body/0", PartName::Label);
    let flat_zone = style(&flat_geometry, "/body/0", PartName::Label);
    assert_close(
        iso_zone.size_px,
        flat_zone.size_px * ISO_ZONE_LABEL_SCALE,
        "zone label size",
    );
    let title_style = |geometry: &PageGeometry| {
        geometry
            .nodes
            .iter()
            .find(|node| node.tag == NodeTag::Title)
            .unwrap()
            .parts
            .iter()
            .find_map(|part| part.text.as_ref())
            .unwrap()
            .style
    };
    assert_eq!(title_style(&iso_geometry), title_style(&flat_geometry));
}

/// Section 12.3 rule 5: a band is a flat wide arrow on its plane, one body polygon and one
/// head per arrowed end, with its tag on the plane itself.
#[test]
fn a_band_pipe_lies_flat_with_its_tag_on_the_floor() {
    let page = iso_page(json!([
        { "tag": "Row", "children": [
            card("Edge", "cloud-run"),
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "requests",
              "arrow": "both", "form": "band" },
            card("Core", "cloud-run") ] }
    ]));
    let geometry = common::layout_with_fixed_metrics(&page);
    let scene = project_page(&geometry).unwrap();
    let pipe = geometry
        .nodes
        .iter()
        .position(|node| node.tag == NodeTag::Pipe)
        .unwrap();
    let solid = scene
        .solids
        .iter()
        .find(|solid| solid.node == pipe)
        .unwrap();
    assert_eq!((solid.shape, solid.height), (SolidShape::Surface, 0.0));
    let tag = label_of(&scene, &geometry.nodes[pipe].pointer.to_string());
    assert_eq!(tag.z, solid.base_z);
    let svg = render_svg(&page, &geometry).unwrap();
    let document = common::parse_xml(&svg.svg);
    let group = common::group(&document, &geometry.nodes[pipe].pointer.to_string());
    let polygons = group
        .descendants()
        .filter(|element| element.has_tag_name("polygon"))
        .count();
    assert_eq!(polygons, 3, "body and two heads");
}

/// Section 12.3 rule 5: a pipe is a tube of polygons, with no flat dot ellipse, and its tag
/// pill lies on the tube's top.
#[test]
fn pipes_are_tubes_with_their_tags_on_top() {
    let mut page: Page = serde_json::from_str(common::G7_JSON).unwrap();
    page.projection = stencil_model::Projection::Iso;
    let geometry = common::layout_with_cosmic_text(&page);
    let iso = render_svg(&page, &geometry).unwrap();
    let document = common::parse_xml(&iso.svg);
    let scene = project_page(&geometry).unwrap();
    let mut examined = 0;
    for node in geometry
        .nodes
        .iter()
        .filter(|node| node.tag == NodeTag::Pipe)
    {
        let group = common::group(&document, &node.pointer.to_string());
        let polygons = group
            .descendants()
            .filter(|element| element.has_tag_name("polygon"))
            .count();
        let ellipses = group
            .descendants()
            .filter(|element| element.has_tag_name("ellipse"))
            .count();
        // Two caps and two body halves at least, then a flange or cone per end.
        assert!(polygons >= 6, "{}: {polygons} polygons", node.pointer);
        assert_eq!(ellipses, 0, "{}", node.pointer);
        let tag = label_of(&scene, &node.pointer.to_string());
        let solid = scene
            .solids
            .iter()
            .find(|solid| {
                solid.node
                    == geometry
                        .nodes
                        .iter()
                        .position(|n| n.pointer == node.pointer)
                        .unwrap()
            })
            .unwrap();
        assert_eq!(
            tag.z,
            solid.base_z + 2.0 * ISO_TUBE_RADIUS_PX,
            "{}",
            node.pointer
        );
        examined += 1;
    }
    assert!(examined > 0);
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

/// A gcp zone around two cards.
fn two_card_zone(container: Value) -> Page {
    iso_page(json!([
        { "tag": "Box", "kind": "gcp", "label": "Google Cloud", "children": [container] }
    ]))
}

/// Three labels give three pairs; the zone label has two later blocks and the first card's
/// label one; each label meets the one slab; each card's content must stay on its block.
#[test]
fn cards_side_by_side_leave_their_labels_clear() {
    let page = two_card_zone(json!({
        "tag": "Row", "gap": 32,
        "children": [
            with_shape(card("Gateway", "cloud-run"), "card"),
            with_shape(card("Warehouse", "bigquery"), "card")
        ]
    }));
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    assert_eq!(scene.labels.len(), 3);
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 3 + 3 + 3 + 2);
    assert_eq!(report.outcome(), CheckOutcome::Passed, "{report:?}");
}

/// Layout px are screen px, so a hand-built label's corners and marks are its boxes.
const IDENTITY: PlaneMap = PlaneMap {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    e: 0.0,
    f: 0.0,
};

/// A 10 px label of a link-tag-like owner `/body/0/<owner>` at (x, 0), whose one mark
/// fills it.
fn label_at(owner: &str, x: f32) -> Label {
    let flat = BoxRect {
        x,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };
    let corners = IDENTITY.corners(flat);
    Label {
        owner: NodePointer::root().child("body").index(0).child(owner),
        node: None,
        z: 0.0,
        axis: Axis::X,
        map: IDENTITY,
        flat,
        corners,
        marks: vec![corners],
        opaque: false,
        contained: false,
        parts: LabelParts::All,
    }
}

/// `label` with one mark, the box `mark`.
fn with_mark(mut label: Label, mark: BoxRect) -> Label {
    label.marks = vec![IDENTITY.corners(mark)];
    label
}

/// A flat footprint far from every hand-built scene's links.
const NO_FOOTPRINT: BoxRect = BoxRect {
    x: -1000.0,
    y: -1000.0,
    width: 1.0,
    height: 1.0,
};

/// The silhouette of a slab of height 0 whose top face is the screen rectangle from x 0
/// to 100 and y 5 to 50.
const FLAT_SLAB: [ScreenPoint; 6] = [
    ScreenPoint { x: 0.0, y: 5.0 },
    ScreenPoint { x: 100.0, y: 5.0 },
    ScreenPoint { x: 100.0, y: 5.0 },
    ScreenPoint { x: 100.0, y: 50.0 },
    ScreenPoint { x: 0.0, y: 50.0 },
    ScreenPoint { x: 0.0, y: 50.0 },
];

/// A block whose silhouette is the screen rectangle from (left, top) to (right, bottom).
fn block(node: usize, left: f32, top: f32, right: f32, bottom: f32) -> Solid {
    let middle = (top + bottom) / 2.0;
    let silhouette = [
        ScreenPoint { x: left, y: top },
        ScreenPoint { x: right, y: top },
        ScreenPoint {
            x: right,
            y: middle,
        },
        ScreenPoint {
            x: right,
            y: bottom,
        },
        ScreenPoint { x: left, y: bottom },
        ScreenPoint { x: left, y: middle },
    ];
    Solid {
        node,
        pointer: NodePointer::root().child("body").index(node),
        shape: SolidShape::Block,
        base_z: 0.0,
        height: 18.0,
        silhouette,
        opaque: true,
        footprint: NO_FOOTPRINT,
        form: Shape::Card,
        outline: silhouette.to_vec(),
    }
}

fn flat_slab(node: usize) -> Solid {
    Solid {
        node,
        pointer: NodePointer::root().child("body").index(node),
        shape: SolidShape::Slab,
        base_z: 0.0,
        height: 0.0,
        silhouette: FLAT_SLAB,
        opaque: false,
        footprint: NO_FOOTPRINT,
        form: Shape::Card,
        outline: FLAT_SLAB.to_vec(),
    }
}

fn scene_with(labels: Vec<Label>) -> IsoScene {
    IsoScene {
        canvas: Size {
            width: 100.0,
            height: 100.0,
        },
        offset: ZERO,
        footer_shift: 0.0,
        solids: Vec::new(),
        labels,
        link_paths: Vec::new(),
        link_kinds: Vec::new(),
        zoom: 1.0,
        origin: (0.0, 0.0),
    }
}

#[test]
fn a_flat_render_is_not_applicable() {
    let report = iso_labels_clear(None);
    assert_eq!(report.check, CheckName::IsoLabelsClear);
    assert_eq!(report.outcome(), CheckOutcome::NotApplicable);
    assert_eq!(report.not_applicable, Some("projection is flat"));
}

#[test]
fn labels_overlapping_by_two_hundredths_are_a_defect_and_touching_ones_are_not() {
    let overlapping = scene_with(vec![label_at("a", 0.0), label_at("b", 9.98)]);
    let report = iso_labels_clear(Some(&overlapping));
    assert_eq!(report.examined, 1);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0/b");
    assert_eq!(
        report.defects[0].message,
        "label /body/0/b at 9.98,0.00 along x overlaps label /body/0/a"
    );

    let touching = scene_with(vec![label_at("a", 0.0), label_at("b", 10.0)]);
    let report = iso_labels_clear(Some(&touching));
    assert_eq!(report.examined, 1);
    assert!(report.passed());
}

#[test]
fn one_label_and_no_block_examines_nothing_and_fails() {
    let report = iso_labels_clear(Some(&scene_with(vec![label_at("a", 0.0)])));
    assert_eq!(report.examined, 0);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn a_block_painted_after_a_label_covers_it_and_under_a_link_tag_every_block_counts() {
    let over = block(1, 15.0, 0.0, 35.0, 8.0);
    let mut covered = label_at("label", 20.0);
    covered.node = Some(0);
    let mut scene = scene_with(vec![covered]);
    scene.solids = vec![over.clone()];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 1);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0/label");
    assert_eq!(
        report.defects[0].message,
        "label /body/0/label at 20.00,0.00 along x is covered by /body/1"
    );

    let mut painted_over_it = label_at("label", 20.0);
    painted_over_it.node = Some(2);
    let mut scene = scene_with(vec![painted_over_it]);
    scene.solids = vec![over.clone()];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(
        report.examined, 0,
        "a block painted before its label is not examined"
    );
    assert!(report.defects.is_empty(), "{report:?}");

    let mut scene = scene_with(vec![label_at("tag", 20.0)]);
    scene.solids = vec![over];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 1);
    assert_eq!(
        report.defects[0].message,
        "label /body/0/tag at 20.00,0.00 along x is covered by /body/1"
    );
}

#[test]
fn a_zone_edge_through_a_label_is_a_defect_and_one_under_an_opaque_label_is_not() {
    let label = label_at("label", 20.0);
    let mut chip = label_at("chip", 60.0);
    chip.opaque = true;
    let mut scene = scene_with(vec![label, chip]);
    scene.solids = vec![flat_slab(0)];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 2);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert_eq!(report.defects[0].pointer.as_str(), "/body/0/label");
    assert_eq!(
        report.defects[0].message,
        "label /body/0/label at 20.00,0.00 along x is crossed by an edge of slab /body/0"
    );
}

#[test]
fn an_edge_hidden_behind_a_later_block_does_not_cross_a_label() {
    let mut label = with_mark(
        label_at("label", 20.0),
        BoxRect {
            x: 20.0,
            y: 4.0,
            width: 10.0,
            height: 2.0,
        },
    );
    label.node = Some(1);
    let mut scene = scene_with(vec![label]);
    scene.solids = vec![flat_slab(0), block(1, 15.0, 0.0, 35.0, 8.0)];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 1);
    assert!(report.passed(), "{report:?}");
}

#[test]
fn a_link_through_a_label_is_a_defect_and_one_under_an_opaque_label_is_not() {
    let label = label_at("label", 20.0);
    let mut tag = label_at("tag", 0.0);
    tag.opaque = true;
    let mut scene = scene_with(vec![label, tag]);
    // The flat points (x, y) with x + y = 10 lie on the screen line y = 5.
    scene.link_paths = vec![vec![
        IsoPoint {
            x: -30.0,
            y: 40.0,
            z: 0.0,
        },
        IsoPoint {
            x: 40.0,
            y: -30.0,
            z: 0.0,
        },
    ]];
    scene.link_kinds = vec![(stencil_model::Line::Solid, Some(1))];
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 2);
    assert_eq!(report.defects.len(), 1, "{report:?}");
    assert_eq!(
        report.defects[0].message,
        "label /body/0/label at 20.00,0.00 along x is crossed by link /links/0"
    );
}

#[test]
fn text_that_leaves_its_block_is_a_defect() {
    let mark = BoxRect {
        x: 5.0,
        y: 5.0,
        width: 20.0,
        height: 10.0,
    };
    let mut inside = with_mark(label_at("text", 5.0), mark);
    inside.node = Some(0);
    inside.contained = true;
    let outside = with_mark(
        inside.clone(),
        BoxRect {
            width: 50.0,
            ..mark
        },
    );
    for (label, defects) in [(inside, 0), (outside, 1)] {
        let mut scene = scene_with(vec![label]);
        scene.solids = vec![block(0, 0.0, 0.0, 40.0, 40.0)];
        let report = iso_labels_clear(Some(&scene));
        assert_eq!(report.examined, 1);
        assert_eq!(report.defects.len(), defects, "{report:?}");
        if defects == 1 {
            assert_eq!(
                report.defects[0].message,
                "label /body/0/text at 5.00,0.00 along x leaves its block"
            );
        }
    }
}

/// Twelve labels give 66 pairs: four zones, two cards, two link tags, and an icon and a floor
/// text each for the tile router and the cylinder warehouse. Painted-later opaque solids:
/// 6, 5, 5, 4, 4, 3, 2, 1, 0 and 0 for the node labels in geometry order, 4 blocks for each
/// of the two link tags. The ten non-opaque labels each meet the four slabs and the four
/// link paths, and the two cards' content and the two icons must stay on their blocks.
#[test]
fn the_hero_labels_are_clear() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene =
        stencil_render::iso::project_page(&geometry, &common::solid_inputs(&page, &geometry))
            .unwrap();
    // 4 Box names, the icon and the text of the two blocks, the tile and the cylinder, and
    // 2 link tags.
    assert_eq!(scene.labels.len(), 14);
    let report = iso_labels_clear(Some(&scene));
    assert_eq!(report.examined, 234);
    assert!(report.defects.is_empty(), "{report:#?}");
    assert!(report.passed());
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
        assert_eq!(iso["projection"]["labels"].as_array().unwrap().len(), 14);
        assert_eq!(iso["projection"]["kind"], "iso");
        for label in iso["projection"]["labels"].as_array().unwrap() {
            let id = label["id"].as_str().unwrap();
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
fn the_measured_projection_lists_each_label_with_its_id_axis_corners_and_z() {
    let page = hero_page();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    let measured = measured_json(&hero_document(), &geometry, Some(&scene));
    let projection = measured["projection"].as_object().unwrap();
    let keys: Vec<&String> = projection.keys().collect();
    assert_eq!(keys, ["canvas", "footer_shift", "kind", "labels", "offset"]);
    assert!(projection.get("billboards").is_none());
    let labels = projection["labels"].as_array().unwrap();
    assert_eq!(labels.len(), scene.labels.len());
    for (written, label) in labels.iter().zip(&scene.labels) {
        let keys: Vec<&String> = written.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["axis", "corners", "id", "z"]);
        assert_eq!(written["id"], label.owner.as_str());
        assert_eq!(written["axis"], label.axis.as_str());
        assert_eq!(written["z"], format_number(label.z).to_json());
        let corners = written["corners"].as_array().unwrap();
        assert_eq!(corners.len(), 4);
        for (corner, expected) in corners.iter().zip(label.corners) {
            assert_eq!(corner["x"], format_number(expected.x).to_json());
            assert_eq!(corner["y"], format_number(expected.y).to_json());
        }
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
        let group = plane_group(common::group(&parsed, HERO_ROUTER)).unwrap();
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
            // A solid link is a tube of polygons under a shading theme; a dashed link, and
            // every link under wire, is one stroked path at the theme's width.
            let paths = common::children_named(link, "path");
            if theme == "wire" || index == 1 {
                assert_eq!(paths.len(), 1, "{theme} link {index}");
                assert_eq!(
                    paths[0].attribute("stroke-width"),
                    Some(width.as_str()),
                    "{theme:?}"
                );
            } else {
                assert!(paths.is_empty(), "{theme} link {index}");
                let polygons = common::children_named(link, "polygon").len();
                assert!(polygons >= 4, "{theme} link {index}: {polygons} polygons");
            }
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
                with_shape(card("Gateway", "cloud-run"), "card") ] } ] }
    ]));
    let geometry = common::layout_with_cosmic_text(&page);
    let scene = project_page(&geometry).unwrap();
    assert_eq!(scene.zoom, ISO_ZOOM_MAX);
    let card_pointer = "/body/0/children/0/children/0";
    let block = solid_of(&scene, card_pointer);
    assert_close(block.height, 18.0 * ISO_ZOOM_MAX, "block height");
    let flat_card = &geometry.nodes[index_of(&geometry, card_pointer)];
    assert_close(
        block.footprint.width,
        flat_card.bounds.width * ISO_ZOOM_MAX,
        "footprint width",
    );
    let label = label_of(&scene, card_pointer);
    assert_close(
        screen_distance(label.corners[0], label.corners[1]),
        label.flat.width * ISO_ZOOM_MAX,
        "label width on screen",
    );
    assert_close(
        screen_distance(label.corners[1], label.corners[2]),
        label.flat.height * ISO_ZOOM_MAX,
        "label height on screen",
    );
    let (zoomed, _) = zoomed_geometry(&geometry).unwrap();
    let icon = flat_card.part(PartName::Icon).unwrap().bounds;
    assert_eq!(
        zoomed.nodes[index_of(&zoomed, card_pointer)]
            .part(PartName::Icon)
            .unwrap()
            .bounds,
        icon,
        "a plane member stays in layout px"
    );
    let (icon_x, icon_y) = center_of(icon);
    let (origin_x, origin_y) = scene.origin;
    let scene_icon = project_point(
        origin_x + (icon_x - origin_x) * ISO_ZOOM_MAX,
        origin_y + (icon_y - origin_y) * ISO_ZOOM_MAX,
        block.base_z + block.height,
        scene.offset,
    );
    let drawn_icon = label.map.apply(icon_x, icon_y);
    assert_close(drawn_icon.x, scene_icon.x, "icon center x");
    assert_close(drawn_icon.y, scene_icon.y, "icon center y");
}

#[test]
fn a_label_at_zoom_two_lies_twice_as_far_from_the_body_origin_as_at_zoom_one() {
    let page = two_card_zone(json!({
        "tag": "Row", "gap": 32,
        "children": [
            with_shape(card("Gateway", "cloud-run"), "card"),
            card("Warehouse", "bigquery")
        ]
    }));
    let geometry = common::layout_with_fixed_metrics(&page);
    let one = project_zoomed(&geometry, 1.0).unwrap();
    let two = project_zoomed(&geometry, 2.0).unwrap();
    // The zone, the card, and the cylinder warehouse's icon and floor text.
    assert_eq!(one.labels.len(), 4);
    for (first, second) in one.labels.iter().zip(&two.labels) {
        assert_eq!(first.owner, second.owner);
        assert_eq!(first.flat, second.flat);
        assert_close(second.z, 2.0 * first.z, "plane height");
        let first_origin = first.map.apply(one.origin.0, one.origin.1);
        let second_origin = second.map.apply(two.origin.0, two.origin.1);
        for (near, far) in first.corners.iter().zip(&second.corners) {
            let what = format!("corner of {}", first.owner);
            assert_close(
                far.x - second_origin.x,
                2.0 * (near.x - first_origin.x),
                &what,
            );
            assert_close(
                far.y - second_origin.y,
                2.0 * (near.y - first_origin.y),
                &what,
            );
        }
    }
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

/// Under iso a card lays its icon left of its name; both lie on the block's top face. A
/// product stands as a block unless the item asks for the card.
#[test]
fn a_card_icon_and_name_lie_on_its_block_top_face_with_the_name_beside_the_icon() {
    let page = two_card_zone(json!({
        "tag": "Row", "gap": 32,
        "children": [
            with_shape(card("Gateway", "cloud-run"), "card"),
            with_shape(card("Model", "vertex-ai"), "card")
        ]
    }));
    let geometry = common::layout_with_cosmic_text(&page);
    let (zoomed, zoom) = zoomed_geometry(&geometry).unwrap();
    let scene = project_zoomed(&zoomed, zoom).unwrap();
    for owner in [
        "/body/0/children/0/children/0",
        "/body/0/children/0/children/1",
    ] {
        assert_eq!(solid_of(&scene, owner).form, Shape::Card, "{owner}");
        let node = &zoomed.nodes[index_of(&zoomed, owner)];
        let block = solid_of(&scene, owner);
        let label = label_of(&scene, owner);
        assert_eq!(label.axis, Axis::X, "{owner}");
        let icon = node.part(PartName::Icon).unwrap().bounds;
        let name = node.part(PartName::FunctionName).unwrap().bounds;
        assert!(name.x >= icon.right(), "{owner}: {name:?} {icon:?}");
        let top_z = block.base_z + block.height;
        let corner = |x: f32, y: f32| project_point(x, y, top_z, scene.offset);
        let face = block.footprint;
        let top_face = [
            corner(face.x, face.y),
            corner(face.right(), face.y),
            corner(face.right(), face.bottom()),
            corner(face.x, face.bottom()),
        ];
        let (icon_x, icon_y) = center_of(icon);
        let (name_x, name_y) = center_of(name);
        for (what, point) in [
            ("icon center", label.map.apply(icon_x, icon_y)),
            ("name center", label.map.apply(name_x, name_y)),
        ] {
            assert!(
                inside_convex(point, &top_face),
                "{owner}: {what} {point:?} off the top face {top_face:?}"
            );
        }
        for corner in label.corners {
            assert!(
                inside_convex(corner, &top_face),
                "{owner}: label corner {corner:?} off the top face {top_face:?}"
            );
        }
    }
}

#[test]
fn the_hero_primary_leaves_the_router_through_a_stub_with_its_tag_along_x() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    assert!(geometry.links[0].points.len() > 2);
    let scene = project_page(&geometry).unwrap();
    let primary = &scene.link_paths[0];
    // The first leg is the approach stub out of the router tile's right side, along x on the
    // on-prem slab top the tile stands on, at least ISO_APPROACH_PX long at the zoom; then
    // the path steps down onto the ground.
    let first = primary[0];
    assert_eq!(first.z, 6.0 * scene.zoom, "{primary:?}");
    let stub = primary
        .iter()
        .take_while(|point| (point.y - first.y).abs() < 1e-3 && point.z == first.z)
        .count();
    let stub_end = primary[stub - 1];
    assert!(
        stub_end.x - first.x >= stencil_layout::ISO_APPROACH_PX * scene.zoom - 1e-3,
        "{primary:?}"
    );
    // The path keeps that height throughout: it floats over the ground between the slabs.
    assert!(
        primary.iter().all(|point| point.z == first.z),
        "{primary:?}"
    );
    let tag = label_of(&scene, "/links/0");
    assert_eq!(tag.axis, Axis::X);
    // The tag pill lies on the tube's top.
    assert_close(
        tag.z,
        first.z + 2.0 * link_tube_radius(Line::Solid, Some(1)) * scene.zoom,
        "tag on the tube top",
    );
    assert!(tag.opaque);
}

/// Section 12.3 rule 8 moves the primary's first leg onto one straight leg; its tag must
/// move with it.
#[test]
fn the_hero_primary_tag_lies_on_its_drawn_leg() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let scene = project_page(&geometry).unwrap();
    let primary = &scene.link_paths[0];
    let tag = label_of(&scene, "/links/0");
    assert_eq!(tag.axis, Axis::X);
    let (center_x, center_y) = center_of(tag.flat);
    let center = tag.map.apply(center_x, center_y);
    let on_path = primary.windows(2).any(|pair| {
        // The tag lies on the tube's top, so the leg is read at the tag's height.
        let start = project_point(pair[0].x, pair[0].y, tag.z, scene.offset);
        let end = project_point(pair[1].x, pair[1].y, tag.z, scene.offset);
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
    assert_eq!(report.examined, 9);
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
        form: Shape::Card,
        outline: [ZERO; 6].to_vec(),
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
fn a_dashed_link_is_one_stroked_path_and_a_solid_one_a_tube_of_polygons() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let svg = render_svg(&hero, &geometry).unwrap();
    let parsed = common::parse_xml(&svg.svg);
    let dashed = common::group(&parsed, "/links/1");
    let paths = common::children_named(dashed, "path");
    assert_eq!(paths.len(), 1);
    let data = paths[0].attribute("d").unwrap();
    assert_eq!(data.matches('M').count(), 1, "{data}");
    assert!(data.contains(" L "), "{data}");
    // The dashed link still ends in a cone, one filled outline under the hollow style.
    assert!(!common::children_named(dashed, "polygon").is_empty());
    let solid = common::group(&parsed, "/links/0");
    assert!(common::children_named(solid, "path").is_empty());
    // A tube per leg (body halves, caps and a shadow), a joint circle per corner, a cone.
    assert!(common::children_named(solid, "polygon").len() >= 8);
    assert!(!common::children_named(solid, "circle").is_empty());
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
        &stencil_render::iso::SolidInputs::new(&geometry, &page.links, 6.0),
    )
    .unwrap();
    let eight = stencil_render::iso::project_page(
        &geometry,
        &stencil_render::iso::SolidInputs::new(&geometry, &page.links, 8.0),
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

#[test]
fn the_hero_hybrid_item_is_a_tile_six_high_on_a_64_px_footprint_with_its_icon_on_top_and_its_text_on_the_floor()
 {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let (zoomed, zoom) = zoomed_geometry(&geometry).unwrap();
    let scene = project_zoomed(&zoomed, zoom).unwrap();
    let tile = solid_of(&scene, HERO_ROUTER);
    assert_eq!((tile.shape, tile.form), (SolidShape::Block, Shape::Tile));
    assert_close(
        tile.base_z,
        6.0 * zoom,
        "the tile stands on the on-prem slab",
    );
    assert_close(tile.height, 6.0 * zoom, "tile height");
    let footprint = zoomed.nodes[index_of(&zoomed, HERO_ROUTER)]
        .part(PartName::Footprint)
        .unwrap()
        .bounds;
    assert_eq!(tile.footprint, footprint);
    assert_close(footprint.width, 64.0 * zoom, "footprint width");
    assert_close(footprint.height, 64.0 * zoom, "footprint height");
    assert_eq!(
        tile.outline,
        tile.silhouette.to_vec(),
        "a box form's outline"
    );

    let labels: Vec<&Label> = scene
        .labels
        .iter()
        .filter(|label| label.owner.as_str() == HERO_ROUTER)
        .collect();
    let parts: Vec<LabelParts> = labels.iter().map(|label| label.parts).collect();
    assert_eq!(parts, [LabelParts::Icon, LabelParts::Text]);
    let (icon, text) = (labels[0], labels[1]);
    assert_close(icon.z, tile.base_z + tile.height, "icon label z");
    assert!(icon.contained);
    assert_close(text.z, tile.base_z, "text label z");
    assert!(!text.contained);
}

#[test]
fn a_bigquery_item_is_a_cylinder_whose_round_outline_cuts_back_the_link_into_it() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let scene =
        stencil_render::iso::project_page(&geometry, &common::solid_inputs(&hero, &geometry))
            .unwrap();
    let cylinder = solid_of(&scene, HERO_WAREHOUSE);
    assert_eq!(cylinder.form, Shape::Cylinder);
    assert_close(cylinder.height, 36.0 * scene.zoom, "cylinder height");
    assert!(cylinder.outline.len() > 6, "{:?}", cylinder.outline);

    let warehouse = index_of(&geometry, HERO_WAREHOUSE);
    let into = geometry
        .links
        .iter()
        .position(|route| route.to_node == warehouse)
        .unwrap();
    let end = *scene.link_paths[into].last().unwrap();
    let drawn_end = project_point(end.x, end.y, end.z, scene.offset);
    // The end lies on the round outline, not inside it, and well inside the six-vertex box,
    // where a cut-back at the box would have stopped.
    let outline_margin = convex_margin(drawn_end, &cylinder.outline);
    assert!(
        outline_margin.abs() <= 0.05,
        "{drawn_end:?} lies {outline_margin} px inside the outline {:?}",
        cylinder.outline
    );
    let box_margin = convex_margin(drawn_end, &cylinder.silhouette);
    assert!(
        box_margin > 1.0,
        "{drawn_end:?} lies {box_margin} px inside the box {:?}",
        cylinder.silhouette
    );
}

/// An item's own `shape` outranks the shape of its icon's product row, and that row
/// outranks the kind's shape; an icon with no shape row and an item with no icon stand as
/// the product kind's block.
#[test]
fn an_item_takes_its_own_shape_before_its_icon_row_shape_and_that_before_card() {
    let page = iso_page(json!([
        { "tag": "Box", "kind": "region", "tint": 1, "label": "Region", "children": [
            { "tag": "Row", "gap": 64, "children": [
                with_shape(card("Own tower", "bigquery"), "tower"),
                card("Row cylinder", "bigquery"),
                with_shape(card("Own card", "bigquery"), "card"),
                card("No row", "cloud-run"),
                { "tag": "Item", "kind": "product", "title": "No icon" } ] } ] }
    ]));
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    for (index, form, footprint) in [
        (0, Shape::Tower, Some(44.0)),
        (1, Shape::Cylinder, Some(56.0)),
        (2, Shape::Card, None),
        (3, Shape::Block, Some(64.0)),
        (4, Shape::Block, Some(64.0)),
    ] {
        let pointer = format!("/body/0/children/0/children/{index}");
        let solid = solid_of(&scene, &pointer);
        assert_eq!(solid.form, form, "{pointer}");
        assert_close(
            solid.height,
            stencil_layout::shape_height_px(form) * zoom,
            &pointer,
        );
        let node = &geometry.nodes[index_of(&geometry, &pointer)];
        let side = node.part(PartName::Footprint).map(|part| part.bounds.width);
        assert_eq!(side, footprint.map(|side: f32| side * zoom), "{pointer}");
        if footprint.is_none() {
            assert_eq!(solid.footprint, node.bounds, "{pointer}");
        }
    }
}

#[test]
fn the_platform_example_stands_one_item_of_each_form_with_its_labels_and_links_clear() {
    let page: Page = serde_json::from_str(PLATFORM_JSON).unwrap();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene =
        stencil_render::iso::project_page(&geometry, &common::solid_inputs(&page, &geometry))
            .unwrap();
    let forms: Vec<Shape> = scene
        .solids
        .iter()
        .filter(|solid| geometry.nodes[solid.node].tag == NodeTag::Pcard)
        .map(|solid| solid.form)
        .collect();
    assert_eq!(forms.len(), 5, "{forms:?}");
    for form in [
        Shape::Block,
        Shape::Tile,
        Shape::Tower,
        Shape::Cylinder,
        Shape::Stack,
    ] {
        let count = forms.iter().filter(|each| **each == form).count();
        assert_eq!(count, 1, "{form:?} in {forms:?}");
    }
    let labels = iso_labels_clear(Some(&scene));
    assert_eq!(labels.examined, 233);
    assert!(labels.defects.is_empty(), "{labels:#?}");
    assert!(labels.passed());
    let links = iso_links_clear(Some(&scene));
    assert_eq!(links.examined, 8);
    assert!(links.defects.is_empty(), "{links:#?}");
    assert!(links.passed());
}

/// The platform example's scene and its SVG under center, after asserting an item stands
/// as `form`.
fn platform_round_solid(form: Shape) -> (IsoScene, String) {
    let page: Page = serde_json::from_str(PLATFORM_JSON).unwrap();
    let geometry = common::layout_with_cosmic_text(&page);
    let scene =
        stencil_render::iso::project_page(&geometry, &common::solid_inputs(&page, &geometry))
            .unwrap();
    let svg = render_svg(&page, &geometry).unwrap().svg;
    assert!(scene.solids.iter().any(|solid| solid.form == form));
    (scene, svg)
}

/// A cylinder's node group draws its visible side as two half paths and its top as one
/// ellipse; a stack draws three such discs, bottom to top.
#[test]
fn a_cylinder_group_holds_two_side_paths_and_a_top_ellipse_and_a_stack_three_of_those_discs() {
    for (form, discs) in [(Shape::Cylinder, 1), (Shape::Stack, 3)] {
        let (scene, svg) = platform_round_solid(form);
        let solid = scene
            .solids
            .iter()
            .find(|solid| solid.form == form)
            .unwrap();
        let parsed = common::parse_xml(&svg);
        let group = common::group(&parsed, solid.pointer.as_str());
        let sides = common::children_named(group, "path");
        let tops: Vec<_> = common::children_named(group, "ellipse")
            .into_iter()
            .filter(|ellipse| ellipse.attribute("filter").is_none())
            .collect();
        assert_eq!(sides.len(), 2 * discs, "{form:?}");
        assert_eq!(tops.len(), discs, "{form:?}");
        let (radius_x, radius_y) = stencil_render::iso::ellipse_radii(
            solid.footprint.width.min(solid.footprint.height) / 2.0,
        );
        for top in tops {
            assert_eq!(
                top.attribute("rx"),
                Some(format_number(radius_x).to_string().as_str())
            );
            assert_eq!(
                top.attribute("ry"),
                Some(format_number(radius_y).to_string().as_str())
            );
        }
    }
}

#[test]
fn a_round_solid_casts_an_ellipse_shadow() {
    for form in [Shape::Cylinder, Shape::Stack] {
        let (scene, svg) = platform_round_solid(form);
        let solid = scene
            .solids
            .iter()
            .find(|solid| solid.form == form)
            .unwrap();
        let parsed = common::parse_xml(&svg);
        let group = common::group(&parsed, solid.pointer.as_str());
        let first = group.children().find(|node| node.is_element()).unwrap();
        assert!(first.has_tag_name("ellipse"), "{form:?}: {first:?}");
        assert_eq!(
            first.attribute("filter"),
            Some("url(#stencil-shadow)"),
            "{form:?}"
        );
        assert!(
            common::children_named(group, "polygon").is_empty(),
            "{form:?}: a round solid draws no box face"
        );
    }
}

/// The label `owner` draws in `scene`.
fn label_of<'a>(scene: &'a IsoScene, owner: &str) -> &'a Label {
    scene
        .labels
        .iter()
        .find(|label| label.owner.as_str() == owner)
        .unwrap_or_else(|| panic!("no label of {owner}"))
}

/// The solid `owner` draws in `scene`.
fn solid_of<'a>(scene: &'a IsoScene, owner: &str) -> &'a Solid {
    scene
        .solids
        .iter()
        .find(|solid| solid.pointer.as_str() == owner)
        .unwrap_or_else(|| panic!("no solid of {owner}"))
}

fn center_of(bounds: BoxRect) -> (f32, f32) {
    (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// The first direct `<g data-plane>` child of a node or link group.
fn plane_group<'a, 'input>(
    group: roxmltree::Node<'a, 'input>,
) -> Option<roxmltree::Node<'a, 'input>> {
    plane_groups(group).into_iter().next()
}

/// Every direct `<g data-plane>` child of a node or link group, in document order.
fn plane_groups<'a, 'input>(
    group: roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    group
        .children()
        .filter(|child| child.has_tag_name("g") && child.attribute("data-plane").is_some())
        .collect()
}

/// The six numbers of a `transform="matrix(a b c d e f)"`.
fn matrix_of(group: roxmltree::Node<'_, '_>) -> [f32; 6] {
    let transform = group.attribute("transform").unwrap();
    let inner = transform
        .strip_prefix("matrix(")
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or_else(|| panic!("not a matrix: {transform}"));
    let numbers: Vec<f32> = inner
        .split(' ')
        .map(|number| number.parse().unwrap())
        .collect();
    numbers.try_into().unwrap()
}

/// True when `point` lies inside the convex `polygon`, whose vertices run clockwise on
/// screen, or within 0.01 px of its boundary.
fn inside_convex(point: ScreenPoint, polygon: &[ScreenPoint]) -> bool {
    convex_margin(point, polygon) >= -0.01
}

/// The distance from `point` to the nearest edge line of the convex `polygon`, whose
/// vertices run clockwise on screen: positive inside, negative outside, 0 on the boundary.
fn convex_margin(point: ScreenPoint, polygon: &[ScreenPoint]) -> f32 {
    (0..polygon.len())
        .map(|index| {
            let start = polygon[index];
            let end = polygon[(index + 1) % polygon.len()];
            let cross =
                (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x);
            cross / (end.x - start.x).hypot(end.y - start.y)
        })
        .fold(f32::INFINITY, f32::min)
}

fn screen_distance(first: ScreenPoint, second: ScreenPoint) -> f32 {
    (first.x - second.x).hypot(first.y - second.y)
}

fn defect_messages(report: &stencil_model::checks::CheckReport) -> Vec<String> {
    report
        .defects
        .iter()
        .map(|defect| format!("{} {}", defect.pointer, defect.message))
        .collect()
}

#[test]
fn every_hero_link_end_lands_on_its_node_and_the_check_needs_a_scene_with_links() {
    let hero = hero_page();
    let geometry = common::layout_with_cosmic_text(&hero);
    let scene = project_page(&geometry).unwrap();
    let report = iso_link_ends(&geometry, Some(&scene));
    assert_eq!(report.check, CheckName::IsoLinkEnds);
    assert_eq!(report.examined, 8);
    assert!(report.passed(), "{:?}", defect_messages(&report));
    assert_eq!(
        iso_link_ends(&geometry, None).not_applicable,
        Some("projection is flat")
    );
    let mut no_links = scene.clone();
    no_links.link_paths.clear();
    assert_eq!(
        iso_link_ends(&geometry, Some(&no_links)).not_applicable,
        Some("page has no links")
    );
}

#[test]
fn a_link_end_off_its_block_on_a_corner_or_without_a_solid_is_a_defect() {
    let page = two_zone_link_page("bottom");
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let report = iso_link_ends(&geometry, Some(&scene));
    assert_eq!(report.examined, 2);
    assert!(report.passed(), "{:?}", defect_messages(&report));
    let to_node = geometry.links[0].to_node;
    let right = scene
        .solids
        .iter()
        .find(|solid| solid.node == to_node)
        .unwrap()
        .clone();
    assert_eq!(right.pointer.as_str(), "/body/0/children/1/children/0");

    // The end lifted 5 px off the floor leaves the block's outline by 5 cos 30.
    let mut lifted = scene.clone();
    lifted.link_paths[0].last_mut().unwrap().z += 5.0;
    let report = iso_link_ends(&geometry, Some(&lifted));
    assert_eq!(
        defect_messages(&report),
        ["/links/0 end lies 4.33 px off the outline of /body/0/children/1/children/0"]
    );

    // The end on the footprint's front corner lies on the outline, at a vertex.
    let mut cornered = scene.clone();
    *cornered.link_paths[0].last_mut().unwrap() = IsoPoint {
        x: right.footprint.x + right.footprint.width,
        y: right.footprint.y + right.footprint.height,
        z: right.base_z,
    };
    let report = iso_link_ends(&geometry, Some(&cornered));
    assert_eq!(
        defect_messages(&report),
        ["/links/0 end lies 0.00 px from a corner of /body/0/children/1/children/0, under 8"]
    );

    let mut unsolid = scene.clone();
    unsolid.solids.retain(|solid| solid.node != to_node);
    let report = iso_link_ends(&geometry, Some(&unsolid));
    assert_eq!(report.examined, 2);
    assert_eq!(
        defect_messages(&report),
        ["/links/0 end node /body/0/children/1/children/0 has no solid"]
    );
}

#[test]
fn a_link_end_on_a_zone_lies_inside_its_footprint() {
    let mut document = common::page_document(
        json!([
            {
                "tag": "Row", "gap": 32,
                "children": [
                    { "tag": "Box", "kind": "region", "tint": 1, "label": "A", "children": [
                        { "tag": "Item", "kind": "product", "id": "left", "title": "Left", "shape": "card" } ] },
                    { "tag": "Box", "kind": "region", "tint": 2, "label": "B", "id": "b", "children": [
                        { "tag": "Item", "kind": "product", "title": "Right", "shape": "card" } ] }
                ]
            }
        ]),
        json!([{ "line": "solid", "tint": 1, "text": "request path" }]),
    );
    document["links"] = json!([{ "from": "left", "to": "b", "line": "solid", "tint": 1 }]);
    document["projection"] = json!("iso");
    let page: Page = serde_json::from_value(document).unwrap();
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let report = iso_link_ends(&geometry, Some(&scene));
    assert_eq!(report.examined, 2);
    assert!(report.passed(), "{:?}", defect_messages(&report));

    let mut outside = scene.clone();
    outside.link_paths[0].last_mut().unwrap().x -= 20.0;
    let report = iso_link_ends(&geometry, Some(&outside));
    assert_eq!(
        defect_messages(&report),
        ["/links/0 end lies 20.00 px outside zone /body/0/children/1"]
    );
}

/// A route ends on its node's footprint edge; a figure's hull is narrower than its
/// footprint, so the drawn start reaches in to meet the hull instead of stopping in the air.
#[test]
fn a_link_from_a_figure_starts_on_the_figure_not_on_its_footprint_edge() {
    let people: Page = serde_json::from_str(PEOPLE_JSON).unwrap();
    let flat = common::layout_with_cosmic_text(&people);
    let (geometry, zoom) = zoomed_geometry(&flat).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let report = iso_link_ends(&geometry, Some(&scene));
    assert_eq!(report.examined, 6);
    assert!(report.passed(), "{:?}", defect_messages(&report));
    let operator = &geometry.links[0];
    let route_start = operator.points[0];
    let drawn_start = scene.link_paths[0][0];
    assert!(
        drawn_start.x < route_start.x - 1.0,
        "{drawn_start:?} should reach into the footprint from {route_start:?}"
    );
    let figure = scene
        .solids
        .iter()
        .find(|solid| solid.node == operator.from_node)
        .unwrap();
    let on_screen = project_point(drawn_start.x, drawn_start.y, drawn_start.z, scene.offset);
    let on_hull = figure
        .outline
        .iter()
        .zip(figure.outline.iter().cycle().skip(1))
        .any(|(a, b)| {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let t = (((on_screen.x - a.x) * dx + (on_screen.y - a.y) * dy) / (dx * dx + dy * dy))
                .clamp(0.0, 1.0);
            (on_screen.x - (a.x + t * dx)).hypot(on_screen.y - (a.y + t * dy)) < 0.05
        });
    assert!(on_hull, "{on_screen:?} is not on the figure's outline");
}

/// Two links between the same two blocks, in the same direction, with the sides given.
fn two_link_page(sides: &[(Option<&str>, Option<&str>)]) -> Page {
    let mut document = common::page_document(
        json!([
            {
                "tag": "Row", "gap": 48,
                "children": [
                    { "tag": "Box", "kind": "region", "tint": 1, "label": "A", "children": [
                        { "tag": "Item", "kind": "product", "id": "left", "title": "Left" } ] },
                    { "tag": "Box", "kind": "region", "tint": 2, "label": "B", "children": [
                        { "tag": "Item", "kind": "product", "id": "right", "title": "Right" } ] }
                ]
            }
        ]),
        json!([
            { "line": "solid", "tint": 1, "text": "request path" },
            { "line": "dash", "text": "failover" }
        ]),
    );
    let links: Vec<Value> = sides
        .iter()
        .enumerate()
        .map(|(index, (from_side, to_side))| {
            let mut link = json!({ "from": "left", "to": "right", "label": format!("L{index}") });
            if index == 0 {
                link["line"] = json!("solid");
                link["tint"] = json!(1);
            } else {
                link["line"] = json!("dash");
            }
            if let Some(side) = from_side {
                link["from_side"] = json!(side);
            }
            if let Some(side) = to_side {
                link["to_side"] = json!(side);
            }
            link
        })
        .collect();
    document["links"] = json!(links);
    document["projection"] = json!("iso");
    serde_json::from_value(document).unwrap()
}

#[test]
fn two_links_on_one_route_are_drawn_side_by_side_with_their_tags_staggered() {
    let page = two_link_page(&[(None, None), (None, None)]);
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let (first, second) = (&scene.link_paths[0], &scene.link_paths[1]);
    // The layout routes both links along one line; the projection moves them apart by
    // their tube radii plus the gap, at least two cone bases, along the sides they share.
    assert_eq!(geometry.links[0].points, geometry.links[1].points);
    let spacing = (second[0].y - first[0].y).abs();
    let least =
        (link_tube_radius(Line::Solid, Some(1)) + link_tube_radius(Line::Dash, None)) * zoom;
    assert!(spacing >= 1.9 * least - 0.01, "{spacing} < {}", 1.9 * least);
    assert!((second.last().unwrap().y - first.last().unwrap().y).abs() - spacing < 0.01);
    let apart = iso_links_apart(&geometry, Some(&scene));
    assert_eq!(apart.examined, 1);
    assert!(apart.passed(), "{:?}", defect_messages(&apart));
    let ends = iso_link_ends(&geometry, Some(&scene));
    assert!(ends.passed(), "{:?}", defect_messages(&ends));
    // The right card's left side is hidden, so the pair hangs toward the back from the
    // attach point: the second (front) link keeps the layout's end, the first moves back.
    let end = geometry.links[0].points.last().unwrap();
    assert!(first.last().unwrap().y < end.y - 1.0, "{first:?} {end:?}");
    // Tags sit at a quarter and three quarters of the leg, not both at its middle.
    let (first_tag, second_tag) = (label_of(&scene, "/links/0"), label_of(&scene, "/links/1"));
    let mapped = |tag: &Label| {
        let (x, y) = center_of(tag.flat);
        tag.map.apply(x, y)
    };
    let (first_center, second_center) = (mapped(first_tag), mapped(second_tag));
    assert!(
        second_center.x - first_center.x > 40.0,
        "{first_center:?} {second_center:?}"
    );
    let labels = iso_labels_clear(Some(&scene));
    assert!(labels.passed(), "{:?}", defect_messages(&labels));
}

#[test]
fn two_links_that_share_a_side_but_not_a_route_move_their_end_legs_apart() {
    let page = two_link_page(&[(None, Some("left")), (None, Some("top"))]);
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    let (first, second) = (&scene.link_paths[0], &scene.link_paths[1]);
    // Both leave the left card's right side at the same point in the layout.
    assert_eq!(geometry.links[0].points[0], geometry.links[1].points[0]);
    let spacing = (second[0].y - first[0].y).abs();
    assert!(spacing > 10.0 * zoom, "{spacing}");
    assert!(iso_links_apart(&geometry, Some(&scene)).passed());
}

#[test]
fn a_shared_stretch_a_crowded_pair_and_two_ends_on_one_point_are_defects() {
    let page = two_link_page(&[(None, None), (None, None)]);
    let (geometry, zoom) = zoomed_geometry(&common::layout_with_fixed_metrics(&page)).unwrap();
    let scene = project_zoomed(&geometry, zoom).unwrap();
    // Undo the projection's separation: the second link back on the first's path.
    let mut stacked = scene.clone();
    stacked.link_paths[1] = scene.link_paths[0].clone();
    let report = iso_links_apart(&geometry, Some(&stacked));
    assert_eq!(report.examined, 1);
    let messages = defect_messages(&report);
    assert_eq!(messages.len(), 3, "{messages:?}");
    assert!(messages[0].starts_with("/links/1 shares "), "{messages:?}");
    assert!(messages[0].ends_with(" px with /links/0"), "{messages:?}");
    assert!(
        messages[1].starts_with("/links/1 ends 0.00 px from the end of /links/0 on /body/0/children/0/children/0, closer than"),
        "{messages:?}"
    );
    assert!(
        messages[2].starts_with("/links/1 ends 0.00 px from the end of /links/0 on /body/0/children/1/children/0, closer than"),
        "{messages:?}"
    );
    // Shifted by 3 px: no shared stretch, but a parallel run closer than the tubes allow.
    let mut crowded = scene.clone();
    crowded.link_paths[1] = scene.link_paths[0]
        .iter()
        .map(|point| IsoPoint {
            x: point.x,
            y: point.y + 3.0,
            z: point.z,
        })
        .collect();
    let report = iso_links_apart(&geometry, Some(&crowded));
    let messages = defect_messages(&report);
    let least =
        (link_tube_radius(Line::Solid, Some(1)) + link_tube_radius(Line::Dash, None)) * zoom + 4.0;
    assert_eq!(
        messages[0],
        format!("/links/1 runs 3.00 px beside /links/0, closer than {least:.2}")
    );
    assert_eq!(
        iso_links_apart(&geometry, None).not_applicable,
        Some("projection is flat")
    );
    let mut lone = scene.clone();
    lone.link_paths.truncate(1);
    assert_eq!(
        iso_links_apart(&geometry, Some(&lone)).not_applicable,
        Some("page has fewer than two links")
    );
}

/// A ring at the far dot end is painted under the body, which comes out of it without a
/// rounded far end; a ring at the near end is painted over the body. Before this, the far
/// ring's dark face covered the tube where it should pass through the ring.
#[test]
fn a_tube_comes_out_of_its_far_flange_and_ends_under_its_near_one() {
    let mut page: Page = serde_json::from_str(common::G7_JSON).unwrap();
    page.projection = stencil_model::Projection::Iso;
    let geometry = common::layout_with_cosmic_text(&page);
    let iso = render_svg(&page, &geometry).unwrap();
    let document = common::parse_xml(&iso.svg);
    let (mut filled, mut hollow) = (0, 0);
    for node in geometry
        .nodes
        .iter()
        .filter(|node| node.tag == NodeTag::Pipe)
    {
        let group = common::group(&document, &node.pointer.to_string());
        // Shadows are the polygons with an opacity; the rest are caps and body halves.
        let fills: Vec<&str> = group
            .descendants()
            .filter(|element| {
                element.has_tag_name("polygon") && element.attribute("fill-opacity").is_none()
            })
            .filter_map(|element| element.attribute("fill"))
            .collect();
        // Every g7 pipe runs +x with no arrow. A hollow pipe (a patterned line) is the far
        // ring (far cap, outline, near cap), the body (outline, near cap) with no far cap,
        // and the near ring, all in the page background. A filled one is the far ring (far
        // cap, lit, shaded, dark near cap), the body (lit, shaded, dark near cap) with no
        // far cap, and the near ring.
        if fills.iter().all(|fill| *fill == fills[0]) {
            assert_eq!(fills.len(), 8, "{}: {fills:?}", node.pointer);
            hollow += 1;
        } else {
            assert_eq!(fills.len(), 11, "{}: {fills:?}", node.pointer);
            let (wire, lit, dark) = (fills[0], fills[1], fills[3]);
            assert_eq!(fills[2], wire, "{}: {fills:?}", node.pointer);
            assert_eq!(
                &fills[4..7],
                &[lit, wire, dark],
                "{}: {fills:?}",
                node.pointer
            );
            assert_eq!(
                &fills[7..11],
                &[wire, lit, wire, dark],
                "{}: {fills:?}",
                node.pointer
            );
            filled += 1;
        }
    }
    assert!(filled > 0 && hollow > 0, "{filled} filled, {hollow} hollow");
}
