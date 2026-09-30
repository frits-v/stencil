#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! `--theme` on `render` and `check` (section 11.4).

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

#[test]
fn render_theme_changes_the_svg_and_keeps_the_measured_json() {
    let (default_svg, default_measured) = render("default", &g7_path(), None);
    let (center_svg, center_measured) = render("center", &g7_path(), Some("center"));
    let (dusk_svg, dusk_measured) = render("dusk", &g7_path(), Some("dusk"));
    let (wire_svg, wire_measured) = render("wire", &g7_path(), Some("wire"));

    assert_eq!(center_svg, default_svg);
    assert_eq!(background_fill(&default_svg), "#FFFFFF");
    assert_eq!(background_fill(&dusk_svg), "#0B1220");
    assert_eq!(background_fill(&wire_svg), "#FFFFFF");
    assert_ne!(wire_svg, default_svg);
    assert!(wire_svg.contains("#222222"));
    for measured in [&center_measured, &dusk_measured, &wire_measured] {
        assert_eq!(measured, &default_measured);
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

    assert_eq!(background_fill(&document_theme_svg), "#0B1220");
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
    for theme in ["center", "dusk", "wire"] {
        let outcome = run_stencil(&["check", &g7_path(), "--theme", theme]);
        assert_eq!(outcome.code, ExitCode::Clean, "{theme}: {}", outcome.stderr);
        assert_eq!(outcome.stdout, baseline.stdout, "{theme}");
        assert_eq!(outcome.stderr, "");
    }
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
