#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use stencil_layout::checks::{
    child_inside_container, links_avoid_boxes, links_routed, siblings_do_not_overlap, text_fits_box,
};
use stencil_layout::{NodeTag, RouteStatus, layout_page};
use stencil_model::parse_page;
use stencil_model::text::FixedMetricsMeasurer;

const ONEPAGER_JSON: &str = include_str!("../../../examples/onepager.json");

#[test]
fn onepager_lays_out_with_every_link_routed_and_no_geometry_defect() {
    let page = parse_page(ONEPAGER_JSON).expect("onepager vets");
    let geometry = layout_page(
        &page,
        &stencil_model::builtin_grammar("gcp").unwrap().unwrap(),
        &mut FixedMetricsMeasurer::default(),
    )
    .expect("onepager lays out");

    for tag in [NodeTag::Text, NodeTag::Callout, NodeTag::Frame] {
        assert!(
            geometry.nodes.iter().any(|node| node.tag == tag),
            "{tag:?} missing"
        );
    }
    assert_eq!(geometry.links.len(), page.links.len());
    for route in &geometry.links {
        assert_eq!(route.status, RouteStatus::Routed, "link {}", route.index);
    }
    for report in [
        child_inside_container(&geometry),
        siblings_do_not_overlap(&geometry),
        text_fits_box(&geometry),
        links_routed(&geometry),
        links_avoid_boxes(&geometry),
    ] {
        assert!(report.passed(), "{report:?}");
    }
}
