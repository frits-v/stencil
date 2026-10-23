#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 14.5 rule 6: the drafting grid and the frame's references are one geometry.
//! Every reference tick stands on a major grid line, every major line on a tick, and the
//! minor lines cut each zone into equal parts close to 20 px.

mod common;

use serde_json::json;
use stencil_layout::{BoxRect, PageGeometry, PartName};
use stencil_model::Page;

const TOLERANCE_PX: f32 = 0.01;
const MINOR_TARGET_PX: f32 = 20.0;

/// The grid lines one axis draws across the whole `Grid` part.
#[derive(Debug, Default)]
struct AxisLines {
    major: Vec<f32>,
    minor: Vec<f32>,
}

/// The positions of the grid's vertical and horizontal lines, read from the paths the page
/// group draws directly. The lines a washed lane band draws again sit inside the Phases
/// group and are not read here.
fn drawn_grid(svg: &str, theme: &str, grid: BoxRect) -> (AxisLines, AxisLines) {
    let theme = common::theme(theme);
    let colors = theme.sheet_colors().grid.unwrap();
    let (minor, major) = (colors.minor.0.clone(), colors.major.0.clone());
    let document = common::parse_xml(svg);
    let page = common::group(&document, "");
    let mut columns = AxisLines::default();
    let mut rows = AxisLines::default();
    for path in page.children().filter(|child| child.has_tag_name("path")) {
        let stroke = path.attribute("stroke").unwrap_or_default();
        let is_major = stroke == major;
        if !is_major && stroke != minor {
            continue;
        }
        let tokens: Vec<&str> = path.attribute("d").unwrap().split_whitespace().collect();
        for chunk in tokens.chunks(5) {
            let number = |index: usize| chunk[index].parse::<f32>().unwrap();
            assert_eq!(chunk[0], "M");
            let (axis, position) = match chunk[3] {
                "V" => {
                    assert_close(number(2), grid.y, "a vertical line's top");
                    assert_close(number(4), grid.bottom(), "a vertical line's bottom");
                    (&mut columns, number(1))
                }
                "H" => {
                    assert_close(number(1), grid.x, "a horizontal line's left");
                    assert_close(number(4), grid.right(), "a horizontal line's right");
                    (&mut rows, number(2))
                }
                other => panic!("{other} in a grid path"),
            };
            if is_major {
                axis.major.push(position);
            } else {
                axis.minor.push(position);
            }
        }
    }
    (columns, rows)
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= TOLERANCE_PX,
        "{what}: {actual} is not {expected}"
    );
}

/// The tick positions across the band: x of the column ticks, y of the row ticks, each
/// once, in order.
fn tick_positions(geometry: &PageGeometry) -> (Vec<f32>, Vec<f32>) {
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    for part in &geometry.nodes[0].parts {
        if part.name != PartName::RefTick {
            continue;
        }
        if part.bounds.width == 0.0 {
            columns.push(part.bounds.x);
        } else {
            assert_eq!(part.bounds.height, 0.0, "a tick is a line");
            rows.push(part.bounds.y);
        }
    }
    for positions in [&mut columns, &mut rows] {
        positions.sort_by(f32::total_cmp);
        positions.dedup();
    }
    (columns, rows)
}

/// Checks one axis and returns how many lines it examined.
fn check_axis(ticks: &[f32], lines: &AxisLines, near: f32, far: f32, axis: &str) -> usize {
    assert_eq!(
        lines.major.len(),
        ticks.len(),
        "{axis}: majors {:?}, ticks {ticks:?}",
        lines.major
    );
    for tick in ticks {
        assert!(
            lines
                .major
                .iter()
                .any(|major| (major - tick).abs() <= TOLERANCE_PX),
            "{axis}: the tick at {tick} has no major line; majors {:?}",
            lines.major
        );
    }
    for major in &lines.major {
        assert!(
            ticks
                .iter()
                .any(|tick| (major - tick).abs() <= TOLERANCE_PX),
            "{axis}: the major line at {major} stands on no tick; ticks {ticks:?}"
        );
    }
    let mut edges = vec![near];
    edges.extend_from_slice(ticks);
    edges.push(far);
    let mut minors = lines.minor.clone();
    minors.sort_by(f32::total_cmp);
    let mut divisions = Vec::new();
    let mut inside_zones = 0;
    for zone in edges.windows(2) {
        let (start, end) = (zone[0], zone[1]);
        let size = end - start;
        let parts = (size / MINOR_TARGET_PX).round().max(1.0);
        divisions.push(parts);
        let inside: Vec<f32> = minors
            .iter()
            .copied()
            .filter(|minor| *minor > start + TOLERANCE_PX && *minor < end - TOLERANCE_PX)
            .collect();
        assert_eq!(
            inside.len() as f32,
            parts - 1.0,
            "{axis}: zone {start} to {end} holds minors {inside:?}"
        );
        inside_zones += inside.len();
        for (index, minor) in inside.iter().enumerate() {
            let expected = start + (index as f32 + 1.0) * size / parts;
            assert_close(
                *minor,
                expected,
                &format!("{axis}: minor {index} of {start}"),
            );
        }
    }
    assert!(
        divisions.windows(2).all(|pair| pair[0] == pair[1]),
        "{axis}: zones divide unevenly {divisions:?}"
    );
    assert_eq!(
        inside_zones,
        minors.len(),
        "{axis}: a minor line lies on a zone edge or outside the grid"
    );
    lines.major.len() + minors.len()
}

/// Checks the grid of one rendered sheet and returns how many ticks and lines it examined.
fn check_sheet(page: &Page, geometry: &PageGeometry, theme: &str) -> usize {
    let svg = stencil_render::render_svg(page, &common::theme(theme), geometry)
        .unwrap()
        .svg;
    let grid = geometry.nodes[0].part(PartName::Grid).unwrap().bounds;
    let (column_ticks, row_ticks) = tick_positions(geometry);
    assert_eq!((column_ticks.len(), row_ticks.len()), (7, 5));
    let (columns, rows) = drawn_grid(&svg, theme, grid);
    let examined_columns = check_axis(&column_ticks, &columns, grid.x, grid.right(), "x");
    let examined_rows = check_axis(&row_ticks, &rows, grid.y, grid.bottom(), "y");
    column_ticks.len() + row_ticks.len() + examined_columns + examined_rows
}

#[test]
fn the_onepager_grid_lies_on_its_reference_zones() {
    let page: Page = serde_json::from_str(include_str!("../../../examples/onepager.json")).unwrap();
    let geometry = common::layout_with_cosmic_text(&page);
    for theme in ["center", "dusk"] {
        let examined = check_sheet(&page, &geometry, theme);
        // 12 ticks, 12 majors and about 75 minor columns and 125 minor rows.
        assert!(examined > 200, "{theme}: examined {examined}");
    }
}

#[test]
fn a_short_narrow_sheet_grid_lies_on_its_reference_zones() {
    let page: Page = serde_json::from_value(json!({
        "title": "Title",
        "kicker": "Kicker",
        "lede": "Lede",
        "badge": "Internal",
        "width": 1280,
        "sheet": {
            "problem": { "points": [{ "text": "A point." }] },
            "answer": { "statement": "A statement." },
            "attention": { "items": [{ "id": "first", "lead": "Lead.", "text": "Text." }] }
        },
        "body": [{
            "tag": "View",
            "title": "System",
            "children": [{ "tag": "Pipe", "dir": "h", "line": "solid", "label": "link" }]
        }],
        "legend": [{ "line": "solid", "text": "request path" }]
    }))
    .unwrap();
    let geometry = common::layout_with_fixed_metrics(&page);
    let examined = check_sheet(&page, &geometry, "center");
    assert!(examined > 12 + 12 + 50, "examined {examined}");
}
