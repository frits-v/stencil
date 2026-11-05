//! Release commands: the release pull request and the tags through release-plz, the binary
//! archives through dist, and the gallery attached to the GitHub release.
//!
//! Every library crate is released with its own tag, `<crate>-vX.Y.Z`, so a change in one bumps
//! the crates that depend on it and the binary with them; `stencil-cli` alone is tagged `vX.Y.Z`
//! and gets the GitHub release and the changelog. `release-plz.toml` holds that configuration
//! and `every_library_crate_is_released` reads it back.

use std::env;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::app_token::{self, AppCredentials, RELEASE, Repository};
use crate::process::{Captured, Exit, run_bounded};

const RELEASE_PLZ_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DIST_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const GIT_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const RUSTUP_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const REPORT_MAX_LINES: usize = 80;
const ARTIFACTS_MAX: usize = 32;
const TAG_MAX_CHARS: usize = 32;
const RELEASED_PACKAGE: &str = "stencil-cli";
/// The crates the binary depends on, each tagged `<crate>-vX.Y.Z` beside the binary's `vX.Y.Z`.
const LIBRARY_CRATES: [&str; 4] = [
    "stencil-model",
    "stencil-text",
    "stencil-layout",
    "stencil-render",
];
/// dist names an archive after the package it holds, not after the binary inside.
const ARTIFACT_PREFIX: &str = "stencil-cli-";
const DRY_RUN_DIR: &str = "target/release-dry-run";
const DISTRIB_DIR: &str = "target/distrib";
const GALLERY_DIR: &str = "target/gallery";
const GALLERY_ZIP: &str = "gallery.zip";
const MACOS_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
const LINUX_TARGETS: [&str; 1] = ["x86_64-unknown-linux-gnu"];

#[derive(Debug, PartialEq, Eq)]
enum ReleasePrOutcome {
    Opened { number: u64, url: String },
    NothingToRelease,
    Failed(Exit),
    Unrecognised(String),
}

#[derive(Debug, PartialEq, Eq)]
enum ReleaseOutcome {
    Released { tag: String, version: String },
    NothingToRelease,
    Failed(Exit),
    Unrecognised(String),
}

#[derive(Debug, PartialEq, Eq)]
enum DryRunOutcome {
    Proposed(Vec<String>),
    NothingToRelease,
    Failed(Exit),
    Unrecognised(String),
}

#[derive(Debug, PartialEq, Eq)]
enum DistOutcome {
    Built(Vec<String>),
    Failed(Exit),
    Unrecognised(String),
}

#[derive(Debug, PartialEq, Eq)]
enum HeadTag {
    Tagged(String),
    Untagged,
    Failed(Exit),
    Unrecognised(String),
}

/// The exit code fully partitions the states: zero uploaded every file, anything else failed.
#[derive(Debug, PartialEq, Eq)]
enum UploadOutcome {
    Uploaded(usize),
    Failed(Exit),
}

fn print_capped(text: &str) {
    for line in text.lines().take(REPORT_MAX_LINES) {
        println!("  {line}");
    }
    let total = text.lines().count();
    if total > REPORT_MAX_LINES {
        println!(
            "  ... {} more lines",
            total.saturating_sub(REPORT_MAX_LINES)
        );
    }
}

/// Reads stdout as one JSON document, or failing that its last line that opens an object.
fn parse_json(stdout: &str) -> Result<serde_json::Value, String> {
    if let Ok(value) = serde_json::from_str(stdout.trim()) {
        return Ok(value);
    }
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| "no JSON object on stdout".to_string())?;
    serde_json::from_str(line).map_err(|e| format!("stdout is not JSON: {e}"))
}

fn decode_release_pr(exit: Exit, stdout: &str) -> ReleasePrOutcome {
    if exit != Exit::Code(0) {
        return ReleasePrOutcome::Failed(exit);
    }
    let value = match parse_json(stdout) {
        Ok(value) => value,
        Err(reason) => return ReleasePrOutcome::Unrecognised(reason),
    };
    let Some(prs) = value.get("prs").and_then(serde_json::Value::as_array) else {
        return ReleasePrOutcome::Unrecognised("no `prs` array".to_string());
    };
    match prs.as_slice() {
        [] => ReleasePrOutcome::NothingToRelease,
        [pr] => {
            let number = pr.get("number").and_then(serde_json::Value::as_u64);
            let url = pr.get("html_url").and_then(serde_json::Value::as_str);
            match (number, url) {
                (Some(number), Some(url)) => ReleasePrOutcome::Opened {
                    number,
                    url: url.to_string(),
                },
                _ => ReleasePrOutcome::Unrecognised("pull request without number or url".into()),
            }
        }
        _ => ReleasePrOutcome::Unrecognised(format!("{} release pull requests", prs.len())),
    }
}

/// One release of the binary, `vX.Y.Z`, with the library crates' `<crate>-vX.Y.Z` tags beside
/// it; a library may stay at its version and then has no entry.
fn decode_release(exit: Exit, stdout: &str) -> ReleaseOutcome {
    if exit != Exit::Code(0) {
        return ReleaseOutcome::Failed(exit);
    }
    let value = match parse_json(stdout) {
        Ok(value) => value,
        Err(reason) => return ReleaseOutcome::Unrecognised(reason),
    };
    let Some(releases) = value.get("releases").and_then(serde_json::Value::as_array) else {
        return ReleaseOutcome::Unrecognised("no `releases` array".to_string());
    };
    if releases.is_empty() {
        return ReleaseOutcome::NothingToRelease;
    }
    let mut binary = None;
    for release in releases {
        let field = |name: &str| release.get(name).and_then(serde_json::Value::as_str);
        match (field("package_name"), field("tag"), field("version")) {
            (Some(RELEASED_PACKAGE), Some(tag), Some(version))
                if valid_tag(tag) && tag.strip_prefix('v') == Some(version) =>
            {
                if binary.is_some() {
                    return ReleaseOutcome::Unrecognised(format!(
                        "two releases of {RELEASED_PACKAGE}"
                    ));
                }
                binary = Some((tag.to_string(), version.to_string()));
            }
            (Some(name), Some(tag), Some(version))
                if LIBRARY_CRATES.contains(&name)
                    && valid_version(version)
                    && tag == format!("{name}-v{version}") => {}
            _ => {
                return ReleaseOutcome::Unrecognised(format!(
                    "unexpected release entry: {release}"
                ));
            }
        }
    }
    match binary {
        Some((tag, version)) => ReleaseOutcome::Released { tag, version },
        None => ReleaseOutcome::Unrecognised(format!(
            "{} releases and none of {RELEASED_PACKAGE}",
            releases.len()
        )),
    }
}

fn decode_dry_run(exit: Exit, porcelain: &str) -> DryRunOutcome {
    if exit != Exit::Code(0) {
        return DryRunOutcome::Failed(exit);
    }
    let mut files = Vec::new();
    for line in porcelain.lines() {
        match line.get(3..) {
            Some(path) if line.len() > 3 => files.push(path.to_string()),
            _ => return DryRunOutcome::Unrecognised(format!("status line `{line}`")),
        }
    }
    if files.is_empty() {
        DryRunOutcome::NothingToRelease
    } else {
        DryRunOutcome::Proposed(files)
    }
}

fn decode_dist(exit: Exit, stdout: &str) -> DistOutcome {
    if exit != Exit::Code(0) {
        return DistOutcome::Failed(exit);
    }
    let value = match parse_json(stdout) {
        Ok(value) => value,
        Err(reason) => return DistOutcome::Unrecognised(reason),
    };
    let Some(artifacts) = value
        .get("artifacts")
        .and_then(serde_json::Value::as_object)
    else {
        return DistOutcome::Unrecognised("no `artifacts` object".to_string());
    };
    let names: Vec<String> = artifacts.keys().cloned().collect();
    let safe = names
        .iter()
        .all(|n| n.starts_with(ARTIFACT_PREFIX) && !n.contains('/') && !n.contains(".."));
    if names.is_empty() || names.len() > ARTIFACTS_MAX || !safe {
        return DistOutcome::Unrecognised(format!("artifact list {names:?}"));
    }
    DistOutcome::Built(names)
}

fn decode_upload(exit: Exit, count: usize) -> UploadOutcome {
    if exit == Exit::Code(0) {
        UploadOutcome::Uploaded(count)
    } else {
        UploadOutcome::Failed(exit)
    }
}

/// The binary's `vX.Y.Z` tag at HEAD; the library tags beside it are not version tags.
fn decode_head_tags(exit: Exit, stdout: &str) -> HeadTag {
    if exit != Exit::Code(0) {
        return HeadTag::Failed(exit);
    }
    let versions: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|t| valid_tag(t))
        .collect();
    match versions.as_slice() {
        [] => HeadTag::Untagged,
        [tag] => HeadTag::Tagged((*tag).to_string()),
        _ => HeadTag::Unrecognised(format!("several version tags at HEAD: {versions:?}")),
    }
}

/// A release tag is `v` and a semantic version of digits and dots, bounded in length.
fn valid_tag(tag: &str) -> bool {
    tag.len() <= TAG_MAX_CHARS && tag.strip_prefix('v').is_some_and(valid_version)
}

fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    version.len() < TAG_MAX_CHARS
        && parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn release_pr_inputs() -> Result<(AppCredentials, Repository), String> {
    let credentials = app_token::read_credentials(
        &RELEASE,
        env::var(RELEASE.client_id_variable).ok(),
        env::var(RELEASE.app_id_variable).ok(),
        crate::secrets::read(RELEASE.key_variable),
    )?;
    let repository = app_token::read_repository(env::var("GITHUB_REPOSITORY").ok())?;
    Ok((credentials, repository))
}

/// `xtask release-pr`: opens or updates the release pull request with an App installation token.
pub fn release_pr(dry_run: bool) -> Result<(), String> {
    if dry_run {
        return release_pr_dry_run();
    }
    let (credentials, repository) = release_pr_inputs()?;
    let token = app_token::mint(&credentials, &repository)?;
    let captured = run_bounded(
        release_plz_command(&["release-pr", "--output", "json"], env::vars_os())
            .env("GIT_TOKEN", token.expose()),
        RELEASE_PLZ_TIMEOUT,
    );
    let reported = report_release_pr(captured);
    app_token::revoke(&token)?;
    reported
}

/// release-plz and the cargo and git processes it starts never see the App's ids or key.
fn release_plz_command(
    args: &[&str],
    parent: impl IntoIterator<Item = (OsString, OsString)>,
) -> Command {
    let mut command = Command::new("release-plz");
    command
        .args(args)
        .env_clear()
        .envs(app_token::without_app_credentials(&RELEASE, parent));
    command
}

fn report_release_pr(captured: Captured) -> Result<(), String> {
    match decode_release_pr(captured.exit, &captured.stdout) {
        ReleasePrOutcome::Opened { number, url } => {
            println!("release-pr: release pull request #{number} at {url}");
            Ok(())
        }
        ReleasePrOutcome::NothingToRelease => {
            println!("release-pr: no releasable commits since the last tag");
            Ok(())
        }
        outcome => {
            print_capped(&captured.stdout);
            Err(format!("release-pr: {outcome:?}"))
        }
    }
}

fn git_in(dir: &Path, args: &[&str]) -> Captured {
    run_bounded(Command::new("git").current_dir(dir).args(args), GIT_TIMEOUT)
}

fn remove_dry_run_worktree(root: &Path) -> Result<(), String> {
    let scratch = root.join(DRY_RUN_DIR);
    if !scratch.exists() {
        return Ok(());
    }
    let path = scratch.to_string_lossy().into_owned();
    let removed = git_in(root, &["worktree", "remove", "--force", &path]);
    if removed.exit != Exit::Code(0) {
        return Err(format!(
            "release-pr --dry-run: could not remove {path}: {:?}",
            removed.exit
        ));
    }
    Ok(())
}

/// Runs `release-plz update` in a detached scratch worktree at HEAD and shows what it would
/// change. release-plz runs `cargo package` in a temporary copy of the tree, so the cargo on
/// PATH has to run outside this repository: an activated mise shell, not its shims.
fn release_pr_dry_run() -> Result<(), String> {
    let root = PathBuf::from(".");
    remove_dry_run_worktree(&root)?;
    let added = git_in(
        &root,
        &[
            "worktree",
            "add",
            "--force",
            "--detach",
            DRY_RUN_DIR,
            "HEAD",
        ],
    );
    if added.exit != Exit::Code(0) {
        return Err(format!(
            "release-pr --dry-run: worktree add ended {:?}",
            added.exit
        ));
    }
    let outcome = dry_run_in(&root.join(DRY_RUN_DIR));
    remove_dry_run_worktree(&root)?;
    outcome
}

fn dry_run_in(scratch: &Path) -> Result<(), String> {
    let mut update = release_plz_command(&["update"], env::vars_os());
    update.current_dir(scratch);
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        update.env("GIT_TOKEN", token);
    }
    let updated = run_bounded(&mut update, RELEASE_PLZ_TIMEOUT);
    print_capped(&updated.stdout);
    let outcome = if updated.exit == Exit::Code(0) {
        proposed_changes(scratch)
    } else {
        DryRunOutcome::Failed(updated.exit)
    };
    match outcome {
        DryRunOutcome::Proposed(files) => {
            println!("release-pr --dry-run: would change {} files", files.len());
            Ok(())
        }
        DryRunOutcome::NothingToRelease => {
            println!("release-pr --dry-run: nothing to release");
            Ok(())
        }
        outcome => Err(format!("release-pr --dry-run: {outcome:?}")),
    }
}

fn proposed_changes(scratch: &Path) -> DryRunOutcome {
    let staged = git_in(scratch, &["add", "--intent-to-add", "--all"]);
    if staged.exit != Exit::Code(0) {
        return DryRunOutcome::Failed(staged.exit);
    }
    let status = git_in(scratch, &["status", "--porcelain"]);
    let outcome = decode_dry_run(status.exit, &status.stdout);
    if let DryRunOutcome::Proposed(_) = outcome {
        let diff = git_in(scratch, &["diff"]);
        if diff.exit != Exit::Code(0) {
            return DryRunOutcome::Failed(diff.exit);
        }
        print_capped(&diff.stdout);
    }
    outcome
}

fn github_token() -> Result<String, String> {
    env::var("GITHUB_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| "GITHUB_TOKEN is not set; release-plz and gh need it".to_string())
}

/// `xtask release`: tags a merged release pull request, creates its GitHub release and attaches
/// the gallery to it. On any other commit it does nothing and succeeds.
pub fn release() -> Result<(), String> {
    let token = github_token()?;
    let captured = run_bounded(
        Command::new("release-plz")
            .args(["release", "--output", "json"])
            .env("GIT_TOKEN", &token),
        RELEASE_PLZ_TIMEOUT,
    );
    let tag = match decode_release(captured.exit, &captured.stdout) {
        ReleaseOutcome::Released { tag, version } => {
            println!("release: released {RELEASED_PACKAGE} {version} as {tag}");
            Some(tag)
        }
        ReleaseOutcome::NothingToRelease => existing_release_at_head()?,
        outcome => {
            print_capped(&captured.stdout);
            return Err(format!("release: {outcome:?}"));
        }
    };
    let Some(tag) = tag else {
        println!("release: this commit is not a merged release pull request");
        return Ok(());
    };
    attach_gallery(&tag, &token)?;
    write_step_output("tag", &tag)
}

/// A rerun of a release commit finds its tag already made; the gallery and the binaries are
/// rebuilt for it.
fn existing_release_at_head() -> Result<Option<String>, String> {
    let tags = git_in(Path::new("."), &["tag", "--points-at", "HEAD"]);
    match decode_head_tags(tags.exit, &tags.stdout) {
        HeadTag::Tagged(tag) => {
            println!("release: {tag} already points at this commit; its assets are rebuilt");
            Ok(Some(tag))
        }
        HeadTag::Untagged => Ok(None),
        outcome => Err(format!("release: tags at HEAD {outcome:?}")),
    }
}

/// The gallery of every example in every theme, as one zip plus the center PNGs, on the release.
fn attach_gallery(tag: &str, token: &str) -> Result<(), String> {
    crate::gallery(GALLERY_DIR)?;
    let zip = Path::new(GALLERY_DIR)
        .parent()
        .ok_or_else(|| format!("{GALLERY_DIR} has no parent"))?;
    crate::run(
        Command::new("zip")
            .args(["-r", "-X", GALLERY_ZIP, "gallery"])
            .current_dir(zip),
    )?;
    let mut files = vec![zip.join(GALLERY_ZIP)];
    let images = crate::center_images(Path::new(GALLERY_DIR))?;
    if images.is_empty() {
        return Err(format!("{GALLERY_DIR} holds no center images"));
    }
    files.extend(images);
    upload(tag, &files, token)
}

fn write_step_output(name: &str, value: &str) -> Result<(), String> {
    let Ok(path) = env::var("GITHUB_OUTPUT") else {
        println!("release: {name}={value} (no GITHUB_OUTPUT outside Actions)");
        return Ok(());
    };
    OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| writeln!(file, "{name}={value}"))
        .map_err(|error| format!("cannot write step output to {path}: {error}"))
}

const fn host_targets() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &MACOS_TARGETS
    } else {
        &LINUX_TARGETS
    }
}

/// `xtask release-binaries [tag]`: builds this host's archives with dist and uploads them to
/// the release of the tag, from the argument or `RELEASE_TAG`.
pub fn release_binaries(tag: Option<String>) -> Result<(), String> {
    let tag = match tag.or_else(|| env::var("RELEASE_TAG").ok()) {
        Some(tag) if valid_tag(&tag) => tag,
        other => {
            return Err(format!(
                "release-binaries: expected a tag like v1.2.3, got {other:?}"
            ));
        }
    };
    let token = github_token()?;
    add_host_targets()?;
    let mut dist = Command::new("dist");
    dist.args([
        "build",
        "--tag",
        &tag,
        "--artifacts",
        "local",
        "--no-local-paths",
        "--output-format",
        "json",
    ]);
    for target in host_targets() {
        dist.args(["--target", target]);
    }
    let built = run_bounded(&mut dist, DIST_TIMEOUT);
    let names = match decode_dist(built.exit, &built.stdout) {
        DistOutcome::Built(names) => names,
        outcome => return Err(format!("release-binaries: dist {outcome:?}")),
    };
    let paths: Vec<PathBuf> = names
        .iter()
        .map(|n| Path::new(DISTRIB_DIR).join(n))
        .collect();
    upload(&tag, &paths, &token)
}

fn add_host_targets() -> Result<(), String> {
    for target in host_targets() {
        let added = run_bounded(
            Command::new("rustup").args(["target", "add", target]),
            RUSTUP_TIMEOUT,
        );
        if added.exit != Exit::Code(0) {
            return Err(format!(
                "release-binaries: rustup target add {target} ended {:?}",
                added.exit
            ));
        }
    }
    Ok(())
}

/// Uploads every file to the release of `tag`, replacing an asset of the same name.
fn upload(tag: &str, paths: &[PathBuf], token: &str) -> Result<(), String> {
    if let Some(missing) = paths.iter().find(|p| !p.is_file()) {
        return Err(format!(
            "release: {} was reported built but is not on disk",
            missing.display()
        ));
    }
    let mut command = Command::new("gh");
    command
        .args(["release", "upload", tag, "--clobber"])
        .args(paths)
        .env("GH_TOKEN", token);
    let captured = run_bounded(&mut command, UPLOAD_TIMEOUT);
    match decode_upload(captured.exit, paths.len()) {
        UploadOutcome::Uploaded(count) => {
            println!("release: uploaded {count} files to {tag}");
            Ok(())
        }
        UploadOutcome::Failed(exit) => Err(format!("release: gh release upload ended {exit:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The `[[package]]` blocks of release-plz.toml as (name, body) pairs.
    fn release_plz_packages() -> Vec<(String, String)> {
        let config = fs::read_to_string("../release-plz.toml").unwrap();
        config
            .split("[[package]]")
            .skip(1)
            .map(|block| {
                let name = block
                    .lines()
                    .find_map(|line| line.strip_prefix("name = "))
                    .unwrap()
                    .trim_matches('"')
                    .to_string();
                (name, block.to_string())
            })
            .collect()
    }

    /// The workspace members under crates/, from Cargo.toml.
    fn workspace_crates() -> Vec<String> {
        let manifest = fs::read_to_string("../Cargo.toml").unwrap();
        manifest
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("\"crates/")
                    .and_then(|rest| rest.strip_suffix("\","))
            })
            .map(str::to_string)
            .collect()
    }

    fn pinned(file: &str, key: &str) -> String {
        let text = fs::read_to_string(file).unwrap();
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key} = ")))
            .unwrap_or_else(|| panic!("{file} pins no {key}"))
            .trim_matches('"')
            .to_string()
    }

    #[test]
    fn every_library_crate_is_released_so_the_binary_bumps_with_it() {
        let crates = workspace_crates();
        assert_eq!(crates.len(), 5, "{crates:?}");
        let packages = release_plz_packages();
        let names: Vec<&str> = packages.iter().map(|(name, _)| name.as_str()).collect();
        for member in &crates {
            assert!(names.contains(&member.as_str()), "{member} is not released");
        }
        for (name, body) in &packages {
            assert!(crates.contains(name), "{name} is not a workspace crate");
            assert!(body.contains("release = true"), "{name} is not released");
            let is_binary = name == RELEASED_PACKAGE;
            assert_eq!(
                body.contains("git_release_enable = true"),
                is_binary,
                "{name}"
            );
            assert_eq!(
                body.contains("changelog_update = true"),
                is_binary,
                "{name}"
            );
            assert_eq!(
                body.contains("git_tag_name = \"v{{ version }}\""),
                is_binary,
                "{name}"
            );
        }
        let binary = packages
            .iter()
            .find(|(name, _)| name == RELEASED_PACKAGE)
            .map(|(_, body)| body)
            .unwrap();
        let mut libraries: Vec<&str> = crates
            .iter()
            .map(String::as_str)
            .filter(|name| *name != RELEASED_PACKAGE)
            .collect();
        libraries.sort_unstable();
        let mut listed = LIBRARY_CRATES.to_vec();
        listed.sort_unstable();
        assert_eq!(listed, libraries);
        for library in libraries {
            assert!(
                binary.contains(&format!("\"{library}\"")),
                "{library} is missing from changelog_include"
            );
        }
        let workspace = fs::read_to_string("../release-plz.toml").unwrap();
        assert!(workspace.contains("git_tag_name = \"{{ package }}-v{{ version }}\""));
        assert!(workspace.contains("git_only = true"));
        assert!(workspace.contains("release_always = false"));
    }

    #[test]
    fn dist_runs_at_the_version_mise_pins() {
        assert_eq!(
            pinned("../dist-workspace.toml", "cargo-dist-version"),
            pinned("../mise.toml", "cargo-dist")
        );
        let cli = fs::read_to_string("../crates/stencil-cli/Cargo.toml").unwrap();
        assert!(cli.contains("[package.metadata.dist]\ndist = true"));
    }

    #[test]
    fn release_pr_decoder_covers_each_state() {
        let opened = r#"{"prs":[{"head_branch":"release-plz-x","base_branch":"main","html_url":"https://github.com/o/r/pull/7","number":7,"releases":[]}]}"#;
        assert_eq!(
            decode_release_pr(Exit::Code(0), opened),
            ReleasePrOutcome::Opened {
                number: 7,
                url: "https://github.com/o/r/pull/7".into()
            }
        );
        assert_eq!(
            decode_release_pr(Exit::Code(0), "log line\n{\"prs\":[]}\n"),
            ReleasePrOutcome::NothingToRelease
        );
        assert_eq!(
            decode_release_pr(Exit::Code(1), ""),
            ReleasePrOutcome::Failed(Exit::Code(1))
        );
        assert_eq!(
            decode_release_pr(Exit::TimedOut, ""),
            ReleasePrOutcome::Failed(Exit::TimedOut)
        );
        for unrecognised in [
            "",
            "not json",
            "{\"prs\":{}}",
            "{\"prs\":[{}]}",
            "{\"prs\":[{},{}]}",
        ] {
            assert!(
                matches!(
                    decode_release_pr(Exit::Code(0), unrecognised),
                    ReleasePrOutcome::Unrecognised(_)
                ),
                "{unrecognised}"
            );
        }
    }

    #[test]
    fn release_decoder_takes_the_binary_with_its_library_tags_and_nothing_else() {
        let released = ReleaseOutcome::Released {
            tag: "v0.1.1".into(),
            version: "0.1.1".into(),
        };
        let binary_only = r#"{"releases":[{"package_name":"stencil-cli","prs":[],"tag":"v0.1.1","version":"0.1.1"}]}"#;
        assert_eq!(decode_release(Exit::Code(0), binary_only), released);
        let with_libraries = r#"{"releases":[
            {"package_name":"stencil-layout","prs":[],"tag":"stencil-layout-v0.1.1","version":"0.1.1"},
            {"package_name":"stencil-render","prs":[],"tag":"stencil-render-v0.1.1","version":"0.1.1"},
            {"package_name":"stencil-cli","prs":[{"html_url":"https://github.com/o/r/pull/103","number":103}],"tag":"v0.1.1","version":"0.1.1"},
            {"package_name":"stencil-model","prs":[],"tag":"stencil-model-v0.2.0","version":"0.2.0"},
            {"package_name":"stencil-text","prs":[],"tag":"stencil-text-v0.1.1","version":"0.1.1"}]}"#;
        assert_eq!(decode_release(Exit::Code(0), with_libraries), released);
        assert_eq!(
            decode_release(Exit::Code(0), "{\"releases\":[]}"),
            ReleaseOutcome::NothingToRelease
        );
        assert_eq!(
            decode_release(Exit::Signal, ""),
            ReleaseOutcome::Failed(Exit::Signal)
        );
        let library_only = r#"{"releases":[{"package_name":"stencil-render","tag":"stencil-render-v0.1.1","version":"0.1.1"}]}"#;
        let other_package =
            r#"{"releases":[{"package_name":"xtask","tag":"v0.1.0","version":"0.1.0"}]}"#;
        let library_with_a_version_tag = r#"{"releases":[{"package_name":"stencil-cli","tag":"v0.1.1","version":"0.1.1"},{"package_name":"stencil-render","tag":"v0.1.1","version":"0.1.1"}]}"#;
        let two_binaries = r#"{"releases":[{"package_name":"stencil-cli","tag":"v0.1.1","version":"0.1.1"},{"package_name":"stencil-cli","tag":"v0.1.2","version":"0.1.2"}]}"#;
        let tag_off_version =
            r#"{"releases":[{"package_name":"stencil-cli","tag":"v0.1.1","version":"0.1.2"}]}"#;
        let bad_tag =
            r#"{"releases":[{"package_name":"stencil-cli","tag":"v1;rm","version":"1"}]}"#;
        for unrecognised in [
            "",
            "{}",
            library_only,
            other_package,
            library_with_a_version_tag,
            two_binaries,
            tag_off_version,
            bad_tag,
        ] {
            assert!(
                matches!(
                    decode_release(Exit::Code(0), unrecognised),
                    ReleaseOutcome::Unrecognised(_)
                ),
                "{unrecognised}"
            );
        }
    }

    #[test]
    fn dry_run_decoder_reads_porcelain_status() {
        assert_eq!(
            decode_dry_run(Exit::Code(0), " A CHANGELOG.md\n M Cargo.lock\n"),
            DryRunOutcome::Proposed(vec!["CHANGELOG.md".into(), "Cargo.lock".into()])
        );
        assert_eq!(
            decode_dry_run(Exit::Code(0), ""),
            DryRunOutcome::NothingToRelease
        );
        assert_eq!(
            decode_dry_run(Exit::Code(128), ""),
            DryRunOutcome::Failed(Exit::Code(128))
        );
        assert!(matches!(
            decode_dry_run(Exit::Code(0), "?"),
            DryRunOutcome::Unrecognised(_)
        ));
    }

    /// The artifact keys as `dist build --artifacts local --output-format json` wrote them for
    /// the two macOS targets of v0.1.0.
    #[test]
    fn dist_decoder_requires_safe_artifact_names() {
        let built = r#"{"dist_version":"0.33.0","announcement_tag":"v0.1.0","artifacts":{
            "stencil-cli-aarch64-apple-darwin.tar.xz":{"name":"stencil-cli-aarch64-apple-darwin.tar.xz","kind":"executable-zip","target_triples":["aarch64-apple-darwin"]},
            "stencil-cli-aarch64-apple-darwin.tar.xz.sha256":{"name":"stencil-cli-aarch64-apple-darwin.tar.xz.sha256","kind":"checksum","target_triples":["aarch64-apple-darwin"]},
            "stencil-cli-x86_64-apple-darwin.tar.xz":{"name":"stencil-cli-x86_64-apple-darwin.tar.xz","kind":"executable-zip","target_triples":["x86_64-apple-darwin"]},
            "stencil-cli-x86_64-apple-darwin.tar.xz.sha256":{"name":"stencil-cli-x86_64-apple-darwin.tar.xz.sha256","kind":"checksum","target_triples":["x86_64-apple-darwin"]}},
            "releases":[{"app_name":"stencil-cli","app_version":"0.1.0"}]}"#;
        assert!(matches!(
            decode_dist(Exit::Code(0), built),
            DistOutcome::Built(names) if names.len() == 4
        ));
        assert_eq!(
            decode_dist(Exit::Code(255), ""),
            DistOutcome::Failed(Exit::Code(255))
        );
        let empty = r#"{"artifacts":{}}"#;
        let escaping = r#"{"artifacts":{"stencil-cli-../../etc":{}}}"#;
        let binary_name = r#"{"artifacts":{"stencil-aarch64-apple-darwin.tar.xz":{}}}"#;
        let global =
            r#"{"artifacts":{"stencil-cli-aarch64-apple-darwin.tar.xz":{},"source.tar.gz":{}}}"#;
        for unrecognised in ["", "{}", empty, escaping, binary_name, global] {
            assert!(
                matches!(
                    decode_dist(Exit::Code(0), unrecognised),
                    DistOutcome::Unrecognised(_)
                ),
                "{unrecognised}"
            );
        }
    }

    #[test]
    fn upload_decoder_counts_only_on_success() {
        assert_eq!(decode_upload(Exit::Code(0), 4), UploadOutcome::Uploaded(4));
        assert_eq!(
            decode_upload(Exit::Code(1), 4),
            UploadOutcome::Failed(Exit::Code(1))
        );
    }

    #[test]
    fn head_tag_decoder_picks_the_binary_tag_among_the_library_tags() {
        assert_eq!(
            decode_head_tags(Exit::Code(0), "v0.1.0\n"),
            HeadTag::Tagged("v0.1.0".into())
        );
        assert_eq!(
            decode_head_tags(
                Exit::Code(0),
                "stencil-layout-v0.1.1\nstencil-render-v0.1.1\nv0.1.1\nstencil-cli-v0.1.1\n"
            ),
            HeadTag::Tagged("v0.1.1".into())
        );
        assert_eq!(decode_head_tags(Exit::Code(0), ""), HeadTag::Untagged);
        assert_eq!(
            decode_head_tags(Exit::Code(0), "stencil-model-v0.2.0\n"),
            HeadTag::Untagged
        );
        assert_eq!(
            decode_head_tags(Exit::Code(128), ""),
            HeadTag::Failed(Exit::Code(128))
        );
        assert!(matches!(
            decode_head_tags(Exit::Code(0), "v0.1.0\nv0.1.1\n"),
            HeadTag::Unrecognised(_)
        ));
    }

    #[test]
    fn release_plz_never_receives_the_app_credentials() {
        let parent: Vec<(OsString, OsString)> = [
            ("PATH", "/usr/bin"),
            (RELEASE.client_id_variable, "Iv23liExample"),
            (RELEASE.app_id_variable, "1"),
            (RELEASE.key_variable, "key-material"),
        ]
        .into_iter()
        .map(|(name, value)| (name.into(), value.into()))
        .collect();
        let command = release_plz_command(&["release-pr"], parent);
        let names: Vec<&std::ffi::OsStr> = command.get_envs().map(|(name, _)| name).collect();
        assert_eq!(names, [std::ffi::OsStr::new("PATH")]);
    }

    #[test]
    fn the_step_holding_the_app_key_runs_a_built_binary_not_a_build() {
        let workflow = include_str!("../../.github/workflows/release.yml");
        let lines: Vec<&str> = workflow.lines().collect();
        let key_lines: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(&format!("secrets.{}", RELEASE.key_variable)))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(key_lines.len(), 1, "{key_lines:?}");
        let step_run = key_lines
            .first()
            .and_then(|key| lines.get(..*key))
            .and_then(|before| before.iter().rev().find(|line| line.contains("run:")))
            .map(|line| line.trim());
        assert_eq!(step_run, Some("- run: target/debug/xtask release-pr"));
        assert!(workflow.contains("- run: cargo build --locked -p xtask"));
    }

    #[test]
    fn tags_are_versions_only() {
        assert!(valid_tag("v0.1.0"));
        assert!(valid_tag("v12.0.31"));
        for bad in [
            "0.1.0",
            "v0.1",
            "v0.1.0-rc1",
            "v0.1.0;x",
            "v..",
            "stencil-cli-v0.1.0",
            "stencil-render-v0.1.0",
            "",
        ] {
            assert!(!valid_tag(bad), "{bad}");
        }
    }
}
