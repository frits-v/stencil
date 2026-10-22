//! The figure description (section 14.1) and description-covers-drawing (section 14.6).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::Value;
use stencil_layout::{NodeTag, PageGeometry, UnitKind, description_units, layout_page};
use stencil_model::checks::{CheckName, CheckOutcome, CheckReport};
use stencil_model::text::FixedMetricsMeasurer;
use stencil_model::{
    BUILTIN_THEMES, Grammar, KeyColour, LegendKey, Node, NodeRef, Page, PageActors, PipeForm,
    Projection, body_nodes, builtin_grammar, entry_key, grammar_reference, line_key, validate_page,
};
use stencil_render::{describe, description_covers_drawing};
use stencil_text::CosmicTextMeasurer;

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn fixture_path(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{stem}.description.md"))
}

/// Every `examples/*.json` stem, sorted.
fn example_stems() -> Vec<String> {
    let mut stems: Vec<String> = fs::read_dir(repository_path("examples"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    stems.sort();
    stems
}

fn example_page(stem: &str) -> Page {
    let text = fs::read_to_string(repository_path(&format!("examples/{stem}.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn grammar_of(page: &Page) -> Grammar {
    builtin_grammar(grammar_reference(page)).unwrap().unwrap()
}

fn laid_out(page: &Page) -> (Grammar, PageGeometry) {
    let grammar = grammar_of(page);
    let mut measurer = CosmicTextMeasurer::new().unwrap();
    let geometry = layout_page(page, &grammar, &mut measurer).unwrap();
    (grammar, geometry)
}

fn description_of(page: &Page) -> (PageGeometry, String) {
    let (grammar, geometry) = laid_out(page);
    let text = describe(page, &grammar, &geometry).text;
    (geometry, text)
}

fn g7() -> (Page, PageGeometry, String) {
    let page = example_page("g7");
    let (geometry, text) = description_of(&page);
    (page, geometry, text)
}

fn messages(report: &CheckReport) -> Vec<(String, String)> {
    report
        .defects
        .iter()
        .map(|defect| (defect.pointer.as_str().to_string(), defect.message.clone()))
        .collect()
}

#[test]
fn every_example_writes_its_description_byte_for_byte() {
    let stems = example_stems();
    let mut examined = 0;
    for stem in &stems {
        let path = fixture_path(stem);
        assert!(path.exists(), "{stem} has no description fixture");
        let (_, text) = description_of(&example_page(stem));
        assert_eq!(
            text,
            fs::read_to_string(&path).unwrap(),
            "{stem}: the description changed"
        );
        examined += 1;
    }
    for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")).unwrap()
    {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if let Some(stem) = name.strip_suffix(".description.md") {
            assert!(
                stems.iter().any(|known| known == stem),
                "fixture {name} has no example"
            );
        }
    }
    assert_eq!(examined, stems.len());
    assert!(examined >= 13, "examined {examined} examples");
}

#[test]
fn the_description_is_identical_in_every_theme() {
    let mut examined = 0;
    for stem in example_stems() {
        let mut projections = 0;
        for projection in [Projection::Flat, Projection::Iso] {
            let mut page = example_page(&stem);
            page.projection = projection;
            let grammar = grammar_of(&page);
            if !validate_page(&page, &grammar).is_empty() {
                continue;
            }
            let (grammar, geometry) = laid_out(&page);
            let expected = describe(&page, &grammar, &geometry).text;
            for theme in BUILTIN_THEMES {
                let mut themed = page.clone();
                themed.theme = Some(theme.to_string());
                assert_eq!(
                    describe(&themed, &grammar, &geometry).text,
                    expected,
                    "{stem} under {theme} in {projection:?}"
                );
                examined += 1;
            }
            projections += 1;
        }
        assert!(projections > 0, "{stem}: no projection vets");
    }
    assert!(examined >= 13 * BUILTIN_THEMES.len(), "examined {examined}");
}

#[test]
fn the_iso_figure_line_gives_the_layout_size() {
    let (_, text) = description_of(&example_page("hero-iso"));
    let paragraph = text
        .lines()
        .skip_while(|line| *line != "## Figure")
        .nth(2)
        .unwrap();
    assert!(
        paragraph.contains("Laid out at 1640 x 728.74 px, projection iso, grammar gcp;"),
        "{paragraph}"
    );
    assert!(
        paragraph.contains("the projected drawing is larger"),
        "{paragraph}"
    );
}

/// The strings that open a block, an inline or a span when Markdown reads them raw.
const HAZARDS: [&str; 23] = [
    "~~~",
    "~x~",
    "&amp;",
    "&#35;",
    "&copy;",
    "===",
    "---",
    "- x",
    "+ x",
    "1. x",
    "1) x",
    "> x",
    "# x",
    "`/links/0`",
    "`B3`",
    "Marks: x",
    "Ref A1",
    "a | b",
    "x; y",
    "[x](y)",
    "<div>",
    "\\`",
    "\\\\`",
];
const PUNCTUATION: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

/// The hazards in order from `start`, then the punctuation run, so every hazard opens some
/// field.
fn hazard_text(start: usize) -> String {
    let mut parts: Vec<&str> = (0..HAZARDS.len())
        .map(|offset| HAZARDS[(start + offset) % HAZARDS.len()])
        .collect();
    parts.push(PUNCTUATION);
    parts.join(" ")
}

/// Replaces every text field of `document` with a hazard string, the k-th field opening
/// with hazard `k + offset`, and returns the strings in the order they were written.
fn fill_with_hazards(
    value: &mut Value,
    offset: usize,
    written: &mut Vec<String>,
    key: Option<&str>,
) {
    // A sheet's wordmark and captions hold at most 24 and 60 characters, shorter than a
    // hazard string, so they keep their text.
    const TEXT_KEYS: [&str; 16] = [
        "title",
        "kicker",
        "lede",
        "foot",
        "badge",
        "label",
        "sub",
        "subtitle",
        "text",
        "hub",
        "heading",
        "lead",
        "statement",
        "note",
        "why",
        "detail",
    ];
    match value {
        Value::String(text) if key.is_some_and(|key| TEXT_KEYS.contains(&key)) => {
            *text = hazard_text(written.len() + offset);
            written.push(text.clone());
        }
        Value::Array(items) if key == Some("body") && items.iter().all(Value::is_string) => {
            for item in items {
                let replacement = hazard_text(written.len() + offset);
                *item = Value::String(replacement.clone());
                written.push(replacement);
            }
        }
        Value::Array(items) => {
            for item in items {
                fill_with_hazards(item, offset, written, None);
            }
        }
        Value::Object(fields) => {
            for (field, item) in fields.iter_mut() {
                // A mark holds the id of its key, not text.
                if field == "marks" || field == "corner_mark" {
                    continue;
                }
                fill_with_hazards(item, offset, written, Some(field.as_str()));
            }
        }
        _ => {}
    }
}

/// The plain text of every leaf block (paragraph, heading, list item text, table cell), and
/// every inline the writer must never produce.
fn blocks_and_stray_inlines(text: &str) -> (Vec<String>, Vec<String>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let mut blocks = Vec::new();
    let mut stray = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, blocks: &mut Vec<String>| {
        if !current.is_empty() {
            blocks.push(std::mem::take(current));
        }
    };
    for event in Parser::new_ext(text, options) {
        match event {
            Event::Text(text) => current.push_str(&text),
            Event::Code(code) => {
                if !code.starts_with('/') && !is_reference(&code) {
                    stray.push(format!("code span {code}"));
                }
                current.push('`');
                current.push_str(&code);
                current.push('`');
            }
            Event::Start(Tag::List(_)) | Event::End(TagEnd::Paragraph | TagEnd::Heading(_)) => {
                flush(&mut current, &mut blocks);
            }
            Event::End(TagEnd::TableCell | TagEnd::Item) => flush(&mut current, &mut blocks),
            Event::Start(
                tag @ (Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough
                | Tag::Link { .. }
                | Tag::Image { .. }
                | Tag::HtmlBlock
                | Tag::BlockQuote(_)
                | Tag::CodeBlock(_)),
            ) => stray.push(format!("{tag:?}")),
            Event::Html(html) | Event::InlineHtml(html) => stray.push(format!("html {html}")),
            Event::Rule => stray.push("thematic break".to_string()),
            _ => {}
        }
    }
    flush(&mut current, &mut blocks);
    (blocks, stray)
}

/// A grid reference the writer puts before a pointer on a sheet: `B3` or `B2-C5`.
fn is_reference(text: &str) -> bool {
    let cell = |cell: &str| {
        let mut characters = cell.chars();
        matches!(
            (characters.next(), characters.next(), characters.next()),
            (Some('A'..='F'), Some('1'..='8'), None)
        )
    };
    match text.split_once('-') {
        Some((first, last)) => cell(first) && cell(last),
        None => cell(text),
    }
}

/// Every hazard opens every text field once over the offsets, under both chromes; a design
/// sheet draws its head in the title block, so it is examined under `chrome: full` only.
#[test]
fn every_markdown_hazard_in_every_text_field_reads_back() {
    let mut examined = 0;
    for stem in example_stems() {
        let is_sheet = example_page(&stem).sheet.is_some();
        for (chrome, offset) in ["full", "none"]
            .into_iter()
            .filter(|chrome| !(is_sheet && *chrome == "none"))
            .flat_map(|chrome| (0..HAZARDS.len()).map(move |offset| (chrome, offset)))
        {
            let mut document: Value = serde_json::to_value(example_page(&stem)).unwrap();
            document["chrome"] = Value::String(chrome.to_string());
            let original: Page = serde_json::from_value(document.clone()).unwrap();
            let grammar = grammar_of(&original);
            let mut measurer = FixedMetricsMeasurer::default();
            let original_geometry = layout_page(&original, &grammar, &mut measurer).unwrap();
            let mut written = Vec::new();
            fill_with_hazards(&mut document, offset, &mut written, None);
            let page: Page = serde_json::from_value(document).unwrap();
            let violations = validate_page(&page, &grammar);
            assert!(violations.is_empty(), "{stem} {chrome}: {violations:?}");
            let geometry = layout_page(&page, &grammar, &mut measurer).unwrap();
            let text = describe(&page, &grammar, &geometry).text;

            let (blocks, stray) = blocks_and_stray_inlines(&text);
            assert!(stray.is_empty(), "{stem} {chrome}: {stray:?}\n{text}");
            let headings = text.lines().filter(|line| line.starts_with('#')).count();
            let writer_headings = text
                .lines()
                .filter(|line| {
                    line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ")
                })
                .count();
            assert_eq!(headings, writer_headings, "{stem} {chrome}");
            for field in &written {
                assert!(
                    blocks.iter().any(|block| block.contains(field.as_str())),
                    "{stem} {chrome}: {field:?} does not read back\n{text}"
                );
            }

            let report = description_covers_drawing(&page, &geometry, &text);
            assert_eq!(
                messages(&report),
                Vec::<(String, String)>::new(),
                "{stem} {chrome}"
            );
            let original_units = description_units(&original, &original_geometry).units.len();
            assert_eq!(report.examined as usize, original_units, "{stem} {chrome}");
            examined += 1;
        }
    }
    assert!(examined >= 26 * HAZARDS.len(), "examined {examined}");
}

/// The rows of the Legend table: (Key cell, pointer). The table ends at the next heading,
/// and on a sheet a reference span comes before the pointer span.
fn legend_rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .skip_while(|line| !line.starts_with("## Legend"))
        .skip(1)
        .take_while(|line| !line.starts_with('#'))
        .filter(|line| line.starts_with("| ") && line.ends_with("` |"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split(" | ").collect();
            let key = cells[0].trim().to_string();
            let pointer = cells
                .last()
                .unwrap()
                .split('`')
                .find(|span| span.starts_with('/'))
                .unwrap()
                .to_string();
            (key, pointer)
        })
        .collect()
}

#[test]
fn the_legend_key_is_the_measured_kind_for_tube_entries() {
    let mut examined = 0;
    for stem in example_stems() {
        let page = example_page(&stem);
        let (geometry, _) = description_of(&page);
        let fixture = fs::read_to_string(fixture_path(&stem)).unwrap();
        let rows = legend_rows(&fixture);
        assert_eq!(rows.len(), page.legend.len(), "{stem}");
        let references: Vec<(String, NodeRef<'_>)> = body_nodes(&page)
            .into_iter()
            .map(|entry| (entry.pointer.as_str().to_string(), entry.node))
            .collect();
        let actors = PageActors::of(&page);
        for (key_cell, pointer) in rows {
            let index: usize = pointer.strip_prefix("/legend/").unwrap().parse().unwrap();
            let entry = &page.legend[index];
            let key = entry_key(entry);
            let unplanned = key_cell.strip_suffix("-planned").unwrap_or(&key_cell);
            let slot_key = unplanned
                .strip_suffix("-band")
                .or_else(|| unplanned.strip_suffix("-rail"))
                .unwrap_or(unplanned);
            // A by-actor Key names a line and no slot; each use it counts is measured with
            // its own actor's slot.
            if key.colour == KeyColour::ByActor {
                assert_eq!(
                    slot_key,
                    format!("{}-by-actor", entry.line.as_str()),
                    "{stem}"
                );
            }
            let use_key = |line, tint, form, actor: Option<&str>| match actors.slot(actor) {
                Some(_) => LegendKey::by_actor(line, form),
                None => LegendKey::new(line, tint, form),
            };
            let mut uses = 0;
            let mut tee = None;
            for node in &geometry.nodes {
                let Some((_, reference)) = references
                    .iter()
                    .find(|(known, _)| known == node.pointer.as_str())
                else {
                    continue;
                };
                let used_key = match *reference {
                    NodeRef::Node(Node::Pipe(pipe)) => {
                        use_key(pipe.line, pipe.tint, pipe.form, actors.pipe_actor(pipe))
                            .with_planned(pipe.planned)
                    }
                    NodeRef::TeeArm(pipe) => use_key(
                        pipe.line,
                        pipe.tint,
                        pipe.form,
                        tee.and_then(|tee| actors.arm_actor(tee, pipe)),
                    )
                    .with_planned(pipe.planned),
                    NodeRef::Node(Node::Tee(tee_node)) => {
                        tee = Some(tee_node);
                        use_key(
                            tee_node.line,
                            tee_node.tint,
                            PipeForm::Tube,
                            actors.tee_actor(tee_node),
                        )
                    }
                    _ => continue,
                };
                if used_key == key {
                    let measured = match key.colour {
                        KeyColour::ByActor => line_key(used_key.line, node.tint),
                        KeyColour::Slot(_) => slot_key,
                    };
                    assert_eq!(node.kind.as_deref(), Some(measured), "{stem} {pointer}");
                    uses += 1;
                }
            }
            for route in &geometry.links {
                let link = &page.links[route.index];
                if use_key(
                    link.line,
                    link.tint,
                    PipeForm::Tube,
                    actors.link_actor(link),
                )
                .with_planned(link.planned)
                    == key
                {
                    let measured = match key.colour {
                        KeyColour::ByActor => line_key(link.line, route.tint),
                        KeyColour::Slot(_) => slot_key,
                    };
                    assert_eq!(route.key(), measured, "{stem} {pointer}");
                    uses += 1;
                }
            }
            assert!(uses > 0, "{stem}: {pointer} counts no line");
            examined += 1;
        }
    }
    assert!(examined >= 13, "examined {examined}");
}

#[test]
fn the_check_passes_on_every_example_and_counts_every_unit() {
    for stem in example_stems() {
        let page = example_page(&stem);
        let (geometry, text) = description_of(&page);
        let report = description_covers_drawing(&page, &geometry, &text);
        assert_eq!(report.check, CheckName::DescriptionCoversDrawing);
        assert_eq!(report.outcome(), CheckOutcome::Passed, "{stem}: {report:?}");
        assert!(
            text.contains(&format!("Drawn elements: {} (", report.examined)),
            "{stem}"
        );
    }
}

#[test]
fn a_pointer_to_nothing_drawn_is_a_defect_and_a_row_pointer_is_not() {
    let (page, geometry, text) = g7();
    assert!(text.contains("- Row, left to right. `/body/0`"));
    let clean = description_covers_drawing(&page, &geometry, &text);
    assert!(clean.defects.is_empty(), "{:?}", clean.defects);

    let line_count = text.lines().count();
    let extra = format!("{text}\nSee `/body/0/children/7` and `/links/0` and `B3`.\n");
    let report = description_covers_drawing(&page, &geometry, &extra);
    let line = line_count + 2;
    assert_eq!(
        messages(&report),
        vec![
            (
                String::new(),
                format!("line {line} names /body/0/children/7, which draws nothing")
            ),
            (
                String::new(),
                format!("line {line} names /links/0, which draws nothing")
            ),
            (
                String::new(),
                format!("line {line} has span B3, neither a pointer nor a reference")
            ),
        ]
    );
    assert_eq!(report.examined, clean.examined);
}

#[test]
fn a_reference_off_a_sheet_is_a_defect() {
    let (page, geometry, text) = g7();
    let referenced = text.replace("`/body/0/children/2`", "`C4` `/body/0/children/2`");
    let report = description_covers_drawing(&page, &geometry, &referenced);
    let line = text
        .lines()
        .position(|line| line.contains("`/body/0/children/2`"))
        .unwrap()
        + 1;
    assert_eq!(
        messages(&report),
        vec![(
            "/body/0/children/2".to_string(),
            format!("line {line} gives C4 where no reference belongs")
        )]
    );
}

#[test]
fn a_unit_on_no_line_or_two_lines_is_a_defect() {
    let (page, geometry, text) = g7();
    let fact = "/body/0/children/2/children/0/children/0/children/1";
    let fact_line = text
        .lines()
        .find(|line| line.contains(&format!("`{fact}`")))
        .unwrap();

    let dropped: String = text
        .lines()
        .filter(|line| *line != fact_line)
        .map(|line| format!("{line}\n"))
        .collect();
    let report = description_covers_drawing(&page, &geometry, &dropped);
    assert_eq!(
        messages(&report),
        vec![(fact.to_string(), "not described".to_string())]
    );

    let doubled = format!("{text}\n{fact_line}\n");
    let report = description_covers_drawing(&page, &geometry, &doubled);
    assert_eq!(
        messages(&report),
        vec![(fact.to_string(), "described on 2 lines".to_string())]
    );

    let row = format!("{text}\n- Row, left to right. `/body/0`\n");
    let report = description_covers_drawing(&page, &geometry, &row);
    assert_eq!(
        messages(&report),
        vec![("/body/0".to_string(), "described on 2 lines".to_string())]
    );
}

#[test]
fn a_line_without_its_units_name_is_a_defect() {
    let (page, geometry, text) = g7();
    let renamed = text.replace("Item Cloud Router A:", "Item Router A:");
    let line = text
        .lines()
        .position(|line| line.contains("Item Cloud Router A:"))
        .unwrap()
        + 1;
    let report = description_covers_drawing(&page, &geometry, &renamed);
    assert_eq!(
        messages(&report),
        vec![(
            "/body/0/children/2/children/0/children/0/children/0".to_string(),
            format!("line {line} lacks \"Cloud Router A\"")
        )]
    );
}

#[test]
fn a_figure_count_that_differs_from_examined_is_a_defect() {
    let (page, geometry, text) = g7();
    let miscounted = text.replace("Drawn elements: 26 (", "Drawn elements: 25 (");
    let report = description_covers_drawing(&page, &geometry, &miscounted);
    assert_eq!(
        messages(&report),
        vec![(
            String::new(),
            "the figure paragraph counts 25, examined 26".to_string()
        )]
    );

    let unheaded = text.replace("## Figure", "## Picture");
    let report = description_covers_drawing(&page, &geometry, &unheaded);
    assert_eq!(
        messages(&report),
        vec![(String::new(), "no figure paragraph".to_string())]
    );
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn a_writer_that_skips_a_tag_fails_the_check() {
    let (page, geometry, text) = g7();
    let item = "/body/0/children/0/children/0/children/1";
    let skipped: String = text
        .lines()
        .filter(|line| !(line.contains("- Item ") && line.contains(&format!("`{item}`"))))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(skipped.lines().count() + 1, text.lines().count());
    let report = description_covers_drawing(&page, &geometry, &skipped);
    assert_eq!(
        messages(&report),
        vec![(item.to_string(), "not described".to_string())]
    );
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn an_empty_description_fails_with_every_unit_undescribed() {
    let (page, geometry, _) = g7();
    let report = description_covers_drawing(&page, &geometry, "");
    assert_eq!(report.examined, 26);
    assert_eq!(report.defects.len(), 27);
    assert_eq!(report.outcome(), CheckOutcome::Failed);
}

#[test]
fn description_units_cover_every_node_that_draws_a_part() {
    let mut examined = 0;
    for stem in example_stems() {
        let page = example_page(&stem);
        let (_, geometry) = laid_out(&page);
        let units = description_units(&page, &geometry);
        let drawing: Vec<&str> = geometry
            .nodes
            .iter()
            .filter(|node| node.parent.is_some() && !node.parts.is_empty())
            .map(|node| node.pointer.as_str())
            .collect();
        let partless: Vec<&str> = geometry
            .nodes
            .iter()
            .filter(|node| node.parent.is_some() && node.parts.is_empty())
            .map(|node| node.pointer.as_str())
            .collect();
        let node_units: Vec<&str> = units
            .units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Node)
            .map(|unit| unit.pointer.as_str())
            .collect();
        let link_units = units
            .units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Link)
            .count();
        assert_eq!(node_units, drawing, "{stem}");
        assert_eq!(link_units, geometry.links.len(), "{stem}");
        let structural: Vec<&str> = units
            .structural
            .iter()
            .map(|pointer| pointer.as_str())
            .collect();
        assert_eq!(structural, partless, "{stem}");
        assert!(
            geometry
                .nodes
                .iter()
                .filter(|node| node.parts.is_empty() && node.parent.is_some())
                .all(|node| matches!(
                    node.tag,
                    NodeTag::Body | NodeTag::Legend | NodeTag::Row | NodeTag::Col
                )),
            "{stem}: a partless node that is not Body, Legend, Row or Col"
        );
        examined += 1;
    }
    assert!(examined >= 13);
}

fn onepager() -> (Page, PageGeometry, String) {
    let page = example_page("onepager");
    let (geometry, text) = description_of(&page);
    (page, geometry, text)
}

/// The drawn margin labels and the grid references name the same cells: a column number
/// drawn over a column is the column `sheet_zone_at` gives under it, and a row letter the
/// row; a box across cells reads first and last cell, letter first.
#[test]
fn a_description_reference_names_the_cell_drawn_around_its_point() {
    use stencil_layout::{BoxRect, PartName, sheet_frame, sheet_zone, sheet_zone_at};
    use stencil_model::PagePoint;
    let (_, geometry, _) = onepager();
    let frame = sheet_frame(&geometry).unwrap();
    let page = &geometry.nodes[0];
    let mut examined = 0;
    for part in &page.parts {
        let Some(run) = &part.text else {
            continue;
        };
        let center_x = part.bounds.x + part.bounds.width / 2.0;
        let center_y = part.bounds.y + part.bounds.height / 2.0;
        let (point, expected) = match part.name {
            PartName::RefColumn | PartName::RefColumnFoot => (
                PagePoint {
                    x: center_x,
                    y: frame.y + 1.0,
                },
                format!("A{}", run.text),
            ),
            PartName::RefRow | PartName::RefRowRight => (
                PagePoint {
                    x: frame.x + 1.0,
                    y: center_y,
                },
                format!("{}1", run.text),
            ),
            _ => continue,
        };
        assert_eq!(sheet_zone_at(point, frame).to_string(), expected);
        examined += 1;
    }
    assert_eq!(examined, 2 * 8 + 2 * 6);
    let cell_width = frame.width / 8.0;
    let row_height = frame.height / 6.0;
    let spanning = BoxRect {
        x: frame.x + cell_width * 1.5,
        y: frame.y + row_height * 1.5,
        width: cell_width * 3.0,
        height: row_height * 1.2,
    };
    assert_eq!(sheet_zone(spanning, frame).to_string(), "B2-C5");
    let edge = BoxRect {
        x: frame.x,
        y: frame.y,
        width: cell_width,
        height: row_height,
    };
    assert_eq!(sheet_zone(edge, frame).to_string(), "A1");
}

/// Each part `ENTRY_PARTS` names is drawn once per entry on its owner, so the i-th
/// occurrence of each is the i-th entry's.
#[test]
fn every_entry_part_is_drawn_once_per_entry() {
    use stencil_layout::ENTRY_PARTS;
    use stencil_model::pointer::NodePointer;
    let (page, geometry, _) = onepager();
    let sheet = page.sheet.as_ref().unwrap();
    let mut examined = 0;
    let phases_pointer = NodePointer::root()
        .child("body")
        .index(0)
        .child("children")
        .index(0);
    let Some(stencil_model::Node::View(view)) = page.body.first() else {
        panic!("onepager opens with a View");
    };
    let Some(stencil_model::Node::Phases(phases)) = view.children.first() else {
        panic!("the Workflow view opens with a Phases node");
    };
    for (list, names) in ENTRY_PARTS {
        let (owner, entries) = match list {
            stencil_model::SheetList::AttentionItems => (
                NodePointer::root().child("sheet").child("attention"),
                sheet.attention.items.len(),
            ),
            stencil_model::SheetList::Lanes => (phases_pointer.clone(), phases.lanes.len()),
            stencil_model::SheetList::Phases => (phases_pointer.clone(), phases.phases.len()),
            stencil_model::SheetList::Parts | stencil_model::SheetList::GeneralNotes => {
                let (tag, field) = match list {
                    stencil_model::SheetList::Parts => {
                        (stencil_layout::NodeTag::PartsList, "parts")
                    }
                    _ => (stencil_layout::NodeTag::GeneralNotes, "notes"),
                };
                let owner = geometry
                    .nodes
                    .iter()
                    .find(|node| node.tag == tag)
                    .unwrap()
                    .pointer
                    .clone();
                let document = serde_json::to_value(&page).unwrap();
                let entries = document
                    .pointer(owner.child(field).as_str())
                    .and_then(serde_json::Value::as_array)
                    .unwrap()
                    .len();
                (owner, entries)
            }
            stencil_model::SheetList::ProblemPoints
            | stencil_model::SheetList::AnswerParagraphs
            | stencil_model::SheetList::Actors
            | stencil_model::SheetList::Steps
            | stencil_model::SheetList::StepLines
            | stencil_model::SheetList::Marks
            | stencil_model::SheetList::Compartments => {
                panic!("{} keys no entry parts", list.name())
            }
        };
        let node = geometry.node(&owner).unwrap();
        for name in names {
            let drawn = node.parts.iter().filter(|part| part.name == *name).count();
            assert_eq!(drawn, entries, "{}", name.as_str());
            examined += 1;
        }
    }
    assert_eq!(examined, 10);
}

/// A reference that names another cell than the one the element is drawn in, a missing
/// reference on a sheet, and a reference on a structural pointer are each a defect.
#[test]
fn a_wrong_reference_is_a_defect() {
    let (page, geometry, text) = onepager();
    let report = description_covers_drawing(&page, &geometry, &text);
    assert!(report.defects.is_empty(), "{:?}", report.defects);

    let point = "`/sheet/problem/points/0`";
    let line = text.lines().find(|line| line.contains(point)).unwrap();
    let number = text
        .lines()
        .position(|candidate| candidate == line)
        .unwrap()
        + 1;
    // The reference span right before the point's pointer; marks' spans come earlier.
    let before = line.split(point).next().unwrap().trim_end();
    let drawn = before
        .trim_end_matches('`')
        .rsplit('`')
        .next()
        .unwrap()
        .to_string();
    let wrong = if drawn == "F8" { "A1" } else { "F8" };
    let moved = text.replace(&format!("`{drawn}` {point}"), &format!("`{wrong}` {point}"));
    let report = description_covers_drawing(&page, &geometry, &moved);
    assert_eq!(
        messages(&report),
        [(
            "/sheet/problem/points/0".to_string(),
            format!("line {number} gives {wrong}, drawn at {drawn}")
        )]
    );

    let missing = text.replace(&format!("`{drawn}` {point}"), point);
    let report = description_covers_drawing(&page, &geometry, &missing);
    assert_eq!(
        messages(&report),
        [(
            "/sheet/problem/points/0".to_string(),
            format!("line {number} gives no reference, drawn at {drawn}")
        )]
    );

    let row = "Row, left to right. `/body/1/children/0`";
    let row_number = text.lines().position(|line| line.ends_with(row)).unwrap() + 1;
    let structural = text.replace(row, "Row, left to right. `C1` `/body/1/children/0`");
    let report = description_covers_drawing(&page, &geometry, &structural);
    assert_eq!(
        messages(&report),
        [(
            "/body/1/children/0".to_string(),
            format!("line {row_number} gives C1 where no reference belongs")
        )]
    );
}

/// The Phases line names a band other than the wash and a `step_fill`; a Step line names
/// its fill when the Step sets one or takes one other than its look's, and never on a text
/// look. With neither field set no fill is named.
#[test]
fn the_description_names_a_lane_band_and_a_step_fill_that_differ_from_the_look() {
    let (mut page, _, _) = onepager();
    let Some(Node::View(view)) = page.body.first_mut() else {
        panic!("onepager opens with a View");
    };
    let Some(Node::Phases(phases)) = view.children.first_mut() else {
        panic!("the Workflow view opens with a Phases node");
    };
    phases.band = stencil_model::BandLook::Wash;
    phases.step_fill = None;
    for child in &mut phases.children {
        if let Node::Step(step) = child {
            step.fill = None;
        }
    }
    let default_page = page.clone();
    let (_, default_text) = description_of(&default_page);
    assert!(!default_text.contains("band "), "{default_text}");
    assert!(!default_text.contains(", fill "), "{default_text}");
    assert!(!default_text.contains("step fill"), "{default_text}");

    let Some(Node::View(view)) = page.body.first_mut() else {
        panic!("onepager opens with a View");
    };
    let Some(Node::Phases(phases)) = view.children.first_mut() else {
        panic!("the Workflow view opens with a Phases node");
    };
    phases.band = stencil_model::BandLook::None;
    phases.step_fill = Some(stencil_model::StepFill::None);
    let mut chip_title = String::new();
    for child in &mut phases.children {
        if let Node::Step(step) = child
            && step.look == stencil_model::StepLook::Chip
        {
            step.fill = Some(stencil_model::StepFill::Tint);
            chip_title = step.title.clone();
        }
    }
    let (_, text) = description_of(&page);
    let phases_line = text
        .lines()
        .find(|line| line.starts_with("- Phases: "))
        .unwrap();
    assert!(
        phases_line.contains("; band none; step fill none. `"),
        "{phases_line}"
    );
    let step_lines: Vec<&str> = text
        .lines()
        .filter(|line| line.trim_start().starts_with("- Step "))
        .collect();
    assert_eq!(step_lines.len(), 12);
    for line in step_lines {
        // A text look takes no fill, and none is a note's and a planned Step's own.
        if line.contains(", look text")
            || line.contains(", look note")
            || line.contains(", look planned")
        {
            assert!(!line.contains(", fill "), "{line}");
        } else if line.contains(&chip_title) {
            assert!(line.contains(", look chip, fill tint. `"), "{line}");
        } else {
            assert!(line.contains(", fill none. `"), "{line}");
        }
    }
}

/// Section 14.6: a drawn where-used text that differs from its key's glyph cells, and a
/// carrier that draws fewer glyphs than it has marks, are defects of the description check.
#[test]
fn a_where_used_text_or_a_missing_glyph_is_a_defect() {
    use stencil_layout::PartName;
    let (page, mut geometry, text) = onepager();
    let notes = geometry
        .nodes
        .iter_mut()
        .find(|node| node.tag == stencil_layout::NodeTag::GeneralNotes)
        .unwrap();
    let pointer = notes.pointer.clone();
    let zones = notes
        .parts
        .iter_mut()
        .find(|part| part.name == PartName::NoteZones)
        .unwrap();
    let drawn = zones.text.as_ref().unwrap().text.clone();
    zones.text.as_mut().unwrap().text = "marked at A1".to_string();
    let report = description_covers_drawing(&page, &geometry, &text);
    assert_eq!(
        messages(&report),
        [(
            format!("{}/notes/0", pointer.as_str()),
            format!("where-used reads \"marked at A1\", marks at {drawn}")
        )]
    );

    let (page, mut geometry, text) = onepager();
    let point = geometry
        .nodes
        .iter_mut()
        .find(|node| node.pointer.as_str() == "/sheet/problem/points/0")
        .unwrap();
    let last = point
        .parts
        .iter()
        .rposition(|part| part.name.is_mark_glyph())
        .unwrap();
    point.parts.remove(last);
    let report = description_covers_drawing(&page, &geometry, &text);
    let found = messages(&report);
    assert!(
        found.contains(&(
            "/sheet/problem/points/0".to_string(),
            "draws 1 mark glyphs for 2 marks".to_string()
        )),
        "{found:?}"
    );
}
