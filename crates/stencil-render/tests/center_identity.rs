//! The `center` theme draws what the renderer drew before themes existed (section 11.1).
//! The fixtures are the SVGs of the three examples rendered before the palette was keyed
//! by theme, so any change to a center color, element or attribute order fails here.

mod common;

use stencil_model::Page;
use stencil_render::render_svg;

const EXAMPLES: [(&str, &str, &str); 3] = [
    (
        "g7",
        include_str!("../../../examples/g7.json"),
        include_str!("fixtures/g7.center.svg"),
    ),
    (
        "hybrid-ai",
        include_str!("../../../examples/hybrid-ai.json"),
        include_str!("fixtures/hybrid-ai.center.svg"),
    ),
    (
        "network-hub-spoke",
        include_str!("../../../examples/network-hub-spoke.json"),
        include_str!("fixtures/network-hub-spoke.center.svg"),
    ),
];

#[test]
fn center_svg_is_byte_identical_to_the_pre_theme_render() {
    for (name, document, expected_svg) in EXAMPLES {
        let page: Page = serde_json::from_str(document).unwrap();
        let geometry = common::layout_with_cosmic_text(&page);
        let rendered = render_svg(&page, &geometry).unwrap();
        assert!(
            rendered.svg == expected_svg,
            "{name}: center SVG differs from tests/fixtures/{name}.center.svg"
        );
    }
}
