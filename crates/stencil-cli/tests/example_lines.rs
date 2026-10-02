#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! `stencil check` and `stencil vet` print, for every example, the lines captured before the
//! core vocabulary replaced Zone, Pcard and the color-named kinds (section 13.14).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use stencil_cli::{ExitCode, run};

const STEMS: [&str; 7] = [
    "g7",
    "hero-iso",
    "hybrid-ai",
    "network-hub-spoke",
    "onepager",
    "platform-iso",
    "stress-dense",
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
