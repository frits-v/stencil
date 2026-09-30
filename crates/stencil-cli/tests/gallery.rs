#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! `stencil gallery` (section 7): every example under every theme, index.html and
//! gallery.json.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_cli::{ExitCode, run};

const THEMES: [&str; 3] = ["center", "dusk", "wire"];

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

fn examples_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-gallery")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// A Row of three cards and one Pipe with its legend entry. With a long word in the first
/// card at width 640, child-inside-container fails; otherwise every check passes or does
/// not apply.
fn row_page(first_card: &str) -> Value {
    json!({
        "title": "Row of cards",
        "kicker": "Test",
        "lede": "One row.",
        "canvas": "internal",
        "width": 640,
        "body": [
            {
                "tag": "Row",
                "children": [
                    { "tag": "Pcard", "fn": first_card, "pn": "Cloud Run" },
                    { "tag": "Pcard", "fn": "Second", "pn": "Cloud Run" },
                    { "tag": "Pcard", "fn": "Third", "pn": "Cloud Run" }
                ]
            },
            { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "request" }
        ],
        "legend": [{ "kind": "blue", "text": "request path" }]
    })
}

fn write_example(directory: &Path, file_name: &str, document: &Value) {
    fs::write(
        directory.join(file_name),
        serde_json::to_string_pretty(document).unwrap(),
    )
    .unwrap();
}

fn example_stems() -> Vec<String> {
    let mut stems: Vec<String> = fs::read_dir(examples_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| path.file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    stems.sort();
    stems
}

#[test]
fn gallery_renders_every_example_under_every_theme() {
    let out_dir = scratch_directory("every_example");
    let examples = examples_directory();

    let outcome = run_stencil(&["gallery", text(&out_dir), "--examples", text(&examples)]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stdout);
    assert_eq!(outcome.stderr, "");
    let stems = example_stems();
    assert!(!stems.is_empty());
    let render_count = stems.len() * THEMES.len();

    let index = fs::read_to_string(out_dir.join("index.html")).unwrap();
    for theme in THEMES {
        assert!(
            index.contains(&format!("<a href=\"#{theme}\">{theme}</a>")),
            "no tab for {theme}"
        );
        assert!(
            index.contains(&format!("<section id=\"{theme}\">")),
            "no section for {theme}"
        );
        for stem in &stems {
            let png = format!("{stem}/{theme}/{stem}.png");
            assert!(index.contains(&format!("<img src=\"{png}\"")), "{png}");
            assert!(out_dir.join(&png).is_file(), "{png}");
            for other in [
                format!("{stem}/{theme}/{stem}.svg"),
                format!("{stem}/{theme}/{stem}.measured.json"),
            ] {
                assert!(index.contains(&format!("href=\"{other}\"")), "{other}");
                assert!(out_dir.join(&other).is_file(), "{other}");
            }
        }
    }

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("gallery.json")).unwrap()).unwrap();
    assert_eq!(manifest["renders"], json!(render_count));
    assert_eq!(manifest["failed"], json!(0));
    assert_eq!(manifest["themes"], json!(THEMES));
    let listed: Vec<&str> = manifest["examples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|example| example["name"].as_str().unwrap())
        .collect();
    assert_eq!(listed, stems);
    for example in manifest["examples"].as_array().unwrap() {
        let title = example["title"].as_str().unwrap();
        let kicker = example["kicker"].as_str().unwrap();
        assert!(index.contains(title), "{title}");
        assert!(index.contains(kicker), "{kicker}");
        assert_eq!(example["renders"].as_array().unwrap().len(), THEMES.len());
    }

    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &format!(
            "stencil gallery: {render_count} renders of {} examples in 3 themes, 0 failed",
            stems.len()
        )
    );
    let render_lines = lines
        .iter()
        .filter(|line| line.starts_with("gallery "))
        .count();
    assert_eq!(render_lines, render_count);
}

#[test]
fn a_failing_check_exits_1_and_the_index_still_lists_every_render() {
    let examples = scratch_directory("failing_check_examples");
    write_example(&examples, "overflow.json", &row_page(&"x".repeat(120)));
    write_example(&examples, "row.json", &row_page("First"));
    let out_dir = scratch_directory("failing_check_out");

    let outcome = run_stencil(&["gallery", text(&out_dir), "--examples", text(&examples)]);

    assert_eq!(outcome.code, ExitCode::Defects, "{}", outcome.stdout);
    assert!(
        outcome
            .stdout
            .contains("check child-inside-container: examined "),
        "{}",
        outcome.stdout
    );
    assert!(
        outcome.stdout.contains("defect child-inside-container "),
        "{}",
        outcome.stdout
    );
    assert!(
        outcome
            .stdout
            .ends_with("stencil gallery: 6 renders of 2 examples in 3 themes, 3 failed\n"),
        "{}",
        outcome.stdout
    );
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("gallery.json")).unwrap()).unwrap();
    assert_eq!(manifest["renders"], json!(6));
    assert_eq!(manifest["failed"], json!(3));
    let overflow = &manifest["examples"][0];
    assert_eq!(overflow["name"], json!("overflow"));
    for render in overflow["renders"].as_array().unwrap() {
        assert_eq!(render["passed"], json!(false));
    }
    let index = fs::read_to_string(out_dir.join("index.html")).unwrap();
    assert!(index.contains("<p class=\"checks failed\">"), "{index}");
    assert!(index.contains("overflow/wire/overflow.png"));
}

#[test]
fn a_document_that_does_not_parse_is_listed_as_not_rendered_and_exits_1() {
    let examples = scratch_directory("broken_examples");
    fs::write(examples.join("broken.json"), "{ \"title\": ").unwrap();
    write_example(&examples, "row.json", &row_page("First"));
    let out_dir = scratch_directory("broken_out");

    let outcome = run_stencil(&["gallery", text(&out_dir), "--examples", text(&examples)]);

    assert_eq!(outcome.code, ExitCode::Defects, "{}", outcome.stdout);
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert!(lines[0].starts_with("error "), "{}", outcome.stdout);
    assert_eq!(lines[1], "gallery broken: not rendered");
    assert_eq!(
        lines.last().unwrap(),
        &"stencil gallery: 3 renders of 2 examples in 3 themes, 3 failed"
    );
    assert!(!out_dir.join("broken").exists());
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("gallery.json")).unwrap()).unwrap();
    let broken = &manifest["examples"][0];
    for render in broken["renders"].as_array().unwrap() {
        assert_eq!(render["summary"], json!("not rendered"));
        assert!(render.get("files").is_none());
    }
}

#[test]
fn an_examples_directory_without_documents_exits_2_and_writes_nothing() {
    let examples = scratch_directory("empty_examples");
    fs::write(examples.join("notes.txt"), "not a document").unwrap();
    fs::create_dir(examples.join("nested.json")).unwrap();
    let out_dir = scratch_directory("empty_out").join("gallery");

    let outcome = run_stencil(&["gallery", text(&out_dir), "--examples", text(&examples)]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("holds no .json documents"),
        "{}",
        outcome.stderr
    );
    assert!(!out_dir.exists());
}

#[test]
fn a_missing_examples_directory_exits_2() {
    let scratch = scratch_directory("missing_examples");
    let out_dir = scratch.join("gallery");

    let outcome = run_stencil(&[
        "gallery",
        text(&out_dir),
        "--examples",
        text(&scratch.join("absent")),
    ]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("cannot read examples directory"),
        "{}",
        outcome.stderr
    );
    assert!(!out_dir.exists());
}

#[test]
fn gallery_without_an_output_directory_is_a_usage_error() {
    let outcome = run_stencil(&["gallery"]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(!outcome.stderr.is_empty());
}
