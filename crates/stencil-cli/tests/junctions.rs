#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! junctions-plain on the examples (section 6): every example stays well under the match
//! threshold, so a change that brings one near it shows here before it shows as a defect.

use std::fs;
use std::path::Path;

use stencil_cli::pipeline::{load_document_from, read_input, render_page};
use stencil_layout::junctions::{JUNCTION_SCORE_MIN, page_hubs, page_ink, scan_junctions};
use stencil_render::DeviceScale;

/// The least room between an example's best score and the threshold.
const MARGIN: f32 = 0.1;

#[test]
fn every_example_scores_well_under_the_junction_threshold() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut paths: Vec<_> = fs::read_dir(&examples)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    for path in &paths {
        let json_text = read_input(path).unwrap();
        let loaded = load_document_from(&json_text, path.parent().unwrap(), None).unwrap();
        let rendered = render_page(&loaded, DeviceScale::DEFAULT).unwrap();
        let geometry = &rendered.geometry;
        let scan = scan_junctions(&page_ink(geometry), &page_hubs(geometry)).unwrap();
        assert!(scan.examined > 0, "{}", path.display());
        assert!(
            scan.best_score < JUNCTION_SCORE_MIN - MARGIN,
            "{}: best score {:.3}",
            path.display(),
            scan.best_score
        );
    }
}
