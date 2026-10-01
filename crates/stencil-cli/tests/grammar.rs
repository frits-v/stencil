#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Grammar resolution on the command line (sections 13.2 and 13.12): built-in names, a
//! .json path beside the document, and the failures of each.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_cli::{ExitCode, run};
use stencil_model::grammar::builtin_grammar_json;

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

/// A fresh, empty directory under target/tmp, one per test.
fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-grammar")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

/// A one-item page in the given grammar reference, with a solid pipe and its legend entry.
fn page(grammar: &str, item_kind: &str) -> Value {
    json!({
        "title": "Grammar figure",
        "kicker": "Test",
        "lede": "One item and one hop.",
        "canvas": "internal",
        "grammar": grammar,
        "body": [
            { "tag": "Item", "kind": item_kind, "title": "API" },
            { "tag": "Pipe", "dir": "h", "line": "solid", "label": "call" }
        ],
        "legend": [{ "line": "solid", "text": "request path" }]
    })
}

fn gcp_value() -> Value {
    serde_json::from_str(builtin_grammar_json("gcp").unwrap()).unwrap()
}

#[test]
fn a_plain_page_vets_and_remembered_constants_is_not_applicable() {
    let directory = scratch_directory("plain_page");
    let document = directory.join("plain.json");
    write_json(&document, &page("plain", "service"));

    let outcome = run_stencil(&["vet", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stdout);
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(
        lines[0],
        "check remembered-constants: examined 0 text fields, not applicable: grammar has no remembered constants"
    );
    assert_eq!(
        lines.last().copied(),
        Some("stencil vet: 0 violations, 2 checks, 1 passed, 0 failed, 1 not applicable")
    );
}

#[test]
fn a_gcp_item_kind_under_plain_is_kind_unknown() {
    let directory = scratch_directory("kind_unknown");
    let document = directory.join("plain.json");
    write_json(&document, &page("plain", "product"));

    let outcome = run_stencil(&["vet", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout.lines().next(),
        Some(
            "violation kind-unknown /body/0/kind: kind \"product\" is not an item kind of grammar plain"
        )
    );
}

#[test]
fn a_grammar_path_resolves_against_the_document_directory() {
    let directory = scratch_directory("grammar_path");
    write_json(&directory.join("grammars/mine.json"), &gcp_value());
    let document = directory.join("figure.json");
    write_json(&document, &page("grammars/mine.json", "product"));

    let outcome = run_stencil(&["check", document.to_str().unwrap()]);

    assert_eq!(
        outcome.code,
        ExitCode::Clean,
        "{}{}",
        outcome.stdout,
        outcome.stderr
    );
    assert!(
        outcome
            .stdout
            .contains("check remembered-constants: examined 6 text fields, 0 defects"),
        "{}",
        outcome.stdout
    );
}

#[test]
fn a_grammar_file_with_an_unknown_parent_exits_1_with_its_violations() {
    let directory = scratch_directory("grammar_invalid");
    let mut grammar = gcp_value();
    grammar["containers"][1]["parents"][0] = json!("cloud");
    let grammar_path = directory.join("bad.json");
    write_json(&grammar_path, &grammar);
    let document = directory.join("figure.json");
    write_json(&document, &page("bad.json", "product"));

    let outcome = run_stencil(&["vet", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{}", outcome.stdout);
    assert!(lines[0].starts_with("error grammar "), "{}", lines[0]);
    assert!(lines[0].ends_with("violates 1 rule(s)"), "{}", lines[0]);
    assert_eq!(
        lines[1],
        "violation grammar-parent-unknown /containers/1/parents/0: parent \"cloud\" is neither a container kind nor \"page\""
    );
    assert_eq!(lines[2], "stencil vet: checks not run");
    assert_eq!(outcome.stderr, "");
}

#[test]
fn a_grammar_file_that_is_not_json_exits_1() {
    let directory = scratch_directory("grammar_not_json");
    fs::write(directory.join("broken.json"), "{ not json").unwrap();
    let document = directory.join("figure.json");
    write_json(&document, &page("broken.json", "product"));

    let outcome = run_stencil(&["check", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(
        outcome.stdout.starts_with("error grammar "),
        "{}",
        outcome.stdout
    );
    assert!(outcome.stdout.contains("is not valid grammar JSON"));
}

#[test]
fn a_missing_grammar_file_could_not_run() {
    let directory = scratch_directory("grammar_missing");
    let document = directory.join("figure.json");
    write_json(&document, &page("absent.json", "product"));

    let outcome = run_stencil(&["vet", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome
            .stderr
            .starts_with("stencil vet: cannot read grammar "),
        "{}",
        outcome.stderr
    );
}

#[test]
fn a_grammar_name_that_is_neither_built_in_nor_a_path_is_grammar_unknown() {
    let directory = scratch_directory("grammar_unknown");
    let document = directory.join("figure.json");
    write_json(&document, &page("gcpx", "product"));

    let outcome = run_stencil(&["vet", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout,
        "violation grammar-unknown /grammar: grammar \"gcpx\" is neither a built-in grammar nor a .json path\nstencil vet: 1 violation, checks not run\n"
    );
}

#[test]
fn an_absent_grammar_is_gcp() {
    let directory = scratch_directory("grammar_absent");
    let document = directory.join("figure.json");
    let mut value = page("gcp", "product");
    value.as_object_mut().unwrap().remove("grammar");
    write_json(&document, &value);

    let outcome = run_stencil(&["vet", document.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stdout);
    assert!(
        outcome
            .stdout
            .contains("check remembered-constants: examined 6 text fields, 0 defects")
    );
}

#[test]
fn every_plain_example_checks_clean_in_center_dusk_and_wire() {
    let mut examined = 0;
    for stem in ["sequence", "org", "onprem-network"] {
        let document = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(format!("{stem}.json"));
        let document = document.to_str().unwrap();
        for theme in ["center", "dusk", "wire"] {
            let outcome = run_stencil(&["check", document, "--theme", theme]);
            assert_eq!(
                outcome.code,
                ExitCode::Clean,
                "{stem} {theme}: {}",
                outcome.stdout
            );
            assert!(
                outcome.stdout.contains(", 0 failed, "),
                "{stem} {theme}: {}",
                outcome.stdout
            );
            assert!(
                outcome
                    .stdout
                    .contains("check remembered-constants: examined 0 text fields, not applicable: grammar has no remembered constants"),
                "{stem} {theme}"
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 9);
}
