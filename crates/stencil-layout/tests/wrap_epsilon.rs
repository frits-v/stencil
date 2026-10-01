#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{node, page_with_body, part};
use serde_json::json;
use stencil_layout::{PartName, layout_page};
use stencil_model::text::FixedMetricsMeasurer;

/// Advances that are not exact in f32, so a Col sized to the tag's max-content can hand the
/// label back a width one ulp short of it (section 2.1).
#[test]
fn tag_label_laid_out_at_its_max_content_width_stays_on_one_line() {
    let mut layouts = 0;
    for step in 0..100_u16 {
        let advance_em = 0.501 + f32::from(step) * 0.002;
        let mut measurer = FixedMetricsMeasurer {
            advance_em,
            missing_glyphs: Vec::new(),
        };
        for x_count in 1..=40 {
            let label = format!("VLAN {}", "x".repeat(x_count));
            let page = page_with_body(
                1280,
                json!([{
                    "tag": "Row",
                    "grow": [0],
                    "children": [{
                        "tag": "Col",
                        "children": [{ "tag": "Pipe", "dir": "h", "line": "solid", "tint": 1, "label": label }]
                    }]
                }]),
            );
            let geometry = layout_page(&page, &common::gcp(), &mut measurer).unwrap();
            let pipe_node = node(&geometry, "/body/0/children/0/children/0");
            let label_part = part(pipe_node, PartName::TagLabel);
            let run = label_part.text.as_ref().unwrap();
            let tag = part(pipe_node, PartName::Tag);
            let context = format!("advance {advance_em} label {label:?}");
            assert_eq!(run.metrics.line_count, 1, "{context}");
            assert!((label_part.bounds.height - 15.6).abs() <= 0.01, "{context}");
            assert!((tag.bounds.height - 30.6).abs() <= 0.01, "{context}");
            layouts += 1;
        }
    }
    assert_eq!(layouts, 4000);
}
