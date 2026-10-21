#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use stencil_cli::pipeline::{all_checks, load_document, read_input, render_page};
use stencil_cli::prime::{
    BASE_BYTES_MAX, INLINE_DEFINITIONS, ROW_DEFINITIONS, TOPIC_BYTES_MAX, TOPIC_ROW_DEFINITIONS,
    Topic, base_text, field_notes,
};
use stencil_cli::{ExitCode, run};
use stencil_model::{
    Arrow, CalloutKind, Chrome, FactSource, IconName, Justify, Line, ListKind, PipeForm, Shape,
    SheetSurface, Side, SurfaceVocabulary, builtin_grammar, page_schema,
};
use stencil_render::DeviceScale;

struct Outcome {
    code: ExitCode,
    stdout: String,
    stderr: String,
}

fn run_stencil(arguments: &[&str]) -> Outcome {
    let mut argv: Vec<OsString> = vec!["stencil".into()];
    argv.extend(arguments.iter().map(OsString::from));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run(argv, &mut stdout, &mut stderr);
    Outcome {
        code,
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

fn g7_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/g7.json")
}

fn base_output() -> String {
    let outcome = run_stencil(&["prime"]);
    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    assert_eq!(outcome.stderr, "");
    outcome.stdout
}

/// Node tags as the schema generated from `Node` names them.
fn node_tags() -> BTreeSet<String> {
    let schema = page_schema();
    schema.as_value()["$defs"]["Node"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| {
            variant["properties"]["tag"]["const"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

/// Everything a vocabulary table may name in its first column, in base or in a topic.
fn table_objects() -> BTreeSet<String> {
    let mut objects = node_tags();
    for name in ["Page", "FactEntry", "LegendEntry", "Link"] {
        objects.insert(name.to_string());
    }
    for name in TOPIC_ROW_DEFINITIONS {
        objects.insert(name.to_string());
    }
    objects
}

/// The node tags a design sheet surface moves out of the base vocabulary.
fn sheet_tags() -> BTreeSet<String> {
    SheetSurface::ALL
        .iter()
        .flat_map(|surface| surface.vocabulary().iter())
        .filter_map(|item| match item {
            SurfaceVocabulary::Tag(tag) => Some((*tag).to_string()),
            SurfaceVocabulary::Field { .. }
            | SurfaceVocabulary::Value { .. }
            | SurfaceVocabulary::Definition(_) => None,
        })
        .collect()
}

fn topic_text(topic: Topic) -> String {
    topic.rendered().unwrap()
}

/// First-column names of the vocabulary table, split at ", ".
fn vocabulary_row_names(text: &str) -> Vec<String> {
    let mut lines = text.lines().skip_while(|line| *line != "| Tag | Fields |");
    assert!(lines.next().is_some(), "no vocabulary table");
    let mut names = Vec::new();
    for line in lines.skip(1) {
        if !line.starts_with("| ") {
            break;
        }
        let cell = line.trim_start_matches("| ").split(" |").next().unwrap();
        names.extend(cell.split(", ").map(str::to_string));
    }
    names
}

fn serialized<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

/// The values printed on the `<name> (<n>): v1 v2 ...` line of the base output.
fn enum_line_values(text: &str, name: &str) -> (usize, Vec<String>) {
    let prefix = format!("{name} (");
    let line = text
        .lines()
        .find(|line| line.starts_with(&prefix) || line.contains(&format!(" {prefix}")))
        .unwrap_or_else(|| panic!("no enum line for {name}"));
    let start = line.find(&prefix).unwrap() + prefix.len();
    let (count, values) = line[start..].split_once("): ").unwrap();
    let values = values.split('.').next().unwrap();
    (
        count.parse().unwrap(),
        values.split_whitespace().map(str::to_string).collect(),
    )
}

fn assert_enum_listed(text: &str, name: &str, expected: Vec<String>) {
    let (count, values) = enum_line_values(text, name);
    assert_eq!(count, expected.len(), "{name} count");
    assert_eq!(values, expected, "{name} values");
}

// Each list below passes its variants through an exhaustive match, so a new variant does not
// compile until it is listed here and therefore checked against the briefing.

fn arrows() -> Vec<Arrow> {
    let all = [Arrow::None, Arrow::End, Arrow::Start, Arrow::Both];
    for arrow in all {
        match arrow {
            Arrow::None | Arrow::End | Arrow::Start | Arrow::Both => {}
        }
    }
    all.to_vec()
}

fn sides() -> Vec<Side> {
    let all = [Side::Top, Side::Right, Side::Bottom, Side::Left];
    for side in all {
        match side {
            Side::Top | Side::Right | Side::Bottom | Side::Left => {}
        }
    }
    all.to_vec()
}

fn shapes() -> Vec<Shape> {
    let all = [
        Shape::Card,
        Shape::Tile,
        Shape::Tower,
        Shape::Cylinder,
        Shape::Stack,
        Shape::Block,
        Shape::Figure,
        Shape::Laptop,
        Shape::Phone,
    ];
    for shape in all {
        match shape {
            Shape::Card
            | Shape::Tile
            | Shape::Tower
            | Shape::Cylinder
            | Shape::Stack
            | Shape::Block
            | Shape::Figure
            | Shape::Laptop
            | Shape::Phone => {}
        }
    }
    all.to_vec()
}

fn pipe_forms() -> Vec<PipeForm> {
    let all = [PipeForm::Tube, PipeForm::Band];
    for form in all {
        match form {
            PipeForm::Tube | PipeForm::Band => {}
        }
    }
    all.to_vec()
}

fn justifies() -> Vec<Justify> {
    let all = [
        Justify::Start,
        Justify::Center,
        Justify::End,
        Justify::SpaceBetween,
    ];
    for justify in all {
        match justify {
            Justify::Start | Justify::Center | Justify::End | Justify::SpaceBetween => {}
        }
    }
    all.to_vec()
}

fn list_kinds() -> Vec<ListKind> {
    let all = [ListKind::Plain, ListKind::Numbered, ListKind::Bulleted];
    for list in all {
        match list {
            ListKind::Plain | ListKind::Numbered | ListKind::Bulleted => {}
        }
    }
    all.to_vec()
}

fn callout_kinds() -> Vec<CalloutKind> {
    let all = [
        CalloutKind::Note,
        CalloutKind::Risk,
        CalloutKind::Decision,
        CalloutKind::Open,
    ];
    for kind in all {
        match kind {
            CalloutKind::Note | CalloutKind::Risk | CalloutKind::Decision | CalloutKind::Open => {}
        }
    }
    all.to_vec()
}

fn chromes() -> Vec<Chrome> {
    let all = [Chrome::Full, Chrome::None];
    for chrome in all {
        match chrome {
            Chrome::Full | Chrome::None => {}
        }
    }
    all.to_vec()
}

fn names<T: Serialize + Copy>(values: &[T]) -> Vec<String> {
    values.iter().map(|value| serialized(*value)).collect()
}

/// Every text `stencil prime` can print, base first.
fn all_texts() -> Vec<(String, String)> {
    let mut texts = vec![("base".to_string(), base_output())];
    for topic in Topic::ALL {
        let outcome = run_stencil(&["prime", topic.name()]);
        texts.push((topic.name().to_string(), outcome.stdout));
    }
    for name in stencil_model::BUILTIN_GRAMMARS {
        let outcome = run_stencil(&["prime", "grammar", name]);
        texts.push((format!("grammar {name}"), outcome.stdout));
    }
    texts
}

#[test]
fn the_budgets_are_7000_bytes_for_the_base_and_5000_for_each_topic() {
    assert_eq!(BASE_BYTES_MAX, 7000);
    assert_eq!(TOPIC_BYTES_MAX, 5000);
}

#[test]
fn prime_prints_the_base_briefing_within_its_byte_budget() {
    let text = base_output();
    assert!(!text.is_empty());
    assert!(
        text.len() <= BASE_BYTES_MAX,
        "base briefing is {} bytes, above {BASE_BYTES_MAX}",
        text.len()
    );
    assert_eq!(text, base_text().unwrap());
    assert!(text.ends_with('\n'));
    assert!(!text.contains("{{"), "an unfilled marker remains");
}

#[test]
fn every_listed_topic_prints_non_empty_text_within_its_budget() {
    let base = base_output();
    let topic_line = base.lines().last().unwrap();
    for topic in Topic::ALL {
        assert!(
            topic_line.contains(topic.name()),
            "last line does not list {}",
            topic.name()
        );
        let outcome = run_stencil(&["prime", topic.name()]);
        assert_eq!(outcome.code, ExitCode::Clean, "{}", topic.name());
        assert_eq!(outcome.stderr, "");
        assert!(
            !outcome.stdout.trim().is_empty(),
            "{} is empty",
            topic.name()
        );
        if topic.has_byte_budget() {
            assert!(
                outcome.stdout.len() <= TOPIC_BYTES_MAX,
                "{} is {} bytes, above {TOPIC_BYTES_MAX}",
                topic.name(),
                outcome.stdout.len()
            );
        }
        assert!(
            !outcome.stdout.contains("{{"),
            "{}: an unfilled marker remains",
            topic.name()
        );
    }
}

#[test]
fn only_the_example_topic_is_exempt_from_the_byte_budget() {
    let exempt: Vec<&str> = Topic::ALL
        .into_iter()
        .filter(|topic| !topic.has_byte_budget())
        .map(Topic::name)
        .collect();
    assert_eq!(exempt, vec!["example"]);
}

#[test]
fn the_example_topic_prints_g7_verbatim() {
    let outcome = run_stencil(&["prime", "example"]);
    assert_eq!(outcome.code, ExitCode::Clean);
    assert_eq!(outcome.stdout, fs::read_to_string(g7_path()).unwrap());
}

#[test]
fn an_unknown_topic_prints_one_line_listing_the_topics_and_exits_2() {
    let outcome = run_stencil(&["prime", "colours"]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert_eq!(outcome.stderr.lines().count(), 1, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("\"colours\""), "{}", outcome.stderr);
    for topic in Topic::ALL {
        assert!(outcome.stderr.contains(topic.name()), "{}", outcome.stderr);
    }
}

/// Every tag a sheet surface does not name has a base row; every sheet tag has a row in
/// the sheet topic instead.
#[test]
fn the_base_briefing_names_every_node_tag_in_its_vocabulary() {
    let text = base_output();
    let rows: BTreeSet<String> = vocabulary_row_names(&text).into_iter().collect();
    let sheet = sheet_tags();
    let base_tags: BTreeSet<String> = node_tags().difference(&sheet).cloned().collect();
    for tag in &base_tags {
        assert!(rows.contains(tag), "vocabulary has no row for {tag}");
    }
    let topic_rows: BTreeSet<String> = vocabulary_row_names(&topic_text(Topic::Sheet))
        .into_iter()
        .collect();
    for tag in &sheet {
        assert!(
            !rows.contains(tag),
            "base has a row for the sheet tag {tag}"
        );
        assert!(
            topic_rows.contains(tag),
            "the sheet topic has no row for {tag}"
        );
    }
    for object in ["Page", "FactEntry", "LegendEntry", "Link"] {
        assert!(rows.contains(object), "vocabulary has no row for {object}");
    }
    assert_eq!(base_tags.len(), 12);
}

/// Section 14.8 rule 3: every name a sheet surface moves is absent from base and present
/// in the rendered topic the surface names.
#[test]
fn every_sheet_surface_is_absent_from_base_and_named_in_its_topic() {
    let base = base_output();
    let base_rows: BTreeSet<String> = vocabulary_row_names(&base).into_iter().collect();
    let mut examined = 0;
    for surface in SheetSurface::ALL {
        let topic = Topic::from_name(surface.topic()).unwrap();
        let text = topic_text(topic);
        let topic_rows: BTreeSet<String> = vocabulary_row_names(&text).into_iter().collect();
        for item in surface.vocabulary() {
            match *item {
                SurfaceVocabulary::Tag(tag) => {
                    assert!(!base_rows.contains(tag), "base has a row for {tag}");
                    assert!(
                        topic_rows.contains(tag),
                        "{} has no row for {tag}",
                        topic.name()
                    );
                }
                SurfaceVocabulary::Field { object, field } => {
                    let base_row = base
                        .lines()
                        .find(|line| line.starts_with(&format!("| {object} |")))
                        .unwrap_or_else(|| panic!("base has no {object} row"));
                    assert!(
                        !base_row.contains(&format!(" {field} ")),
                        "base lists {object}.{field}"
                    );
                    assert!(
                        text.lines()
                            .any(|line| line.starts_with(&format!("| {object} |"))
                                && line.contains(&format!(" {field} "))),
                        "{} does not list {object}.{field}",
                        topic.name()
                    );
                }
                SurfaceVocabulary::Value { definition, value } => {
                    let (_, values) = enum_line_values(&base, definition);
                    assert!(!values.contains(&value.to_string()), "base lists {value}");
                    assert!(text.contains(value), "{} never names {value}", topic.name());
                }
                SurfaceVocabulary::Definition(definition) => {
                    assert!(!base.contains(&format!("{definition} (")), "{definition}");
                    assert!(text.contains(definition), "{definition}");
                }
            }
            examined += 1;
        }
    }
    assert!(examined >= SheetSurface::ALL.len(), "examined {examined}");
}

/// Every `$defs` entry is printed somewhere: an enum line, the node table, a base row, a
/// topic row, or inline where a field uses it.
#[test]
fn every_schema_definition_has_a_row_a_line_or_an_inline_form() {
    let schema = page_schema();
    let definitions = schema.as_value()["$defs"].as_object().unwrap();
    let mut unprinted = Vec::new();
    for (name, definition) in definitions {
        let printed = definition.get("enum").is_some()
            || name == "Node"
            || ROW_DEFINITIONS.contains(&name.as_str())
            || TOPIC_ROW_DEFINITIONS.contains(&name.as_str())
            || INLINE_DEFINITIONS.contains(&name.as_str());
        if !printed {
            unprinted.push(name.clone());
        }
    }
    assert_eq!(unprinted, Vec::<String>::new());
    assert!(definitions.len() > 20, "examined {}", definitions.len());
}

#[test]
fn the_base_briefing_describes_the_core_vocabulary() {
    let text = base_output();
    assert!(text.contains("Every node object carries \"tag\"; the Page does not."));
    let sequence_lines: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("Sequence and timeline figures"))
        .collect();
    assert_eq!(sequence_lines.len(), 1, "{text}");
    for phrase in ["Lanes", "`stencil prime grammar plain`"] {
        assert!(sequence_lines[0].contains(phrase), "{}", sequence_lines[0]);
    }
    assert!(!text.contains("refused"), "{text}");
    assert!(
        text.contains("from id"),
        "Pipe targets are in the vocabulary"
    );
    assert!(text.contains("chrome none drops badge, kicker, title and lede"));
    assert!(text.contains("--print-width"));
    for word in [
        "Box", "Item", "Lanes", "facts", "source", "line", "tint", "chrome", "grammar",
    ] {
        assert!(text.contains(word), "the vocabulary names no {word}");
    }
    assert!(text.contains("grammar ref =gcp; a built-in name or a .json path"));
    assert!(text.contains("chrome Chrome =full"));
    assert!(text.contains("source FactSource =doc"));
    assert!(text.contains("tint 1-8"));
    assert!(text.contains("`stencil prime grammar <name>`"));
    for name in stencil_model::BUILTIN_GRAMMARS {
        assert!(text.contains(name), "the briefing names no grammar {name}");
    }
}

#[test]
fn prime_grammar_prints_each_built_in_grammar_within_the_topic_budget() {
    for name in stencil_model::BUILTIN_GRAMMARS {
        let outcome = run_stencil(&["prime", "grammar", name]);
        assert_eq!(outcome.code, ExitCode::Clean, "{name}: {}", outcome.stderr);
        assert_eq!(outcome.stderr, "");
        assert!(
            outcome.stdout.len() <= TOPIC_BYTES_MAX,
            "{name} briefing is {} bytes",
            outcome.stdout.len()
        );
        assert!(!outcome.stdout.contains("{{"), "{name}: an unfilled marker");
        let grammar = builtin_grammar(name).unwrap().unwrap();
        for container in &grammar.containers {
            let row = format!("| {} | ", container.name);
            assert!(
                outcome.stdout.contains(&row),
                "{name} briefing has no row for {}",
                container.name
            );
        }
        for item in &grammar.items {
            assert!(
                outcome.stdout.contains(&format!("| {} | ", item.name)),
                "{name} briefing has no row for {}",
                item.name
            );
        }
    }
    let gcp = run_stencil(&["prime", "grammar", "gcp"]).stdout;
    assert!(gcp.contains("| apis | group | neutral | no |"), "{gcp}");
    assert!(
        gcp.contains("| region | group | neutral | yes, default 1 |"),
        "{gcp}"
    );
}

#[test]
fn prime_grammar_plain_covers_lanes_and_names_each_plain_example_on_one_line() {
    let plain = run_stencil(&["prime", "grammar", "plain"]).stdout;
    let lanes_lines: Vec<&str> = plain
        .lines()
        .filter(|line| line.starts_with("Lanes:"))
        .collect();
    assert_eq!(lanes_lines.len(), 1, "{plain}");
    for phrase in [
        "order",
        "lifeline",
        "dash for a reply",
        "deny for a rejected call",
    ] {
        assert!(lanes_lines[0].contains(phrase), "Lanes line lacks {phrase}");
    }
    for stem in ["sequence", "org", "onprem-network"] {
        let path = format!("examples/{stem}.json");
        let lines = plain.lines().filter(|line| line.contains(&path)).count();
        assert_eq!(lines, 1, "{stem}: {plain}");
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(&path)
                .is_file(),
            "{path} is named but missing"
        );
    }
}

#[test]
fn prime_grammar_with_an_unknown_or_missing_name_exits_2_with_one_line() {
    for arguments in [vec!["prime", "grammar", "nope"], vec!["prime", "grammar"]] {
        let outcome = run_stencil(&arguments);
        assert_eq!(outcome.code, ExitCode::CouldNotRun, "{arguments:?}");
        assert_eq!(outcome.stdout, "");
        assert_eq!(outcome.stderr.lines().count(), 1, "{}", outcome.stderr);
        assert!(outcome.stderr.contains("gcp, plain"), "{}", outcome.stderr);
    }
    let extra = run_stencil(&["prime", "themes", "gcp"]);
    assert_eq!(extra.code, ExitCode::CouldNotRun);
    assert_eq!(extra.stdout, "");
}

/// The base briefing has 3 bytes to spare, so it names the designed themes and leaves the
/// imported tier to the themes topic.
#[test]
fn the_base_briefing_names_every_designed_theme() {
    let text = base_output();
    let section = text.split("# Themes").nth(1).unwrap();
    for name in stencil_model::DESIGNED_THEMES {
        assert!(section.contains(name), "{name}");
    }
}

#[test]
fn the_themes_topic_lists_the_imported_tier_one_line_each_with_its_warning() {
    let outcome = run_stencil(&["prime", "themes"]);
    assert_eq!(outcome.code, ExitCode::Clean);
    for name in stencil_model::IMPORTED_THEMES {
        let lines: Vec<&str> = outcome
            .stdout
            .lines()
            .filter(|line| line.starts_with(&format!("- {name}: ")))
            .collect();
        assert_eq!(lines.len(), 1, "{name}");
        assert!(
            lines[0].contains("editor heritage, below the colorblind bar"),
            "{name}"
        );
    }
    assert!(outcome.stdout.contains("stencil theme check <name>"));
}

#[test]
fn the_base_briefing_lists_every_enum_value_from_the_model() {
    let text = base_output();
    assert_enum_listed(&text, "Line", names(&Line::ALL));
    assert_enum_listed(&text, "FactSource", names(&FactSource::ALL));
    assert_enum_listed(&text, "Chrome", names(&chromes()));
    assert_enum_listed(&text, "IconName", names(&IconName::ALL));
    assert_enum_listed(&text, "Arrow", names(&arrows()));
    assert_enum_listed(&text, "Side", names(&sides()));
    assert_enum_listed(&text, "Justify", names(&justifies()));
    assert_enum_listed(&text, "ListKind", names(&list_kinds()));
    assert_enum_listed(&text, "CalloutKind", names(&callout_kinds()));
    assert_enum_listed(&text, "Shape", names(&shapes()));
    assert_enum_listed(&text, "PipeForm", names(&pipe_forms()));
    assert!(text.contains(&format!("IconName ({}):", IconName::ALL.len())));
}

#[test]
fn the_iso_topic_names_every_shape_and_pipe_form() {
    let text = topic_text(Topic::Iso);
    let text = text.as_str();
    assert_enum_listed(text, "Shape", names(&shapes()));
    assert_enum_listed(text, "PipeForm", names(&pipe_forms()));
    for shape in names(&shapes()) {
        let mentions = text
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|word| *word == shape)
            .count();
        assert!(
            mentions >= 2,
            "the iso topic says nothing about the {shape} shape"
        );
    }
}

#[test]
fn the_links_field_table_names_only_link_fields() {
    let schema = page_schema();
    let schema = schema.as_value();
    let link = schema["$defs"]["Link"]["properties"].as_object().unwrap();
    let links = topic_text(Topic::Links);
    let rows: Vec<&str> = links
        .lines()
        .skip_while(|line| *line != "| Field | Meaning |")
        .skip(2)
        .take_while(|line| line.starts_with("| "))
        .collect();
    assert_eq!(rows.len(), 8, "{rows:?}");
    let mut named = BTreeSet::new();
    for row in rows {
        let cell = row.trim_start_matches("| ").split(" |").next().unwrap();
        for field in cell.split(", ") {
            assert!(
                link.contains_key(field),
                "the links table lists {field}, which Link has no field for"
            );
            named.insert(field.to_string());
        }
    }
    let fields: BTreeSet<String> = link.keys().cloned().collect();
    assert_eq!(
        named, fields,
        "the links table and the Link schema list different fields"
    );
}

#[test]
fn the_base_briefing_covers_every_check_the_pipeline_runs() {
    let loaded = load_document(&read_input(&g7_path()).unwrap()).unwrap();
    let rendered = render_page(&loaded, DeviceScale::DEFAULT).unwrap();
    let reports = all_checks(
        &loaded.page,
        &loaded.grammar,
        &rendered.geometry,
        rendered.scene.as_ref(),
        None,
    );
    let text = base_output();
    for report in &reports {
        let row = format!("| {} |", report.check.as_str());
        assert!(
            text.lines().any(|line| line.starts_with(&row)),
            "the checks table has no row for {}",
            report.check.as_str()
        );
    }
    for report in &reports {
        let name = report.check.as_str();
        let topic = if name.starts_with("iso-") {
            Topic::Iso
        } else {
            Topic::Checks
        };
        assert!(
            topic_text(topic).contains(&format!("| {name} |")),
            "the {} topic has no row for {name}",
            topic.name()
        );
        let other = if topic == Topic::Iso {
            Topic::Checks
        } else {
            Topic::Iso
        };
        assert!(
            !topic_text(other).contains(&format!("| {name} |")),
            "the {} topic also has a row for {name}",
            other.name()
        );
    }
    let topic = topic_text(Topic::Checks);
    assert!(
        topic.contains("runs remembered-constants, legend-consistency and icon-matches-product"),
        "{topic}"
    );
    assert!(
        topic.contains("`--print-width <inches>` (0.5 to 200)"),
        "{topic}"
    );
}

#[test]
fn no_prime_text_names_a_tag_that_does_not_exist() {
    let tags = node_tags();
    let objects = table_objects();
    let base = base_output();
    for name in vocabulary_row_names(&base) {
        assert!(objects.contains(&name), "vocabulary names unknown {name}");
    }
    for name in vocabulary_row_names(&topic_text(Topic::Sheet)) {
        assert!(
            objects.contains(&name),
            "the sheet topic names unknown {name}"
        );
    }
    for (topic, text) in all_texts() {
        for marker in ["\"tag\": \"", "\"tag\":\""] {
            for occurrence in text.split(marker).skip(1) {
                let tag = occurrence.split('"').next().unwrap();
                assert!(tags.contains(tag), "{topic} names tag {tag}");
            }
        }
        for word in text.split(|character: char| !character.is_ascii_alphanumeric()) {
            assert!(
                !["Grid", "Xcard", "Zone", "Pcard", "Group", "Edge"].contains(&word),
                "{topic} names {word}, which is not a tag"
            );
        }
    }
}

#[test]
fn every_field_note_names_a_field_the_schema_has() {
    let schema = page_schema();
    let schema = schema.as_value();
    let object = |name: &str| -> Value {
        if name == "Page" {
            return schema.clone();
        }
        let variant = schema["$defs"]["Node"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["tag"]["const"] == name);
        match variant {
            Some(variant) => variant.clone(),
            None => schema["$defs"][name].clone(),
        }
    };
    let notes = field_notes().unwrap();
    assert!(!notes.is_empty());
    for note in &notes {
        let properties = object(note.object)["properties"].clone();
        assert!(
            properties.get(note.field).is_some(),
            "{}.{} is not in the schema",
            note.object,
            note.field
        );
    }
}

#[test]
fn prime_texts_follow_the_house_style() {
    for (topic, text) in all_texts() {
        if topic == "example" {
            continue;
        }
        assert!(!text.contains('\u{2014}'), "{topic} has an emdash");
        assert!(!text.contains("**"), "{topic} has bold");
        assert!(
            !text.lines().any(|line| line.trim() == "---"),
            "{topic} has a horizontal rule"
        );
        assert!(
            !text.lines().any(|line| line.starts_with("##")),
            "{topic} has a heading deeper than one level"
        );
    }
}

/// A figure's page sits in whatever field its file names, so a `cue export` line teaches a
/// placeholder and never one figure's field, which a reader copies verbatim.
#[test]
fn every_cue_export_line_takes_a_field_placeholder() {
    let mut texts = all_texts();
    texts.push((
        "README.md".to_string(),
        include_str!("../../../README.md").to_string(),
    ));
    let mut examined = 0;
    for (name, text) in &texts {
        for line in text.lines().filter(|line| line.contains("cue export ")) {
            examined += 1;
            assert!(
                line.contains("-e <field>"),
                "{name}: an export names a fixed field: {line}"
            );
        }
    }
    assert!(examined >= 2, "only {examined} cue export lines found");
}
