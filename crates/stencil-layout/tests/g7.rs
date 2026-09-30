#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{assert_close, layout, node, part};
use stencil_layout::PartName;
use stencil_layout::checks::pipes_land;

const ON_PREM_COL: &str = "/body/0/children/0";
const METRO_1: &str = "/body/0/children/0/children/0";
const METRO_2: &str = "/body/0/children/0/children/1";
const VLAN_COL: &str = "/body/0/children/1";
const UPPER_HALF: &str = "/body/0/children/1/children/0";
const LOWER_HALF: &str = "/body/0/children/1/children/1";
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

/// gcp zone: 6 border + 35.6 bar + 30 body padding + 388.2 VPC. VPC: 4 + 20 + 14.4 + 8 +
/// 127.6 region A + 8 + 70.6 failover pipe + 8 + 127.6 region B. Region: 3 + 24 + 22.4 +
/// 46 card + 8 + 24.2 fact. None of these strings wrap under the fixed-metrics measurer. The
/// gcp zone is the Row's tallest child, and the on-prem Col and the VLAN Col both split that
/// height with `grow [1, 1]` and gap 8, so each metro zone and each gutter half is
/// (459.8 - 8) / 2 = 225.9 tall.
#[test]
fn g7_heights_match_the_section_9_4_derivation() {
    let geometry = layout(&common::g7_page());
    assert_close(node(&geometry, GCP_ZONE).bounds.height, 459.8, "gcp zone");
    assert_close(node(&geometry, VPC_ZONE).bounds.height, 388.2, "VPC zone");
    for (zone, half) in [(METRO_1, UPPER_HALF), (METRO_2, LOWER_HALF)] {
        let zone_bounds = node(&geometry, zone).bounds;
        let half_bounds = node(&geometry, half).bounds;
        assert_close(zone_bounds.height, (459.8 - 8.0) / 2.0, zone);
        assert_close(
            zone_bounds.y,
            half_bounds.y,
            "metro zone and gutter half top",
        );
        assert_close(
            zone_bounds.height,
            half_bounds.height,
            "metro zone and gutter half height",
        );
    }
}

#[test]
fn g7_each_vlan_pipe_points_into_its_metro_zone() {
    let geometry = layout(&common::g7_page());
    for (pipe_pointer, zone_pointer) in [
        ("/body/0/children/1/children/0/children/0", METRO_1),
        ("/body/0/children/1/children/0/children/1", METRO_1),
        ("/body/0/children/1/children/1/children/0", METRO_2),
        ("/body/0/children/1/children/1/children/1", METRO_2),
    ] {
        let pipe = node(&geometry, pipe_pointer).bounds;
        let zone = node(&geometry, zone_pointer).bounds;
        let center = pipe.y + pipe.height / 2.0;
        assert!(
            center >= zone.y && center <= zone.bottom(),
            "{pipe_pointer} center y {center} is outside {zone_pointer} ({} to {})",
            zone.y,
            zone.bottom()
        );
    }
    let report = pipes_land(&common::g7_page(), &geometry);
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.examined, 8);
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
