#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::render_svg;
use stencil_model::text::{TEXT_STYLES, TextMeasurer};
use stencil_render::{DeviceScale, PNG_PIXELS_MAX, RenderError, format_number, render_png};
use stencil_text::CosmicTextMeasurer;

fn one_line_svg(family: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="40" viewBox="0 0 200 40"><text x="0" y="20" xml:space="preserve" font-family="{family}" font-size="12" font-weight="400" fill="#000000">Hello</text></svg>"##
    )
}

fn scale(value: u8) -> DeviceScale {
    DeviceScale::new(value).unwrap()
}

fn rounded(value: f32) -> f64 {
    format_number(value).to_string().parse().unwrap()
}

#[test]
fn png_dimensions_are_the_rounded_canvas_times_the_scale() {
    let rendered = common::render_g7();
    let canvas = rendered.geometry.canvas;
    for value in [1_u8, 4] {
        let png = render_png(&rendered.svg.svg, rendered.svg.text_elements, scale(value)).unwrap();
        let pixmap = common::decode_png(&png);
        let factor = f64::from(value);
        assert_eq!(
            f64::from(pixmap.width()),
            (rounded(canvas.width) * factor).ceil()
        );
        assert_eq!(
            f64::from(pixmap.height()),
            (rounded(canvas.height) * factor).ceil()
        );
    }
}

#[test]
fn default_scale_renders_g7_at_the_reference_width() {
    let rendered = common::render_g7();
    let png = render_png(
        &rendered.svg.svg,
        rendered.svg.text_elements,
        DeviceScale::DEFAULT,
    )
    .unwrap();
    assert_eq!(common::decode_png(&png).width(), 2640);
}

#[test]
fn canvas_height_that_rounds_down_sets_the_png_height() {
    let mut rendered = common::render_g7();
    rendered.geometry.canvas.height = 652.004;
    let svg = render_svg(&rendered.page, &rendered.geometry).unwrap();
    assert!(svg.svg.contains(r#"height="652""#));
    let png = render_png(&svg.svg, svg.text_elements, scale(2)).unwrap();
    assert_eq!(common::decode_png(&png).height(), 1304);
}

#[test]
fn two_renders_are_byte_identical() {
    let rendered = common::render_g7();
    let first = render_png(&rendered.svg.svg, rendered.svg.text_elements, scale(2)).unwrap();
    let second = render_png(&rendered.svg.svg, rendered.svg.text_elements, scale(2)).unwrap();
    assert_eq!(first, second);
}

#[test]
fn g7_text_elements_all_render() {
    let rendered = common::render_g7();
    assert!(rendered.svg.text_elements > 0);
    render_png(&rendered.svg.svg, rendered.svg.text_elements, scale(1)).unwrap();
}

#[test]
fn missing_family_is_text_not_rendered() {
    let error = render_png(&one_line_svg("Helvetica Neue"), 1, scale(1)).unwrap_err();
    assert!(
        matches!(error, RenderError::TextNotRendered { count: 1 }),
        "{error:?}"
    );
}

#[test]
fn missing_family_is_an_error_even_when_the_count_matches() {
    let error = render_png(&one_line_svg("Helvetica Neue"), 0, scale(1)).unwrap_err();
    assert!(
        matches!(error, RenderError::FontNotResolved { lookups: 1 }),
        "{error:?}"
    );
}

#[test]
fn a_generic_family_does_not_fall_back_to_inter() {
    let error = render_png(&one_line_svg("sans-serif"), 1, scale(1)).unwrap_err();
    assert!(
        matches!(error, RenderError::TextNotRendered { count: 1 }),
        "{error:?}"
    );
}

#[test]
fn inter_renders_with_the_exact_count() {
    render_png(&one_line_svg("Inter"), 1, scale(1)).unwrap();
}

#[test]
fn inter_with_too_high_a_count_is_text_not_rendered() {
    let error = render_png(&one_line_svg("Inter"), 2, scale(1)).unwrap_err();
    assert!(
        matches!(error, RenderError::TextNotRendered { count: 1 }),
        "{error:?}"
    );
}

#[test]
fn inter_with_too_low_a_count_is_text_count_exceeded() {
    let error = render_png(&one_line_svg("Inter"), 0, scale(1)).unwrap_err();
    assert!(
        matches!(
            error,
            RenderError::TextCountExceeded {
                expected: 0,
                found: 1
            }
        ),
        "{error:?}"
    );
}

#[test]
fn a_glyph_inter_lacks_is_not_substituted() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="40" viewBox="0 0 200 40"><text x="0" y="20" font-family="Inter" font-size="12" font-weight="400">Hello 一</text></svg>"#;
    let error = render_png(svg, 1, scale(1)).unwrap_err();
    assert!(
        matches!(error, RenderError::FontNotResolved { lookups: 1 }),
        "{error:?}"
    );
}

/// 1320 x 60000 at scale 2 is 316.8 million pixels, above the 2^27 budget. The same SVG at
/// scale 1, 79.2 million pixels, is under it, so the rejection comes from the scale.
#[test]
fn a_canvas_above_the_pixel_budget_is_rejected_before_allocation() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="1320" height="60000" viewBox="0 0 1320 60000"><rect x="0" y="0" width="1320" height="60000" fill="#FFFFFF"/></svg>"##;
    let error = render_png(svg, 0, scale(2)).unwrap_err();
    assert!(
        matches!(
            error,
            RenderError::PixmapAllocation {
                width: 2640,
                height: 120000
            }
        ),
        "{error:?}"
    );
    const { assert!(2640 * 120_000 > PNG_PIXELS_MAX) };
    const { assert!(1320 * 60_000 <= PNG_PIXELS_MAX) };
}

/// usvg's default string resolver reads a non-data href from disk. The SVG writer only
/// embeds `data:` URIs, and render_png resolves nothing else. The file is an SVG because
/// resvg is built without `raster-images`, so a PNG file would render white whichever
/// resolver ran, while usvg decodes an SVG sub-image itself.
#[test]
fn an_image_href_to_a_file_is_not_read() {
    let directory =
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("stencil-render-image-href");
    std::fs::create_dir_all(&directory).unwrap();
    let image_path = directory.join("red.svg");
    std::fs::write(
        &image_path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8" viewBox="0 0 8 8"><rect x="0" y="0" width="8" height="8" fill="#FF0000"/></svg>"##,
    )
    .unwrap();
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8" viewBox="0 0 8 8"><rect x="0" y="0" width="8" height="8" fill="#FFFFFF"/><image x="0" y="0" width="8" height="8" href="{}"/></svg>"##,
        image_path.display()
    );
    let png = render_png(&svg, 0, scale(1)).unwrap();
    let pixmap = common::decode_png(&png);
    assert!(pixmap.pixels().iter().all(common::white_pixel));
}

#[test]
fn unparseable_svg_is_an_svg_error() {
    let error = render_png("<svg", 0, scale(1)).unwrap_err();
    assert!(matches!(error, RenderError::Svg { .. }), "{error:?}");
}

/// Section 10 parity: the rendered ink never extends past the measured box by more than
/// 1 px, and some ink falls inside it. Ink bounds and advance widths are never compared
/// for equality.
#[test]
fn rendered_text_stays_inside_the_measured_box_for_every_style() {
    const RENDER_SCALE: u8 = 4;
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    for named in TEXT_STYLES {
        let source = "Region A · Metro 1 ↔ Region B";
        let text = if named.uppercase {
            source.to_uppercase()
        } else {
            source.to_string()
        };
        let style = named.style;
        let metrics = measurer.measure(&text, &style, None).unwrap();
        assert_eq!(metrics.line_count, 1);
        let baseline = metrics.lines[0].baseline_px;
        let letter_spacing_px = style.letter_spacing_em * style.size_px;
        let letter_spacing = if format_number(letter_spacing_px).to_string() == "0" {
            String::new()
        } else {
            format!(r#" letter-spacing="{}""#, format_number(letter_spacing_px))
        };
        let canvas_width = (metrics.width_px + 40.0).ceil();
        let canvas_height = (metrics.height_px + 40.0).ceil();
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{canvas_width}" height="{canvas_height}" viewBox="0 0 {canvas_width} {canvas_height}"><rect x="0" y="0" width="{canvas_width}" height="{canvas_height}" fill="#FFFFFF"/><text x="0" y="{}" xml:space="preserve" font-family="Inter" font-size="{}" font-weight="{}"{letter_spacing} fill="#202124">{text}</text></svg>"##,
            format_number(baseline),
            format_number(style.size_px),
            style.weight.css_value(),
        );
        let png = render_png(&svg, 1, scale(RENDER_SCALE)).unwrap();
        let pixmap = common::decode_png(&png);

        let factor = f32::from(RENDER_SCALE);
        let max_x = (metrics.width_px + 1.0) * factor;
        let max_y = (metrics.height_px + 1.0) * factor;
        let box_x = metrics.width_px * factor;
        let box_y = metrics.height_px * factor;
        let mut ink_inside_box = false;
        let width = pixmap.width() as usize;
        for (index, pixel) in pixmap.pixels().iter().enumerate() {
            if common::white_pixel(pixel) {
                continue;
            }
            let x = (index % width) as f32;
            let y = (index / width) as f32;
            assert!(
                x <= max_x,
                "{}: ink at x {x} past {max_x}",
                named.name.as_str()
            );
            assert!(
                y <= max_y,
                "{}: ink at y {y} past {max_y}",
                named.name.as_str()
            );
            if x < box_x && y < box_y {
                ink_inside_box = true;
            }
        }
        assert!(
            ink_inside_box,
            "{}: no ink inside the measured box",
            named.name.as_str()
        );
    }
}
