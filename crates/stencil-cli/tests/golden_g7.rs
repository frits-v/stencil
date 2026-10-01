#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! SPEC section 9.4: render examples/g7.json through the CLI pipeline, assert exact check
//! counts and the gold's structure, and compose a side-by-side image for a person to review.
//! The reference was rendered by Chrome in Helvetica Neue, so no pixel metric is asserted.

use std::fs;
use std::path::{Path, PathBuf};

use resvg::tiny_skia::{Color, Pixmap, PixmapPaint, Transform};
use stencil_cli::pipeline::{LoadedDocument, RenderedPage, all_checks, load_document, render_page};
use stencil_layout::{BoxRect, NodeGeometry, PageGeometry, PartName};
use stencil_model::checks::{CheckName, CheckOutcome};
use stencil_render::DeviceScale;

const EPSILON_PX: f32 = 0.01;
const REFERENCE_WIDTH_PX: u32 = 2640;
const REFERENCE_HEIGHT_PX: u32 = 1800;

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn node<'a>(geometry: &'a PageGeometry, pointer: &str) -> &'a NodeGeometry {
    geometry
        .nodes
        .iter()
        .find(|candidate| candidate.pointer.as_str() == pointer)
        .unwrap_or_else(|| panic!("no geometry node at {pointer}"))
}

fn bounds(geometry: &PageGeometry, pointer: &str) -> BoxRect {
    node(geometry, pointer).bounds
}

fn center_x(rect: &BoxRect) -> f32 {
    rect.x + rect.width / 2.0
}

fn center_y(rect: &BoxRect) -> f32 {
    rect.y + rect.height / 2.0
}

fn assert_close(actual: f32, expected: f32, tolerance: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} is not within {tolerance} of {expected}"
    );
}

fn assert_inside(inner: &BoxRect, outer: &BoxRect, what: &str) {
    assert!(
        inner.x >= outer.x - EPSILON_PX
            && inner.y >= outer.y - EPSILON_PX
            && inner.right() <= outer.right() + EPSILON_PX
            && inner.bottom() <= outer.bottom() + EPSILON_PX,
        "{what}: {inner:?} is not inside {outer:?}"
    );
}

const ON_PREM_COL: &str = "/body/0/children/0";
const METRO_1: &str = "/body/0/children/0/children/0";
const METRO_2: &str = "/body/0/children/0/children/1";
const VLAN_COL: &str = "/body/0/children/1";
const UPPER_HALF: &str = "/body/0/children/1/children/0";
const LOWER_HALF: &str = "/body/0/children/1/children/1";
const VLAN_PIPES: [&str; 4] = [
    "/body/0/children/1/children/0/children/0",
    "/body/0/children/1/children/0/children/1",
    "/body/0/children/1/children/1/children/0",
    "/body/0/children/1/children/1/children/1",
];
const GCP_ZONE: &str = "/body/0/children/2";
const VPC_ZONE: &str = "/body/0/children/2/children/0";
const REGION_A: &str = "/body/0/children/2/children/0/children/0";
const FAILOVER_PIPE: &str = "/body/0/children/2/children/0/children/1";
const REGION_B: &str = "/body/0/children/2/children/0/children/2";

fn render_g7() -> (LoadedDocument, RenderedPage) {
    let json_text = fs::read_to_string(repository_path("examples/g7.json")).unwrap();
    let loaded = load_document(&json_text).unwrap();
    let rendered = render_page(&loaded, DeviceScale::DEFAULT).unwrap();
    (loaded, rendered)
}

#[test]
fn golden_g7() {
    let (loaded, rendered) = render_g7();
    let geometry = &rendered.geometry;

    // Step 1: all three outputs exist, and the PNG matches the reference width and the
    // rounded canvas height.
    assert!(rendered.svg.svg.starts_with("<svg "));
    assert!(rendered.svg.text_elements > 0);
    assert!(rendered.measured.is_object());
    let rendered_png = Pixmap::decode_png(&rendered.png).unwrap();
    let reference_png =
        Pixmap::load_png(repository_path("examples/golden/g7-gold-chrome.png")).unwrap();
    assert_eq!(reference_png.width(), REFERENCE_WIDTH_PX);
    assert_eq!(reference_png.height(), REFERENCE_HEIGHT_PX);
    assert_eq!(rendered_png.width(), reference_png.width());
    let measured_height = rendered.measured["canvas"]["height"].as_f64().unwrap();
    assert_eq!(
        f64::from(rendered_png.height()),
        (measured_height * 2.0).ceil()
    );

    // Step 2: exact examined counts, derived in SPEC section 9.4.
    let reports = all_checks(
        &loaded.page,
        &loaded.grammar,
        geometry,
        rendered.scene.as_ref(),
    );
    let examined: Vec<(CheckName, u64)> = reports
        .iter()
        .map(|report| (report.check, report.examined))
        .collect();
    assert_eq!(
        examined,
        vec![
            (CheckName::ChildInsideContainer, 33),
            (CheckName::SiblingsDoNotOverlap, 32),
            (CheckName::TextFitsBox, 40),
            (CheckName::RememberedConstants, 36),
            (CheckName::LegendConsistency, 8),
            (CheckName::LinksRouted, 0),
            (CheckName::LinksAvoidBoxes, 0),
            (CheckName::PipesLand, 8),
            (CheckName::IsoLabelsClear, 0),
            (CheckName::IsoLinksClear, 0),
        ]
    );
    for report in &reports {
        let expected = match report.check {
            CheckName::LinksRouted
            | CheckName::LinksAvoidBoxes
            | CheckName::IsoLabelsClear
            | CheckName::IsoLinksClear => CheckOutcome::NotApplicable,
            CheckName::ChildInsideContainer
            | CheckName::SiblingsDoNotOverlap
            | CheckName::TextFitsBox
            | CheckName::RememberedConstants
            | CheckName::LegendConsistency
            | CheckName::PipesLand => CheckOutcome::Passed,
        };
        assert_eq!(report.outcome(), expected, "{report:?}");
    }

    // Step 3: the gold's structure.
    assert_eq!(geometry.canvas.width, 1320.0);
    assert!(
        (560.0..=720.0).contains(&geometry.canvas.height),
        "canvas height {}",
        geometry.canvas.height
    );

    let on_prem = bounds(geometry, ON_PREM_COL);
    assert_close(on_prem.x, 20.0, EPSILON_PX, "on-prem Col x");
    assert!(
        (185.0..=230.0).contains(&on_prem.width),
        "on-prem Col width {}",
        on_prem.width
    );

    let vlan = bounds(geometry, VLAN_COL);
    assert_close(vlan.x, on_prem.right() + 8.0, EPSILON_PX, "VLAN Col x");
    assert!(
        (120.0..=170.0).contains(&vlan.width),
        "VLAN Col width {}",
        vlan.width
    );

    let gcp = bounds(geometry, GCP_ZONE);
    assert_close(gcp.x, vlan.right() + 8.0, EPSILON_PX, "gcp Zone x");
    assert_close(gcp.right(), 1300.0, EPSILON_PX, "gcp Zone right");

    let upper = bounds(geometry, UPPER_HALF);
    let lower = bounds(geometry, LOWER_HALF);
    assert!(upper.bottom() <= lower.y + EPSILON_PX);
    let mut previous_center_y = f32::NEG_INFINITY;
    for (index, pointer) in VLAN_PIPES.iter().enumerate() {
        let pipe = node(geometry, pointer);
        let half = if index < 2 { &upper } else { &lower };
        assert_inside(&pipe.bounds, half, pointer);
        assert!(center_y(&pipe.bounds) > previous_center_y, "{pointer}");
        previous_center_y = center_y(&pipe.bounds);
        assert_close(pipe.bounds.x, half.x, EPSILON_PX, "VLAN pipe x");
        assert_close(pipe.bounds.width, half.width, EPSILON_PX, "VLAN pipe width");
        let wire_start = pipe.part(PartName::WireStart).unwrap().bounds.width;
        let wire_end = pipe.part(PartName::WireEnd).unwrap().bounds.width;
        assert_close(wire_start, wire_end, EPSILON_PX, "VLAN pipe wires");
    }

    let region_a = bounds(geometry, REGION_A);
    let region_b = bounds(geometry, REGION_B);
    let failover = bounds(geometry, FAILOVER_PIPE);
    let vpc_content = node(geometry, VPC_ZONE).content;
    assert!(failover.y >= region_a.bottom() - EPSILON_PX);
    assert!(failover.bottom() <= region_b.y + EPSILON_PX);
    assert_close(
        center_x(&failover),
        center_x(&vpc_content),
        0.5,
        "failover center x",
    );
    assert_close(region_a.x, region_b.x, EPSILON_PX, "region x");
    assert_close(region_a.width, region_b.width, EPSILON_PX, "region width");
    assert_close(
        region_a.width,
        vpc_content.width,
        EPSILON_PX,
        "region width against the VPC content box",
    );

    // The on-prem Col and the VLAN Col both split the Row height with `grow [1, 1]` and gap
    // 8, so metro zone i and gutter half i share their top and height, and each VLAN pipe's
    // center y lies inside the metro zone its half sits beside.
    for (zone_pointer, half) in [(METRO_1, &upper), (METRO_2, &lower)] {
        let zone = bounds(geometry, zone_pointer);
        assert_close(zone.y, half.y, EPSILON_PX, zone_pointer);
        assert_close(zone.height, half.height, EPSILON_PX, zone_pointer);
    }
    for (index, pointer) in VLAN_PIPES.iter().enumerate() {
        let zone = bounds(geometry, if index < 2 { METRO_1 } else { METRO_2 });
        let center = center_y(&bounds(geometry, pointer));
        assert!(
            center >= zone.y && center <= zone.bottom(),
            "{pointer} center y {center} is outside its metro zone {zone:?}"
        );
    }

    // Step 4: the side-by-side for review at 200 percent.
    let side_by_side = compose_side_by_side(&reference_png, &rendered_png);
    let target_directory = Path::new(env!("CARGO_TARGET_TMPDIR")).parent().unwrap();
    let golden_directory = target_directory.join("golden");
    fs::create_dir_all(&golden_directory).unwrap();
    let side_by_side_path = golden_directory.join("g7-side-by-side.png");
    side_by_side.save_png(&side_by_side_path).unwrap();
    assert!(side_by_side_path.is_file());
}

fn compose_side_by_side(reference: &Pixmap, rendered: &Pixmap) -> Pixmap {
    let width = reference.width() + rendered.width();
    let height = reference.height().max(rendered.height());
    let mut canvas = Pixmap::new(width, height).unwrap();
    canvas.fill(Color::WHITE);
    canvas.draw_pixmap(
        0,
        0,
        reference.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    canvas.draw_pixmap(
        i32::try_from(reference.width()).unwrap(),
        0,
        rendered.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    canvas
}
