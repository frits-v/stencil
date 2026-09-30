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
use stencil_layout::{BoxRect, LinkRoute, PageGeometry, PartName, RouteStatus};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_model::{LINK_SEGMENTS_MAX, PagePoint};

/// A customer page at width 640 with a blue legend entry so links of kind blue are listed.
fn linked_page(body: Value, links: Value) -> Value {
    json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "width": 640,
        "canvas": "customer",
        "body": body,
        "legend": [{ "kind": "blue", "text": "request path" }],
        "links": links
    })
}

fn card(id: &str) -> Value {
    json!({ "tag": "Pcard", "id": id, "fn": id })
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
    let geometry = layout(&page_from(row_with_obstacle(
        json!([{ "from": "a", "to": "c", "kind": "blue" }]),
    )));
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
    assert!(links_routed(&geometry).passed());
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
        json!([{ "from": "a", "to": "c", "kind": "blue", "via": [{ "x": via.x, "y": via.y }] }]),
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
    let geometry = layout(&page_from(linked_page(
        body,
        json!([{ "from": "center", "to": "target", "kind": "blue" }]),
    )));
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

    let report = links_routed(&geometry);
    assert_eq!(report.examined, 1);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
    assert_eq!(report.defects.len(), 1);
    assert_eq!(report.defects[0].pointer.as_str(), "/links/0");
}

#[test]
fn link_checks_do_not_apply_to_a_page_without_links() {
    let geometry = layout(&page_from(row_with_obstacle(json!([]))));
    assert!(geometry.links.is_empty());
    for report in [links_routed(&geometry), links_avoid_boxes(&geometry)] {
        assert_eq!(report.examined, 0);
        assert!(report.defects.is_empty());
        assert_eq!(report.outcome(), CheckOutcome::NotApplicable, "{report:?}");
        assert!(!report.passed());
    }
    assert_eq!(links_routed(&geometry).check, CheckName::LinksRouted);
    assert_eq!(
        links_avoid_boxes(&geometry).check,
        CheckName::LinksAvoidBoxes
    );
}

#[test]
fn link_checks_count_links_and_segment_obstacle_pairs() {
    let geometry = layout(&page_from(row_with_obstacle(json!([
        { "from": "a", "to": "b", "kind": "blue" },
        { "from": "a", "to": "c", "kind": "blue" }
    ]))));
    let routed = links_routed(&geometry);
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

#[test]
fn a_tag_is_sized_like_a_pipe_tag_and_centered_on_the_longest_segment() {
    let geometry = layout(&page_from(row_with_obstacle(json!([
        { "from": "a", "to": "b", "kind": "blue", "label": "step 1", "sub": "HTTPS" }
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
        { "from": "a", "to": "b", "kind": "blue", "from_side": "top", "to_side": "top" }
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
        { "from": "a", "to": "c", "kind": "blue", "label": "one" },
        { "from": "c", "to": "a", "kind": "blue" }
    ])));
    assert_eq!(layout(&page).links, layout(&page).links);
}
