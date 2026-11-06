#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{assert_close, layout, node, page_from};
use serde_json::{Value, json};
use stencil_layout::checks::{links_avoid_boxes, links_routed, text_fits_box};
use stencil_layout::sheet_lines::{OUTLINE_AIR_PX, legs_shared};
use stencil_layout::{
    BoxRect, LINK_ARROW_STUB_PX, LinkRoute, OBSTACLE_CLEARANCE_PX, PageGeometry, PartName,
    RouteStatus, shape_height_px,
};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::{LINK_SEGMENTS_MAX, PagePoint, Shape};

/// A customer page at width 640 with a blue legend entry so links of kind blue are listed.
fn linked_page(body: Value, links: Value) -> Value {
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": 640,
        "badge": "Customer",
        "grammar": "gcp",
        "body": body,
        "legend": [{ "line": "solid", "tint": 1, "text": "request path" }],
        "links": links
    })
}

fn card(id: &str) -> Value {
    json!({ "tag": "Item", "kind": "product", "id": id, "title": id })
}

fn route_of(geometry: &PageGeometry, index: usize) -> &LinkRoute {
    geometry
        .links
        .iter()
        .find(|route| route.index == index)
        .unwrap_or_else(|| panic!("no route for link {index}"))
}

/// Strict interior test, the one the router and links-avoid-boxes use, written out again.
fn segment_enters(start: PagePoint, end: PagePoint, bounds: &BoxRect) -> bool {
    start.x.max(end.x) > bounds.x + 0.01
        && start.x.min(end.x) < bounds.right() - 0.01
        && start.y.max(end.y) > bounds.y + 0.01
        && start.y.min(end.y) < bounds.bottom() - 0.01
}

fn assert_orthogonal(points: &[PagePoint]) {
    assert!(points.len() >= 2 && points.len() <= LINK_SEGMENTS_MAX + 1);
    for pair in points.windows(2) {
        assert!(
            pair[0].x == pair[1].x || pair[0].y == pair[1].y,
            "diagonal segment {pair:?}"
        );
    }
}

fn on_segment(point: PagePoint, start: PagePoint, end: PagePoint) -> bool {
    let within = |value: f32, a: f32, b: f32| value >= a.min(b) - 0.01 && value <= a.max(b) + 0.01;
    within(point.x, start.x, end.x) && within(point.y, start.y, end.y)
}

/// Three cards in a Row: a link from the first to the third has the middle card in its way.
fn row_with_obstacle(links: Value) -> Value {
    linked_page(
        json!([{ "tag": "Row", "gap": 64, "children": [card("a"), card("b"), card("c")] }]),
        links,
    )
}

#[test]
fn a_link_routed_around_one_obstacle_never_enters_it() {
    let page = page_from(row_with_obstacle(
        json!([{ "from": "a", "to": "c", "line": "solid", "tint": 1 }]),
    ));
    let geometry = layout(&page);
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    assert_orthogonal(&route.points);
    let from = node(&geometry, "/body/0/children/0").bounds;
    let obstacle = node(&geometry, "/body/0/children/1").bounds;
    let to = node(&geometry, "/body/0/children/2").bounds;

    let first = route.points[0];
    let last = *route.points.last().unwrap();
    assert_close(first.x, from.right(), "starts on the from box's right side");
    assert_close(first.y, from.y + from.height / 2.0, "at its midpoint");
    assert_close(last.x, to.x, "ends on the to box's left side");
    assert_close(last.y, to.y + to.height / 2.0, "at its midpoint");
    for pair in route.points.windows(2) {
        assert!(
            !segment_enters(pair[0], pair[1], &obstacle),
            "segment {pair:?} enters the obstacle {obstacle:?}"
        );
    }
    assert!(
        route.points.len() > 2,
        "the straight line is blocked, so the route turns"
    );

    let report = links_avoid_boxes(&geometry);
    assert!(report.passed(), "{report:?}");
    assert!(links_routed(&page, &geometry).passed());
}

#[test]
fn a_via_route_passes_its_point() {
    let body = json!([{ "tag": "Col", "gap": 64, "children": [
        { "tag": "Row", "gap": 64, "children": [card("a"), card("c")] },
        { "tag": "Fact", "text": "Below" }
    ]}]);
    let unlinked = layout(&page_from(linked_page(body.clone(), json!([]))));
    let from = node(&unlinked, "/body/0/children/0/children/0").bounds;
    let via = PagePoint {
        x: from.right() + 32.0,
        y: from.bottom() + 32.0,
    };
    let geometry = layout(&page_from(linked_page(
        body,
        json!([{ "from": "a", "to": "c", "line": "solid", "tint": 1, "via": [{ "x": via.x, "y": via.y }] }]),
    )));
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    assert_orthogonal(&route.points);
    assert!(
        route
            .points
            .windows(2)
            .any(|pair| on_segment(via, pair[0], pair[1])),
        "{:?} misses {via:?}",
        route.points
    );
    assert!(links_avoid_boxes(&geometry).passed());
}

#[test]
fn an_enclosed_endpoint_falls_back_and_fails_links_routed() {
    let grid_row = |ids: [&str; 3]| json!({ "tag": "Row", "gap": 0, "children": [card(ids[0]), card(ids[1]), card(ids[2])] });
    let body = json!([{ "tag": "Col", "gap": 64, "children": [
        { "tag": "Col", "gap": 0, "children": [
            grid_row(["n1", "n2", "n3"]),
            grid_row(["w", "center", "e"]),
            grid_row(["s1", "s2", "s3"])
        ]},
        card("target")
    ]}]);
    let page = page_from(linked_page(
        body,
        json!([{ "from": "center", "to": "target", "line": "solid", "tint": 1 }]),
    ));
    let geometry = layout(&page);
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Fallback);

    let start = route.points[0];
    let end = *route.points.last().unwrap();
    let corner = PagePoint {
        x: end.x,
        y: start.y,
    };
    let expected: Vec<PagePoint> =
        [start, corner, end]
            .into_iter()
            .fold(Vec::new(), |mut points, point| {
                if points.last() != Some(&point) {
                    points.push(point);
                }
                points
            });
    assert_eq!(route.points, expected, "an L, horizontal leg first");

    let report = links_routed(&page, &geometry);
    assert_eq!(report.examined, 1);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/links/0");
}

#[test]
fn link_checks_do_not_apply_to_a_page_without_links() {
    let page = page_from(row_with_obstacle(json!([])));
    let geometry = layout(&page);
    assert!(geometry.links.is_empty());
    for report in [links_routed(&page, &geometry), links_avoid_boxes(&geometry)] {
        assert_eq!(report.examined, 0);
        assert!(report.defects.is_empty());
        assert_eq!(report.outcome(), CheckOutcome::NotApplicable, "{report:?}");
        assert!(!report.passed());
    }
    assert_eq!(links_routed(&page, &geometry).check, CheckName::LinksRouted);
    assert_eq!(
        links_avoid_boxes(&geometry).check,
        CheckName::LinksAvoidBoxes
    );
}

#[test]
fn link_checks_count_links_and_segment_obstacle_pairs() {
    let page = page_from(row_with_obstacle(json!([
        { "from": "a", "to": "b", "line": "solid", "tint": 1 },
        { "from": "a", "to": "c", "line": "solid", "tint": 1 }
    ])));
    let geometry = layout(&page);
    let routed = links_routed(&page, &geometry);
    assert_eq!(routed.examined, 2);
    assert!(routed.passed());
    // Obstacles outside the endpoints: /kicker, /title, /lede, the legend entry and the
    // card that is not an endpoint. Neither link has a tag.
    let segments: usize = geometry
        .links
        .iter()
        .map(|route| route.points.len() - 1)
        .sum();
    assert_eq!(links_avoid_boxes(&geometry).examined, (segments * 5) as u64);
}

/// A gutter column draws nothing, so a tag inside it lies on open floor; only the column's
/// children can meet it.
#[test]
fn a_tag_inside_a_gutter_column_is_not_a_defect() {
    let tall = |id: &str| {
        json!({ "tag": "Item", "kind": "product", "id": id, "title": id, "facts": [
            { "text": "one", "source": "built" }, { "text": "two", "source": "built" },
            { "text": "three", "source": "built" }, { "text": "four", "source": "built" }
        ] })
    };
    let body = json!([{ "tag": "Row", "grow": [0, 1, 0], "children": [
        tall("a"),
        { "tag": "Col", "children": [{ "tag": "Col", "children": [
            { "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": "trunk" }
        ] }] },
        tall("b")
    ] }]);
    let plain = page_from(linked_page(body.clone(), json!([])));
    let geometry = layout(&plain);
    let column = node(&geometry, "/body/0/children/1");
    let via_x = column.bounds.x + column.bounds.width / 2.0;
    let via_y = column.bounds.y + 12.0;
    let page = page_from(linked_page(
        body,
        json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "label": "HTTPS",
                 "via": [{ "x": via_x, "y": via_y }] }]),
    ));
    let geometry = layout(&page);
    let tag = route_of(&geometry, 0)
        .tag
        .expect("a labelled link has a tag");
    let column = node(&geometry, "/body/0/children/1");
    assert!(
        tag.x < column.bounds.right() && tag.right() > column.bounds.x,
        "the tag {tag:?} lies across the column {:?}",
        column.bounds
    );
    let report = links_avoid_boxes(&geometry);
    assert!(report.passed(), "{report:?}");
}

#[test]
fn a_tag_is_sized_like_a_pipe_tag_and_centered_on_the_longest_segment() {
    let geometry = layout(&page_from(row_with_obstacle(json!([
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "label": "step 1", "sub": "HTTPS" }
    ]))));
    let route = route_of(&geometry, 0);
    assert_eq!(route.points.len(), 2, "adjacent cards link straight across");
    let tag = route.tag.expect("a labeled link has a tag");
    // Label 6 characters and sub 5 at 6 px, 15.6 px lines, 2 px apart, inside the 1.5 px
    // border and 6/8 padding of a pipe tag.
    assert_close(tag.width, 36.0 + 16.0 + 3.0, "tag width");
    assert_close(tag.height, 15.6 + 2.0 + 15.6 + 12.0 + 3.0, "tag height");
    let (start, end) = (route.points[0], route.points[1]);
    assert_close(
        tag.x + tag.width / 2.0,
        (start.x + end.x) / 2.0,
        "centered across",
    );
    assert_close(tag.y + tag.height / 2.0, start.y, "centered on the line");
    let names: Vec<PartName> = route.parts.iter().map(|part| part.name).collect();
    assert_eq!(
        names,
        vec![PartName::Tag, PartName::TagLabel, PartName::TagSub]
    );
    assert_close(route.parts[2].bounds.width, 30.0, "sub width");
    assert_close(
        route.parts[2].bounds.x,
        tag.x + 1.5 + 8.0 + 3.0,
        "sub centered under the label",
    );
    let report = text_fits_box(&geometry);
    assert!(report.passed(), "{report:?}");
}

#[test]
fn a_forced_side_leaves_through_that_side() {
    let geometry = layout(&page_from(row_with_obstacle(json!([
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": "none",
          "from_side": "top", "to_side": "top" }
    ]))));
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    let from = node(&geometry, "/body/0/children/0").bounds;
    let to = node(&geometry, "/body/0/children/1").bounds;
    let first = route.points[0];
    assert_close(first.x, from.x + from.width / 2.0, "top midpoint x");
    assert_close(first.y, from.y, "top midpoint y");
    assert!(route.points[1].y < first.y, "the first segment goes up");
    let last = *route.points.last().unwrap();
    assert_close(last.y, to.y, "arrives on the top side");
    let before_last = route.points[route.points.len() - 2];
    assert!(before_last.y < last.y, "the last segment comes down");
}

#[test]
fn routing_is_a_pure_function_of_the_geometry() {
    let page = page_from(row_with_obstacle(json!([
        { "from": "a", "to": "c", "line": "solid", "tint": 1, "label": "one" },
        { "from": "c", "to": "a", "line": "solid", "tint": 1 }
    ])));
    assert_eq!(layout(&page).links, layout(&page).links);
}

/// Section 13.11 rule 3: asked for the sides and via points layout used, `reroute_link`
/// gives back layout's route, tag included.
#[test]
fn reroute_link_with_an_unchanged_request_returns_the_layout_route() {
    use stencil_layout::reroute_link;
    use stencil_model::Side;

    let geometry = layout(&page_from(row_with_obstacle(json!([
        { "from": "a", "to": "c", "line": "solid", "tint": 1, "label": "VLAN 1" },
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "from_side": "bottom", "to_side": "bottom" },
        { "from": "c", "to": "a", "line": "solid", "tint": 1, "via": [{ "x": 400.0, "y": 300.0 }] }
    ]))));
    let requests: [(usize, Side, Vec<PagePoint>); 3] = [
        (0, Side::Right, Vec::new()),
        (1, Side::Bottom, Vec::new()),
        (2, Side::Left, vec![PagePoint { x: 400.0, y: 300.0 }]),
    ];
    for (index, from_side, via) in requests {
        let routed = route_of(&geometry, index);
        let rerouted = reroute_link(&geometry, index, from_side, &via, false).unwrap();
        assert_eq!(&rerouted, routed, "link {index}");
    }
    assert!(reroute_link(&geometry, 3, Side::Right, &[], false).is_none());

    // Two links leaving one side spread along it; an unchanged request keeps each end.
    let geometry = layout(&page_from(fan_out_page(json!([
        { "from": "front", "to": "alpha", "line": "solid", "tint": 1, "label": "1",
          "from_side": "right", "to_side": "left" },
        { "from": "front", "to": "beta", "line": "solid", "tint": 1, "label": "2",
          "from_side": "right", "to_side": "left" }
    ]))));
    for index in 0..2 {
        let routed = route_of(&geometry, index);
        let rerouted = reroute_link(&geometry, index, Side::Right, &[], false).unwrap();
        assert_eq!(&rerouted, routed, "fan-out link {index}");
    }
}

/// A different request routes through the given via point, leaves from the given side and
/// keeps the to side; the tag moves with the new longest segment.
#[test]
fn reroute_link_routes_through_a_new_via_point() {
    use stencil_layout::reroute_link;
    use stencil_model::Side;

    let geometry = layout(&page_from(row_with_obstacle(json!([
        { "from": "a", "to": "c", "line": "solid", "tint": 1, "label": "VLAN 1" }
    ]))));
    let routed = route_of(&geometry, 0).clone();
    let a = node(&geometry, "/body/0/children/0").bounds;
    let via = PagePoint {
        x: a.x + a.width / 2.0,
        y: a.bottom() + 40.0,
    };
    let rerouted = reroute_link(&geometry, 0, Side::Bottom, &[via], false).unwrap();
    assert_eq!(rerouted.status, RouteStatus::Routed);
    assert_orthogonal(&rerouted.points);
    assert_eq!(
        rerouted.points[0],
        PagePoint {
            x: via.x,
            y: a.bottom()
        }
    );
    assert!(
        rerouted
            .points
            .windows(2)
            .any(|pair| on_segment(via, pair[0], pair[1])),
        "{:?} misses {via:?}",
        rerouted.points
    );
    assert_eq!(rerouted.points.last(), routed.points.last());
    assert_ne!(rerouted.points, routed.points);
    let (old_tag, new_tag) = (routed.tag.unwrap(), rerouted.tag.unwrap());
    assert_eq!(
        (new_tag.width, new_tag.height),
        (old_tag.width, old_tag.height)
    );
    assert_eq!(rerouted.parts.len(), routed.parts.len());
    for (new_part, old_part) in rerouted.parts.iter().zip(&routed.parts) {
        assert_close(
            new_part.bounds.x - old_part.bounds.x,
            new_tag.x - old_tag.x,
            "part moves with the tag",
        );
    }
    assert_eq!(
        route_of(&geometry, 0),
        &routed,
        "the geometry is not changed"
    );
}

/// Section 12.4: a stub out of a block end never reaches past the nearest obstacle ahead
/// of it, so a link that skips over its neighbours in a Row still finds a route.
#[test]
fn an_iso_link_over_two_row_neighbours_routes_around_them() {
    // A figure, a laptop, a card and a phone: the row of the people example, linked end
    // to end.
    let mut document = linked_page(
        json!([{ "tag": "Box", "kind": "region", "label": "Front door", "children": [
            { "tag": "Row", "children": [
                { "tag": "Item", "kind": "person", "id": "a", "title": "Operator", "subtitle": "on call" },
                { "tag": "Item", "kind": "device", "id": "b", "title": "Laptop", "subtitle": "staff" },
                { "tag": "Item", "kind": "product", "id": "c", "title": "API", "subtitle": "gateway" },
                { "tag": "Item", "kind": "device", "id": "d", "title": "Phone", "subtitle": "customer",
                  "shape": "phone" } ] } ] }]),
        json!([{ "from": "a", "to": "c", "line": "solid", "tint": 1 }]),
    );
    document["projection"] = json!("iso");
    document["width"] = json!(1100);
    let page = page_from(document);
    let geometry = layout(&page);
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed, "{:?}", route.points);
    let avoided = links_avoid_boxes(&geometry);
    assert!(avoided.passed(), "{avoided:?}");
    assert!(avoided.examined > 0);
    // Under iso the laptop between the ends covers the floor behind and left of its
    // footprint by its height on screen, so no leg runs through that strip either
    // (section 12.4): the route keeps the obstacle clearance from the footprint grown by
    // the laptop's height toward -x and -y.
    let laptop = geometry
        .nodes
        .iter()
        .find(|node| node.pointer.as_str() == "/body/0/children/0/children/1")
        .unwrap();
    let footprint = laptop.part(PartName::Footprint).unwrap().bounds;
    let height = shape_height_px(Shape::Laptop);
    let grown = BoxRect {
        x: footprint.x - height - OBSTACLE_CLEARANCE_PX + 0.01,
        y: footprint.y - height - OBSTACLE_CLEARANCE_PX + 0.01,
        width: footprint.width + height + 2.0 * OBSTACLE_CLEARANCE_PX - 0.02,
        height: footprint.height + height + 2.0 * OBSTACLE_CLEARANCE_PX - 0.02,
    };
    for pair in route.points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let crosses = a.x.min(b.x) < grown.right()
            && a.x.max(b.x) > grown.x
            && a.y.min(b.y) < grown.bottom()
            && a.y.max(b.y) > grown.y;
        assert!(
            !crosses,
            "leg {a:?} to {b:?} runs through the laptop's shadow {grown:?}"
        );
    }
}

/// A route laid along the top and right edges of a box it does not join passes as a
/// polyline, but its bend rounds into the box: links-avoid-boxes reads the drawn line.
#[test]
fn links_avoid_boxes_reads_the_rounded_bend_not_the_polyline() {
    let page = page_from(linked_page(
        json!([{ "tag": "Row", "gap": 64, "children": [card("a"), card("middle"), card("b")] }]),
        json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": "none" }]),
    ));
    let mut geometry = layout(&page);
    let middle = node(&geometry, "/body/0/children/1").bounds;
    let route = &mut geometry.links[0];
    route.points = vec![
        PagePoint {
            x: middle.x - 30.0,
            y: middle.y,
        },
        PagePoint {
            x: middle.right(),
            y: middle.y,
        },
        PagePoint {
            x: middle.right(),
            y: middle.bottom() + 30.0,
        },
    ];
    route.corner = 0.0;
    let square = links_avoid_boxes(&geometry);
    assert!(square.defects.is_empty(), "{:?}", square.defects);

    geometry.links[0].corner = 16.0;
    let rounded = links_avoid_boxes(&geometry);
    assert_eq!(rounded.examined, square.examined);
    assert_eq!(rounded.defects.len(), 1, "{:?}", rounded.defects);
    assert!(
        rounded.defects[0].message.starts_with("segment 0 "),
        "{}",
        rounded.defects[0].message
    );
}

fn leg_length(from: PagePoint, to: PagePoint) -> f32 {
    (to.x - from.x).abs() + (to.y - from.y).abs()
}

/// A card beside a card nested in a region and a subnet, whose center line lies below the
/// first card's by the two labels: the route steps down beside the target.
fn nested_target_page(gap: u32, arrow: &str) -> Value {
    linked_page(
        json!([{ "tag": "Row", "gap": gap, "children": [
            card("a"),
            { "tag": "Box", "kind": "region", "label": "Region", "children": [
                { "tag": "Box", "kind": "subnet", "label": "Subnet", "children": [card("b")] } ] } ] }]),
        json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": arrow }]),
    )
}

/// Section 11.2: the router searches to a point LINK_ARROW_STUB_PX out from an arrowed
/// end, so the stroke runs straight into the arrowhead's base instead of into its flank.
/// The obstacle clearance alone left a last leg of 8 px, under the 10 px arrowhead.
#[test]
fn an_arrowed_end_arrives_on_a_straight_of_the_arrow_stub() {
    let mut examined = 0;
    for (arrow, gaps) in [("end", &[8, 16, 24, 32, 64][..]), ("both", &[32, 64][..])] {
        for &gap in gaps {
            let page = page_from(nested_target_page(gap, arrow));
            let geometry = layout(&page);
            let route = route_of(&geometry, 0);
            assert_eq!(route.status, RouteStatus::Routed, "{arrow} gap {gap}");
            assert_orthogonal(&route.points);
            let count = route.points.len();
            assert!(
                count > 2,
                "{arrow} gap {gap}: the route steps to the lower center line"
            );
            let first = leg_length(route.points[0], route.points[1]);
            let last = leg_length(route.points[count - 2], route.points[count - 1]);
            assert!(
                last >= LINK_ARROW_STUB_PX - 0.01,
                "{arrow} gap {gap}: last leg {last:.2} in {:?}",
                route.points
            );
            if arrow == "both" {
                assert!(
                    first >= LINK_ARROW_STUB_PX - 0.01,
                    "gap {gap}: first leg {first:.2} in {:?}",
                    route.points
                );
            }
            // The step leg needs the first leg's length, the arrow stub at an arrowed start
            // and 16 px otherwise, plus OUTLINE_AIR_PX before the Region it enters.
            let first_min = if arrow == "both" {
                LINK_ARROW_STUB_PX
            } else {
                16.0
            };
            let checked = links_routed(&page, &geometry);
            assert_eq!(checked.examined, 1);
            if (gap as f32) < first_min + OUTLINE_AIR_PX {
                assert!(
                    !checked.defects.is_empty()
                        && checked
                            .defects
                            .iter()
                            .all(|defect| defect.message.contains("along the outline of")),
                    "{arrow} gap {gap}: {checked:?}"
                );
            } else {
                assert!(checked.passed(), "{arrow} gap {gap}: {checked:?}");
            }
            examined += 1;
        }
    }
    assert_eq!(examined, 7);
}

/// A stub with no room falls back to the L of section 11.2. links-routed reports the
/// fallback, and an arrowed end of the L on a leg shorter than the stub as well.
#[test]
fn a_blocked_arrow_stub_falls_back_and_links_routed_reports_the_short_leg() {
    // A card in a region 8 px from the source: the target's left side lies 21.5 px from
    // the source's right side, so its stub reaches into the source card.
    let page = page_from(linked_page(
        json!([{ "tag": "Row", "gap": 8, "children": [
            card("a"),
            { "tag": "Box", "kind": "region", "label": "Region", "children": [card("b")] } ] }]),
        json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1 }]),
    ));
    let geometry = layout(&page);
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Fallback, "{:?}", route.points);
    let count = route.points.len();
    let last = leg_length(route.points[count - 2], route.points[count - 1]);
    assert!(last < LINK_ARROW_STUB_PX, "{:?}", route.points);
    let report = links_routed(&page, &geometry);
    assert_eq!(report.examined, 1);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
    let messages: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.message.as_str())
        .collect();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages[0]
            .ends_with("avoids every obstacle with a 22.00 px arrow stub; drawn as a fallback L"),
        "{}",
        messages[0]
    );
    assert_eq!(
        messages[1],
        format!("last leg is {last:.2} px, shorter than the arrowhead and a bend (22.00 px)")
    );
    assert!(
        report
            .defects
            .iter()
            .all(|defect| defect.pointer.as_str() == "/links/0")
    );

    // The same pair at gap 32 routes, and the report counts the link with no defect.
    let page = page_from(linked_page(
        json!([{ "tag": "Row", "gap": 32, "children": [
            card("a"),
            { "tag": "Box", "kind": "region", "label": "Region", "children": [card("b")] } ] }]),
        json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1 }]),
    ));
    let geometry = layout(&page);
    let report = links_routed(&page, &geometry);
    assert_eq!(report.examined, 1);
    assert!(report.defects.is_empty(), "{:?}", report.defects);
    assert_eq!(report.outcome(), CheckOutcome::Passed);
}

/// The lede lies 16 px above the row, so a stub up from a card reaches into it, and two
/// facing stubs closer than twice the stub pass each other: neither has a route.
#[test]
fn an_arrow_stub_into_an_obstacle_or_past_the_other_stub_falls_back() {
    let page = page_from(row_with_obstacle(json!([
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "from_side": "top", "to_side": "top" },
        { "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": "none",
          "from_side": "top", "to_side": "top" }
    ])));
    let geometry = layout(&page);
    assert_eq!(route_of(&geometry, 0).status, RouteStatus::Fallback);
    assert_eq!(route_of(&geometry, 1).status, RouteStatus::Routed);

    let both = |gap: u32| {
        let page = page_from(linked_page(
            json!([{ "tag": "Row", "gap": gap, "children": [card("a"), card("b")] }]),
            json!([{ "from": "a", "to": "b", "line": "solid", "tint": 1, "arrow": "both" }]),
        ));
        route_of(&layout(&page), 0).status
    };
    assert_eq!(both(40), RouteStatus::Fallback);
    assert_eq!(both(44), RouteStatus::Routed);
}

/// Section 11.2: on a flat page each drawn end mark of a link, an arrowhead or a deny line's
/// stop plate, is clear of every tag and of every other link's end mark. Three ends spread
/// along a card's 44 px side sit 11 px apart, and a 16 px plate reaches the arrowhead
/// beside it.
#[test]
fn a_stop_plate_over_another_links_arrowhead_is_a_links_routed_defect() {
    let page = page_from(linked_page(
        json!([{ "tag": "Row", "gap": 64, "children": [
            { "tag": "Col", "gap": 24, "children": [card("a"), card("b"), card("d")] },
            { "tag": "Col", "justify": "center", "children": [card("c")] }
        ] }]),
        json!([
            { "from": "a", "to": "c", "line": "deny", "from_side": "right", "to_side": "left" },
            { "from": "b", "to": "c", "line": "solid", "tint": 1, "from_side": "right", "to_side": "left" },
            { "from": "d", "to": "c", "line": "solid", "tint": 1, "from_side": "right", "to_side": "left" }
        ]),
    ));
    let geometry = layout(&page);
    let (deny, solid) = (route_of(&geometry, 0), route_of(&geometry, 1));
    let (deny_end, solid_end) = (deny.points.last().unwrap(), solid.points.last().unwrap());
    assert_close(deny_end.x, solid_end.x, "both end on c's left side");
    assert_close(
        solid_end.y - deny_end.y,
        11.0,
        "a quarter of the side apart",
    );
    let report = links_routed(&page, &geometry);
    assert_eq!(report.examined, 3);
    let messages: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.message.as_str())
        .collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(report.defects[0].pointer.as_str(), "/links/0");
    assert!(
        messages[0].starts_with("stop plate 3.00x16.00")
            && messages[0].contains("overlaps the arrowhead 10.00x8.00")
            && messages[0].ends_with("of /links/1"),
        "{messages:?}"
    );
}

#[test]
fn a_stop_plate_under_a_tag_is_a_links_routed_defect_and_a_clear_one_passes() {
    let page_with = |label: &str| {
        page_from(linked_page(
            json!([{ "tag": "Row", "gap": 64, "children": [card("a"), card("b")] }]),
            json!([{ "from": "a", "to": "b", "line": "deny", "label": label }]),
        ))
    };
    let tight = page_with("no direct write to the store");
    let report = links_routed(&tight, &layout(&tight));
    assert!(
        report
            .defects
            .iter()
            .any(|defect| defect.message.starts_with("stop plate")
                && defect.message.contains("overlaps the tag")
                && defect.message.ends_with("of /links/0")),
        "{report:?}"
    );
    let open = page_with("no");
    let report = links_routed(&open, &layout(&open));
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.examined, 1);
}

/// Three rows: a card, two cards a gutter apart, a card. A link from the top card to the
/// bottom one passes the middle row through its gutter.
fn rows_with_gutter(row_gap: u32, gutter: u32, grow: Value) -> Value {
    linked_page(
        json!([{ "tag": "Col", "gap": row_gap, "children": [
            { "tag": "Row", "children": [card("p")] },
            { "tag": "Row", "gap": gutter, "grow": grow, "children": [card("q"), card("r")] },
            { "tag": "Row", "children": [card("s")] }
        ]}]),
        json!([{
            "from": "p", "to": "s", "line": "solid", "tint": 1,
            "from_side": "bottom", "to_side": "top"
        }]),
    )
}

/// The distance between a leg and a parallel card edge beside it, for each pair that runs
/// alongside for more than the epsilon.
fn parallel_gaps(points: &[PagePoint], cards: &[BoxRect]) -> Vec<f32> {
    let mut gaps = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        for card in cards {
            if a.y == b.y {
                let overlap = a.x.max(b.x).min(card.right()) - a.x.min(b.x).max(card.x);
                if overlap > 0.01 {
                    gaps.extend([(a.y - card.y).abs(), (a.y - card.bottom()).abs()]);
                }
            } else {
                let overlap = a.y.max(b.y).min(card.bottom()) - a.y.min(b.y).max(card.y);
                if overlap > 0.01 {
                    gaps.extend([(a.x - card.x).abs(), (a.x - card.right()).abs()]);
                }
            }
        }
    }
    gaps
}

/// The router lays a leg on the grid line 8 px outside a card; on a flat page that leg
/// moves until it keeps OUTLINE_AIR_PX from every card it runs beside.
#[test]
fn a_leg_beside_a_card_moves_off_its_outline() {
    let page = page_from(rows_with_gutter(48, 40, json!([1, 3])));
    let geometry = layout(&page);
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    let cards: Vec<BoxRect> = [
        "/body/0/children/0/children/0",
        "/body/0/children/1/children/0",
        "/body/0/children/1/children/1",
        "/body/0/children/2/children/0",
    ]
    .iter()
    .map(|pointer| node(&geometry, pointer).bounds)
    .collect();
    let gaps = parallel_gaps(&route.points, &cards);
    assert!(!gaps.is_empty(), "{:?}", route.points);
    for gap in gaps {
        assert!(gap >= OUTLINE_AIR_PX, "{gap} px in {:?}", route.points);
    }
    let report = links_routed(&page, &geometry);
    assert!(report.passed(), "{report:?}");
}

/// A link straight through a 10 px gutter runs 5 px from each card and no shift frees it:
/// links-routed names the card whose outline it crowds.
#[test]
fn a_link_through_a_gutter_narrower_than_twice_the_air_is_a_defect() {
    let page = page_from(rows_with_gutter(24, 10, json!([1, 1])));
    let geometry = layout(&page);
    let report = links_routed(&page, &geometry);
    assert_eq!(report.examined, 1);
    let messages: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.message.as_str())
        .collect();
    assert!(
        messages.iter().any(|message| message.starts_with(
            "leg 0 runs within 12.00 px along the outline of /body/0/children/1/children/0"
        )),
        "{messages:?}"
    );
}

/// A plain page at width 640: a Box labeled `label` holding a Row of `inside`, then
/// `beside` to its right, every Row child at its content width, with the given links.
fn labeled_box_page_with(
    label: &str,
    inside: Vec<Value>,
    beside: Vec<Value>,
    links: Value,
) -> Value {
    let inner_grow = vec![0; inside.len()];
    let zone = json!({ "tag": "Box", "kind": "boundary", "label": label,
        "children": [{ "tag": "Row", "gap": 64, "grow": inner_grow, "children": inside }] });
    let mut row: Vec<Value> = vec![zone];
    row.extend(beside);
    let outer_grow = vec![0; row.len()];
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": 640,
        "body": [{ "tag": "Row", "grow": outer_grow, "children": row }],
        "legend": [{ "line": "deny", "text": "blocked path" }],
        "links": links
    })
}

fn labeled_box_page(inside: Vec<Value>, beside: Vec<Value>, links: Value) -> Value {
    labeled_box_page_with("SHARED VPC · US-CENTRAL1", inside, beside, links)
}

fn service(id: &str) -> Value {
    json!({ "tag": "Item", "kind": "service", "id": id, "title": id })
}

/// Two items in a labeled Box linked top to top with no arrow: the leg runs along the
/// label's bottom edge and the tag, centered on it, lies over the label. The Box is an
/// ancestor of both ends, and its label is still text the tag must keep off.
#[test]
fn a_tag_over_the_label_of_the_box_that_holds_both_ends_is_a_defect() {
    let geometry = layout(&page_from(labeled_box_page(
        vec![service("api"), service("worker")],
        vec![],
        json!([{ "from": "api", "to": "worker", "line": "deny", "arrow": "none",
                 "label": "no person", "from_side": "top", "to_side": "top" }]),
    )));
    let label = node(&geometry, "/body/0/children/0")
        .part(PartName::Label)
        .expect("a labeled Box has a label part")
        .bounds;
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    for pair in route.points.windows(2) {
        assert!(
            !segment_enters(pair[0], pair[1], &label),
            "segment {pair:?} enters the label {label:?}"
        );
    }
    let tag = route.tag.expect("a labeled link has a tag");
    assert!(
        tag.x < label.right()
            && tag.right() > label.x
            && tag.y < label.bottom()
            && tag.bottom() > label.y,
        "the tag {tag:?} lies over the label {label:?}"
    );
    let report = links_avoid_boxes(&geometry);
    let messages: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("overlaps label of /body/0/children/0")),
        "{messages:?}"
    );
    assert!(
        report
            .defects
            .iter()
            .all(|defect| defect.pointer.to_string() == "/links/0"),
        "{report:?}"
    );
}

/// The same two items linked top to top with an arrowed end: the arrow stub out of the
/// item's top reaches into the Box's label, so the link falls back and links-routed says so.
#[test]
fn an_arrow_stub_into_the_label_of_the_box_that_holds_both_ends_falls_back() {
    let page = page_from(labeled_box_page(
        vec![service("api"), service("worker")],
        vec![],
        json!([{ "from": "api", "to": "worker", "line": "deny", "label": "no person",
                 "from_side": "top", "to_side": "top" }]),
    ));
    let geometry = layout(&page);
    assert_eq!(route_of(&geometry, 0).status, RouteStatus::Fallback);
    let report = links_routed(&page, &geometry);
    assert!(!report.passed(), "{report:?}");
    assert_eq!(report.defects[0].pointer.to_string(), "/links/0");
}

/// The same two items linked right to left run straight between them, under the label: the
/// label pair is examined for the segment and for the tag, and neither is a defect.
#[test]
fn a_link_under_the_label_of_the_box_that_holds_both_ends_passes_with_the_label_examined() {
    let geometry = layout(&page_from(labeled_box_page(
        vec![service("api"), service("worker")],
        vec![],
        json!([{ "from": "api", "to": "worker", "line": "deny", "label": "deny" }]),
    )));
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    assert_eq!(route.points.len(), 2, "adjacent items link straight across");
    let report = links_avoid_boxes(&geometry);
    assert!(report.passed(), "{report:?}");
    // One segment against the kicker, title, lede, legend entry and the Box label; the tag
    // against the same four text nodes, the two items and the Box label.
    assert_eq!(report.examined, 5 + 7);
}

/// The label obstacle is the text the label draws, not its part, which spans the Box: two
/// items standing right of a short label, linked top to top, route through the label band
/// beside the text.
#[test]
fn a_link_passes_through_the_label_band_beside_the_label_text() {
    let page = page_from(labeled_box_page_with(
        "VPC",
        vec![
            json!({ "tag": "Fact", "text": "a fact wide enough to push both items past the label" }),
            service("api"),
            service("worker"),
        ],
        vec![],
        json!([{ "from": "api", "to": "worker", "line": "deny", "label": "deny",
                 "from_side": "top", "to_side": "top" }]),
    ));
    let geometry = layout(&page);
    let label = node(&geometry, "/body/0/children/0")
        .part(PartName::Label)
        .expect("a labeled Box has a label part");
    let ink = label.ink_box();
    assert!(
        ink.width < label.bounds.width / 2.0,
        "{ink:?} inside {:?}",
        label.bounds
    );
    let route = route_of(&geometry, 0);
    assert_eq!(route.status, RouteStatus::Routed);
    let across = route
        .points
        .windows(2)
        .find(|pair| pair[0].y == pair[1].y)
        .expect("a top-to-top link has a leg across");
    assert!(
        across[0].y > label.bounds.y && across[0].y < label.bounds.bottom(),
        "the leg {across:?} lies in the label band {:?}",
        label.bounds
    );
    assert!(
        across[0].x.min(across[1].x) > ink.right(),
        "the leg {across:?} lies right of the label text {ink:?}"
    );
    let report = links_avoid_boxes(&geometry);
    assert!(report.passed(), "{report:?}");
    let report = links_routed(&page, &geometry);
    assert!(report.passed(), "{report:?}");
}

/// A customer page: `front` in a Row with a Col of `alpha` over `beta`, at gap 64, with the
/// given links.
fn fan_out_page(links: Value) -> Value {
    linked_page(
        json!([{ "tag": "Row", "gap": 64, "children": [
            card("front"),
            { "tag": "Col", "gap": 64, "children": [card("alpha"), card("beta")] }
        ] }]),
        links,
    )
}

/// Two links that leave one side of a node spread along it, at a third and two thirds, so
/// their first legs run apart instead of as one wire that forks; one link alone leaves from
/// the side's middle.
#[test]
fn two_links_leaving_one_side_spread_along_it_and_one_alone_leaves_its_middle() {
    let page = page_from(fan_out_page(json!([
        { "from": "front", "to": "alpha", "line": "solid", "tint": 1, "label": "1",
          "from_side": "right", "to_side": "left" },
        { "from": "front", "to": "beta", "line": "solid", "tint": 1, "label": "2",
          "from_side": "right", "to_side": "left" }
    ])));
    let geometry = layout(&page);
    let front = node(&geometry, "/body/0/children/0").bounds;
    let (first, second) = (route_of(&geometry, 0), route_of(&geometry, 1));
    assert_close(
        first.points[0].y,
        front.y + front.height / 3.0,
        "first at a third",
    );
    assert_close(
        second.points[0].y,
        front.y + front.height * 2.0 / 3.0,
        "second at two thirds",
    );
    for (mine, theirs) in first.points.windows(2).zip(second.points.windows(2)) {
        assert!(
            !legs_shared((mine[0], mine[1]), (theirs[0], theirs[1])),
            "{mine:?} shares a line with {theirs:?}"
        );
    }
    let report = links_routed(&page, &geometry);
    assert!(report.passed(), "{report:?}");

    let alone = layout(&page_from(fan_out_page(json!([
        { "from": "front", "to": "alpha", "line": "solid", "tint": 1,
          "from_side": "right", "to_side": "left" }
    ]))));
    assert_close(
        route_of(&alone, 0).points[0].y,
        front.y + front.height / 2.0,
        "alone at the middle",
    );
}

/// Two links whose drawn legs run along one line are a links-routed defect on a flat page,
/// at the later link, naming the earlier one and the shared stretch.
#[test]
fn a_leg_along_another_links_leg_is_a_links_routed_defect_on_a_flat_page() {
    let page = page_from(fan_out_page(json!([
        { "from": "front", "to": "alpha", "line": "solid", "tint": 1,
          "from_side": "right", "to_side": "left" },
        { "from": "front", "to": "beta", "line": "solid", "tint": 1,
          "from_side": "right", "to_side": "left" }
    ])));
    let mut geometry = layout(&page);
    assert!(links_routed(&page, &geometry).passed());
    // The second link's first leg drawn over the first's, as a fork no move freed would be.
    let shared = route_of(&geometry, 0).points.clone();
    let second = geometry
        .links
        .iter_mut()
        .find(|route| route.index == 1)
        .unwrap();
    let end = *second.points.last().unwrap();
    second.points = vec![
        shared[0],
        shared[1],
        PagePoint {
            x: shared[1].x,
            y: end.y,
        },
        end,
    ];
    let report = links_routed(&page, &geometry);
    assert_eq!(report.examined, 2);
    let messages: Vec<&str> = report
        .defects
        .iter()
        .map(|defect| defect.message.as_str())
        .collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(report.defects[0].pointer.as_str(), "/links/1");
    assert_eq!(
        messages[0],
        format!(
            "leg 0 runs along a leg of /links/0 from ({:.2}, {:.2}) to ({:.2}, {:.2})",
            shared[0].x, shared[0].y, shared[1].x, shared[1].y
        )
    );
}
