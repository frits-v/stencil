//! Every process `xtask review` starts. A child starts from an empty environment and gets the
//! variables in `INHERITED` by exact name, so the secrets the run holds in its own environment
//! (`secrets::SECRET_VARIABLES`) reach no child, nor do git's hook variables or a token that
//! another tool exported. Nothing removes variables from this process's environment, which is
//! unsound once a second thread may read it.

use std::env;
use std::ffi::OsStr;
use std::process::Command;

/// What git and mise need: where they are and whose home they read, the locale, the
/// temporary directory, and the directories mise reads its tools and settings from.
pub const INHERITED: [&str; 16] = [
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LC_ALL",
    "TMPDIR",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "MISE_CACHE_DIR",
    "MISE_CONFIG_DIR",
    "MISE_DATA_DIR",
    "MISE_STATE_DIR",
    "MISE_TRUSTED_CONFIG_PATHS",
];

/// `program` with an empty environment; the caller sets each variable it passes.
pub fn empty(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_clear();
    command
}

/// `program` with an empty environment and each `INHERITED` variable this process has.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = empty(program);
    for name in INHERITED {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::SECRET_VARIABLES;

    const CHILD: &str = "child::tests::a_child_under_the_run_secrets_lists_its_environment";
    const LISTING_START: &str = "--- child environment ---";

    #[test]
    fn no_inherited_variable_is_a_secret_or_git_hook_variable() {
        for name in INHERITED {
            assert!(!SECRET_VARIABLES.contains(&name), "{name}");
            assert!(!name.starts_with("GIT_"), "{name}");
            assert!(!name.contains("TOKEN") && !name.contains("KEY"), "{name}");
        }
    }

    /// Runs only in a child process that `no_secret_reaches_a_child` starts with every secret
    /// variable set, so no test changes this process's environment.
    #[test]
    #[ignore = "run by no_secret_reaches_a_child in a child process"]
    fn a_child_under_the_run_secrets_lists_its_environment() {
        for name in SECRET_VARIABLES {
            assert!(env::var_os(name).is_some(), "the parent sets {name}");
        }
        let output = command("env").output().unwrap();
        assert!(output.status.success());
        println!("{LISTING_START}");
        print!("{}", String::from_utf8(output.stdout).unwrap());
    }

    #[test]
    fn no_secret_reaches_a_child() {
        let mut parent = Command::new(env::current_exe().unwrap());
        parent
            .args(["--ignored", "--exact", "--nocapture", CHILD])
            .env("GIT_DIR", "/nonexistent/hook/.git")
            .env("XTASK_PLAIN_VARIABLE", "dropped");
        for name in SECRET_VARIABLES {
            parent.env(name, format!("value of {name}"));
        }
        let output = parent.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{stdout}");
        assert!(
            stdout.contains("test result: ok. 1 passed"),
            "the child must run exactly one test: {stdout}"
        );
        let listing = stdout.split_once(LISTING_START).unwrap().1;
        let names: Vec<&str> = listing
            .lines()
            .filter_map(|line| line.split_once('=').map(|(name, _)| name))
            .collect();
        assert!(names.contains(&"PATH"), "{listing}");
        for name in &names {
            assert!(
                INHERITED.contains(name),
                "{name} reached the child: {listing}"
            );
        }
        for name in SECRET_VARIABLES {
            assert!(!listing.contains(&format!("value of {name}")), "{name}");
        }
    }
}
