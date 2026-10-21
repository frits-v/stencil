#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! `stencil check` and `stencil vet` print, for every example, the lines captured before the
//! core vocabulary replaced Zone, Pcard and the color-named kinds (section 13.14).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use stencil_cli::pipeline::{all_checks, load_document_from, read_input, render_page};
use stencil_cli::{ExitCode, run};
use stencil_render::DeviceScale;

const STEMS: [&str; 10] = [
    "g7",
    "hero-iso",
    "hybrid-ai",
    "network-hub-spoke",
    "onepager",
    "onprem-iso",
    "people-iso",
    "platform-iso",
    "stress-dense",
    "transit-iso",
];

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn run_stencil(subcommand: &str, document: &Path) -> (ExitCode, String) {
    let argv: Vec<OsString> = vec![
        "stencil".into(),
        subcommand.into(),
        document.as_os_str().to_os_string(),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = run(argv, &mut stdout, &mut stderr);
    (code, String::from_utf8(stdout).unwrap())
}

fn assert_lines_equal_fixture(subcommand: &str) {
    let mut examined = 0;
    for stem in STEMS {
        let document = repository_path(&format!("examples/{stem}.json"));
        let fixture = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(format!("{stem}.{subcommand}.txt")),
        )
        .unwrap();
        let (code, stdout) = run_stencil(subcommand, &document);
        assert_eq!(code, ExitCode::Clean, "{stem}: stencil {subcommand} failed");
        assert_eq!(
            stdout, fixture,
            "{stem}: stencil {subcommand} lines changed"
        );
        examined += 1;
    }
    assert_eq!(examined, STEMS.len());
}

#[test]
fn check_prints_the_captured_lines_for_every_example() {
    assert_lines_equal_fixture("check");
}

#[test]
fn vet_prints_the_captured_lines_for_every_example() {
    assert_lines_equal_fixture("vet");
}

#[test]
fn the_captured_check_lines_follow_check_name_order() {
    let mut examined = 0;
    for stem in STEMS {
        let document = repository_path(&format!("examples/{stem}.json"));
        let json_text = read_input(&document).unwrap();
        let loaded = load_document_from(&json_text, document.parent().unwrap(), None).unwrap();
        let rendered = render_page(&loaded, DeviceScale::DEFAULT).unwrap();
        let reports = all_checks(
            &loaded.page,
            &loaded.grammar,
            &rendered.geometry,
            rendered.scene.as_ref(),
            None,
        );
        let fixture = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(format!("{stem}.check.txt")),
        )
        .unwrap();
        let lines: Vec<&str> = fixture.lines().collect();
        let check_lines: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|line| line.starts_with("check "))
            .collect();
        assert_eq!(check_lines.len(), reports.len(), "{stem}");
        for (line, report) in check_lines.iter().zip(&reports) {
            let prefix = format!("check {}: ", report.check.as_str());
            assert!(line.starts_with(&prefix), "{stem}: {line} is not {prefix}");
        }
        let trailer = format!("stencil check: {} checks, ", reports.len());
        assert_eq!(lines.len(), reports.len() + 1, "{stem}");
        assert!(
            lines.last().is_some_and(|last| last.starts_with(&trailer)),
            "{stem}: {:?}",
            lines.last()
        );
        examined += 1;
    }
    assert_eq!(examined, STEMS.len());
}
