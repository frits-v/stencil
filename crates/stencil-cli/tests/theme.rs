#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Theme references on `render`, `check` and `vet`, and `stencil theme` (sections 13.4 and
//! 13.12).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use stencil_cli::{ExitCode, run};

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

fn g7_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/g7.json")
        .to_str()
        .unwrap()
        .to_string()
}

fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-theme")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

/// Renders `input` into a fresh directory and returns (svg, measured JSON bytes).
fn render(test_name: &str, input: &str, theme: Option<&str>) -> (String, Vec<u8>) {
    let out_dir = scratch_directory(test_name);
    let mut arguments = vec!["render", input, "--out-dir", out_dir.to_str().unwrap()];
    if let Some(theme) = theme {
        arguments.extend(["--theme", theme]);
    }
    let outcome = run_stencil(&arguments);
    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    let stem = Path::new(input).file_stem().unwrap().to_str().unwrap();
    let svg = fs::read_to_string(out_dir.join(format!("{stem}.svg"))).unwrap();
    let measured = fs::read(out_dir.join(format!("{stem}.measured.json"))).unwrap();
    (svg, measured)
}

fn background_fill(svg: &str) -> &str {
    let rect_start = svg.find("<rect ").unwrap();
    let fill_start = rect_start + svg[rect_start..].find("fill=\"").unwrap() + 6;
    let fill_end = fill_start + svg[fill_start..].find('"').unwrap();
    &svg[fill_start..fill_end]
}

const THEMES: [&str; 6] = ["center", "paper", "dusk", "clear", "clear-dark", "wire"];

fn center_theme_json() -> &'static str {
    include_str!("../../stencil-render/themes/center.json")
}

#[test]
fn render_theme_changes_the_svg_and_keeps_the_measured_json() {
    let (default_svg, default_measured) = render("default", &g7_path(), None);
    assert_eq!(background_fill(&default_svg), "#FFFFFF");
    for theme in THEMES {
        let (svg, measured) = render(theme, &g7_path(), Some(theme));
        let page: Value =
            serde_json::from_str(&run_stencil(&["theme", "show", theme]).stdout).unwrap();
        assert_eq!(background_fill(&svg), page["page"], "{theme}");
        assert_eq!(measured, default_measured, "{theme}");
        if theme == "center" {
            assert_eq!(svg, default_svg);
        } else {
            assert_ne!(svg, default_svg, "{theme}");
        }
    }
}

#[test]
fn the_flag_overrides_the_document_theme() {
    let directory = scratch_directory("override_input");
    let mut document: Value =
        serde_json::from_str(&fs::read_to_string(g7_path()).unwrap()).unwrap();
    document["theme"] = json!("dusk");
    let input = directory.join("dusk-g7.json");
    fs::write(&input, serde_json::to_string_pretty(&document).unwrap()).unwrap();
    let input = input.to_str().unwrap();

    let (document_theme_svg, _) = render("override_none", input, None);
    let (overridden_svg, overridden_measured) = render("override_wire", input, Some("wire"));

    assert_eq!(background_fill(&document_theme_svg), "#0D0F17");
    assert_eq!(background_fill(&overridden_svg), "#FFFFFF");
    assert!(overridden_svg.contains("#222222"));
    let measured: Value = serde_json::from_slice(&overridden_measured).unwrap();
    assert_eq!(
        measured["document"]["theme"],
        json!("dusk"),
        "input kept as written"
    );
}

#[test]
fn check_accepts_every_theme_with_the_same_report() {
    let baseline = run_stencil(&["check", &g7_path()]);
    assert_eq!(baseline.code, ExitCode::Clean, "{}", baseline.stderr);
    for theme in THEMES {
        let outcome = run_stencil(&["check", &g7_path(), "--theme", theme]);
        assert_eq!(outcome.code, ExitCode::Clean, "{theme}: {}", outcome.stderr);
        assert_eq!(outcome.stdout, baseline.stdout, "{theme}");
        assert_eq!(outcome.stderr, "");
    }
}

/// `path` relative to the current directory, which cargo sets to the package root.
fn relative_to_current_directory(path: &Path) -> String {
    let current = std::env::current_dir().unwrap();
    let mut prefix = PathBuf::new();
    let mut base = current.as_path();
    for _ in 0..16 {
        if let Ok(rest) = path.strip_prefix(base) {
            return prefix.join(rest).to_str().unwrap().to_string();
        }
        prefix.push("..");
        base = base.parent().unwrap();
    }
    panic!(
        "{} shares no ancestor with {}",
        path.display(),
        current.display()
    );
}

#[test]
fn a_theme_file_renders_from_the_flag_and_from_the_page() {
    let directory = scratch_directory("theme_file");
    let mut theme: Value = serde_json::from_str(center_theme_json()).unwrap();
    theme["name"] = json!("brand");
    theme["page"] = json!("#FAFAF7");
    fs::write(directory.join("brand.json"), theme.to_string()).unwrap();

    let flag = relative_to_current_directory(&directory.join("brand.json"));
    assert!(!flag.starts_with('/'), "{flag}");
    let (flag_svg, _) = render("theme_file_flag", &g7_path(), Some(&flag));
    assert_eq!(background_fill(&flag_svg), "#FAFAF7");

    let mut document: Value =
        serde_json::from_str(&fs::read_to_string(g7_path()).unwrap()).unwrap();
    document["theme"] = json!("brand.json");
    let input = directory.join("branded.json");
    fs::write(&input, document.to_string()).unwrap();
    let (page_svg, _) = render("theme_file_page", input.to_str().unwrap(), None);
    assert_eq!(
        page_svg, flag_svg,
        "a page theme path resolves against the document"
    );
}

#[test]
fn a_missing_theme_file_could_not_run_and_a_malformed_one_is_a_defect() {
    let directory = scratch_directory("theme_faults");
    let missing = directory.join("missing.json");
    let outcome = run_stencil(&["check", &g7_path(), "--theme", missing.to_str().unwrap()]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("cannot read theme"),
        "{}",
        outcome.stderr
    );

    let mut theme: Value = serde_json::from_str(center_theme_json()).unwrap();
    theme["page"] = json!("#ffffff");
    theme["tints"][1]["name"] = json!("blue");
    let malformed = directory.join("malformed.json");
    fs::write(&malformed, theme.to_string()).unwrap();
    let outcome = run_stencil(&["check", &g7_path(), "--theme", malformed.to_str().unwrap()]);
    assert_eq!(outcome.code, ExitCode::Defects);
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert!(lines[0].starts_with("error theme "), "{}", lines[0]);
    assert!(lines[0].ends_with("violates 2 rule(s)"), "{}", lines[0]);
    assert!(lines[1].starts_with("violation theme-color-malformed /page: "));
    assert!(lines[2].starts_with("violation theme-tint-name-duplicate /tints/1/name: "));
    assert_eq!(lines[3], "stencil check: checks not run");

    fs::write(&malformed, "{\"name\": ").unwrap();
    let outcome = run_stencil(&[
        "render",
        &g7_path(),
        "--out-dir",
        directory.to_str().unwrap(),
        "--theme",
        malformed.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(
        outcome.stdout.contains("is not valid theme JSON"),
        "{}",
        outcome.stdout
    );
}

#[test]
fn page_overrides_apply_and_their_faults_point_into_theme_overrides() {
    let directory = scratch_directory("overrides");
    let mut document: Value =
        serde_json::from_str(&fs::read_to_string(g7_path()).unwrap()).unwrap();
    document["theme_overrides"] = json!({ "page": "#FAFAF7" });
    let input = directory.join("overridden.json");
    fs::write(&input, document.to_string()).unwrap();
    let (svg, _) = render("overrides_render", input.to_str().unwrap(), None);
    assert_eq!(background_fill(&svg), "#FAFAF7");

    document["theme_overrides"] = json!({ "iso": { "glow": 1 }, "card": "#FFFFFF" });
    fs::write(&input, document.to_string()).unwrap();
    let outcome = run_stencil(&["vet", input.to_str().unwrap()]);
    assert_eq!(outcome.code, ExitCode::Defects);
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    assert!(
        lines[0].starts_with("error theme center with /theme_overrides violates 2"),
        "{}",
        lines[0]
    );
    assert!(
        lines[1].starts_with("violation theme-override-not-object /theme_overrides/card: "),
        "{}",
        lines[1]
    );
    assert!(
        lines[2].starts_with("violation theme-override-unknown-role /theme_overrides/iso/glow: "),
        "{}",
        lines[2]
    );
    assert_eq!(lines[3], "stencil vet: checks not run");
}

#[test]
fn a_page_theme_that_is_neither_built_in_nor_json_fails_vet() {
    let directory = scratch_directory("theme_unknown");
    let mut document: Value =
        serde_json::from_str(&fs::read_to_string(g7_path()).unwrap()).unwrap();
    document["theme"] = json!("night");
    let input = directory.join("night.json");
    fs::write(&input, document.to_string()).unwrap();
    let outcome = run_stencil(&["vet", input.to_str().unwrap()]);
    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(outcome.stdout.starts_with(
        "violation theme-unknown /theme: theme \"night\" is neither a built-in theme nor a .json path\n"
    ), "{}", outcome.stdout);
}

#[test]
fn theme_show_prints_the_embedded_file_and_refuses_an_unknown_name() {
    let outcome = run_stencil(&["theme", "show", "center"]);
    assert_eq!(outcome.code, ExitCode::Clean);
    assert_eq!(outcome.stdout, center_theme_json());
    let unknown = run_stencil(&["theme", "show", "sepia"]);
    assert_eq!(unknown.code, ExitCode::CouldNotRun);
    assert_eq!(unknown.stdout, "");
    assert!(
        unknown
            .stderr
            .contains("center, paper, dusk, clear, clear-dark, wire")
    );
}

#[test]
fn theme_check_prints_every_row_and_exits_by_the_failures() {
    let center = run_stencil(&["theme", "check", "center"]);
    assert_eq!(center.code, ExitCode::Defects);
    let lines: Vec<&str> = center.stdout.lines().collect();
    assert_eq!(lines.len(), 224);
    assert_eq!(
        lines[0],
        "row contrast ink.primary on page: 16.10, at least 4.50, passed"
    );
    assert!(lines.contains(&"row contrast ask.ink on ask.fill: 4.34, at least 4.50, FAILED"));
    assert!(lines.contains(
        &"row separation wires deuteranopia: 13.16 between pink and green, at least 12.00, passed"
    ));
    assert_eq!(
        lines[223],
        "stencil theme check center: 223 rows, 222 passed, 1 failed"
    );

    let paper = run_stencil(&["theme", "check", "paper"]);
    assert_eq!(paper.code, ExitCode::Clean);
    assert!(
        paper
            .stdout
            .ends_with("stencil theme check paper: 223 rows, 223 passed, 0 failed\n")
    );

    let wire = run_stencil(&["theme", "check", "wire"]);
    assert_eq!(wire.code, ExitCode::Clean);
    let not_applicable = wire
        .stdout
        .lines()
        .filter(|line| line.ends_with(": not applicable: tints are told apart by line"))
        .count();
    assert_eq!(not_applicable, 8);
    assert!(
        wire.stdout.contains(
            "row separation fills normal: not applicable: tints are told apart by line\n"
        )
    );
    assert!(
        wire.stdout.ends_with(
            "stencil theme check wire: 223 rows, 215 passed, 0 failed, 8 not applicable\n"
        )
    );

    let missing = run_stencil(&["theme", "check", "nowhere/missing.json"]);
    assert_eq!(missing.code, ExitCode::CouldNotRun);
    assert_eq!(missing.stdout, "");
}

#[test]
fn an_unknown_theme_or_a_theme_on_vet_could_not_run() {
    let out_dir = scratch_directory("unknown_theme");
    let unknown_render = run_stencil(&[
        "render",
        &g7_path(),
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--theme",
        "sepia",
    ]);
    assert_eq!(unknown_render.code, ExitCode::CouldNotRun);
    assert_eq!(unknown_render.stdout, "");
    assert!(fs::read_dir(&out_dir).unwrap().next().is_none());

    let unknown_check = run_stencil(&["check", &g7_path(), "--theme", "sepia"]);
    assert_eq!(unknown_check.code, ExitCode::CouldNotRun);

    let vet = run_stencil(&["vet", &g7_path(), "--theme", "dusk"]);
    assert_eq!(vet.code, ExitCode::CouldNotRun);
}
