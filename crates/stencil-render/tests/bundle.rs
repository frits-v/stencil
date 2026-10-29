#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 5.2: a theme bundle of a real example holds no theme colour as a literal where
//! the themes disagree.

mod common;

use std::collections::BTreeSet;

use stencil_model::Page;
use stencil_render::{FadeFloor, ThemeFile, bundle_themes};

const THEMES: [&str; 3] = ["center", "clear-dark", "wire"];

/// Every `#RRGGBB` literal in `text`.
fn colour_literals(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find('#') {
        rest = &rest[start..];
        let candidate = rest.get(..7).unwrap_or_default();
        let digits = candidate.strip_prefix('#').unwrap_or_default();
        if digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            found.insert(candidate.to_string());
        }
        rest = rest.get(1..).unwrap_or_default();
    }
    found
}

/// The lines after the root tag and the style blocks: the drawing.
fn drawing(svg: &str) -> String {
    let mut lines = svg.lines().skip(1).peekable();
    let mut body = Vec::new();
    let mut in_style = false;
    for line in lines.by_ref() {
        let trimmed = line.trim_start();
        if trimmed == "<style>" {
            in_style = true;
        } else if trimmed == "</style>" {
            in_style = false;
        } else if !in_style && trimmed != "<defs>" && trimmed != "</defs>" {
            body.push(line);
        }
    }
    body.join("\n")
}

#[test]
fn a_bundle_of_g7_replaces_every_colour_the_themes_disagree_on_with_a_variable() {
    let page: Page = serde_json::from_str(common::G7_JSON).unwrap();
    let geometry = common::layout_with_cosmic_text(&page);
    let themes: Vec<_> = THEMES.iter().map(|name| common::theme(name)).collect();
    let documents: Vec<_> = themes
        .iter()
        .map(|theme| stencil_render::render_svg(&page, theme, &geometry).unwrap())
        .collect();
    let written: Vec<String> = documents
        .iter()
        .map(|document| document.self_contained_with(FadeFloor::Variable).unwrap())
        .collect();
    let files: Vec<ThemeFile<'_>> = themes
        .iter()
        .zip(&written)
        .zip(&documents)
        .map(|((theme, svg), document)| ThemeFile {
            theme,
            svg,
            dark_page: document.dark_page,
        })
        .collect();

    let bundled = bundle_themes(&files).unwrap();

    assert!(bundled.starts_with("<svg "), "{:.80}", bundled);
    assert!(
        bundled
            .lines()
            .next()
            .unwrap()
            .contains(r#" data-themes="center clear-dark wire">"#)
    );
    let drawings: Vec<BTreeSet<String>> = written
        .iter()
        .map(|svg| colour_literals(&drawing(svg)))
        .collect();
    let shared: BTreeSet<String> = drawings
        .iter()
        .skip(1)
        .fold(drawings[0].clone(), |shared, next| &shared & next);
    let disputed: BTreeSet<String> = drawings
        .iter()
        .flatten()
        .filter(|colour| !shared.contains(*colour))
        .cloned()
        .collect();
    assert!(disputed.len() > 20, "{} disputed colours", disputed.len());
    // A line only one theme draws keeps that theme's literal: it is switched off elsewhere
    // by `display`, or it is a definition the others never reference.
    let bundled_drawing = drawing(&bundled);
    let left: Vec<&str> = bundled_drawing
        .lines()
        .filter(|line| {
            !line.contains("display=\"var(--stencil-display-") && !line.contains(" id=\"")
        })
        .filter(|line| disputed.iter().any(|colour| line.contains(colour.as_str())))
        .collect();
    assert!(left.is_empty(), "literals left in the bundle: {left:#?}");
    let switched = bundled_drawing
        .lines()
        .filter(|line| line.contains("display=\"var(--stencil-display-"))
        .count();
    assert!(switched > 20, "{switched} switched lines");
    let variables = bundled_drawing.matches("var(--stencil-").count();
    assert!(variables > 100, "{variables} variable uses");
    for name in THEMES {
        assert!(
            bundled.contains(&format!(
                r#"svg[data-themes][data-theme="{name}"] {{ --stencil-fade: "#
            )),
            "{name} has no block"
        );
    }
    assert!(bundled.contains("@media (prefers-color-scheme: dark) { svg[data-themes]:not([data-theme]) { --stencil-fade: 0.35;"));
    assert!(bundled.contains("--stencil-ink-primary: #202124;"));
    assert!(
        bundled.contains(r#"display="var(--stencil-display-"#),
        "wire's legend labels alternate"
    );
}
