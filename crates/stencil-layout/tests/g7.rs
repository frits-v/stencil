#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, part};
use stencil_layout::PartName;

const ON_PREM_COL: &str = "/body/0/children/0";
const VLAN_COL: &str = "/body/0/children/1";
const GCP_ZONE: &str = "/body/0/children/2";
const VPC_ZONE: &str = "/body/0/children/2/children/0";

#[test]
fn g7_columns_sit_side_by_side_across_the_page() {
    let geometry = layout(&common::g7_page());
    let on_prem = node(&geometry, ON_PREM_COL);
    let vlan = node(&geometry, VLAN_COL);
    let gcp = node(&geometry, GCP_ZONE);
    assert_eq!(on_prem.bounds.x, 20.0);
    assert_close(vlan.bounds.x, on_prem.bounds.right() + 8.0, "VLAN Col x");
    assert_close(gcp.bounds.x, vlan.bounds.right() + 8.0, "gcp zone x");
    assert_close(gcp.bounds.right(), 1300.0, "gcp zone right edge");
}

/// On-prem zone: 3 border + 24 padding + 14.4 + 8 label band + two 46 px cards + 8 gap.
/// gcp zone: 6 border + 35.6 bar + 30 body padding + 388.2 VPC. VPC: 4 + 20 + 14.4 + 8 +
/// 127.6 region A + 8 + 70.6 failover pipe + 8 + 127.6 region B. Region: 3 + 24 + 22.4 +
/// 46 card + 8 + 24.2 fact. None of these strings wrap under the fixed-metrics measurer.
#[test]
fn g7_heights_match_the_section_9_4_derivation() {
    let geometry = layout(&common::g7_page());
    for zone in [
        "/body/0/children/0/children/0",
        "/body/0/children/0/children/1",
    ] {
        assert_close(
            node(&geometry, zone).bounds.height,
            3.0 + 24.0 + 14.4 + 8.0 + 46.0 + 8.0 + 46.0,
            zone,
        );
    }
    assert_close(node(&geometry, GCP_ZONE).bounds.height, 459.8, "gcp zone");
    assert_close(node(&geometry, VPC_ZONE).bounds.height, 388.2, "VPC zone");
    let metro_2 = node(&geometry, "/body/0/children/0/children/1");
    assert!(node(&geometry, GCP_ZONE).bounds.bottom() - metro_2.bounds.bottom() >= 150.0);
}

#[test]
fn g7_vlan_pipes_fill_their_half_cols_in_order() {
    let geometry = layout(&common::g7_page());
    let upper = node(&geometry, "/body/0/children/1/children/0");
    let lower = node(&geometry, "/body/0/children/1/children/1");
    assert!(upper.bounds.bottom() <= lower.bounds.y + 0.01);
    let mut previous_center = f32::MIN;
    for (half, pipe_index) in [(upper, 0), (upper, 1), (lower, 0), (lower, 1)] {
        let pipe_node = node(
            &geometry,
            &format!("{}/children/{pipe_index}", half.pointer.as_str()),
        );
        let center = pipe_node.bounds.y + pipe_node.bounds.height / 2.0;
        assert!(center > previous_center);
        previous_center = center;
        assert!(
            pipe_node.bounds.y >= half.bounds.y
                && pipe_node.bounds.bottom() <= half.bounds.bottom()
        );
        assert_close(
            pipe_node.bounds.width,
            half.bounds.width,
            "pipe spans its half",
        );
        let start = part(pipe_node, PartName::WireStart).bounds.width;
        let end = part(pipe_node, PartName::WireEnd).bounds.width;
        assert!((start - end).abs() <= 0.01);
    }
}

#[test]
fn g7_failover_pipe_sits_between_the_regions() {
    let geometry = layout(&common::g7_page());
    let vpc = node(&geometry, VPC_ZONE);
    let region_a = node(&geometry, "/body/0/children/2/children/0/children/0");
    let failover = node(&geometry, "/body/0/children/2/children/0/children/1");
    let region_b = node(&geometry, "/body/0/children/2/children/0/children/2");
    assert!(failover.bounds.y >= region_a.bounds.bottom());
    assert!(failover.bounds.bottom() <= region_b.bounds.y);
    let failover_center = failover.bounds.x + failover.bounds.width / 2.0;
    assert!((failover_center - (vpc.content.x + vpc.content.width / 2.0)).abs() <= 0.5);
    for region in [region_a, region_b] {
        assert_close(region.bounds.x, vpc.content.x, "region x");
        assert_close(region.bounds.width, vpc.content.width, "region width");
    }
}
