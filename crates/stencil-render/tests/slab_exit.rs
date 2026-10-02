#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 13.11 rule 2: under iso a link leaves the slab its from node stands on through
//! the edge nearest its target.

mod common;

use serde_json::{Value, json};
use stencil_layout::PartName;
use stencil_layout::{BoxRect, PageGeometry};
use stencil_model::{Page, Projection, Side};
use stencil_render::iso::{IsoPoint, IsoScene, SolidInputs, SolidShape, project_page};

const HERO_JSON: &str = include_str!("../../../examples/hero-iso.json");
const SLAB: &str = "/body/0/children/0/children/0";

/// A Row of an on-prem Box holding item `a`, and a Col holding `fillers` items above item
/// `b`, 64 apart, with one link from `a` to `b` carrying `extra` fields.
fn site_page(fillers: usize, extra: Value) -> Page {
    let mut link = json!({ "from": "a", "to": "b", "line": "solid", "tint": 1 });
    for (key, value) in extra.as_object().unwrap() {
        link[key] = value.clone();
    }
    let mut column: Vec<Value> = (0..fillers)
        .map(|index| json!({ "tag": "Item", "kind": "product", "title": format!("Cache {index}"), "shape": "card" }))
        .collect();
    column.push(
        json!({ "tag": "Item", "kind": "product", "id": "b", "title": "Gateway", "shape": "card" }),
    );
    let mut document = common::page_document(
        json!([{ "tag": "Row", "gap": 64, "grow": [0, 0], "children": [
            { "tag": "Col", "children": [
                { "tag": "Box", "kind": "onprem", "tint": 1, "label": "Site", "children": [
                    { "tag": "Item", "kind": "product", "id": "a", "title": "Router", "shape": "card" }
                ] }
            ] },
            { "tag": "Col", "gap": 64, "children": column }
        ] }]),
        json!([{ "line": "solid", "tint": 1, "text": "attachment" }]),
    );
    document["links"] = json!([link]);
    let mut page: Page = serde_json::from_value(document).unwrap();
    page.projection = Projection::Iso;
    page
}

fn scene_of(page: &Page, geometry: &PageGeometry) -> IsoScene {
    project_page(geometry, &SolidInputs::new(geometry, &page.links, 6.0)).unwrap()
}

fn slab_footprint(scene: &IsoScene, pointer: &str) -> BoxRect {
    scene
        .solids
        .iter()
        .find(|solid| solid.pointer.as_str() == pointer && solid.shape == SolidShape::Slab)
        .unwrap()
        .footprint
}

/// The sides a drawn path passes through `bounds` by, in order, sampled every 0.25 px.
fn sides_crossed(path: &[IsoPoint], bounds: BoxRect) -> Vec<Side> {
    let inside =
        |x: f32, y: f32| x > bounds.x && x < bounds.right() && y > bounds.y && y < bounds.bottom();
    let side_of = |x: f32, y: f32| {
        if x >= bounds.right() {
            Side::Right
        } else if y >= bounds.bottom() {
            Side::Bottom
        } else if x <= bounds.x {
            Side::Left
        } else {
            Side::Top
        }
    };
    let mut crossed = Vec::new();
    let mut was_inside = inside(path[0].x, path[0].y);
    for leg in path.windows(2) {
        let length = ((leg[1].x - leg[0].x).powi(2) + (leg[1].y - leg[0].y).powi(2)).sqrt();
        let steps = (length / 0.25).ceil().max(1.0) as usize;
        for step in 1..=steps {
            let fraction = step as f32 / steps as f32;
            let x = leg[0].x + (leg[1].x - leg[0].x) * fraction;
            let y = leg[0].y + (leg[1].y - leg[0].y) * fraction;
            let now_inside = inside(x, y);
            if now_inside != was_inside {
                crossed.push(if now_inside {
                    side_of(leg[0].x, leg[0].y)
                } else {
                    side_of(x, y)
                });
            }
            was_inside = now_inside;
        }
    }
    crossed
}

#[test]
fn a_link_that_leaves_on_the_far_edge_is_rerouted_through_the_edge_nearest_its_target() {
    let page = site_page(4, json!({}));
    let geometry = common::layout_with_fixed_metrics(&page);
    let route = &geometry.links[0];
    let router = geometry.nodes[route.from_node].bounds;
    assert_eq!(
        route.points[0].y,
        router.bottom(),
        "the flat route leaves the router's bottom"
    );
    let scene = scene_of(&page, &geometry);
    let slab = slab_footprint(&scene, SLAB);
    let path = &scene.link_paths[0];
    assert_eq!(sides_crossed(path, slab), [Side::Right], "{path:?}");

    // Without the slab exit the drawn path leaves through the bottom edge.
    let unrouted = project_page(&geometry, &SolidInputs::new(&geometry, &[], 6.0)).unwrap();
    assert_eq!(
        sides_crossed(&unrouted.link_paths[0], slab)[0],
        Side::Bottom
    );

    // The flat route does not change.
    assert_eq!(geometry, common::layout_with_fixed_metrics(&page));
}

#[test]
fn a_link_that_already_leaves_through_the_exit_edge_keeps_its_route() {
    let page = site_page(0, json!({}));
    let geometry = common::layout_with_fixed_metrics(&page);
    let with_exit = scene_of(&page, &geometry);
    let without = project_page(&geometry, &SolidInputs::new(&geometry, &[], 6.0)).unwrap();
    let slab = slab_footprint(&with_exit, SLAB);
    assert_eq!(sides_crossed(&without.link_paths[0], slab), [Side::Right]);
    assert_eq!(with_exit.link_paths, without.link_paths);
    assert_eq!(with_exit, without);
}

#[test]
fn an_authored_via_or_from_side_is_never_rerouted() {
    // A via point straight under the router and below the slab, so the authored route
    // leaves through the bottom edge.
    let plain = common::layout_with_fixed_metrics(&site_page(4, json!({})));
    let router = plain.links[0].from_node;
    let (router_box, slab_box) = (plain.nodes[router].bounds, plain.nodes[router - 1].bounds);
    let under =
        json!({ "x": router_box.x + router_box.width / 2.0, "y": slab_box.bottom() + 40.0 });
    let authored = [json!({ "from_side": "bottom" }), json!({ "via": [under] })];
    for extra in authored {
        let page = site_page(4, extra.clone());
        let geometry = common::layout_with_fixed_metrics(&page);
        let with_exit = scene_of(&page, &geometry);
        let without = project_page(&geometry, &SolidInputs::new(&geometry, &[], 6.0)).unwrap();
        let slab = slab_footprint(&with_exit, SLAB);
        assert_eq!(with_exit.link_paths, without.link_paths, "{extra}");
        assert_eq!(
            sides_crossed(&with_exit.link_paths[0], slab)[0],
            Side::Bottom,
            "{extra}"
        );
    }
}

/// The hero's VLAN 2 is authored with a `via` in the gap between the sites, so it keeps
/// the route its author drew: no slab exit moves it, and it passes through the waypoint.
#[test]
fn the_hero_vlan_2_keeps_its_authored_route() {
    let page: Page = serde_json::from_str(HERO_JSON).unwrap();
    let vlan_2 = &page.links[1];
    assert_eq!(vlan_2.label.as_deref(), Some("VLAN 2"));
    assert_eq!(vlan_2.via.len(), 1);

    let geometry = common::layout_with_cosmic_text(&page);
    let with_exit = scene_of(&page, &geometry);
    let without = project_page(&geometry, &SolidInputs::new(&geometry, &[], 6.0)).unwrap();
    assert_eq!(with_exit, without, "no hero link takes a slab exit");

    let route = &geometry.links[1];
    let via = vlan_2.via[0];
    assert!(
        route
            .points
            .iter()
            .any(|point| (point.x - via.x).abs() < 0.01 && (point.y - via.y).abs() < 0.01),
        "{:?}",
        route.points
    );
    let router = &geometry.nodes[route.from_node];
    // The router stands on a footprint (a tile), which is where its links attach.
    let footprint = router.part(PartName::Footprint).unwrap().bounds;
    let start = route.points[0];
    assert!(
        start.x >= footprint.x && start.x <= footprint.right(),
        "{start:?} {footprint:?}"
    );
}
