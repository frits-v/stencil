//! The flat writer's assembly surfaces on the onepager (section 14.5 rule 9): the hollow
//! planned line with every bend drawn, and the strip's rail and planned entries.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use serde_json::Value;

fn onepager_document() -> Value {
    serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap()
}

/// The index of the link labelled `label`.
fn link_index(document: &Value, label: &str) -> usize {
    document["links"]
        .as_array()
        .unwrap()
        .iter()
        .position(|link| link["label"] == label)
        .unwrap()
}

/// A planned line draws each run as a casing in its wire and a paper core on the same
/// path, dashed apart on a run and solid on a joint; no other line draws a paper stroke.
#[test]
fn a_planned_line_is_hollow_and_no_other_line_is() {
    let document = onepager_document();
    let rendered = common::render_document_with_fixed_metrics(document.clone());
    let svg = common::parse_xml(&rendered.svg.svg);
    let paper = common::theme("center")
        .sheet_colors()
        .paper
        .as_str()
        .to_string();
    let planned = link_index(&document, "schedule replays");
    let group = common::group(&svg, &format!("/links/{planned}"));
    let paths = common::children_named(group, "path");
    assert!(
        paths.len() >= 2 && paths.len().is_multiple_of(2),
        "{} paths",
        paths.len()
    );
    let (mut joints, mut runs) = (0, 0);
    for pair in paths.chunks(2) {
        let (casing, core) = (pair[0], pair[1]);
        assert_eq!(casing.attribute("d"), core.attribute("d"));
        assert_ne!(casing.attribute("stroke"), Some(paper.as_str()));
        assert_eq!(core.attribute("stroke"), Some(paper.as_str()));
        let casing_width: f32 = casing.attribute("stroke-width").unwrap().parse().unwrap();
        let core_width: f32 = core.attribute("stroke-width").unwrap().parse().unwrap();
        assert_eq!(casing_width - core_width, 2.0);
        match (
            casing.attribute("stroke-dasharray"),
            core.attribute("stroke-dasharray"),
        ) {
            (None, None) => joints += 1,
            (Some("7 4"), Some("5 6")) => runs += 1,
            other => panic!("unexpected dash pair {other:?}"),
        }
    }
    // The onepager's planned line runs straight down, so it has runs and no joint; the
    // joints of a bent line are pinned in crates/stencil-layout/tests/assembly.rs.
    assert_eq!((joints, runs), (0, 1));

    let mut examined = 0;
    for (index, _) in document["links"].as_array().unwrap().iter().enumerate() {
        if index == planned {
            continue;
        }
        let group = common::group(&svg, &format!("/links/{index}"));
        for path in common::children_named(group, "path") {
            if path.attribute("fill") == Some("none") {
                assert_ne!(
                    path.attribute("stroke"),
                    Some(paper.as_str()),
                    "/links/{index}"
                );
                examined += 1;
            }
        }
    }
    assert!(examined > 15, "examined {examined}");
}

/// Leaving the Trigger API to its right, the planned line bends twice: each bend is a
/// joint whose casing and core are both drawn solid, so the corner never falls in a gap.
#[test]
fn a_bent_planned_line_draws_its_casing_and_core_solid_around_each_bend() {
    let mut document = onepager_document();
    let planned = link_index(&document, "schedule replays");
    document["links"][planned]["from_side"] = serde_json::json!("right");
    let rendered = common::render_document_with_fixed_metrics(document);
    let svg = common::parse_xml(&rendered.svg.svg);
    let group = common::group(&svg, &format!("/links/{planned}"));
    let paths = common::children_named(group, "path");
    let joints: Vec<_> = paths
        .chunks(2)
        .filter(|pair| pair[0].attribute("stroke-dasharray").is_none())
        .collect();
    assert!(joints.len() >= 2, "{} joints", joints.len());
    for pair in joints {
        assert!(pair[0].attribute("d").unwrap().contains(" A "));
        assert_eq!(pair[1].attribute("stroke-dasharray"), None);
        assert_eq!(pair[0].attribute("d"), pair[1].attribute("d"));
    }
}

/// The strip names a rail entry and a planned entry by their look.
#[test]
fn the_strip_names_a_planned_entry_planned() {
    let document = onepager_document();
    let rendered = common::render_document_with_fixed_metrics(document.clone());
    let svg = common::parse_xml(&rendered.svg.svg);
    let labels: Vec<String> = document["legend"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let group = common::group(&svg, &format!("/legend/{index}"));
            common::children_named(group, "text")[0]
                .text()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        labels,
        [
            "Solid, actor colour",
            "Dashed, actor colour",
            "Solid, actor colour, rail",
            "Solid, actor colour, planned",
            "Solid gray",
            "Dashed red"
        ]
    );
}
