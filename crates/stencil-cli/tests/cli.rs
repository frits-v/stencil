#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

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

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn g7_path() -> String {
    repository_path("examples/g7.json")
        .to_str()
        .unwrap()
        .to_string()
}

fn g7_document() -> Value {
    serde_json::from_str(&fs::read_to_string(g7_path()).unwrap()).unwrap()
}

/// A fresh, empty directory under target/tmp, one per test.
fn scratch_directory(test_name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stencil-cli")
        .join(test_name);
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn write_document(test_name: &str, file_name: &str, document: &Value) -> String {
    let path = scratch_directory(test_name).join(file_name);
    fs::write(&path, serde_json::to_string_pretty(document).unwrap()).unwrap();
    path.to_str().unwrap().to_string()
}

fn minimal_page(body: Value, legend: Value) -> Value {
    json!({
        "title": "Minimal figure",
        "kicker": "Test",
        "lede": "One row.",
        "canvas": "internal",
        "body": body,
        "legend": legend,
    })
}

#[test]
fn vet_g7_passes_both_model_checks() {
    let outcome = run_stencil(&["vet", &g7_path()]);
    assert_eq!(outcome.code, ExitCode::Clean);
    assert_eq!(
        outcome.stdout_lines(),
        vec![
            "check remembered-constants: examined 36 text fields, 0 defects",
            "check legend-consistency: examined 8 relations, 0 defects",
            "stencil vet: 0 violations, 2 checks, 2 passed, 0 failed",
        ]
    );
    assert_eq!(outcome.stderr, "");
}

#[test]
fn vet_reports_a_remembered_constant_in_a_pipe_sub() {
    let mut document = g7_document();
    let vlan_1 = document
        .pointer_mut("/body/0/children/1/children/0/children/0")
        .unwrap();
    vlan_1["sub"] = json!("EAD 1 · BGP ASN 64512");
    let path = write_document("vet_remembered_constant", "g7-asn.json", &document);

    let outcome = run_stencil(&["vet", &path]);

    assert_eq!(outcome.code, ExitCode::Defects);
    let lines = outcome.stdout_lines();
    assert_eq!(
        lines[0],
        "check remembered-constants: examined 36 text fields, 1 defect"
    );
    assert!(
        lines[1].starts_with(
            "defect remembered-constants /body/0/children/1/children/0/children/0/sub: "
        ),
        "{}",
        lines[1]
    );
    assert!(lines[1].contains("64512"), "{}", lines[1]);
    assert_eq!(
        lines.last().unwrap(),
        &"stencil vet: 0 violations, 2 checks, 1 passed, 1 failed"
    );
}

#[test]
fn vet_fails_a_page_with_no_pipes_because_nothing_was_examined() {
    let document = minimal_page(
        json!([{ "tag": "Pcard", "fn": "Service", "pn": "Cloud Run" }]),
        json!([]),
    );
    let path = write_document("vet_nothing_examined", "no-pipes.json", &document);

    let outcome = run_stencil(&["vet", &path]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(
        outcome
            .stdout_lines()
            .contains(&"check legend-consistency: examined 0 relations, FAILED: nothing examined"),
        "{}",
        outcome.stdout
    );
    assert_eq!(
        outcome.stdout_lines().last().unwrap(),
        &"stencil vet: 0 violations, 2 checks, 1 passed, 1 failed"
    );
}

#[test]
fn vet_prints_a_violation_and_does_not_run_checks() {
    let mut document = g7_document();
    document["body"][0]["gap"] = json!(65);
    let path = write_document("vet_violation", "g7-gap.json", &document);

    let outcome = run_stencil(&["vet", &path]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout_lines(),
        vec![
            "violation gap-out-of-range /body/0/gap: gap 65 is above 64",
            "stencil vet: 1 violation, checks not run",
        ]
    );
}

#[test]
fn vet_on_malformed_json_is_a_defect() {
    let directory = scratch_directory("vet_malformed");
    let path = directory.join("broken.json");
    fs::write(&path, "{ \"title\": ").unwrap();

    let outcome = run_stencil(&["vet", path.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    let lines = outcome.stdout_lines();
    assert_eq!(lines.len(), 2, "{}", outcome.stdout);
    assert!(
        lines[0].starts_with("error document is not valid stencil JSON at line 1, column "),
        "{}",
        lines[0]
    );
    assert_eq!(
        lines[1],
        "stencil vet: document does not parse, checks not run"
    );
    assert_eq!(outcome.stderr, "");
}

#[test]
fn vet_on_a_missing_required_field_is_a_defect() {
    let mut document = g7_document();
    document["legend"][0]
        .as_object_mut()
        .unwrap()
        .remove("kind");
    let path = write_document("vet_missing_field", "g7-no-kind.json", &document);

    let outcome = run_stencil(&["vet", &path]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert!(
        outcome.stdout.contains("missing field `kind`"),
        "{}",
        outcome.stdout
    );
}

#[test]
fn vet_on_a_missing_file_could_not_run() {
    let path = scratch_directory("vet_missing_file").join("absent.json");

    let outcome = run_stencil(&["vet", path.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("cannot read input"),
        "{}",
        outcome.stderr
    );
    assert!(outcome.stderr.contains("caused by: "), "{}", outcome.stderr);
}

#[test]
fn check_and_render_on_a_missing_file_could_not_run() {
    let directory = scratch_directory("check_render_missing_file");
    let path = directory.join("absent.json");
    let out_dir = directory.join("out");

    for arguments in [
        vec!["check", path.to_str().unwrap()],
        vec![
            "render",
            path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ],
    ] {
        let outcome = run_stencil(&arguments);
        assert_eq!(outcome.code, ExitCode::CouldNotRun, "{arguments:?}");
        assert_eq!(outcome.stdout, "", "{arguments:?}");
        assert!(
            outcome.stderr.contains("cannot read input"),
            "{}",
            outcome.stderr
        );
    }
    assert!(!out_dir.exists());
}

#[cfg(unix)]
#[test]
fn input_past_the_byte_limit_could_not_run() {
    let outcome = run_stencil(&["vet", "/dev/zero"]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome
            .stderr
            .contains("input is larger than 67108864 bytes"),
        "{}",
        outcome.stderr
    );
}

#[test]
fn input_at_the_byte_limit_is_read() {
    let limit = usize::try_from(stencil_cli::pipeline::INPUT_BYTES_MAX).unwrap();
    let path = scratch_directory("input_at_byte_limit").join("padded.json");
    let mut bytes = serde_json::to_vec(&g7_document()).unwrap();
    bytes.resize(limit, b' ');
    fs::write(&path, &bytes).unwrap();

    let outcome = run_stencil(&["vet", path.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    fs::remove_file(&path).unwrap();
}

#[test]
fn a_document_past_the_node_limit_is_a_vet_violation() {
    // 16 Rows + 256 cards + 15 * 255 cards = 4097 nodes, no list above 256.
    let card = json!({ "tag": "Pcard", "fn": "a" });
    let mut rows = vec![json!({ "tag": "Row", "children": vec![card.clone(); 256] })];
    rows.extend((0..15).map(|_| json!({ "tag": "Row", "children": vec![card.clone(); 255] })));
    let document = minimal_page(Value::Array(rows), json!([]));
    let path = write_document("vet_nodes_exceeded", "large.json", &document);

    for command in ["vet", "check"] {
        let outcome = run_stencil(&[command, &path]);
        assert_eq!(
            outcome.code,
            ExitCode::Defects,
            "{command}: {}",
            outcome.stderr
        );
        assert_eq!(
            outcome.stdout_lines(),
            vec![
                "violation nodes-exceeded /body: more than 4096 nodes".to_string(),
                format!("stencil {command}: 1 violation, checks not run"),
            ],
            "{command}"
        );
    }
}

#[test]
fn unknown_subcommand_and_unknown_flag_could_not_run() {
    for arguments in [
        vec!["draw", "examples/g7.json"],
        vec!["vet", "--colour", "examples/g7.json"],
        vec![],
    ] {
        let outcome = run_stencil(&arguments);
        assert_eq!(outcome.code, ExitCode::CouldNotRun, "{arguments:?}");
        assert_eq!(outcome.stdout, "", "{arguments:?}");
        assert!(!outcome.stderr.is_empty(), "{arguments:?}");
    }
}

#[test]
fn help_and_version_go_to_stdout_with_exit_zero() {
    for arguments in [vec!["--help"], vec!["render", "--help"], vec!["--version"]] {
        let outcome = run_stencil(&arguments);
        assert_eq!(outcome.code, ExitCode::Clean, "{arguments:?}");
        assert!(!outcome.stdout.is_empty(), "{arguments:?}");
        assert_eq!(outcome.stderr, "", "{arguments:?}");
    }
    let version = run_stencil(&["--version"]);
    assert_eq!(
        version.stdout,
        format!("stencil {}\n", env!("CARGO_PKG_VERSION"))
    );
    let help = run_stencil(&["--help"]);
    for command in ["vet", "render", "check", "schema"] {
        assert!(help.stdout.contains(command), "{}", help.stdout);
    }
}

#[test]
fn render_writes_three_files_and_prints_their_absolute_paths() {
    let directory = scratch_directory("render_writes");
    let out_dir = directory.join("nested").join("out");

    let outcome = run_stencil(&["render", &g7_path(), "--out-dir", out_dir.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    let canonical = fs::canonicalize(&out_dir).unwrap();
    let expected: Vec<PathBuf> = ["g7.svg", "g7.png", "g7.measured.json"]
        .iter()
        .map(|name| canonical.join(name))
        .collect();
    let printed: Vec<PathBuf> = outcome.stdout_lines().iter().map(PathBuf::from).collect();
    assert_eq!(printed, expected);
    assert!(
        printed
            .iter()
            .all(|path| path.is_absolute() && path.is_file())
    );
    assert_eq!(outcome.stderr, "");

    let svg = fs::read_to_string(&expected[0]).unwrap();
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    let png = fs::read(&expected[1]).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let measured: Value = serde_json::from_slice(&fs::read(&expected[2]).unwrap()).unwrap();
    assert_eq!(measured["document"], g7_document());
    assert_eq!(measured["canvas"]["width"], json!(1320));
}

#[test]
fn render_overwrites_existing_outputs_with_identical_bytes() {
    let out_dir = scratch_directory("render_overwrites");
    let out_dir_text = out_dir.to_str().unwrap();
    fs::write(out_dir.join("g7.svg"), "stale").unwrap();

    let first = run_stencil(&["render", &g7_path(), "--out-dir", out_dir_text]);
    assert_eq!(first.code, ExitCode::Clean, "{}", first.stderr);
    let first_bytes: Vec<Vec<u8>> = first
        .stdout_lines()
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect();
    assert_ne!(first_bytes[0], b"stale");

    let second = run_stencil(&["render", &g7_path(), "--out-dir", out_dir_text]);
    assert_eq!(second.code, ExitCode::Clean, "{}", second.stderr);
    let second_bytes: Vec<Vec<u8>> = second
        .stdout_lines()
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect();
    assert_eq!(first_bytes, second_bytes);
}

#[test]
fn render_scale_one_halves_the_default_png() {
    let out_dir = scratch_directory("render_scale_one");

    let outcome = run_stencil(&[
        "render",
        &g7_path(),
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--scale",
        "1",
    ]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    let png = resvg::tiny_skia::Pixmap::load_png(out_dir.join("g7.png")).unwrap();
    assert_eq!(png.width(), 1320);
}

#[test]
fn render_rejects_scale_five_before_writing() {
    let out_dir = scratch_directory("render_scale_five").join("out");

    let outcome = run_stencil(&[
        "render",
        &g7_path(),
        "--out-dir",
        out_dir.to_str().unwrap(),
        "--scale",
        "5",
    ]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("device scale 5 is outside 1 to 4"),
        "{}",
        outcome.stderr
    );
    assert!(!out_dir.exists());
}

#[cfg(unix)]
#[test]
fn render_into_a_read_only_directory_could_not_run() {
    use std::os::unix::fs::PermissionsExt;

    let read_only = scratch_directory("render_read_only");
    fs::set_permissions(&read_only, fs::Permissions::from_mode(0o555)).unwrap();
    let out_dir = read_only.join("out");

    let outcome = run_stencil(&["render", &g7_path(), "--out-dir", out_dir.to_str().unwrap()]);

    fs::set_permissions(&read_only, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("cannot create output directory"),
        "{}",
        outcome.stderr
    );
}

#[test]
fn render_writes_files_when_the_legend_omits_a_used_kind() {
    let mut document = g7_document();
    document["legend"].as_array_mut().unwrap().pop();
    let path = write_document("render_legend_gap", "g7-no-dash.json", &document);
    let out_dir = scratch_directory("render_legend_gap_out");

    let outcome = run_stencil(&["render", &path, "--out-dir", out_dir.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    assert_eq!(outcome.stdout_lines().len(), 3);
    for name in [
        "g7-no-dash.svg",
        "g7-no-dash.png",
        "g7-no-dash.measured.json",
    ] {
        assert!(out_dir.join(name).is_file(), "{name}");
    }
}

#[test]
fn render_stops_on_a_violation_and_writes_nothing() {
    let mut document = g7_document();
    document["title"] = json!(" Four lines.");
    let path = write_document("render_violation", "g7-untrimmed.json", &document);
    let out_dir = scratch_directory("render_violation_out").join("out");

    let outcome = run_stencil(&["render", &path, "--out-dir", out_dir.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout_lines(),
        vec![
            "violation text-untrimmed /title: text starts or ends with whitespace",
            "stencil render: 1 violation, checks not run",
        ]
    );
    assert!(!out_dir.exists());
}

#[test]
fn check_g7_passes_six_checks_and_skips_the_link_checks() {
    let outcome = run_stencil(&["check", &g7_path()]);
    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    assert_eq!(
        outcome.stdout_lines(),
        vec![
            "check child-inside-container: examined 33 relations, 0 defects",
            "check siblings-do-not-overlap: examined 32 pairs, 0 defects",
            "check text-fits-box: examined 40 text runs, 0 defects",
            "check remembered-constants: examined 36 text fields, 0 defects",
            "check legend-consistency: examined 8 relations, 0 defects",
            "check links-routed: examined 0 links, not applicable: page has no links",
            "check links-avoid-boxes: examined 0 pairs, not applicable: page has no links",
            "check pipes-land: examined 8 pipe ends, 0 defects",
            "stencil check: 8 checks, 6 passed, 0 failed, 2 not applicable",
        ]
    );
    assert_eq!(outcome.stderr, "");
}

/// One Pipe and its legend entry make legend-consistency pass, so the overflow is the only
/// failing check and the exit code depends on it. The Pipe sits in the body with no Row or
/// Col sibling, so pipes-land does not apply.
#[test]
fn check_reports_overflowing_text_with_its_pointer() {
    let long_word = "x".repeat(120);
    let mut document = minimal_page(
        json!([
            {
                "tag": "Row",
                "children": [
                    { "tag": "Pcard", "fn": long_word, "pn": "Cloud Run" },
                    { "tag": "Pcard", "fn": "Second", "pn": "Cloud Run" },
                    { "tag": "Pcard", "fn": "Third", "pn": "Cloud Run" }
                ]
            },
            { "tag": "Pipe", "dir": "h", "kind": "blue", "label": "request" }
        ]),
        json!([{ "kind": "blue", "text": "request path" }]),
    );
    document["width"] = json!(640);
    let path = write_document("check_overflow", "overflow.json", &document);

    let outcome = run_stencil(&["check", &path]);

    assert_eq!(outcome.code, ExitCode::Defects, "{}", outcome.stdout);
    let lines = outcome.stdout_lines();
    let check_lines: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.starts_with("check "))
        .collect();
    assert_eq!(check_lines.len(), 8, "{}", outcome.stdout);
    assert!(
        check_lines[0].starts_with("check child-inside-container: examined "),
        "{}",
        outcome.stdout
    );
    assert!(
        !check_lines[0].ends_with(", 0 defects"),
        "{}",
        outcome.stdout
    );
    for passing in &check_lines[1..5] {
        assert!(passing.ends_with(", 0 defects"), "{}", outcome.stdout);
    }
    for skipped in &check_lines[5..7] {
        assert!(
            skipped.ends_with("not applicable: page has no links"),
            "{}",
            outcome.stdout
        );
    }
    assert_eq!(
        check_lines[7],
        "check pipes-land: examined 0 pipe ends, not applicable: no pipe has a neighbor",
        "{}",
        outcome.stdout
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("defect child-inside-container /body/0/children/0: ")),
        "{}",
        outcome.stdout
    );
    assert_eq!(
        lines.last().copied(),
        Some("stencil check: 8 checks, 4 passed, 1 failed, 3 not applicable"),
        "{}",
        outcome.stdout
    );
}

fn document_with_missing_glyph() -> Value {
    let mut document = g7_document();
    document["body"][0]["children"][0]["children"][0]["children"][0]["fn"] =
        json!("On-prem router \u{4E00}");
    document
}

#[test]
fn check_reports_a_missing_glyph_as_an_error_line() {
    let path = write_document(
        "check_missing_glyph",
        "g7-cjk.json",
        &document_with_missing_glyph(),
    );

    let outcome = run_stencil(&["check", &path]);

    assert_eq!(outcome.code, ExitCode::Defects);
    let lines = outcome.stdout_lines();
    assert_eq!(lines.len(), 2, "{}", outcome.stdout);
    assert!(
        lines[0].starts_with(
            "error text at /body/0/children/0/children/0/children/0/fn could not be measured: "
        ),
        "{}",
        lines[0]
    );
    assert!(lines[0].contains("U+4E00"), "{}", lines[0]);
    assert_eq!(lines[1], "stencil check: checks not run");
    assert_eq!(outcome.stderr, "");
}

#[test]
fn render_with_a_missing_glyph_writes_no_files() {
    let path = write_document(
        "render_missing_glyph",
        "g7-cjk.json",
        &document_with_missing_glyph(),
    );
    let out_dir = scratch_directory("render_missing_glyph_out").join("out");

    let outcome = run_stencil(&["render", &path, "--out-dir", out_dir.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout_lines().last().unwrap(),
        &"stencil render: checks not run"
    );
    assert!(!out_dir.exists());
}

#[test]
fn check_with_a_violation_does_not_run_checks() {
    let mut document = g7_document();
    document["body"][0]["grow"] = json!([0, 1]);
    let path = write_document("check_violation", "g7-grow.json", &document);

    let outcome = run_stencil(&["check", &path]);

    assert_eq!(outcome.code, ExitCode::Defects);
    assert_eq!(
        outcome.stdout_lines(),
        vec![
            "violation grow-length-mismatch /body/0/grow: grow has 2 weights for 3 children",
            "stencil check: 1 violation, checks not run",
        ]
    );
}

#[test]
fn schema_prints_the_committed_schema() {
    let committed = fs::read_to_string(repository_path("schema/stencil.schema.json")).unwrap();

    let outcome = run_stencil(&["schema"]);

    assert_eq!(outcome.code, ExitCode::Clean);
    assert_eq!(outcome.stdout, committed);
    assert_eq!(outcome.stderr, "");
}

#[test]
fn input_that_is_not_utf8_is_a_parse_defect() {
    let directory = scratch_directory("not_utf8");
    let path = directory.join("latin1.json");
    fs::write(&path, b"{\n  \"title\": \"\xff\"}").unwrap();
    let path_text = path.to_str().unwrap();

    for command in ["vet", "check"] {
        let outcome = run_stencil(&[command, path_text]);
        assert_eq!(
            outcome.code,
            ExitCode::Defects,
            "{command}: {}",
            outcome.stderr
        );
        assert_eq!(
            outcome.stdout_lines(),
            vec![
                "error document is not valid stencil JSON at line 2, column 13: input is not valid UTF-8".to_string(),
                format!("stencil {command}: document does not parse, checks not run"),
            ]
        );
        assert_eq!(outcome.stderr, "");
    }
}

#[test]
fn render_refuses_to_overwrite_its_input() {
    let directory = scratch_directory("render_over_input");
    let input = directory.join("figure.svg");
    let original = fs::read(g7_path()).unwrap();
    fs::write(&input, &original).unwrap();

    let outcome = run_stencil(&[
        "render",
        input.to_str().unwrap(),
        "--out-dir",
        directory.to_str().unwrap(),
    ]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun);
    assert_eq!(outcome.stdout, "");
    assert!(
        outcome.stderr.contains("is the input file"),
        "{}",
        outcome.stderr
    );
    assert_eq!(fs::read(&input).unwrap(), original);
    assert!(!directory.join("figure.png").exists());
    assert!(!directory.join("figure.measured.json").exists());
}

#[cfg(unix)]
#[test]
fn render_replaces_a_symlink_at_an_output_name_instead_of_following_it() {
    let directory = scratch_directory("render_symlink");
    let outside = directory.join("outside.txt");
    fs::write(&outside, "keep").unwrap();
    let out_dir = directory.join("out");
    fs::create_dir_all(&out_dir).unwrap();
    std::os::unix::fs::symlink(&outside, out_dir.join("g7.svg")).unwrap();

    let outcome = run_stencil(&["render", &g7_path(), "--out-dir", out_dir.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::Clean, "{}", outcome.stderr);
    assert_eq!(fs::read_to_string(&outside).unwrap(), "keep");
    let svg_metadata = fs::symlink_metadata(out_dir.join("g7.svg")).unwrap();
    assert!(svg_metadata.file_type().is_file());
    let leftovers: Vec<_> = fs::read_dir(&out_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn a_directory_at_an_output_name_leaves_existing_outputs_untouched() {
    let out_dir = scratch_directory("render_directory_at_output");
    let old_svg = b"<svg>old</svg>".to_vec();
    fs::write(out_dir.join("g7.svg"), &old_svg).unwrap();
    fs::create_dir(out_dir.join("g7.png")).unwrap();

    let outcome = run_stencil(&["render", &g7_path(), "--out-dir", out_dir.to_str().unwrap()]);

    assert_eq!(outcome.code, ExitCode::CouldNotRun, "{}", outcome.stdout);
    assert_eq!(outcome.stdout, "");
    assert!(outcome.stderr.contains("g7.png"), "{}", outcome.stderr);
    assert!(
        fs::read(out_dir.join("g7.svg")).unwrap() == old_svg,
        "g7.svg was replaced"
    );
    assert!(out_dir.join("g7.png").is_dir());
    assert!(!out_dir.join("g7.measured.json").exists());
    let leftovers: Vec<_> = fs::read_dir(&out_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

/// 160 stacked cards give a canvas of about 8400 px: under the 2^27 pixel budget at
/// scale 1 (1320 wide) and above it at scale 4 (5280 wide, about 33,600 tall).
#[test]
fn a_vetted_tall_document_is_rejected_above_the_pixel_budget() {
    let cards: Vec<Value> = (0..160)
        .map(|index| json!({ "tag": "Pcard", "fn": format!("Card {index}") }))
        .collect();
    let document = minimal_page(Value::Array(cards), json!([]));
    let path = write_document("tall_budget", "tall.json", &document);
    let out_dir = scratch_directory("tall_budget_out").join("out");
    let out_dir_text = out_dir.to_str().unwrap();

    let too_large = run_stencil(&["render", &path, "--out-dir", out_dir_text, "--scale", "4"]);
    assert_eq!(
        too_large.code,
        ExitCode::CouldNotRun,
        "{}",
        too_large.stdout
    );
    assert_eq!(too_large.stdout, "");
    assert!(
        too_large.stderr.contains("cannot allocate a 5280x"),
        "{}",
        too_large.stderr
    );
    assert!(!out_dir.exists());

    let within = run_stencil(&["render", &path, "--out-dir", out_dir_text, "--scale", "1"]);
    assert_eq!(within.code, ExitCode::Clean, "{}", within.stderr);
    let png = resvg::tiny_skia::Pixmap::load_png(out_dir.join("tall.png")).unwrap();
    assert_eq!(png.width(), 1320);
    assert!(png.height() > 8000, "{}", png.height());
}

/// Two separate processes, so a per-process hash seed or map order would show up here. The
/// default scale is the one users run.
#[test]
fn two_processes_render_byte_identical_outputs() {
    let binary = env!("CARGO_BIN_EXE_stencil");
    let mut outputs = Vec::new();
    for run_index in 0..2 {
        let out_dir = scratch_directory(&format!("two_processes_{run_index}"));
        let status = std::process::Command::new(binary)
            .args(["render", &g7_path(), "--out-dir"])
            .arg(&out_dir)
            .output()
            .unwrap();
        assert!(status.status.success(), "{:?}", status);
        let bytes: Vec<Vec<u8>> = ["g7.svg", "g7.png", "g7.measured.json"]
            .iter()
            .map(|name| fs::read(out_dir.join(name)).unwrap())
            .collect();
        outputs.push(bytes);
    }
    assert_eq!(outputs[0], outputs[1]);
}
