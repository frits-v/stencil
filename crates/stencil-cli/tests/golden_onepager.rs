#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Section 11.5: `examples/onepager.json` renders under every theme and `check` exits 0 with
//! every check passing, links included, and the same report in each theme.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
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

fn onepager_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/onepager.json")
        .to_str()
        .unwrap()
        .to_string()
}

fn scratch_directory(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-onepager")
        .join(name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn check_passes_every_check_under_every_theme() {
    let mut reports = Vec::new();
    for theme in THEMES {
        let outcome = run_stencil(&["check", &onepager_path(), "--theme", theme]);
        assert_eq!(outcome.code, ExitCode::Clean, "{theme}: {}", outcome.stderr);
        assert_eq!(outcome.stderr, "", "{theme}");
        assert!(
            outcome
                .stdout
                .contains("check links-routed: examined 7 links, 0 defects"),
            "{theme}: {}",
            outcome.stdout
        );
        assert!(
            outcome.stdout.contains(
                "check pipes-land: examined 0 pipe ends, not applicable: page has no pipes"
            ),
            "{theme}: {}",
            outcome.stdout
        );
        assert!(
            outcome
                .stdout
                .contains("stencil check: 10 checks, 7 passed, 0 failed, 3 not applicable"),
            "{theme}: {}",
            outcome.stdout
        );
        reports.push(outcome.stdout);
    }
    assert_eq!(reports[0], reports[1]);
    assert_eq!(reports[0], reports[2]);
}

#[test]
fn render_writes_every_theme_with_identical_measured_json_and_all_links() {
    let mut measured_outputs = Vec::new();
    let mut svg_outputs = Vec::new();
    for theme in THEMES {
        let out_dir = scratch_directory(theme);
        let outcome = run_stencil(&[
            "render",
            &onepager_path(),
            "--out-dir",
            out_dir.to_str().unwrap(),
            "--theme",
            theme,
        ]);
        assert_eq!(outcome.code, ExitCode::Clean, "{theme}: {}", outcome.stderr);
        let png = fs::read(out_dir.join("onepager.png")).unwrap();
        assert!(png.starts_with(b"\x89PNG"), "{theme}");
        svg_outputs.push(fs::read_to_string(out_dir.join("onepager.svg")).unwrap());
        measured_outputs.push(fs::read(out_dir.join("onepager.measured.json")).unwrap());
    }
    assert_eq!(measured_outputs[0], measured_outputs[1]);
    assert_eq!(measured_outputs[0], measured_outputs[2]);
    assert_ne!(svg_outputs[0], svg_outputs[1]);
    assert_ne!(svg_outputs[0], svg_outputs[2]);
    for svg in &svg_outputs {
        for index in 0..7 {
            assert!(
                svg.contains(&format!(r#"data-id="/links/{index}" data-tag="Link""#)),
                "link {index}"
            );
        }
    }

    let measured: Value = serde_json::from_slice(&measured_outputs[0]).unwrap();
    let links = measured["links"].as_array().unwrap();
    assert_eq!(links.len(), 7);
    for link in links {
        assert_eq!(link["status"], "routed", "{}", link["id"]);
        assert!(link["tag"].is_object(), "{}", link["id"]);
    }
}
