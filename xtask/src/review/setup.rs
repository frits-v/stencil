//! What a review run starts from: its commits, its credentials, the Claude binary and its own
//! directory, for each mode.

use std::env;
use std::fs;
use std::os::unix::fs::DirBuilderExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::passes::TOKEN_LIFETIME_MIN;
use super::pull_request::CommitId;
use super::{Context, GitHub, Grants, Mode, git_in, langfuse, pull_request};
use crate::anthropic_token;
use crate::app_token::{self, REVIEW};
use crate::checkout;
use crate::github_api::Client;
use crate::process::{Exit, run_bounded};

const EVENT_MAX_BYTES: u64 = 1 << 20;
const GIT_TIMEOUT: Duration = Duration::from_secs(120);
const FETCH_TIMEOUT: Duration = Duration::from_secs(300);
const MISE_TIMEOUT: Duration = Duration::from_secs(60);
const RUN_TIME_MAX: Duration = Duration::from_secs(30 * 60);
const DRY_RUN_BASE: &str = "origin/main";
/// The diff, the earlier findings and one instructions file per pass.
const FILES_MAX: usize = 64;
const PATH_MAX_BYTES: usize = 4096;
/// The App's reviews one pull request takes; past it a run refuses, so no exchange of
/// comments and reviews goes on without end.
const REVIEWS_PER_PULL_REQUEST_MAX: usize = 20;

/// The run's grants: the Anthropic token, the App's token in a posting run, and the run time
/// the former leaves. Made by their own call and never returned with the context, which the
/// run prints from, so nothing that holds a token ever shares a value with what is printed.
pub fn grants(mode: &Mode) -> Result<Grants, String> {
    match mode {
        Mode::DryRun => Ok(Grants {
            anthropic: anthropic_minter(false)?,
            app: None,
            run_time: RUN_TIME_MAX,
        }),
        Mode::Post => {
            let credentials = review_credentials()?;
            let repository = app_token::read_repository(env::var("GITHUB_REPOSITORY").ok())?;
            let anthropic = anthropic_minter(true)?;
            let app = app_token::mint(&credentials, &repository)?;
            Ok(Grants {
                anthropic,
                app: Some(app),
                run_time: RUN_TIME_MAX,
            })
        }
    }
}

/// The run's context; a posting run reads the pull request with the App's token it borrows
/// from `grants`.
pub fn context(mode: &Mode, grants: &Grants) -> Result<Context, String> {
    match mode {
        Mode::DryRun => dry_run_context(),
        Mode::Post => post_context(grants),
    }
}

fn review_credentials() -> Result<app_token::AppCredentials, String> {
    app_token::read_credentials(
        &REVIEW,
        env::var(REVIEW.client_id_variable).ok(),
        env::var(REVIEW.app_id_variable).ok(),
        crate::secrets::read(REVIEW.key_variable),
    )
}

fn required(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is unset; the review job sets it"))
}

fn read_trigger() -> Result<pull_request::Trigger, String> {
    let name = required("GITHUB_EVENT_NAME")?;
    let path = required("GITHUB_EVENT_PATH")?;
    let size = fs::metadata(&path)
        .map_err(|error| format!("cannot read the event file: {error}"))?
        .len();
    if size > EVENT_MAX_BYTES {
        return Err(format!("the event file is over {EVENT_MAX_BYTES} bytes"));
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read the event file: {error}"))?;
    pull_request::decode_trigger(&name, &text)
}

fn current_user() -> Option<String> {
    nix::unistd::User::from_uid(nix::unistd::Uid::current())
        .ok()
        .flatten()
        .map(|user| user.name)
}

/// Mints Anthropic tokens by workload identity federation when the federation rule is set,
/// else none: the CLI then uses the developer's own login. A first exchange checks the rule
/// before any pass and its token is dropped; each pass mints its own, since GitHub's OIDC
/// token caps an exchanged token at its own validity window, ten minutes.
fn anthropic_minter(required_here: bool) -> Result<Option<anthropic_token::Minter>, String> {
    let rule = env::var(anthropic_token::FEDERATION_RULE_VARIABLE).ok();
    if rule.as_deref().is_none_or(|rule| rule.trim().is_empty()) && !required_here {
        return Ok(None);
    }
    let federation = anthropic_token::read_federation(
        rule,
        env::var(anthropic_token::ORGANIZATION_VARIABLE).ok(),
        env::var(anthropic_token::SERVICE_ACCOUNT_VARIABLE).ok(),
        env::var(anthropic_token::WORKSPACE_VARIABLE).ok(),
    )?;
    let oidc = anthropic_token::read_oidc_request(
        crate::secrets::read(anthropic_token::REQUEST_URL_VARIABLE),
        crate::secrets::read(anthropic_token::REQUEST_TOKEN_VARIABLE),
    )?;
    let minter = anthropic_token::Minter::new(federation, oidc);
    let lifetime = minter.mint()?.expires_in_seconds;
    check_token_lifetime(lifetime)?;
    println!(
        "xtask review: the federation rule issues tokens for {lifetime} s; each pass mints its own"
    );
    Ok(Some(minter))
}

/// A token too short for one pass fails the run before any pass, naming the lifetime.
fn check_token_lifetime(seconds: u64) -> Result<(), String> {
    if Duration::from_secs(seconds) < TOKEN_LIFETIME_MIN {
        return Err(format!(
            "the access token lasts {seconds} s; a pass needs at least {} s",
            TOKEN_LIFETIME_MIN.as_secs()
        ));
    }
    Ok(())
}

fn read_langfuse() -> Result<Option<langfuse::Langfuse>, String> {
    langfuse::read(
        env::var(langfuse::BASE_URL_VARIABLE).ok(),
        env::var(langfuse::PUBLIC_KEY_VARIABLE).ok(),
        crate::secrets::read(langfuse::SECRET_KEY_VARIABLE),
    )
}

fn head_commit(dir: &Path) -> Result<CommitId, String> {
    let captured = run_bounded(git_in(dir).args(["rev-parse", "HEAD"]), GIT_TIMEOUT);
    match captured.exit {
        Exit::Code(0) => CommitId::parse(captured.stdout.trim()),
        other => Err(format!(
            "git rev-parse HEAD in {} ended {other:?}",
            dir.display()
        )),
    }
}

fn uncommitted(dir: &Path) -> Result<usize, String> {
    let captured = run_bounded(git_in(dir).args(["status", "--porcelain"]), GIT_TIMEOUT);
    match captured.exit {
        Exit::Code(0) => Ok(captured
            .stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()),
        other => Err(format!("git status --porcelain ended {other:?}")),
    }
}

/// The commits a dry run reviews: `origin/main` against a clean `HEAD`, the branch the run
/// was started on.
fn dry_run_commits(root: &Path) -> Result<(CommitId, CommitId, String), String> {
    let changes = uncommitted(root)?;
    if changes > 0 {
        return Err(format!(
            "{changes} uncommitted changes: the model reads the working tree, so commit first"
        ));
    }
    let head = head_commit(root)?;
    let merge_base = run_bounded(
        git_in(root).args(["merge-base", DRY_RUN_BASE, "HEAD"]),
        GIT_TIMEOUT,
    );
    if merge_base.exit != Exit::Code(0) {
        return Err(format!(
            "git merge-base {DRY_RUN_BASE} HEAD ended {:?}",
            merge_base.exit
        ));
    }
    let base = CommitId::parse(merge_base.stdout.trim())?;
    let session = format!("dry-run:{}", super::short(head.as_str()));
    Ok((base, head, session))
}

fn dry_run_context() -> Result<Context, String> {
    let root = checkout::running_checkout()?;
    let (base, head, session) = dry_run_commits(&root)?;
    let langfuse = read_langfuse()?;
    Ok(Context {
        tag: "dry-run",
        checkout: root,
        worktree: None,
        base,
        head,
        claude: PathBuf::from("claude"),
        github: None,
        session,
        posted: Vec::new(),
        user: current_user(),
        langfuse,
        files: create_files()?,
    })
}

/// `mise which claude` printed one absolute path to a file.
fn decode_which(exit: &Exit, stdout: &str) -> Result<PathBuf, String> {
    let path = stdout.trim();
    match exit {
        Exit::Code(0)
            if path.starts_with('/')
                && path.len() <= PATH_MAX_BYTES
                && !path.contains('\n')
                && Path::new(path).is_file() =>
        {
            Ok(PathBuf::from(path))
        }
        Exit::Code(0) => Err(format!("mise which claude printed `{path}`")),
        other => Err(format!("mise which claude ended {other:?}")),
    }
}

/// The Claude binary the base checkout's `mise.toml` pins, by absolute path: a mise shim run
/// with its working directory in the pull request's checkout would take that checkout's pin.
fn resolve_claude(root: &Path) -> Result<PathBuf, String> {
    let captured = run_bounded(
        crate::child::command("mise")
            .args(["which", "claude"])
            .current_dir(root),
        MISE_TIMEOUT,
    );
    decode_which(&captured.exit, &captured.stdout)
}

/// What a posting run reads before it mints the App's token: none of it needs one.
#[derive(Debug)]
struct Request {
    root: PathBuf,
    repository: app_token::Repository,
    trigger: pull_request::Trigger,
    credentials: app_token::AppCredentials,
    langfuse: Option<langfuse::Langfuse>,
    claude: PathBuf,
}

fn read_request() -> Result<Request, String> {
    let root = checkout::running_checkout()?;
    Ok(Request {
        repository: app_token::read_repository(env::var("GITHUB_REPOSITORY").ok())?,
        trigger: read_trigger()?,
        credentials: review_credentials()?,
        langfuse: read_langfuse()?,
        claude: resolve_claude(&root)?,
        root,
    })
}

/// With the App's token: the pull request at its current head, and what the App posted on it
/// before, within the cap.
fn read_pull_request(
    request: &Request,
    token: &str,
    bot: &str,
) -> Result<(pull_request::PullRequest, pull_request::Earlier), String> {
    let client = Client::github();
    let full_name = request.repository.full_name();
    let pull_request = match &request.trigger {
        pull_request::Trigger::Ready(pull_request) => pull_request.clone(),
        pull_request::Trigger::Asked { number } => {
            pull_request::fetch(&client, token, &full_name, *number)?
        }
    };
    let earlier = pull_request::earlier(&client, token, &full_name, pull_request.number, bot)?;
    if earlier.reviews >= REVIEWS_PER_PULL_REQUEST_MAX {
        return Err(format!(
            "the App has posted {} reviews on this pull request, the most one takes",
            earlier.reviews
        ));
    }
    Ok((pull_request, earlier))
}

/// The pull request's head at one commit, as a detached worktree of the base checkout's
/// repository outside the workspace.
#[derive(Debug)]
pub struct Worktree {
    root: PathBuf,
    dir: PathBuf,
}

/// git in `dir` with hooks off and Git LFS not smudging: checking out a pull request's head
/// runs nothing it carries.
fn git_without_hooks(dir: &Path) -> Command {
    let mut command = git_in(dir);
    command
        .args(["-c", "core.hooksPath=/dev/null"])
        .env("GIT_LFS_SKIP_SMUDGE", "1");
    command
}

/// Fetches exactly `head` from the base checkout's `origin` and checks it out as a detached
/// worktree. The review reads that commit and posts on it, whatever the branch points to by
/// then, so nothing pushed after the run resolved the commit can change what it reviews.
fn check_out_head(root: &Path, head: &CommitId, dir: PathBuf) -> Result<Worktree, String> {
    let fetched = run_bounded(
        git_without_hooks(root)
            .args(["fetch", "--quiet", "--no-tags", "--no-recurse-submodules"])
            .args(["origin", head.as_str()]),
        FETCH_TIMEOUT,
    );
    if fetched.exit != Exit::Code(0) {
        return Err(format!("git fetch origin {head} ended {:?}", fetched.exit));
    }
    let added = run_bounded(
        git_without_hooks(root)
            .args(["worktree", "add", "--quiet", "--detach"])
            .arg(&dir)
            .arg(head.as_str()),
        GIT_TIMEOUT,
    );
    if added.exit != Exit::Code(0) {
        return Err(format!("git worktree add {head} ended {:?}", added.exit));
    }
    let worktree = Worktree {
        root: root.to_path_buf(),
        dir,
    };
    match head_commit(&worktree.dir) {
        Ok(checked_out) if &checked_out == head => Ok(worktree),
        checked_out => {
            remove_worktree(&worktree);
            Err(format!("the worktree for {head} is at {checked_out:?}"))
        }
    }
}

/// Removes the head's worktree; a failure is reported, not fatal.
pub fn remove_worktree(worktree: &Worktree) {
    let removed = run_bounded(
        git_without_hooks(&worktree.root)
            .args(["worktree", "remove", "--force"])
            .arg(&worktree.dir),
        GIT_TIMEOUT,
    );
    if removed.exit != Exit::Code(0) {
        eprintln!(
            "xtask review: git worktree remove {} ended {:?}",
            worktree.dir.display(),
            removed.exit
        );
    }
}

/// The pull request's head and the run's own directory, each removed again if the other
/// cannot be made.
fn prepare_trees(root: &Path, head: &CommitId) -> Result<(Worktree, PathBuf), String> {
    let dir = temporary_base().join(format!("stencil-review-head-{}", std::process::id()));
    let worktree = check_out_head(root, head, dir)?;
    match create_files() {
        Ok(files) => Ok((worktree, files)),
        Err(reason) => {
            remove_worktree(&worktree);
            Err(reason)
        }
    }
}

fn post_context(grants: &Grants) -> Result<Context, String> {
    let token = grants
        .app
        .as_ref()
        .ok_or("a posting run holds the App's token")?;
    let request = read_request()?;
    let bot = app_token::bot_login(&request.credentials)?;
    let (pull_request, earlier) = read_pull_request(&request, token.expose(), &bot)?;
    let full_name = request.repository.full_name();
    refuse_fork(&pull_request, &full_name)?;
    let (worktree, files) = prepare_trees(&request.root, &pull_request.head)?;
    Ok(Context {
        tag: "post",
        checkout: worktree.dir.clone(),
        worktree: Some(worktree),
        base: pull_request.base.clone(),
        head: pull_request.head.clone(),
        claude: request.claude,
        session: format!("{full_name}#{}", pull_request.number),
        github: Some(GitHub {
            repository: full_name,
            pull_request,
        }),
        posted: earlier.posted,
        user: current_user(),
        langfuse: request.langfuse,
        files,
    })
}

/// A pull request whose head lives in another repository is refused before its head is
/// fetched: the repository lets only collaborators open pull requests, and the review job,
/// which holds the App's key, never reads a tree someone without write access wrote.
fn refuse_fork(pull_request: &pull_request::PullRequest, repository: &str) -> Result<(), String> {
    if pull_request.head_repository == repository {
        return Ok(());
    }
    Err(format!(
        "the head of #{} is in {}, not {repository}; a fork's pull request is not reviewed",
        pull_request.number, pull_request.head_repository
    ))
}

/// The runner's temporary directory in Actions, outside the workspace; the system's elsewhere.
fn temporary_base() -> PathBuf {
    env::var_os("RUNNER_TEMP")
        .filter(|value| !value.is_empty())
        .map_or_else(env::temp_dir, PathBuf::from)
}

/// A new 0700 directory under the runner's temporary directory, or the system's outside
/// Actions; one left by an earlier run with the same pid is an error, not reused.
fn create_files() -> Result<PathBuf, String> {
    let dir = temporary_base().join(format!("stencil-review-{}", std::process::id()));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    Ok(dir)
}

/// Removes the files this run wrote and their directory; a failure is reported, not fatal.
pub fn remove_files(dir: &Path) {
    let removed = fs::read_dir(dir).and_then(|entries| {
        for entry in entries.take(FILES_MAX) {
            fs::remove_file(entry?.path())?;
        }
        fs::remove_dir(dir)
    });
    if let Err(error) = removed {
        eprintln!("xtask review: cannot remove {}: {error}", dir.display());
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::review::pull_request::PullRequest;

    /// A posting context for a pull request of `o/r`, with the given earlier findings.
    pub fn context(posted: Vec<pull_request::Posted>) -> Context {
        Context {
            tag: "post",
            checkout: PathBuf::from("/r/pull-request"),
            worktree: None,
            base: CommitId::parse(&"b".repeat(40)).unwrap(),
            head: CommitId::parse(&"a".repeat(40)).unwrap(),
            claude: PathBuf::from("/opt/claude"),
            github: Some(GitHub {
                repository: "o/r".to_string(),
                pull_request: PullRequest {
                    number: 1,
                    base: CommitId::parse(&"b".repeat(40)).unwrap(),
                    head: CommitId::parse(&"a".repeat(40)).unwrap(),
                    head_repository: "o/r".to_string(),
                },
            }),
            session: "o/r#1".to_string(),
            posted,
            user: None,
            langfuse: None,
            files: PathBuf::from("/t/stencil-review-1"),
        }
    }

    #[test]
    fn git_and_mise_children_get_only_the_inherited_variables() {
        let git = git_in(Path::new("/r"));
        let mise = crate::child::command("mise");
        for command in [&git, &mise] {
            for (name, _) in command.get_envs() {
                let name = name.to_string_lossy();
                assert!(crate::child::INHERITED.contains(&name.as_ref()), "{name}");
            }
        }
    }

    fn git(dir: &Path, arguments: &[&str]) {
        let status = git_in(dir)
            .args(["-c", "user.name=xtask", "-c", "user.email=xtask@localhost"])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(arguments)
            .status()
            .unwrap();
        assert!(status.success(), "git {arguments:?}");
    }

    #[test]
    fn the_head_is_fetched_by_its_commit_and_a_later_push_changes_nothing() {
        let scratch = env::temp_dir().join(format!("xtask-review-head-{}", std::process::id()));
        if scratch.exists() {
            fs::remove_dir_all(&scratch).unwrap();
        }
        let (origin, root) = (scratch.join("origin"), scratch.join("root"));
        fs::create_dir_all(&origin).unwrap();
        git(&origin, &["init", "-q"]);
        fs::write(origin.join("a.txt"), "base\n").unwrap();
        git(&origin, &["add", "a.txt"]);
        git(&origin, &["commit", "-q", "-m", "base"]);
        git(&scratch, &["clone", "-q", origin.to_str().unwrap(), "root"]);
        fs::write(origin.join("a.txt"), "reviewed\n").unwrap();
        git(&origin, &["commit", "-q", "-am", "reviewed"]);
        let reviewed = head_commit(&origin).unwrap();
        let worktree = check_out_head(&root, &reviewed, scratch.join("head")).unwrap();
        fs::write(origin.join("a.txt"), "pushed later\n").unwrap();
        git(&origin, &["commit", "-q", "-am", "pushed later"]);
        assert_eq!(head_commit(&worktree.dir).unwrap(), reviewed);
        assert_eq!(
            fs::read_to_string(worktree.dir.join("a.txt")).unwrap(),
            "reviewed\n"
        );
        remove_worktree(&worktree);
        assert!(!worktree.dir.exists());
        fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn a_head_in_another_repository_is_refused() {
        let mut pull_request = PullRequest {
            number: 7,
            base: CommitId::parse(&"b".repeat(40)).unwrap(),
            head: CommitId::parse(&"a".repeat(40)).unwrap(),
            head_repository: "o/r".to_string(),
        };
        assert_eq!(refuse_fork(&pull_request, "o/r"), Ok(()));
        pull_request.head_repository = "someone/r".to_string();
        let refusal = refuse_fork(&pull_request, "o/r").unwrap_err();
        assert!(refusal.contains("#7 is in someone/r"), "{refusal}");
    }

    /// Grants with an installation token of the given value and no Anthropic token.
    pub fn grants(token: &str) -> Grants {
        Grants {
            anthropic: None,
            app: Some(app_token::InstallationToken::for_tests(token)),
            run_time: RUN_TIME_MAX,
        }
    }

    #[test]
    fn a_token_too_short_for_one_pass_is_refused_with_its_lifetime() {
        assert_eq!(check_token_lifetime(598), Ok(()));
        assert_eq!(check_token_lifetime(TOKEN_LIFETIME_MIN.as_secs()), Ok(()));
        let refused = check_token_lifetime(TOKEN_LIFETIME_MIN.as_secs() - 1).unwrap_err();
        assert!(refused.contains("lasts 239 s"), "{refused}");
        assert!(refused.contains("at least 240 s"), "{refused}");
    }

    #[test]
    fn only_an_absolute_path_to_a_file_is_the_claude_binary() {
        let file = env::current_exe().unwrap();
        let printed = format!("{}\n", file.display());
        assert_eq!(decode_which(&Exit::Code(0), &printed), Ok(file));
        assert!(decode_which(&Exit::Code(0), "claude\n").is_err());
        assert!(decode_which(&Exit::Code(0), "/no/such/claude\n").is_err());
        assert!(decode_which(&Exit::Code(1), "").is_err());
    }

    /// `mise which claude` runs with only `child::INHERITED`, and a posting run needs it
    /// before any pass; the dry run never calls it.
    #[test]
    fn mise_names_the_pinned_claude_under_the_inherited_variables_alone() {
        let root = checkout::running_checkout().unwrap();
        let claude = resolve_claude(&root).unwrap();
        assert!(
            claude.is_absolute() && claude.is_file(),
            "{}",
            claude.display()
        );
    }

    #[test]
    fn a_files_directory_is_private_and_removed_with_its_files() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = create_files().unwrap();
        let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        fs::write(dir.join("diff.patch"), "x").unwrap();
        assert!(create_files().is_err());
        remove_files(&dir);
        assert!(!dir.exists());
    }
}
