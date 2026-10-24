//! The lines of a unified diff a review comment can name: for each file, the head-side line
//! numbers inside its hunks. GitHub refuses a whole review when one inline comment names a
//! line outside them.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DiffMap {
    /// Head-side lines inside a hunk, added or context, by path; a deleted file has none.
    pub commentable: BTreeMap<String, BTreeSet<u32>>,
    pub files: usize,
    pub hunks: usize,
}

impl DiffMap {
    pub fn anchors(&self, path: &str, line: u32) -> bool {
        self.commentable
            .get(path)
            .is_some_and(|lines| lines.contains(&line))
    }
}

/// The head-side start and line count of a `@@ -a,b +c,d @@` header; a count left out is 1.
fn hunk_start(header: &str) -> Option<(u32, u32)> {
    let after = header.strip_prefix("@@ -")?;
    let head_range = after.split(' ').nth(1)?.strip_prefix('+')?;
    let (start, count) = match head_range.split_once(',') {
        Some((start, count)) => (start.parse().ok()?, count.parse().ok()?),
        None => (head_range.parse().ok()?, 1),
    };
    Some((start, count))
}

fn head_path(line: &str) -> Result<Option<String>, String> {
    // git ends the label with a tab when the path holds a space.
    let target = line.trim_start_matches("+++ ").trim_end_matches('\t');
    if target == "/dev/null" {
        return Ok(None);
    }
    match target.strip_prefix("b/") {
        Some(path) if !path.is_empty() => Ok(Some(path.to_string())),
        _ => Err(format!(
            "the diff names a head path git quoted or did not prefix: {}",
            target.chars().take(120).collect::<String>()
        )),
    }
}

/// Parses `git diff` output made with `--no-color --no-ext-diff --unified=<n>`.
pub fn parse_diff(text: &str) -> Result<DiffMap, String> {
    let mut map = DiffMap::default();
    let mut current: Option<String> = None;
    let mut head_line: u32 = 0;
    let mut remaining: u32 = 0;
    for line in text.lines() {
        if remaining > 0 {
            match line.chars().next() {
                Some(' ' | '+') => {
                    if let Some(path) = &current {
                        map.commentable
                            .entry(path.clone())
                            .or_default()
                            .insert(head_line);
                    }
                    head_line = head_line.saturating_add(1);
                    remaining = remaining.saturating_sub(1);
                }
                Some('-' | '\\') => {}
                _ => return Err(format!("a hunk ends early at: {line}")),
            }
            continue;
        }
        if line.starts_with("diff --git ") {
            map.files = map.files.saturating_add(1);
            current = None;
        } else if line.starts_with("+++ ") {
            current = head_path(line)?;
            if let Some(path) = &current {
                map.commentable.entry(path.clone()).or_default();
            }
        } else if line.starts_with("@@ ") {
            let (start, count) =
                hunk_start(line).ok_or_else(|| format!("unreadable hunk header: {line}"))?;
            map.hunks = map.hunks.saturating_add(1);
            head_line = start;
            remaining = count;
        }
    }
    if remaining > 0 {
        return Err("the diff ends inside a hunk".to_string());
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "\
diff --git a/src/a.rs b/src/a.rs
index 1..2 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -10,3 +10,4 @@ fn a() {
 keep
-old
+new one
+new two
 keep
\\ No newline at end of file
@@ -40 +41 @@
-x
+y
diff --git a/gone.rs b/gone.rs
deleted file mode 100644
--- a/gone.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-a
-b
diff --git a/bin.png b/bin.png
Binary files a/bin.png and b/bin.png differ
diff --git a/new.rs b/new.rs
new file mode 100644
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
++++ b/looks like a header
+--- a/so does this
";

    #[test]
    fn head_lines_inside_hunks_are_commentable_and_nothing_else() {
        let map = parse_diff(DIFF).unwrap();
        assert_eq!(map.files, 4);
        assert_eq!(map.hunks, 4);
        let lines = |path: &str| map.commentable.get(path).cloned().unwrap_or_default();
        assert_eq!(lines("src/a.rs"), BTreeSet::from([10, 11, 12, 13, 41]));
        assert_eq!(lines("new.rs"), BTreeSet::from([1, 2]));
        assert!(!map.commentable.contains_key("gone.rs"));
        assert!(!map.commentable.contains_key("looks like a header"));
        assert!(map.anchors("src/a.rs", 41));
        assert!(!map.anchors("src/a.rs", 14));
        assert!(!map.anchors("src/a.rs", 40));
        assert!(!map.anchors("other.rs", 1));
    }

    #[test]
    fn a_quoted_path_a_bad_header_or_a_truncated_hunk_is_an_error() {
        assert!(parse_diff("diff --git a/x b/x\n+++ \"b/a\\tb\"\n").is_err());
        assert!(parse_diff("diff --git a/x b/x\n+++ b/x\n@@ nonsense @@\n").is_err());
        assert!(parse_diff("diff --git a/x b/x\n+++ b/x\n@@ -1,2 +1,2 @@\n a\n").is_err());
        assert!(parse_diff("diff --git a/x b/x\n+++ b/x\n@@ -1,2 +1,2 @@\n a\nzzz\n").is_err());
    }

    #[test]
    fn a_path_with_a_space_loses_the_tab_git_appends() {
        let map = parse_diff(
            "diff --git a/my notes.md b/my notes.md\n--- a/my notes.md\t\n+++ b/my notes.md\t\n@@ -1 +1 @@\n-a\n+b\n",
        )
        .unwrap();
        assert!(map.anchors("my notes.md", 1));
    }

    #[test]
    fn an_empty_diff_has_no_files() {
        assert_eq!(parse_diff("").unwrap(), DiffMap::default());
    }
}
