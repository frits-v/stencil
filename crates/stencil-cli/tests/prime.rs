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
use stencil_cli::prime::{BASE_BYTES_MAX, TOPIC_BYTES_MAX, Topic, base_text, field_notes};
use stencil_cli::{ExitCode, run};
use stencil_model::{
    Arrow, CalloutKind, Chrome, FactSource, IconName, Justify, Line, ListKind, Side,
    builtin_grammar, page_schema,
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

/// Everything the vocabulary table may name in its first column.
fn table_objects() -> BTreeSet<String> {
    let mut objects = node_tags();
    for name in ["Page", "FactEntry", "LegendEntry", "Link"] {
        objects.insert(name.to_string());
    }
    objects
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
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no enum line for {name}"));
    let (count, values) = line
        .strip_prefix(&prefix)
        .unwrap()
        .split_once("): ")
        .unwrap();
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

#[test]
fn the_base_briefing_names_every_node_tag_in_its_vocabulary() {
    let text = base_output();
    let rows: BTreeSet<String> = vocabulary_row_names(&text).into_iter().collect();
    for tag in node_tags() {
        assert!(rows.contains(&tag), "vocabulary has no row for {tag}");
    }
    for object in ["Page", "FactEntry", "LegendEntry", "Link"] {
        assert!(rows.contains(object), "vocabulary has no row for {object}");
    }
    assert_eq!(node_tags().len(), 12);
}

#[test]
fn the_base_briefing_describes_the_core_vocabulary() {
    let text = base_output();
    assert!(text.contains("Every node object carries \"tag\"; the Page does not."));
    assert!(
        text.contains("Sequence and timeline figures are refused until a timeline grammar exists.")
    );
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

#[test]
fn the_base_briefing_names_every_built_in_theme() {
    let text = base_output();
    let section = text.split("# Themes").nth(1).unwrap();
    for name in stencil_model::BUILTIN_THEMES {
        assert!(section.contains(name), "{name}");
    }
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
    assert!(text.contains(&format!("IconName ({}):", IconName::ALL.len())));
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
    let topic = Topic::Checks.text();
    for report in &reports {
        assert!(
            topic.contains(&format!("| {} |", report.check.as_str())),
            "the checks topic has no row for {}",
            report.check.as_str()
        );
    }
}

#[test]
fn no_prime_text_names_a_tag_that_does_not_exist() {
    let tags = node_tags();
    let objects = table_objects();
    let base = base_output();
    for name in vocabulary_row_names(&base) {
        assert!(objects.contains(&name), "vocabulary names unknown {name}");
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
