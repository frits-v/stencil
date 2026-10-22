#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 14.5 rule 12: onepager's detail frame, its tags and its ladder, painted from the
//! theme's sheet roles only.

mod common;

use serde_json::{Value, json};

const PHASES: &str = "/body/0/children/0";
const LADDER: &str = "/body/0/children/1";

fn rendered(theme: &str) -> common::Rendered {
    let mut document: Value =
        serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    document["theme"] = json!(theme);
    common::render_document_with_fixed_metrics(document)
}

#[test]
fn the_detail_frame_and_the_ladder_draw_in_the_frame_ink() {
    let mut examined = 0;
    for name in ["center", "dusk", "wire"] {
        let theme = common::theme(name);
        let frame_ink = theme.sheet_colors().frame.as_str().to_string();
        let cell = theme.sheet_colors().cell.as_str().to_string();
        let rendered = rendered(name);
        let svg = common::parse_xml(&rendered.svg.svg);

        let phases = common::group(&svg, PHASES);
        let frames: Vec<_> = phases
            .descendants()
            .filter(|element| {
                element.has_tag_name("rect")
                    && element.attribute("stroke-dasharray") == Some("6 4")
                    && element.attribute("rx").is_some()
            })
            .collect();
        assert_eq!(frames.len(), 1, "{name}");
        assert_eq!(frames[0].attribute("stroke"), Some(frame_ink.as_str()));
        assert_eq!(frames[0].attribute("fill"), Some("none"));

        let ladder = common::group(&svg, LADDER);
        for group in [phases, ladder] {
            let tag = group
                .descendants()
                .find(|element| {
                    element.has_tag_name("circle")
                        && element.attribute("stroke") == Some(frame_ink.as_str())
                        && element.attribute("fill") == Some(cell.as_str())
                })
                .unwrap_or_else(|| panic!("{name}: no detail tag"));
            assert!(tag.attribute("r").is_some());
            let letter = group
                .descendants()
                .find(|element| element.has_tag_name("text") && element.text() == Some("A"))
                .unwrap_or_else(|| panic!("{name}: no detail letter"));
            assert_eq!(letter.attribute("fill"), Some(frame_ink.as_str()));
        }
        let arrow = ladder
            .descendants()
            .find(|element| {
                element.has_tag_name("line") && element.attribute("stroke-dasharray") == Some("4 3")
            })
            .unwrap_or_else(|| panic!("{name}: no escalation arrow"));
        assert_eq!(arrow.attribute("stroke"), Some(frame_ink.as_str()));
        let head = ladder
            .descendants()
            .find(|element| {
                element.has_tag_name("path")
                    && element.attribute("fill") == Some(frame_ink.as_str())
            })
            .unwrap_or_else(|| panic!("{name}: no arrowhead"));
        assert!(head.attribute("d").unwrap().ends_with('Z'));
        examined += 1;
    }
    assert_eq!(examined, 3);
}

/// A planned rung without an actor is dashed in the frame ink with no fill; an actor's rung
/// takes its actor's fill and wire.
#[test]
fn a_planned_rung_is_dashed_and_unfilled_and_an_actor_rung_takes_its_fill() {
    let theme = common::theme("center");
    let rendered = rendered("center");
    let svg = common::parse_xml(&rendered.svg.svg);
    let planned = common::group(&svg, &format!("{LADDER}/rungs/3"));
    let front = planned
        .descendants()
        .find(|element| element.has_tag_name("rect"))
        .unwrap();
    assert_eq!(front.attribute("stroke-dasharray"), Some("7 4"));
    assert_eq!(front.attribute("fill"), Some("none"));
    assert_eq!(
        front.attribute("stroke"),
        Some(theme.sheet_colors().frame.as_str())
    );
    let operator = common::group(&svg, &format!("{LADDER}/rungs/2"));
    let front = operator
        .descendants()
        .find(|element| element.has_tag_name("rect"))
        .unwrap();
    assert_eq!(front.attribute("fill"), Some(theme.tint(4).fill.as_str()));
    assert_eq!(front.attribute("stroke"), Some(theme.tint(4).wire.as_str()));
}
