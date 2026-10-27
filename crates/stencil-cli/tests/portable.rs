#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg;
use stencil_cli::pipeline::{RenderedPage, load_document_from, read_input, render_page};
use stencil_cli::{ExitCode, run};
use stencil_render::DeviceScale;

/// A channel of a portable pixel may differ from the PNG by this much: glyph places are
/// written to 0.01 px and scales to six decimals, which moves an antialiased edge by a
/// shade, never a glyph.
const CHANNEL_TOLERANCE: u8 = 48;
/// Share of the canvas allowed past `CHANNEL_TOLERANCE`; the examples reach 0.00009.
const DIFFERING_SHARE_MAX: f64 = 0.0002;
/// Share the self-contained SVG must differ by when drawn the same way, so the comparison
/// sees missing text; the examples lose 0.006 (onprem-iso) to over 0.02.
const MISSING_TEXT_SHARE_MIN: f64 = 0.002;

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn example_stems() -> Vec<String> {
    let mut stems: Vec<String> = fs::read_dir(repository_path("examples"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    stems.sort();
    stems
}

fn render_example(stem: &str) -> RenderedPage {
    let input = repository_path(&format!("examples/{stem}.json"));
    let json_text = read_input(&input).unwrap();
    let loaded = load_document_from(&json_text, input.parent().unwrap(), None).unwrap();
    render_page(&loaded, DeviceScale::DEFAULT).unwrap()
}

/// The lines of the `<style>` other than the `@font-face` rules and the text fallback rules,
/// which name faces: the transition and the hover rules.
fn hover_lines(svg: &str) -> Vec<&str> {
    let style = svg.split_once("<style>").unwrap().1;
    let style = style.split_once("</style>").unwrap().0;
    let lines: Vec<&str> = style
        .lines()
        .filter(|line| {
            !line.contains("@font-face") && !line.trim_start().starts_with("text[font-family=")
        })
        .collect();
    assert!(lines.iter().any(|line| line.contains(":hover")));
    lines
}

/// usvg with an empty font database and no image loader: whatever draws, draws from the
/// file alone.
fn rasterize_without_fonts(svg: &str, scale: f32) -> Pixmap {
    let options = usvg::Options {
        fontdb: std::sync::Arc::new(usvg::fontdb::Database::new()),
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(svg, &options).unwrap();
    let width = (tree.size().width() * scale).ceil() as u32;
    let height = (tree.size().height() * scale).ceil() as u32;
    let mut pixmap = Pixmap::new(width, height).unwrap();
    pixmap.fill(Color::WHITE);
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
}

/// Share of the pixels of `drawn` that differ from `expected` past `CHANNEL_TOLERANCE`.
fn differing_share(drawn: &Pixmap, expected: &Pixmap) -> f64 {
    let differing = drawn
        .data()
        .chunks(4)
        .zip(expected.data().chunks(4))
        .filter(|(left, right)| {
            left.iter()
                .zip(right.iter())
                .any(|(a, b)| a.abs_diff(*b) > CHANNEL_TOLERANCE)
        })
        .count();
    differing as f64 / f64::from(drawn.width() * drawn.height())
}

#[test]
fn every_example_draws_its_png_from_the_portable_svg_with_no_font_and_no_image_loader() {
    let stems = example_stems();
    assert!(stems.len() >= 10, "{} examples", stems.len());
    let mut texts_examined = 0;
    for stem in &stems {
        let rendered = render_example(stem);
        let portable = rendered.svg.portable().unwrap();
        let expected = Pixmap::decode_png(&rendered.png).unwrap();
        let drawn = rasterize_without_fonts(&portable, f32::from(DeviceScale::DEFAULT.get()));
        assert_eq!(
            (drawn.width(), drawn.height()),
            (expected.width(), expected.height()),
            "{stem}"
        );
        let share = differing_share(&drawn, &expected);
        assert!(
            share <= DIFFERING_SHARE_MAX,
            "{stem}: {share:.5} of the canvas differs from the PNG"
        );
        let written = rendered.svg.self_contained().unwrap();
        let fontless = rasterize_without_fonts(&written, f32::from(DeviceScale::DEFAULT.get()));
        let missing = differing_share(&fontless, &expected);
        assert!(
            missing >= MISSING_TEXT_SHARE_MIN,
            "{stem}: the self-contained SVG without fonts differs by only {missing:.5}"
        );
        texts_examined += rendered.svg.text_elements;
    }
    assert!(texts_examined > 1000, "{texts_examined} text elements");
}

#[test]
fn the_portable_svg_needs_no_font_or_data_image_and_keeps_the_hover_rules() {
    let mut examples_with_icons = 0;
    for stem in example_stems() {
        let rendered = render_example(&stem);
        let written = rendered.svg.self_contained().unwrap();
        let portable = rendered.svg.portable().unwrap();
        assert!(!portable.contains("@font-face"), "{stem}");
        assert!(!portable.contains("<image"), "{stem}");
        assert!(!portable.contains("href=\"data:"), "{stem}");
        assert_eq!(hover_lines(&portable), hover_lines(&written), "{stem}");

        let texts: Vec<&str> = portable
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("<text "))
            .collect();
        assert_eq!(texts.len(), rendered.svg.text_elements, "{stem}");
        assert!(
            texts
                .iter()
                .all(|line| line.starts_with(r#"<text fill-opacity="0" "#)),
            "{stem}"
        );
        let outlines = portable
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("<g") && line.contains("<use href=\"#stencil-glyph-"))
            .count();
        assert_eq!(outlines, rendered.svg.text_elements, "{stem}");
        let images = written.matches("<image ").count();
        if images > 0 {
            examples_with_icons += 1;
        }
        let nested = portable
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("<svg x="))
            .count();
        assert_eq!(nested, images, "{stem}");
    }
    assert!(
        examples_with_icons >= 3,
        "{examples_with_icons} examples with icons"
    );
}

#[test]
fn render_with_portable_writes_the_portable_svg() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli")
        .join("render_portable");
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    let input = repository_path("examples/g7.json");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let argv: Vec<OsString> = [
        "stencil",
        "render",
        input.to_str().unwrap(),
        "--out-dir",
        directory.to_str().unwrap(),
        "--portable",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    let code = run(argv, &mut stdout, &mut stderr);
    assert_eq!(
        code,
        ExitCode::Clean,
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    let written = fs::read_to_string(directory.join("g7.svg")).unwrap();
    assert_eq!(written, render_example("g7").svg.portable().unwrap());
}
