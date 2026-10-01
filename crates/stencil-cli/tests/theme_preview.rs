#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! `stencil theme preview` and its figure, which every built-in theme draws in the README.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use stencil_cli::{ExitCode, run};
use stencil_model::BUILTIN_THEMES;

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

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn preview_value() -> Value {
    serde_json::from_str(&fs::read_to_string(manifest_path("preview/theme-preview.json")).unwrap())
        .unwrap()
}

fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-theme-preview")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

/// Every object in the document, depth first.
fn objects(value: &Value) -> Vec<&serde_json::Map<String, Value>> {
    let mut found = Vec::new();
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
        match current {
            Value::Object(map) => {
                found.push(map);
                pending.extend(map.values());
            }
            Value::Array(items) => pending.extend(items),
            _ => {}
        }
    }
    found
}

#[test]
fn the_preview_passes_check_under_every_built_in_theme() {
    let directory = scratch_directory("check");
    let mut examined = 0;
    for name in BUILTIN_THEMES {
        let mut document = preview_value();
        document["kicker"] = Value::String(format!("theme preview · {name}"));
        let path = directory.join(format!("{name}.json"));
        fs::write(&path, document.to_string()).unwrap();
        let outcome = run_stencil(&["check", path.to_str().unwrap(), "--theme", name]);
        assert_eq!(
            outcome.code,
            ExitCode::Clean,
            "{name}: {}{}",
            outcome.stdout,
            outcome.stderr
        );
        assert!(!outcome.stdout.contains("defect "), "{name}");
        examined += 1;
    }
    assert_eq!(examined, 13);
}

#[test]
fn the_preview_uses_every_tint_tone_and_line_kind() {
    let document = preview_value();
    let nodes = objects(&document);
    let field_values = |field: &str| -> BTreeSet<String> {
        nodes
            .iter()
            .filter_map(|node| node.get(field))
            .filter_map(|value| match value {
                Value::String(text) => Some(text.clone()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            })
            .collect()
    };
    for kind in [
        "gcp",
        "vpc",
        "region",
        "subnet",
        "onprem",
        "project",
        "optional",
        "k8s",
        "perimeter",
        "apis",
        "product",
    ] {
        assert!(field_values("kind").contains(kind), "{kind}");
    }
    for tag in ["Tee", "Pipe", "Callout", "Text", "Item", "Box"] {
        assert!(field_values("tag").contains(tag), "{tag}");
    }
    for line in ["solid", "dash", "gray", "deny"] {
        assert!(field_values("line").contains(line), "{line}");
    }
    let region_tints: BTreeSet<String> = nodes
        .iter()
        .filter(|node| node.get("kind") == Some(&Value::String("region".to_string())))
        .filter_map(|node| node.get("tint").map(Value::to_string))
        .collect();
    assert_eq!(region_tints.len(), 8);
    assert_eq!(document["links"].as_array().unwrap().len(), 1);
    assert_eq!(document["legend"].as_array().unwrap().len(), 5);
    assert_eq!(document["width"], 1100);
}

#[test]
fn theme_preview_writes_the_committed_readme_png() {
    let directory = scratch_directory("png");
    let mut examined = 0;
    for name in ["center", "nord"] {
        let png = directory.join(format!("{name}.png"));
        let outcome = run_stencil(&[
            "theme",
            "preview",
            name,
            "-o",
            png.to_str().unwrap(),
            "--scale",
            "1",
        ]);
        assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
        assert_eq!(
            outcome.stdout,
            format!("{}\n", std::path::absolute(&png).unwrap().display())
        );
        let committed = fs::read(manifest_path(&format!("../../docs/themes/{name}.png"))).unwrap();
        assert!(
            fs::read(&png).unwrap() == committed,
            "docs/themes/{name}.png is out of date; run mise run theme-docs"
        );
        examined += 1;
    }
    assert_eq!(examined, 2);
}

#[test]
fn a_theme_file_previews_like_its_built_in() {
    let directory = scratch_directory("path");
    let from_name = directory.join("name.png");
    let from_path = directory.join("path.png");
    let theme_path = manifest_path("../stencil-render/themes/imported/dracula.json");
    for (reference, png) in [
        ("dracula", &from_name),
        (theme_path.to_str().unwrap(), &from_path),
    ] {
        let outcome = run_stencil(&["theme", "preview", reference, "-o", png.to_str().unwrap()]);
        assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    }
    assert_eq!(fs::read(&from_name).unwrap(), fs::read(&from_path).unwrap());
}

#[test]
fn a_missing_theme_file_exits_2_and_writes_nothing() {
    let directory = scratch_directory("missing");
    let png = directory.join("out.png");
    let outcome = run_stencil(&[
        "theme",
        "preview",
        "absent.json",
        "-o",
        png.to_str().unwrap(),
    ]);
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(!png.exists());
}
