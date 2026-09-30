#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! `--projection` on `render` and `check`, and the hero figure (section 12.9 and 12.10).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use stencil_cli::{ExitCode, run};

struct Outcome {
    code: ExitCode,
    stdout: String,
    stderr: String,
}

impl Outcome {
    fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
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

fn example_path(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
        .to_str()
        .unwrap()
        .to_string()
}

fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-iso")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

const THEMES: [&str; 3] = ["center", "dusk", "wire"];

#[test]
fn check_hero_passes_every_check_in_every_theme() {
    for theme in THEMES {
        let outcome = run_stencil(&["check", &example_path("hero-iso.json"), "--theme", theme]);
        assert_eq!(outcome.code, ExitCode::Clean, "{theme}: {}", outcome.stdout);
        let lines = outcome.stdout_lines();
        assert!(
            lines.contains(&"check iso-labels-clear: examined 81 pairs, 0 defects"),
            "{theme}: {}",
            outcome.stdout
        );
        assert_eq!(
            lines.last().copied(),
            Some("stencil check: 9 checks, 9 passed, 0 failed"),
            "{theme}"
        );
        assert_eq!(outcome.stderr, "");
    }
}

#[test]
fn check_hero_with_projection_flat_skips_the_iso_check() {
    let outcome = run_stencil(&[
        "check",
        &example_path("hero-iso.json"),
        "--projection",
        "flat",
    ]);
    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stdout);
    let lines = outcome.stdout_lines();
    assert!(
        lines.contains(
            &"check iso-labels-clear: examined 0 pairs, not applicable: projection is flat"
        )
    );
    assert_eq!(
        lines.last().copied(),
        Some("stencil check: 9 checks, 8 passed, 0 failed, 1 not applicable")
    );
}

#[test]
fn check_prints_iso_labels_clear_last() {
    let outcome = run_stencil(&["check", &example_path("hero-iso.json")]);
    let check_lines: Vec<&str> = outcome
        .stdout_lines()
        .into_iter()
        .filter(|line| line.starts_with("check "))
        .collect();
    assert_eq!(check_lines.len(), 9);
    assert!(check_lines[8].starts_with("check iso-labels-clear: "));
}

#[test]
fn render_hero_writes_a_png_as_wide_as_the_drawn_canvas() {
    for theme in THEMES {
        let out_dir = scratch_directory(&format!("hero-{theme}"));
        let outcome = run_stencil(&[
            "render",
            &example_path("hero-iso.json"),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "--theme",
            theme,
        ]);
        assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
        assert_eq!(outcome.stdout_lines().len(), 3);
        let measured: Value =
            serde_json::from_slice(&fs::read(out_dir.join("hero-iso.measured.json")).unwrap())
                .unwrap();
        let drawn_width = measured["projection"]["canvas"]["width"].as_f64().unwrap();
        let png = fs::read(out_dir.join("hero-iso.png")).unwrap();
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
        assert_eq!(
            f64::from(pixmap.width()),
            (drawn_width * 2.0).ceil(),
            "{theme}"
        );
        let svg = fs::read_to_string(out_dir.join("hero-iso.svg")).unwrap();
        assert!(svg.contains(r#"data-projection="iso""#));
    }
}

#[test]
fn render_g7_with_projection_iso_keeps_the_flat_nodes() {
    let flat_dir = scratch_directory("g7-flat");
    let iso_dir = scratch_directory("g7-iso");
    let g7 = example_path("g7.json");
    let flat = run_stencil(&["render", &g7, "--out-dir", flat_dir.to_str().unwrap()]);
    let iso = run_stencil(&[
        "render",
        &g7,
        "--out-dir",
        iso_dir.to_str().unwrap(),
        "--projection",
        "iso",
    ]);
    assert_eq!(flat.code, ExitCode::Clean, "{}", flat.stderr);
    assert_eq!(iso.code, ExitCode::Clean, "{}", iso.stderr);
    let read = |directory: &Path| -> Value {
        serde_json::from_slice(&fs::read(directory.join("g7.measured.json")).unwrap()).unwrap()
    };
    let flat_measured = read(&flat_dir);
    let iso_measured = read(&iso_dir);
    assert_eq!(flat_measured["nodes"], iso_measured["nodes"]);
    assert!(flat_measured.get("projection").is_none());
    assert_eq!(iso_measured["projection"]["kind"], "iso");
    let iso_svg = fs::read_to_string(iso_dir.join("g7.svg")).unwrap();
    assert!(iso_svg.contains(r#"data-projection="iso""#));
    let flat_svg = fs::read_to_string(flat_dir.join("g7.svg")).unwrap();
    assert!(!flat_svg.contains("data-projection"));
}

#[test]
fn vet_rejects_the_projection_flag() {
    let outcome = run_stencil(&["vet", &example_path("g7.json"), "--projection", "iso"]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
}

#[test]
fn an_unknown_projection_value_is_a_usage_error() {
    let outcome = run_stencil(&["check", &example_path("g7.json"), "--projection", "oblique"]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
}
