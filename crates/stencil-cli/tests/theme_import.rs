#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! `stencil theme import --base16` and the imported tier (sections 13.9 and 13.14).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use stencil_cli::{ExitCode, run};
use stencil_model::theme::lightness;
use stencil_model::{IMPORTED_THEMES, Theme, parse_theme};

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

fn imported_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../stencil-render/themes/imported")
}

fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli-theme-import")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn path_text(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// Imports `scheme` into `<directory>/<name>.json` and returns the outcome.
fn import(scheme: &Path, name: Option<&str>, theme_path: &Path) -> Outcome {
    let mut arguments = vec!["theme", "import", "--base16", path_text(scheme)];
    if let Some(name) = name {
        arguments.extend(["--name", name]);
    }
    arguments.extend(["-o", path_text(theme_path)]);
    run_stencil(&arguments)
}

fn files_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The pinned tokyo-night scheme, in the tinted-theming `palette:` layout.
fn tokyo_night_scheme() -> String {
    fs::read_to_string(imported_directory().join("tokyo-night.base16.yaml")).unwrap()
}

/// The sixteen `baseXX: "#RRGGBB"` lines of the tokyo-night scheme, keys and values.
fn tokyo_night_colors() -> Vec<(String, String)> {
    tokyo_night_scheme()
        .lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once(':')?;
            key.starts_with("base")
                .then(|| (key.to_string(), value.trim().trim_matches('"').to_string()))
        })
        .collect()
}

fn write_scheme(directory: &Path, file_name: &str, text: &str) -> PathBuf {
    let path = directory.join(file_name);
    fs::write(&path, text).unwrap();
    path
}

fn imported_theme(directory: &Path, name: &str) -> Theme {
    let text = fs::read_to_string(directory.join(format!("{name}.json"))).unwrap();
    parse_theme(&text, name).unwrap()
}

#[test]
fn every_pinned_scheme_reimports_to_its_committed_theme_and_report_byte_for_byte() {
    let directory = scratch_directory("reimport");
    let mut examined = 0;
    for name in IMPORTED_THEMES {
        let scheme = imported_directory().join(format!("{name}.base16.yaml"));
        let theme_path = directory.join(format!("{name}.json"));
        let outcome = import(&scheme, Some(name), &theme_path);
        // None of the tier passes the thresholds (section 13.9), so every import exits 1.
        assert_eq!(
            outcome.code,
            ExitCode::Defects,
            "{name}: {}",
            outcome.stderr
        );
        assert_eq!(outcome.stderr, "");

        let committed_theme =
            fs::read_to_string(imported_directory().join(format!("{name}.json"))).unwrap();
        let written_theme = fs::read_to_string(&theme_path).unwrap();
        assert!(
            written_theme == committed_theme,
            "{name}.json differs from a re-import; run `mise run import-themes`"
        );
        let committed_report =
            fs::read_to_string(imported_directory().join(format!("{name}.report.txt"))).unwrap();
        let report_path = directory.join(format!("{name}.report.txt"));
        let written_report = fs::read_to_string(&report_path).unwrap();
        assert!(
            written_report == committed_report,
            "{name}.report.txt differs from a re-import; run `mise run import-themes`"
        );

        let expected_stdout = format!(
            "{committed_report}{}\n{}\n",
            std::path::absolute(&theme_path).unwrap().display(),
            std::path::absolute(&report_path).unwrap().display()
        );
        assert_eq!(outcome.stdout, expected_stdout, "{name}");
        examined += 1;
    }
    assert_eq!(examined, 7);
}

#[test]
fn every_committed_report_is_what_theme_check_prints_for_the_built_in() {
    let mut examined = 0;
    for name in IMPORTED_THEMES {
        let committed =
            fs::read_to_string(imported_directory().join(format!("{name}.report.txt"))).unwrap();
        let outcome = run_stencil(&["theme", "check", name]);
        assert_eq!(outcome.code, ExitCode::Defects, "{name}");
        assert!(outcome.stdout == committed, "{name}");
        let summary = committed.lines().last().unwrap();
        assert!(
            summary.starts_with(&format!("stencil theme check {name}: 223 rows, ")),
            "{summary}"
        );
        examined += 1;
    }
    assert_eq!(examined, 7);
}

#[test]
fn every_pinned_scheme_hashes_to_its_sources_line() {
    let sources = fs::read_to_string(imported_directory().join("SOURCES.md")).unwrap();
    let mut examined = 0;
    for name in IMPORTED_THEMES {
        let bytes = fs::read(imported_directory().join(format!("{name}.base16.yaml"))).unwrap();
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let line = sources
            .lines()
            .find(|line| line.starts_with(&format!("| {name} | ")))
            .unwrap_or_else(|| panic!("SOURCES.md has no line for {name}"));
        assert!(line.contains(&format!("`{digest}`")), "{name}: {line}");
        assert!(
            line.contains(
                "https://raw.githubusercontent.com/tinted-theming/schemes/d70255b752ac8328ee3d549c72a1a55ce5fc794f/base16/"
            ),
            "{name}: {line}"
        );
        examined += 1;
    }
    assert_eq!(examined, 7);
}

#[test]
fn every_imported_theme_names_its_slots_by_the_fixed_table() {
    for name in IMPORTED_THEMES {
        let theme = stencil_render::builtin_theme(name).unwrap().unwrap();
        let names: Vec<&str> = theme.tints.iter().map(|tint| tint.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "blue", "pink", "cyan", "yellow", "red", "green", "orange", "brown"
            ],
            "{name}"
        );
        assert_eq!(
            theme.tints[0].wire, theme.kicker,
            "{name}: slot 1 is base0D"
        );
        assert_eq!(
            theme.deny.color, theme.tints[4].wire,
            "{name}: deny is base08"
        );
    }
}

#[test]
fn a_scheme_missing_base0b_exits_1_and_writes_nothing() {
    let directory = scratch_directory("missing-key");
    let text: String = tokyo_night_scheme()
        .lines()
        .filter(|line| !line.trim_start().starts_with("base0B"))
        .map(|line| format!("{line}\n"))
        .collect();
    let scheme = write_scheme(&directory, "partial.yaml", &text);
    let outcome = import(&scheme, None, &directory.join("partial.json"));
    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout,
        format!("error base16 {}: missing base0B\n", scheme.display())
    );
    assert_eq!(files_in(&directory), ["partial.yaml"]);
}

#[test]
fn a_key_given_twice_exits_1_and_writes_nothing() {
    let directory = scratch_directory("repeated-key");
    let text = format!("{}  base0d: \"#000000\"\n", tokyo_night_scheme());
    let scheme = write_scheme(&directory, "twice.yaml", &text);
    let outcome = import(&scheme, None, &directory.join("twice.json"));
    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(
        outcome.stdout.starts_with("error base16 ")
            && outcome.stdout.contains("base0D is given twice"),
        "{}",
        outcome.stdout
    );
    assert_eq!(files_in(&directory), ["twice.yaml"]);
}

#[test]
fn a_system_other_than_base16_or_base24_exits_1() {
    let directory = scratch_directory("system");
    let text = tokyo_night_scheme().replace("system: \"base16\"", "system: \"base17\"");
    let scheme = write_scheme(&directory, "odd.yaml", &text);
    let outcome = import(&scheme, None, &directory.join("odd.json"));
    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(outcome.stdout.contains("\"base17\""), "{}", outcome.stdout);
    assert_eq!(files_in(&directory), ["odd.yaml"]);
}

#[test]
fn a_scheme_over_65536_bytes_exits_1_and_writes_nothing() {
    let directory = scratch_directory("oversize");
    let padding = "# padding\n".repeat(6_600);
    let scheme = write_scheme(
        &directory,
        "large.yaml",
        &format!("{}{padding}", tokyo_night_scheme()),
    );
    let outcome = import(&scheme, None, &directory.join("large.json"));
    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(
        outcome.stdout.contains("larger than 65536 bytes"),
        "{}",
        outcome.stdout
    );
    assert_eq!(files_in(&directory), ["large.yaml"]);
}

#[test]
fn an_unreadable_scheme_exits_2_with_empty_stdout() {
    let directory = scratch_directory("unreadable");
    let outcome = import(
        &directory.join("absent.yaml"),
        None,
        &directory.join("absent.json"),
    );
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("cannot read scheme"),
        "{}",
        outcome.stderr
    );
    let as_directory = import(&directory, None, &directory.join("dir.json"));
    assert_eq!(as_directory.code, ExitCode::CouldNotRun);
    assert!(files_in(&directory).is_empty());
}

#[test]
fn a_failed_write_exits_2() {
    let directory = scratch_directory("unwritable");
    let scheme = imported_directory().join("nord.base16.yaml");
    let outcome = import(&scheme, Some("nord"), &directory.join("missing/nord.json"));
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("cannot write"),
        "{}",
        outcome.stderr
    );
}

#[test]
fn a_base24_scheme_gives_the_theme_of_its_first_sixteen_keys() {
    let directory = scratch_directory("base24");
    let base16_text = tokyo_night_scheme();
    let extra: String = (0x10..=0x17)
        .map(|key| format!("  base{key:02X}: \"#FF00FF\"\n"))
        .collect();
    let base24_text = format!("{}{extra}", base16_text.replace("base16", "base24"));
    let base16 = write_scheme(&directory, "sixteen.yaml", &base16_text);
    let base24 = write_scheme(&directory, "twentyfour.yaml", &base24_text);
    let first = import(&base16, Some("same"), &directory.join("first.json"));
    let second = import(&base24, Some("same"), &directory.join("second.json"));
    assert_eq!(first.code, ExitCode::Defects);
    assert_eq!(second.code, ExitCode::Defects);
    assert_eq!(
        fs::read(directory.join("first.json")).unwrap(),
        fs::read(directory.join("second.json")).unwrap()
    );
}

#[test]
fn a_legacy_flat_scheme_with_bare_hex_imports_as_the_palette_layout() {
    let directory = scratch_directory("legacy");
    let mut legacy = String::from("scheme: \"Tokyo Night Dark\"\nauthor: \"Michaël Ball\"\n");
    for (key, value) in tokyo_night_colors() {
        let bare = value.trim_start_matches('#').to_lowercase();
        legacy.push_str(&format!("{key}: \"{bare}\" # {key}\n"));
    }
    let scheme = write_scheme(&directory, "legacy.yaml", &legacy);
    let outcome = import(
        &scheme,
        Some("tokyo-night"),
        &directory.join("tokyo-night.json"),
    );
    assert_eq!(outcome.code, ExitCode::Defects, "{}", outcome.stdout);
    assert_eq!(
        fs::read_to_string(directory.join("tokyo-night.json")).unwrap(),
        fs::read_to_string(imported_directory().join("tokyo-night.json")).unwrap()
    );
}

#[test]
fn the_name_defaults_to_the_sanitized_file_stem() {
    let directory = scratch_directory("stem-name");
    let scheme = write_scheme(&directory, "Tokyo_Night.yaml", &tokyo_night_scheme());
    let outcome = import(&scheme, None, &directory.join("out.json"));
    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(imported_theme(&directory, "out").name, "tokyo-night");
    assert!(directory.join("out.report.txt").exists());
    let summary = outcome.stdout.lines().rev().nth(2).unwrap();
    assert!(
        summary.starts_with("stencil theme check tokyo-night: "),
        "{summary}"
    );
}

#[test]
fn a_name_outside_the_theme_name_pattern_exits_1_and_writes_nothing() {
    let directory = scratch_directory("bad-name");
    let scheme = write_scheme(&directory, "9lives.yaml", &tokyo_night_scheme());
    let from_stem = import(&scheme, None, &directory.join("lives.json"));
    assert_eq!(from_stem.code, ExitCode::Defects);
    assert!(
        from_stem.stdout.contains("pass --name"),
        "{}",
        from_stem.stdout
    );
    let from_flag = import(&scheme, Some("Night"), &directory.join("lives.json"));
    assert_eq!(from_flag.code, ExitCode::Defects);
    assert_eq!(files_in(&directory), ["9lives.yaml"]);
}

#[test]
fn a_card_surface_far_from_the_page_is_capped_at_fourteen_lightness() {
    for (name, base00, base02) in [
        ("solarized-light", "#FDF6E3", "#93A1A1"),
        ("solarized-dark", "#002B36", "#586E75"),
    ] {
        let theme = stencil_render::builtin_theme(name).unwrap().unwrap();
        assert_eq!(theme.page.as_str(), base00);
        assert_ne!(theme.card.fill.as_str(), base02, "{name}");
        let step = (lightness(&theme.page).unwrap() - lightness(&theme.card.fill).unwrap()).abs();
        assert!((13.0..=14.5).contains(&step), "{name}: {step}");
    }
    // nord's base02 lies within 14 L* of its page, so the card keeps it.
    let nord = stencil_render::builtin_theme("nord").unwrap().unwrap();
    assert_eq!(nord.card.fill.as_str(), "#434C5E");
}

#[test]
fn a_light_ink_is_ink_primary_never_base07_alone() {
    // nord's base07 is the teal accent #8FBCBB; its brightest neutral is base06.
    let nord = stencil_render::builtin_theme("nord").unwrap().unwrap();
    assert_eq!(nord.ink.primary.as_str(), "#ECEFF4");
    for tint in &nord.tints {
        assert_ne!(tint.ink.as_str(), "#8FBCBB", "{}", tint.name);
    }
    assert_ne!(nord.iso.tabs.top.ink.as_str(), "#8FBCBB");
}
