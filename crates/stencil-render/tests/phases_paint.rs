#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 14.5 rule 4: a Phases node's lane band and its Steps' fills, painted from the
//! theme only.

mod common;

use resvg::usvg::roxmltree;
use serde_json::{Value, json};
use stencil_layout::{BoxRect, NodeGeometry, PageGeometry, PartName, sheet_zone_edges};
use stencil_model::{StepFill, StepLook, Theme};

const PHASES: &str = "/body/0/children/0";
const GRID_PITCH_PX: f32 = 20.0;

fn onepager() -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    // The example sets its own looks; each test starts from the defaults.
    let phases = &mut document["body"][0]["children"][0];
    phases.as_object_mut().unwrap().remove("step_fill");
    phases.as_object_mut().unwrap().remove("band");
    for step in phases["children"].as_array_mut().unwrap() {
        step.as_object_mut().unwrap().remove("fill");
    }
    document
}

fn render(mut document: Value, theme: &str) -> common::Rendered {
    document["theme"] = json!(theme);
    common::render_document_with_fixed_metrics(document)
}

fn node<'a>(geometry: &'a PageGeometry, pointer: &str) -> &'a NodeGeometry {
    geometry
        .nodes
        .iter()
        .find(|node| node.pointer.as_str() == pointer)
        .unwrap_or_else(|| panic!("no node at {pointer}"))
}

fn parts(node: &NodeGeometry, name: PartName) -> Vec<BoxRect> {
    node.parts
        .iter()
        .filter(|part| part.name == name)
        .map(|part| part.bounds)
        .collect()
}

/// Each Step of onepager's Phases node: its pointer, its lane's tint slot, its look and
/// its lines.
fn steps(document: &Value) -> Vec<(String, u8, StepLook, Vec<String>)> {
    let actors = document["actors"].as_array().unwrap();
    document["body"][0]["children"][0]["children"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let lane = step["lane"].as_str().unwrap();
            let slot = actors
                .iter()
                .find(|actor| actor["id"] == lane)
                .and_then(|actor| actor["tint"].as_u64())
                .unwrap();
            let look: StepLook =
                serde_json::from_value(step.get("look").cloned().unwrap_or(json!("box"))).unwrap();
            let lines = step["lines"]
                .as_array()
                .map(|lines| {
                    lines
                        .iter()
                        .map(|line| {
                            line.as_str()
                                .or_else(|| line["text"].as_str())
                                .unwrap()
                                .to_string()
                        })
                        .collect()
                })
                .unwrap_or_default();
            (
                format!("{PHASES}/children/{index}"),
                u8::try_from(slot).unwrap(),
                look,
                lines,
            )
        })
        .collect()
}

/// The direct shape children of a group, in order.
fn shapes<'a, 'input>(group: roxmltree::Node<'a, 'input>) -> Vec<roxmltree::Node<'a, 'input>> {
    group
        .children()
        .filter(|child| {
            child.is_element() && !child.has_tag_name("g") && !child.has_tag_name("text")
        })
        .collect()
}

/// The `M x y V y2` and `M x y H x2` segments of a grid path, as (x1, y1, x2, y2).
fn segments(path: &str) -> Vec<(f32, f32, f32, f32)> {
    let tokens: Vec<&str> = path.split_whitespace().collect();
    tokens
        .chunks(5)
        .map(|chunk| {
            let number = |index: usize| chunk[index].parse::<f32>().unwrap();
            assert_eq!(chunk[0], "M", "{path}");
            match chunk[3] {
                "V" => (number(1), number(2), number(1), number(4)),
                "H" => (number(1), number(2), number(4), number(2)),
                other => panic!("{other} in {path}"),
            }
        })
        .collect()
}

fn lane_band(theme: &Theme, slot: u8) -> String {
    theme.lane_band(slot).unwrap().0
}

/// The page grid's lines along one axis between its zone `edges`, as (position, major):
/// a major line on each inner edge and minors cutting each zone into equal parts.
fn grid_lines(edges: &[f32]) -> Vec<(f32, bool)> {
    let mut lines = Vec::new();
    for (zone, bounds) in edges.windows(2).enumerate() {
        let (start, end) = (bounds[0], bounds[1]);
        let parts = ((end - start) / GRID_PITCH_PX).round().max(1.0);
        if zone > 0 {
            lines.push((start, true));
        }
        for part in 1..parts as usize {
            lines.push((start + part as f32 * (end - start) / parts, false));
        }
    }
    lines
}

/// A washed lane fills its band in the band colour and then draws, over it, the grid
/// lines that fall inside it, placed from the page grid's zones and cut to the band.
#[test]
fn a_washed_lane_draws_its_band_then_the_grid_over_it() {
    let document = onepager();
    let rendered = render(document.clone(), "center");
    let theme = common::theme("center");
    let (minor, major) = {
        let grid = theme.sheet_colors().grid.unwrap();
        (grid.minor.0.clone(), grid.major.0.clone())
    };
    let grid = parts(node(&rendered.geometry, ""), PartName::Grid)[0];
    let edges = sheet_zone_edges(grid);
    let columns = grid_lines(&edges.columns);
    let rows = grid_lines(&edges.rows);
    let on_line = |lines: &[(f32, bool)], position: f32| -> bool {
        let (_, is_major) = lines
            .iter()
            .find(|(line, _)| (line - position).abs() <= 0.01)
            .unwrap_or_else(|| panic!("{position} is off the grid"));
        *is_major
    };
    let bands = parts(node(&rendered.geometry, PHASES), PartName::LaneBand);
    let slots: Vec<u8> = document["body"][0]["children"][0]["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|lane| {
            let actor = document["actors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|actor| actor["id"] == *lane)
                .unwrap();
            u8::try_from(actor["tint"].as_u64().unwrap()).unwrap()
        })
        .collect();
    let svg = common::parse_xml(&rendered.svg.svg);
    let elements = shapes(common::group(&svg, PHASES));
    let mut examined = 0;
    for (band, slot) in bands.iter().zip(&slots) {
        let position = elements
            .iter()
            .position(|element| {
                element.has_tag_name("rect")
                    && element.attribute("y").unwrap().parse::<f32>().unwrap() == band.y
                    && element.attribute("fill") == Some(lane_band(&theme, *slot).as_str())
            })
            .unwrap_or_else(|| panic!("no band rect at y {}", band.y));
        let mut verticals = Vec::new();
        for path in elements.iter().skip(position + 1).take(2) {
            assert!(path.has_tag_name("path"));
            let stroke = path.attribute("stroke").unwrap();
            assert!(stroke == minor || stroke == major, "{stroke}");
            for (x1, y1, x2, y2) in segments(path.attribute("d").unwrap()) {
                assert!(x1 >= band.x && x2 <= band.right(), "x {x1} to {x2}");
                assert!(y1 >= band.y && y2 <= band.bottom(), "y {y1} to {y2}");
                if x1 == x2 {
                    assert_eq!((y1, y2), (band.y, band.bottom()));
                    assert_eq!(stroke == major, on_line(&columns, x1), "x {x1}");
                    verticals.push(x1);
                } else {
                    assert_eq!((x1, x2), (band.x, band.right()));
                    assert_eq!(stroke == major, on_line(&rows, y1), "y {y1}");
                }
                examined += 1;
            }
        }
        let expected = columns
            .iter()
            .filter(|(x, _)| *x >= band.x && *x < band.right().min(grid.right()))
            .count();
        assert_eq!(verticals.len(), expected, "band at y {}", band.y);
    }
    assert_eq!(bands.len(), 4);
    assert!(examined > 4 * 60, "examined {examined}");
}

/// `band: none` draws no band, so the paper and the page grid show; the lane bars, the lane
/// rules and the phase separators stay, and the geometry does not move.
#[test]
fn a_lane_with_band_none_draws_no_band_and_keeps_its_bar_and_rules() {
    let theme = common::theme("center");
    let washed = render(onepager(), "center");
    let mut document = onepager();
    document["body"][0]["children"][0]["band"] = json!("none");
    let bare = render(document, "center");
    assert_eq!(washed.geometry, bare.geometry);

    let svg = common::parse_xml(&bare.svg.svg);
    let elements = shapes(common::group(&svg, PHASES));
    let bands: Vec<String> = (1..=8).map(|slot| lane_band(&theme, slot)).collect();
    assert!(
        elements.iter().all(|element| element
            .attribute("fill")
            .is_none_or(|fill| !bands.iter().any(|band| band == fill))),
        "a band is painted"
    );
    let grid_strokes = {
        let grid = theme.sheet_colors().grid.unwrap();
        [grid.minor.0.clone(), grid.major.0.clone()]
    };
    assert!(
        elements.iter().all(|element| element
            .attribute("stroke")
            .is_none_or(|stroke| !grid_strokes.iter().any(|grid| grid == stroke))),
        "no grid is drawn again inside the node"
    );
    let bars = elements
        .iter()
        .filter(|element| element.has_tag_name("rect") && element.attribute("width") == Some("4"))
        .count();
    let rule = theme.sheet_colors().rule.0.clone();
    let lines = elements
        .iter()
        .filter(|element| {
            element.has_tag_name("line") && element.attribute("stroke") == Some(rule.as_str())
        })
        .count();
    // Three lane rules and three phase separators.
    assert_eq!((bars, lines), (4, 6));
}

/// The front of every Step under each `step_fill`, and with none set: a tint fill paints the
/// actor's fill, cell the sheet cell, none nothing; absent, each look keeps its own fill.
#[test]
fn each_step_fill_paints_the_front_of_every_look() {
    let theme = common::theme("center");
    let cell = theme.sheet_colors().cell.0.clone();
    let mut examined = 0;
    for fill in [
        None,
        Some(StepFill::Tint),
        Some(StepFill::Cell),
        Some(StepFill::None),
    ] {
        let mut document = onepager();
        if let Some(fill) = fill {
            document["body"][0]["children"][0]["step_fill"] = json!(fill.as_str());
        }
        let rendered = render(document.clone(), "center");
        let svg = common::parse_xml(&rendered.svg.svg);
        for (pointer, slot, look, _) in steps(&document) {
            let group = common::group(&svg, &pointer);
            let rects = common::children_named(group, "rect");
            if look == StepLook::Text {
                assert!(rects.is_empty(), "{pointer}: a text look draws no box");
                continue;
            }
            let front = rects.last().unwrap();
            let expected = match fill.unwrap_or(look.default_fill()) {
                StepFill::Tint => theme.tint(slot).fill.0.clone(),
                StepFill::Cell => cell.clone(),
                StepFill::None => "none".to_string(),
            };
            assert_eq!(
                front.attribute("fill"),
                Some(expected.as_str()),
                "{pointer} {look:?} under {fill:?}"
            );
            let stroke = if look == StepLook::Note {
                theme.sheet_colors().rule.0.clone()
            } else {
                theme.tint(slot).wire.0.clone()
            };
            assert_eq!(
                front.attribute("stroke"),
                Some(stroke.as_str()),
                "{pointer}"
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 4 * 11);
}

/// A Step's own fill wins over its parent's `step_fill`.
#[test]
fn a_step_fill_overrides_the_phases_step_fill() {
    let theme = common::theme("center");
    let mut document = onepager();
    document["body"][0]["children"][0]["step_fill"] = json!("none");
    document["body"][0]["children"][0]["children"][0]["fill"] = json!("tint");
    let rendered = render(document.clone(), "center");
    let svg = common::parse_xml(&rendered.svg.svg);
    let (first, slot, _, _) = steps(&document)[0].clone();
    let front = common::children_named(common::group(&svg, &first), "rect")[0];
    assert_eq!(
        front.attribute("fill"),
        Some(theme.tint(slot).fill.0.as_str())
    );
    let (second, _, _, _) = steps(&document)[2].clone();
    let front = common::children_named(common::group(&svg, &second), "rect")[0];
    assert_eq!(front.attribute("fill"), Some("none"));
}

/// An unfilled stack draws its two back sheets as open paths of the edges that stand clear
/// of the sheet in front, so no back edge crosses the front; a filled stack keeps three
/// rects.
#[test]
fn an_unfilled_stack_draws_its_back_sheets_as_the_edges_clear_of_the_front() {
    let theme = common::theme("center");
    let document = onepager();
    let (pointer, slot, _, _) = steps(&document)
        .into_iter()
        .find(|(_, _, look, _)| *look == StepLook::Stack)
        .unwrap();
    let wire = theme.tint(slot).wire.0.clone();

    let filled = render(document.clone(), "center");
    let svg = common::parse_xml(&filled.svg.svg);
    let group = common::group(&svg, &pointer);
    assert_eq!(common::children_named(group, "rect").len(), 3);
    assert!(common::children_named(group, "path").is_empty());

    let mut unfilled_document = document;
    unfilled_document["body"][0]["children"][0]["step_fill"] = json!("none");
    let unfilled = render(unfilled_document, "center");
    let svg = common::parse_xml(&unfilled.svg.svg);
    let group = common::group(&svg, &pointer);
    let rects = common::children_named(group, "rect");
    let paths = common::children_named(group, "path");
    assert_eq!((rects.len(), paths.len()), (1, 2));
    let front = node(&unfilled.geometry, &pointer);
    let front = parts(front, PartName::Front)[0];
    for path in paths {
        assert_eq!(path.attribute("fill"), Some("none"));
        assert_eq!(path.attribute("stroke"), Some(wire.as_str()));
        let d = path.attribute("d").unwrap();
        let tokens: Vec<&str> = d.split_whitespace().collect();
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.parse::<f32>().is_err())
                .count(),
            5,
            "{d}: one move and four edges"
        );
        let numbers: Vec<f32> = tokens
            .iter()
            .filter_map(|token| token.parse().ok())
            .collect();
        // M x y0 V y1 H x2 V y3 H x4: the left side stops at or above the nearer sheet's
        // top, and the bottom edge ends at or right of the nearer sheet's right edge, so no
        // edge runs inside the front.
        let (left, top_end, right, bottom, bottom_end) =
            (numbers[0], numbers[1], numbers[3], numbers[4], numbers[5]);
        assert!(top_end <= front.y + 1.0, "{d}");
        assert!(left > front.x && right > front.right(), "{d}");
        assert!(bottom < front.bottom(), "{d}");
        assert!(bottom_end >= front.right() - 1.0, "{d}");
    }
}

/// On a tint fill a Step's lines take the actor's ink; on a cell or no fill the secondary
/// ink.
#[test]
fn step_lines_take_the_actor_ink_on_a_tint_fill_only() {
    let theme = common::theme("center");
    let mut examined = 0;
    for fill in [StepFill::Tint, StepFill::Cell, StepFill::None] {
        let mut document = onepager();
        document["body"][0]["children"][0]["step_fill"] = json!(fill.as_str());
        let rendered = render(document.clone(), "center");
        let svg = common::parse_xml(&rendered.svg.svg);
        for (pointer, slot, look, lines) in steps(&document) {
            if look == StepLook::Text {
                continue;
            }
            let expected = match fill {
                StepFill::Tint => theme.tint(slot).ink.0.clone(),
                StepFill::Cell | StepFill::None => theme.ink.secondary.0.clone(),
            };
            let group = common::group(&svg, &pointer);
            for text in group
                .descendants()
                .filter(|element| element.has_tag_name("text"))
                .filter(|element| {
                    let content = element.text().unwrap_or_default().trim();
                    // A mark's numeral is a number, never a line's text.
                    let numeral = content.chars().all(|character| character.is_ascii_digit());
                    // A title is bold; a wrapped title line can repeat a word of a line.
                    let bold = element.attribute("font-weight") == Some("700");
                    !content.is_empty()
                        && !numeral
                        && !bold
                        && lines.iter().any(|line| line.contains(content))
                })
            {
                assert_eq!(
                    text.attribute("fill"),
                    Some(expected.as_str()),
                    "{pointer} under {fill:?}"
                );
                examined += 1;
            }
        }
    }
    assert!(examined >= 3 * 10, "examined {examined}");
}

/// Wire paints only white fills: a tint fill and a washed band are white there, and the
/// geometry is the one every other theme lays out.
#[test]
fn wire_paints_a_tint_fill_and_a_washed_band_white() {
    let mut document = onepager();
    document["body"][0]["children"][0]["step_fill"] = json!("tint");
    let wire = render(document.clone(), "wire");
    let center = render(document.clone(), "center");
    assert_eq!(wire.geometry, center.geometry);
    let svg = common::parse_xml(&wire.svg.svg);
    let mut examined = 0;
    for element in common::group(&svg, PHASES)
        .descendants()
        .filter(|element| element.has_tag_name("rect") || element.has_tag_name("path"))
    {
        let fill = element.attribute("fill").unwrap();
        assert!(
            ["#FFFFFF", "none", "#222222"].contains(&fill),
            "fill {fill}"
        );
        examined += 1;
    }
    for (pointer, _, look, _) in steps(&document) {
        if look == StepLook::Text {
            continue;
        }
        let front = *common::children_named(common::group(&svg, &pointer), "rect")
            .last()
            .unwrap();
        assert_eq!(front.attribute("fill"), Some("#FFFFFF"), "{pointer}");
    }
    assert!(examined > 20, "examined {examined}");
}

/// Onepager as authored, in every theme a design sheet is judged in: each lane is a wash
/// and each Step on it paints an opaque front, so neither the wash nor the grid shows
/// through a box.
#[test]
fn the_onepager_paints_its_steps_opaque_over_the_lane_wash() {
    let document: Value =
        serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    let lanes: Vec<String> = document["body"][0]["children"][0]["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|lane| lane.as_str().unwrap().to_string())
        .collect();
    let mut examined = 0;
    for theme_name in ["center", "paper", "dusk", "clear", "clear-dark", "wire"] {
        let theme = common::theme(theme_name);
        let rendered = render(document.clone(), theme_name);
        let svg = common::parse_xml(&rendered.svg.svg);
        let bands = parts(node(&rendered.geometry, PHASES), PartName::LaneBand);
        assert_eq!(bands.len(), lanes.len(), "{theme_name}");
        let band_rects = shapes(common::group(&svg, PHASES));
        for (index, (pointer, slot, look, _)) in steps(&document).into_iter().enumerate() {
            if look == StepLook::Text {
                continue;
            }
            let lane = document["body"][0]["children"][0]["children"][index]["lane"]
                .as_str()
                .unwrap();
            let band = bands[lanes.iter().position(|name| name == lane).unwrap()];
            let step = node(&rendered.geometry, &pointer).bounds;
            assert!(
                step.x >= band.x
                    && step.right() <= band.right()
                    && step.y >= band.y
                    && step.bottom() <= band.bottom(),
                "{theme_name} {pointer}: off its lane band"
            );
            let wash = lane_band(&theme, slot);
            assert!(
                band_rects.iter().any(|element| element.has_tag_name("rect")
                    && element.attribute("y").unwrap().parse::<f32>().unwrap() == band.y
                    && element.attribute("fill") == Some(wash.as_str())),
                "{theme_name} {pointer}: lane {lane} is not washed"
            );
            let front = *common::children_named(common::group(&svg, &pointer), "rect")
                .last()
                .unwrap();
            let fill = front.attribute("fill").unwrap();
            assert_ne!(fill, "none", "{theme_name} {pointer} {look:?}: no fill");
            assert!(
                front.attribute("fill-opacity").is_none() && front.attribute("opacity").is_none(),
                "{theme_name} {pointer}: a translucent front"
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 6 * 11);
}
