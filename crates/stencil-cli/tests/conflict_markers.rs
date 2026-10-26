//! A rebase that resolves one conflict in a file and continues leaves the file's other
//! conflicts committed. Rust and JSON with a marker fail to build or parse; Markdown does
//! not, so this test reads every Markdown file under the repository's own directories.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

/// Directories that hold no source: build output and git's own store.
const SKIPPED_DIRECTORIES: [&str; 2] = ["target", ".git"];
/// More directories than the repository has, so the walk is bounded.
const DIRECTORIES_MAX: usize = 4096;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Markdown files under `root`, outside the skipped directories.
fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    for _ in 0..DIRECTORIES_MAX {
        let Some(directory) = pending.pop() else {
            return files;
        };
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if !SKIPPED_DIRECTORIES.contains(&name.as_str()) {
                    pending.push(path);
                }
            } else if name.ends_with(".md") {
                files.push(path);
            }
        }
    }
    panic!(
        "the walk from {} passed {DIRECTORIES_MAX} directories",
        root.display()
    );
}

/// The 1-based numbers of the lines git writes around a conflict.
fn conflict_lines(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            line.starts_with("<<<<<<< ") || line.starts_with(">>>>>>> ") || *line == "======="
        })
        .map(|(index, _)| index + 1)
        .collect()
}

#[test]
fn conflict_lines_finds_each_marker_git_writes() {
    let text = "intro\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> 3b4d307 (feat: x)\nend\n";
    assert_eq!(conflict_lines(text), vec![2, 4, 6]);
    assert!(conflict_lines("a table\n|---|---|\n== not a marker\n").is_empty());
}

#[test]
fn no_markdown_file_holds_a_conflict_marker() {
    let files = markdown_files(&repository_root());
    assert!(
        files.len() >= 10,
        "only {} Markdown files found",
        files.len()
    );
    for file in &files {
        let lines = conflict_lines(&fs::read_to_string(file).unwrap());
        assert!(lines.is_empty(), "{}: lines {lines:?}", file.display());
    }
}
