//! Repository tasks, run through the mise tasks: `cargo run -q -p xtask -- <command>`.
//! Builds go through `mbx` by name: a child of `cargo run` finds rustup's cargo on PATH, not
//! the mise wrapper that routes cargo through the mbx cache.

mod decode;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

const RULES: &str = "opengrep/rust.yml";
const RULE_FIXTURES: &str = "opengrep";
const CI_OUT: &str = "target/ci";
const SARIF_DIR: &str = "target/ci/sarif";
const STENCIL: &str = "target/release/stencil";
const GALLERY_DIRS_MAX: usize = 256;
const GALLERY_IMAGES_MAX: usize = 64;

const USAGE: &str = "usage: xtask <command>

  ci              every check CI runs, writing the gallery and any docs mismatch under target/ci
  opengrep        the opengrep rules against their fixtures, then over the repository
  workflows       zizmor and jactionlint over .github at pedantic
  gallery <dir>   build the release binary and render every example into <dir>
  release         publish the GitHub release of tag GITHUB_REF_NAME with the gallery attached";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match words.as_slice() {
        ["ci"] => ci(),
        ["opengrep"] => opengrep().map(|summary| println!("{summary}")),
        ["workflows"] => workflows().map(|summary| println!("{summary}")),
        ["gallery", dir] => gallery(dir),
        ["release"] => release(),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn describe(command: &Command) -> String {
    let mut words = vec![command.get_program().to_string_lossy().into_owned()];
    words.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );
    words.join(" ")
}

fn run(command: &mut Command) -> Result<(), String> {
    let description = describe(command);
    println!("xtask: {description}");
    let status = command
        .status()
        .map_err(|error| format!("cannot start {description}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{description} exited with {status}"))
    }
}

fn capture(command: &mut Command) -> Result<Output, String> {
    let description = describe(command);
    println!("xtask: {description}");
    command
        .output()
        .map_err(|error| format!("cannot start {description}: {error}"))
}

/// The exit code when it is one of `allowed`; stderr goes into the error otherwise.
fn exit_code(output: &Output, allowed: &[i32], tool: &str) -> Result<i32, String> {
    match output.status.code() {
        Some(code) if allowed.contains(&code) => Ok(code),
        Some(code) => Err(format!(
            "{tool} exited {code}:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        None => Err(format!("{tool} was stopped by a signal")),
    }
}

fn parse_json(output: &Output, tool: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("{tool} wrote no JSON report: {error}"))
}

/// Runs every step even after one fails, so one CI run reports all of them.
fn ci() -> Result<(), String> {
    // target/ comes back with the mbx store in CI, so the last run's outputs are cleared
    // before this run's artifact is written.
    if Path::new(CI_OUT).exists() {
        fs::remove_dir_all(CI_OUT).map_err(|error| format!("cannot clear {CI_OUT}: {error}"))?;
    }
    let gallery_dir = format!("{CI_OUT}/gallery");
    let gallery_docs = format!("{CI_OUT}/gallery-docs");
    let theme_docs = format!("{CI_OUT}/theme-docs");
    let mut outcomes: Vec<(&str, Result<(), String>)> = Vec::new();
    outcomes.push((
        "fmt",
        run(Command::new("cargo").args(["fmt", "--all", "--check"])),
    ));
    outcomes.push((
        "clippy",
        run(Command::new("mbx").args(["clippy", "--workspace", "--all-targets"])),
    ));
    outcomes.push((
        "test",
        run(Command::new("mbx").args(["test", "--workspace"])),
    ));
    outcomes.push(("cue", run(Command::new("cue/check.sh").env("CUE", "cue"))));
    outcomes.push(("gallery", gallery(&gallery_dir)));
    outcomes.push((
        "gallery docs",
        run(Command::new("scripts/check-gallery-docs.sh").args([STENCIL, &gallery_docs])),
    ));
    outcomes.push((
        "theme docs",
        run(Command::new("scripts/check-theme-docs.sh").args([STENCIL, &theme_docs])),
    ));
    outcomes.push(("opengrep", opengrep().map(|summary| println!("{summary}"))));
    outcomes.push((
        "workflows",
        workflows().map(|summary| println!("{summary}")),
    ));

    let failed: Vec<&str> = outcomes
        .iter()
        .filter(|(_, outcome)| outcome.is_err())
        .map(|(name, _)| *name)
        .collect();
    println!();
    for (name, outcome) in &outcomes {
        match outcome {
            Ok(()) => println!("ci: {name} passed"),
            Err(message) => println!("ci: {name} FAILED: {message}"),
        }
    }
    if failed.is_empty() {
        println!("ci: {} steps passed", outcomes.len());
        Ok(())
    } else {
        Err(format!(
            "{} of {} steps failed: {}",
            failed.len(),
            outcomes.len(),
            failed.join(", ")
        ))
    }
}

/// Where a tool's SARIF log goes for the code scanning upload; creates the directory.
fn sarif_path(tool: &str) -> Result<String, String> {
    fs::create_dir_all(SARIF_DIR).map_err(|error| format!("cannot create {SARIF_DIR}: {error}"))?;
    Ok(format!("{SARIF_DIR}/{tool}.sarif"))
}

fn check_sarif(tool: &str, path: &str) -> Result<(), String> {
    let log = fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let value: serde_json::Value = serde_json::from_slice(&log)
        .map_err(|error| format!("{tool} wrote no SARIF log to {path}: {error}"))?;
    decode::sarif_runs(&value, tool).map(|_| ())
}

fn write_sarif(tool: &str, output: &Output) -> Result<(), String> {
    let path = sarif_path(tool)?;
    fs::write(&path, &output.stdout).map_err(|error| format!("cannot write {path}: {error}"))?;
    check_sarif(tool, &path)
}

fn opengrep() -> Result<String, String> {
    let rules =
        fs::read_to_string(RULES).map_err(|error| format!("cannot read {RULES}: {error}"))?;
    let rule_ids = decode::rule_ids(&rules);
    if rule_ids.is_empty() {
        return Err(format!("{RULES} defines no rules"));
    }

    let test = capture(Command::new("opengrep").args(["test", "--json", RULE_FIXTURES]))?;
    exit_code(&test, &[0, 1], "opengrep test")?;
    let checks = decode::fixture_checks(&parse_json(&test, "opengrep test")?)?;
    decode::fixtures_cover(&rule_ids, &checks)?;
    println!("opengrep: {} rules pass their fixtures", checks.len());

    let sarif = sarif_path("opengrep")?;
    let scan = capture(
        Command::new("opengrep")
            .args([
                "scan",
                "--config",
                RULES,
                "--error",
                "--disable-version-check",
                "--quiet",
                "--exclude",
                RULE_FIXTURES,
                "--json",
            ])
            .arg(format!("--sarif-output={sarif}"))
            .arg("."),
    )?;
    let code = exit_code(&scan, &[0, 1], "opengrep scan")?;
    let summary = decode::scan_summary(&parse_json(&scan, "opengrep scan")?)?;
    check_sarif("opengrep", &sarif)?;
    if summary.files == 0 {
        return Err("opengrep scan examined no files".to_string());
    }
    if !summary.findings.is_empty() {
        for finding in &summary.findings {
            eprintln!("{finding}");
        }
        return Err(format!(
            "opengrep: {} findings in {} files",
            summary.findings.len(),
            summary.files
        ));
    }
    if code != 0 {
        return Err("opengrep scan exited 1 and reported no findings".to_string());
    }
    Ok(format!(
        "opengrep: {} rules, {} files, no findings",
        rule_ids.len(),
        summary.files
    ))
}

/// zizmor and jactionlint look actions up on GitHub (impostor commits, known advisories);
/// without a token those audits cannot run, which is a failure rather than a pass.
fn github_token() -> Result<String, String> {
    for name in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(token) = env::var(name)
            && !token.is_empty()
        {
            return Ok(token);
        }
    }
    if let Ok(output) = Command::new("gh").args(["auth", "token"]).output() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if output.status.success() && !token.is_empty() {
            return Ok(token);
        }
    }
    Err(
        "no GitHub token in GH_TOKEN, GITHUB_TOKEN or `gh auth token`; the online audits need one"
            .to_string(),
    )
}

/// The SARIF logs are written before the gating runs, so a run that fails still uploads
/// the findings it failed on.
fn workflows() -> Result<String, String> {
    let token = github_token()?;
    let zizmor_sarif = capture(
        Command::new("zizmor")
            .args([
                "--persona=pedantic",
                "--no-progress",
                "--no-exit-codes",
                "--format=sarif",
                ".github",
            ])
            .env("GH_TOKEN", &token),
    )?;
    exit_code(&zizmor_sarif, &[0], "zizmor")?;
    write_sarif("zizmor", &zizmor_sarif)?;
    let lint_sarif = capture(
        Command::new("jactionlint")
            .args(["--online=strict", "--format=sarif"])
            .env("GH_TOKEN", &token),
    )?;
    exit_code(&lint_sarif, &[0, 1], "jactionlint")?;
    write_sarif("jactionlint", &lint_sarif)?;

    run(Command::new("zizmor")
        .args(["--persona=pedantic", "--no-progress", ".github"])
        .env("GH_TOKEN", &token))?;

    // The profile and the action allow list live in .github/jactionlint.yaml.
    let lint = capture(
        Command::new("jactionlint")
            .args(["--strict-exit", "--online=strict", "--verbose"])
            .env("GH_TOKEN", &token),
    )?;
    print!("{}", String::from_utf8_lossy(&lint.stdout));
    let log = String::from_utf8_lossy(&lint.stderr);
    for line in log.lines().filter(|line| !line.starts_with("verbose: ")) {
        eprintln!("{line}");
    }
    let files = decode::jactionlint_files(&log)?;
    if files == 0 {
        return Err("jactionlint examined no workflow files".to_string());
    }
    exit_code(&lint, &[0], "jactionlint")?;
    Ok(format!(
        "workflows: zizmor and jactionlint pass, {files} files"
    ))
}

fn gallery(dir: &str) -> Result<(), String> {
    run(Command::new("mbx").args(["build", "--release"]))?;
    run(Command::new(STENCIL).args(["gallery", dir]))
}

/// The center-theme PNG of every example, `<dir>/*/center/*.png`, sorted.
fn center_images(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    let mut images = Vec::new();
    for entry in entries.take(GALLERY_DIRS_MAX) {
        let entry = entry.map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
        let center = entry.path().join("center");
        if !center.is_dir() {
            continue;
        }
        let files = fs::read_dir(&center)
            .map_err(|error| format!("cannot read {}: {error}", center.display()))?;
        for file in files.take(GALLERY_IMAGES_MAX) {
            let path = file
                .map_err(|error| format!("cannot read {}: {error}", center.display()))?
                .path();
            if path.extension().is_some_and(|extension| extension == "png") {
                images.push(path);
            }
        }
    }
    images.sort();
    Ok(images)
}

fn release() -> Result<(), String> {
    let tag = env::var("GITHUB_REF_NAME")
        .map_err(|_| "GITHUB_REF_NAME is not set; a release runs on a tag push".to_string())?;
    let gallery_dir = "target/gallery";
    gallery(gallery_dir)?;
    run(Command::new("zip")
        .args(["-r", "-X", "../gallery.zip", "gallery"])
        .current_dir("target"))?;
    let images = center_images(Path::new(gallery_dir))?;
    if images.is_empty() {
        return Err(format!("{gallery_dir} holds no center images"));
    }
    run(Command::new("gh")
        .args([
            "release",
            "create",
            &tag,
            "--verify-tag",
            "--generate-notes",
            "gallery.zip",
        ])
        .args(&images))
}
