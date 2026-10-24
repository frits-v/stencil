//! The checkout `xtask review` runs in: the one it was built from, which must be the one it
//! runs in, so a dry run reviews the tree whose xtask it is.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// The nearest directory at or above `path` that holds `.git`, a directory in the main
/// checkout and a file in a linked worktree.
fn checkout_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

fn resolve(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|error| format!("cannot resolve {}: {error}", path.display()))
}

/// The checkout both paths belong to, or a refusal naming both when they differ. A nested
/// worktree is its own checkout, so `.worktrees/x` is not the checkout above it.
pub fn same_checkout(built_from: &Path, current: &Path) -> Result<PathBuf, String> {
    let built = resolve(built_from)?;
    let here = resolve(current)?;
    let both = format!(
        "this xtask was built from {} (CARGO_MANIFEST_DIR) and runs in {}",
        built.display(),
        here.display()
    );
    let Some(built_root) = checkout_root(&built) else {
        return Err(format!("{both}; the first is in no git checkout"));
    };
    let Some(here_root) = checkout_root(&here) else {
        return Err(format!("{both}; the second is in no git checkout"));
    };
    if built_root == here_root {
        return Ok(here_root);
    }
    Err(format!(
        "{both}, another checkout; run `mise run review` from the checkout under review"
    ))
}

/// The checkout this xtask was built from, asserted to be the current directory's.
pub fn running_checkout() -> Result<PathBuf, String> {
    let current = env::current_dir()
        .map_err(|error| format!("cannot read the current directory: {error}"))?;
    same_checkout(Path::new(env!("CARGO_MANIFEST_DIR")), &current)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A main checkout with a linked worktree under `.worktrees`, as this repository lays them out.
    fn layout(name: &str) -> PathBuf {
        let root =
            env::temp_dir().join(format!("xtask-checkout-test-{}-{name}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let main = root.join("stencil");
        for dir in [
            ".git",
            "xtask",
            "crates/stencil-cli",
            ".worktrees/branch/xtask",
        ] {
            fs::create_dir_all(main.join(dir)).unwrap();
        }
        fs::write(
            main.join(".worktrees/branch/.git"),
            "gitdir: ../../.git/worktrees/branch\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("loose/xtask")).unwrap();
        root
    }

    #[test]
    fn an_xtask_run_inside_its_own_checkout_is_accepted() {
        let root = layout("accepted");
        let main = root.join("stencil");
        let found = same_checkout(&main.join("xtask"), &main.join("crates/stencil-cli"));
        assert_eq!(found, Ok(resolve(&main).unwrap()));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_xtask_from_another_checkout_is_refused_with_both_paths() {
        let root = layout("refused");
        let main = root.join("stencil");
        let worktree = main.join(".worktrees/branch");
        for (built, current) in [
            (worktree.join("xtask"), main.clone()),
            (main.join("xtask"), worktree.clone()),
        ] {
            let refusal = same_checkout(&built, &current).unwrap_err();
            let built_shown = resolve(&built).unwrap().display().to_string();
            let current_shown = resolve(&current).unwrap().display().to_string();
            assert!(refusal.contains(&built_shown), "{refusal}");
            assert!(refusal.contains(&current_shown), "{refusal}");
        }
        for (built, current) in [
            (root.join("loose/xtask"), main.clone()),
            (main.join("xtask"), root.join("loose")),
        ] {
            let refusal = same_checkout(&built, &current).unwrap_err();
            assert!(refusal.contains("is in no git checkout"), "{refusal}");
            for path in [&built, &current] {
                let shown = resolve(path).unwrap().display().to_string();
                assert!(refusal.contains(&shown), "{refusal}");
            }
        }
        assert!(same_checkout(&main.join("absent"), &main).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
