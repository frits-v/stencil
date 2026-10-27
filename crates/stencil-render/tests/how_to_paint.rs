#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 14.5 rules 5 and 12: onepager's how-tos, painted from the theme's terminal and
//! sheet roles, and from the roles the terminal derives from when a theme leaves it out;
//! and their terminal text, set in the monospace face in whole advances.

mod common;

use resvg::usvg::roxmltree;
use serde_json::Value;
use stencil_layout::{NodeTag, PartName};
use stencil_model::text::{FontFamily, TextMeasurer, TextStyleName};
use stencil_model::{DESIGNED_THEMES, IMPORTED_THEMES, Theme};
use stencil_render::render_svg;
use stencil_text::CosmicTextMeasurer;

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
        ("PLACE A HOLD", sheet.frame.as_str()),
        ("hold place", terminal.text.as_str()),
        ("then", theme.ink.secondary.as_str()),
        ("hold confirm", terminal.text.as_str()),
        (
            "Place the hold for the patron, check the branch it names, then confirm it. No copy is set aside before that.",
            theme.ink.secondary.as_str(),
        ),
        ("$", terminal.prompt.as_str()),
        (
            "hold place --card 2104-8831 --branch east \"Rivers of the North\"",
            terminal.text.as_str(),
        ),
        ("hold h-5120 placed, 3 of 5 in line", terminal.text.as_str()),
        ("#", terminal.comment.as_str()),
        ("the hold waits for confirmation", terminal.comment.as_str()),
        ("\u{2713}", terminal.prompt.as_str()),
        ("h-5120 confirmed for East branch", terminal.prompt.as_str()),
    ];
    for (content, ink) in expected {
        assert_eq!(text_fill(item, content), ink, "{label}: {content}");
    }
    let prompts = common::children_named(item, "text")
        .into_iter()
        .filter(|text| matches!(text.text(), Some("$" | "#" | "\u{2713}")))
        .count();
    assert_eq!(
        prompts, 4,
        "{label}: two commands, one comment and one ok line draw a prompt"
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
    assert_eq!(examined, 39);
}

/// Each line kind of a terminal draws its own prefix before its text, so the kinds read
/// apart without colour: under wire an ok line and an output line share one ink.
#[test]
fn every_terminal_kind_draws_its_own_prefix_in_every_theme() {
    let document: Value =
        serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    let lines = document["body"][2]["children"][0]["items"][0]["terminal"]
        .as_array()
        .unwrap();
    let page = onepager();
    let geometry = common::layout_with_fixed_metrics(&page);
    let mut examined = 0;
    for name in ["center", "dusk", "wire"] {
        let theme = common::theme(name);
        let svg = render_svg(&page, &theme, &geometry).unwrap().svg;
        let xml = common::parse_xml(&svg);
        let texts = common::children_named(common::group(&xml, FIRST_ITEM), "text");
        let x = |text: &roxmltree::Node<'_, '_>| -> f32 {
            text.attribute("x").unwrap().parse().unwrap()
        };
        let mut prefixes = std::collections::BTreeSet::new();
        for line in lines {
            let content = line["text"].as_str().unwrap();
            let drawn = texts
                .iter()
                .find(|text| text.text() == Some(content))
                .unwrap_or_else(|| panic!("{name}: no line {content:?}"));
            let prefix: String = texts
                .iter()
                .filter(|text| text.attribute("y") == drawn.attribute("y") && x(text) < x(drawn))
                .filter_map(|text| text.text())
                .collect();
            prefixes.insert((line["kind"].as_str().unwrap(), prefix));
            examined += 1;
        }
        let expected = [
            ("command", "$"),
            ("comment", "#"),
            ("ok", "\u{2713}"),
            ("output", ""),
        ]
        .map(|(kind, prefix)| (kind, prefix.to_string()));
        assert_eq!(prefixes, expected.into_iter().collect(), "{name}");
    }
    assert_eq!(examined, 18);
}

/// A theme without the terminal role fills the terminal and chips with the fact fill and
/// draws commands and ok lines, with their prompts, in the primary ink and comments in the
/// secondary ink.
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
        text_fill(item, "h-5120 confirmed for East branch"),
        theme.ink.primary.as_str()
    );
    assert_eq!(
        text_fill(item, "the hold waits for confirmation"),
        theme.ink.secondary.as_str()
    );
    assert_eq!(
        assert_first_item_paint(&theme, &svg, "derived"),
        13,
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
            "terminal_prompt/5",
        ]
    );
}

/// The parts a how-to sets in the terminal face: its chips' text, its prompts and its lines.
fn is_terminal_text(name: PartName) -> bool {
    matches!(
        name,
        PartName::CommandText | PartName::TerminalPrompt | PartName::TerminalLine
    )
}

/// Section 2.9: a how-to's chips, prompts and terminal lines are measured in JetBrains Mono
/// NL and written in it, and every other run on the sheet keeps Inter.
#[test]
fn terminal_runs_are_measured_and_written_in_the_monospace_family() {
    let page = onepager();
    let geometry = common::layout_with_cosmic_text(&page);
    let mut terminal_lines = 0;
    let mut other_runs = 0;
    for node in &geometry.nodes {
        for part in &node.parts {
            let Some(run) = &part.text else { continue };
            if is_terminal_text(part.name) {
                assert_eq!(node.tag, NodeTag::HowToItem, "{}", node.pointer);
                assert_eq!(
                    run.style.family,
                    FontFamily::JetBrainsMonoNl,
                    "{}",
                    run.text
                );
                terminal_lines += run.metrics.lines.len();
            } else {
                assert_eq!(run.style.family, FontFamily::Inter, "{}", run.text);
                other_runs += 1;
            }
        }
    }
    // Five chips, 17 terminal lines and the nine prompts of the command, comment and ok lines.
    assert_eq!(terminal_lines, 5 + 17 + 9);
    assert!(other_runs > 0);

    let svg = common::render_svg(&page, &geometry).unwrap().svg;
    let document = common::parse_xml(&svg);
    let texts: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("text"))
        .collect();
    let monospace = texts
        .iter()
        .filter(|text| text.attribute("font-family") == Some("JetBrains Mono NL"))
        .count();
    let inter = texts
        .iter()
        .filter(|text| text.attribute("font-family") == Some("Inter"))
        .count();
    assert_eq!(monospace, terminal_lines);
    assert_eq!(monospace + inter, texts.len());
}

/// Section 14.4 rule 31: the prompt column and each indent level are whole advances of the
/// monospace face, so an indented line starts on a column of the line above it, and a
/// line's width is its character count in advances.
#[test]
fn terminal_indents_and_the_prompt_column_are_whole_monospace_advances() {
    let style = TextStyleName::TerminalText.text_style().style;
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    let advance = measurer.measure("x", &style, None).unwrap().width_px;
    assert!((advance - 0.6 * 11.5).abs() < 1e-3, "{advance}");

    let page = onepager();
    let geometry = common::layout_with_cosmic_text(&page);
    let mut examined = 0;
    for (index, item) in page_how_to_items(&page).iter().enumerate() {
        let pointer = format!("/body/2/children/0/items/{index}");
        let node = geometry
            .nodes
            .iter()
            .find(|node| node.pointer.as_str() == pointer)
            .unwrap();
        let terminal = node
            .parts
            .iter()
            .find(|part| part.name == PartName::Terminal)
            .unwrap();
        let lines: Vec<_> = node
            .parts
            .iter()
            .filter(|part| part.name == PartName::TerminalLine)
            .collect();
        assert_eq!(lines.len(), item.terminal.len(), "{pointer}");
        let inner_left = terminal.bounds.x + 1.0 + 10.0;
        for (line, source) in lines.iter().zip(&item.terminal) {
            let columns = 2.0 + 2.0 * f32::from(source.indent);
            assert!(
                (line.bounds.x - inner_left - columns * advance).abs() < 1e-3,
                "{pointer}: {:?} starts at {}",
                source.text,
                line.bounds.x - inner_left
            );
            let characters = source.text.chars().count() as f32;
            let run = line.text.as_ref().unwrap();
            assert!(
                (run.metrics.width_px - characters * advance).abs() < 1e-2,
                "{pointer}: {:?} is {} wide",
                source.text,
                run.metrics.width_px
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 17);
}

/// The items of onepager's one HowTo, the first child of its third View.
fn page_how_to_items(page: &stencil_model::Page) -> Vec<stencil_model::HowToItem> {
    let document = serde_json::to_value(page).unwrap();
    let items = document["body"][2]["children"][0]["items"].clone();
    serde_json::from_value(items).unwrap()
}
