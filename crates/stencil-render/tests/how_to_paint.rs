#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 14.5 rules 5 and 12: onepager's how-tos, painted from the theme's terminal and
//! sheet roles, and from the roles the terminal derives from when a theme leaves it out.

mod common;

use resvg::usvg::roxmltree;
use serde_json::Value;
use stencil_model::{DESIGNED_THEMES, IMPORTED_THEMES, Theme};
use stencil_render::render_svg;

const FIRST_ITEM: &str = "/body/2/children/0/items/0";

fn onepager() -> stencil_model::Page {
    let document: Value =
        serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    serde_json::from_value(document).unwrap()
}

/// The fill of the `<text>` in `group` that reads `content`.
fn text_fill<'a>(group: roxmltree::Node<'a, '_>, content: &str) -> &'a str {
    group
        .children()
        .find(|element| element.has_tag_name("text") && element.text() == Some(content))
        .unwrap_or_else(|| panic!("no text {content:?}"))
        .attribute("fill")
        .unwrap()
}

/// The inks a theme gives the first how-to: the number, the title, the chips and the
/// words between them, the text, the terminal and each line by its kind.
fn assert_first_item_paint(theme: &Theme, svg: &str, label: &str) -> usize {
    let document = common::parse_xml(svg);
    let item = common::group(&document, FIRST_ITEM);
    let sheet = theme.sheet_colors();
    let terminal = theme.terminal_colors();
    let rects = common::children_named(item, "rect");
    assert_eq!(rects.len(), 3, "{label}: two chips and the terminal");
    for chip in &rects[..2] {
        assert_eq!(
            chip.attribute("fill"),
            Some(terminal.fill.as_str()),
            "{label}"
        );
        assert_eq!(
            chip.attribute("stroke"),
            Some(sheet.rule.as_str()),
            "{label}"
        );
    }
    assert_eq!(
        rects[2].attribute("fill"),
        Some(terminal.fill.as_str()),
        "{label}"
    );
    assert_eq!(
        rects[2].attribute("stroke"),
        Some(sheet.rule.as_str()),
        "{label}"
    );
    let rule = common::children_named(item, "line")[0];
    assert_eq!(
        rule.attribute("stroke"),
        Some(sheet.rule.as_str()),
        "{label}"
    );

    let expected = [
        ("4.1", sheet.label_ink.as_str()),
        ("WRITE A TRIGGER", sheet.frame.as_str()),
        ("trig create", terminal.text.as_str()),
        ("then", theme.ink.secondary.as_str()),
        ("trig approve", terminal.text.as_str()),
        (
            "Describe the event, read the plan the service returns, then approve it. Nothing runs before the approval.",
            theme.ink.secondary.as_str(),
        ),
        ("$", terminal.prompt.as_str()),
        (
            "trig create --camera dock-4 \"a forklift enters while the bay door is open\"",
            terminal.text.as_str(),
        ),
        (
            "trigger t-31 created, plan version 1",
            terminal.text.as_str(),
        ),
        ("#", terminal.comment.as_str()),
        ("the plan waits for approval", terminal.comment.as_str()),
        ("t-31 live from plan version 1", terminal.prompt.as_str()),
    ];
    for (content, ink) in expected {
        assert_eq!(text_fill(item, content), ink, "{label}: {content}");
    }
    let prompts = common::children_named(item, "text")
        .into_iter()
        .filter(|text| matches!(text.text(), Some("$" | "#")))
        .count();
    assert_eq!(
        prompts, 3,
        "{label}: two commands and one comment draw a prompt"
    );
    expected.len()
}

/// Every designed theme sets the terminal role, and a light paper keeps a light terminal
/// so a how-to reads as part of the sheet; the imported themes leave it out.
#[test]
fn every_designed_theme_sets_the_terminal_role() {
    let mut examined = 0;
    for name in DESIGNED_THEMES {
        let theme = common::theme(name);
        let terminal = theme.terminal.as_ref().unwrap_or_else(|| panic!("{name}"));
        let paper = theme.sheet_colors().paper;
        let lightness = |color: &stencil_model::theme::Color| {
            let channels = color.channels().unwrap();
            channels
                .iter()
                .map(|channel| u32::from(*channel))
                .sum::<u32>()
        };
        assert_eq!(
            lightness(&terminal.fill) > 3 * 128,
            lightness(paper) > 3 * 128,
            "{name}: the terminal and the paper differ in lightness"
        );
        examined += 1;
    }
    let wire = common::theme("wire");
    let terminal = wire.terminal.as_ref().unwrap();
    assert_eq!(terminal.fill.as_str(), "#FFFFFF");
    assert_eq!(
        [
            terminal.text.as_str(),
            terminal.prompt.as_str(),
            terminal.comment.as_str()
        ],
        ["#222222", "#222222", "#555555"]
    );
    for name in IMPORTED_THEMES {
        assert_eq!(common::theme(name).terminal, None, "{name}");
    }
    assert_eq!(examined, 6);
}

/// Each line of a terminal takes the ink of its kind, its prompt too, and the chips fill
/// as the terminal does, in a light, a dark and the wire theme.
#[test]
fn terminal_lines_take_the_ink_of_their_kind() {
    let page = onepager();
    let geometry = common::layout_with_fixed_metrics(&page);
    let mut examined = 0;
    for name in ["center", "dusk", "wire"] {
        let theme = common::theme(name);
        let svg = render_svg(&page, &theme, &geometry).unwrap().svg;
        examined += assert_first_item_paint(&theme, &svg, name);
    }
    assert_eq!(examined, 36);
}

/// A theme without the terminal role fills the terminal and chips with the fact fill and
/// draws commands and ok lines in the primary ink and comments in the secondary ink.
#[test]
fn a_theme_without_the_terminal_role_draws_the_derived_colours() {
    let page = onepager();
    let geometry = common::layout_with_fixed_metrics(&page);
    let mut theme = common::theme("center");
    theme.terminal = None;
    let svg = render_svg(&page, &theme, &geometry).unwrap().svg;
    let document = common::parse_xml(&svg);
    let item = common::group(&document, FIRST_ITEM);
    let rects = common::children_named(item, "rect");
    assert_eq!(rects[2].attribute("fill"), Some(theme.fact.fill.as_str()));
    assert_eq!(text_fill(item, "$"), theme.ink.primary.as_str());
    assert_eq!(
        text_fill(item, "t-31 live from plan version 1"),
        theme.ink.primary.as_str()
    );
    assert_eq!(
        text_fill(item, "the plan waits for approval"),
        theme.ink.secondary.as_str()
    );
    assert_eq!(
        assert_first_item_paint(&theme, &svg, "derived"),
        12,
        "the derived role paints as a set one"
    );
}

/// The measured JSON numbers a command run's chip, text and words by the run's index and
/// a prompt by its line's, so each part key names the field it draws.
#[test]
fn how_to_part_keys_follow_the_run_and_line_they_draw() {
    let page = onepager();
    let geometry = common::layout_with_fixed_metrics(&page);
    let document = serde_json::to_value(&page).unwrap();
    let measured = stencil_render::measured_json(&document, &geometry, None);
    let item = measured["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == FIRST_ITEM)
        .unwrap();
    let mut keys: Vec<&str> = item["parts"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .filter(|key| key.starts_with("command_") || key.starts_with("terminal_"))
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "command_chip/0",
            "command_chip/2",
            "command_text/0",
            "command_text/2",
            "command_word/1",
            "terminal_line/0",
            "terminal_line/1",
            "terminal_line/2",
            "terminal_line/3",
            "terminal_line/4",
            "terminal_line/5",
            "terminal_prompt/0",
            "terminal_prompt/3",
            "terminal_prompt/4",
        ]
    );
}
