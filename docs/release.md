# Releases

The `release` workflow runs on every push to `main`, through the xtask. A release is a GitHub
release of a `vX.Y.Z` tag with the `stencil` archives for macOS (Apple silicon and Intel) and
Linux x86_64, `gallery.zip` and the center PNG of every example attached.

## What runs

- `xtask release-pr` runs `release-plz release-pr`. It opens or updates one release pull
  request with the next versions and the changelog, derived from the conventional commits
  since the last tags. Only commits whose type is `feat`, `fix`, `perf`, `refactor` or
  `revert` open one. Nothing is published to crates.io.
- Merging the release pull request makes `xtask release` tag the merge commit, create the
  GitHub release, build the gallery and attach `gallery.zip` and the center PNGs to it. The
  `binaries` job then runs `xtask release-binaries` on macOS and Linux, which builds the
  archives with dist and uploads them. Re-running the workflow on a release commit rebuilds
  and replaces every asset. The release pull request lands like any other, with
  `mise run land -- <number>`.
- `mise run release-pr -- --dry-run` shows the next release pull request locally without a
  token, in a scratch worktree under `target/release-dry-run`. release-plz runs
  `cargo package` in a temporary copy of the tree, so it needs the cargo of an activated mise
  shell on `PATH`; through the mise shims the copy's `mise.toml` is untrusted and the run fails.

## Versions and tags

Every workspace crate but the xtask is released. The four library crates are tagged
`<crate>-vX.Y.Z` and get no GitHub release and no changelog; `stencil-cli` is tagged `vX.Y.Z`
and gets both, and its `CHANGELOG.md` at the repository root includes the commits of the
library crates. release-plz bumps a crate when a commit touches its files, and bumps every
crate that depends on it in the same pull request, so a change in `stencil-layout` alone
releases `stencil-layout`, `stencil-render` and `stencil-cli`. Releasing the binary alone would
open no pull request for that change: release-plz reads only the released crates' files.
`every_library_crate_is_released_so_the_binary_bumps_with_it` in `xtask/src/release.rs` reads
`release-plz.toml` against the workspace members.

`dist-workspace.toml` names the targets; `[package.metadata.dist]` in `crates/stencil-cli`
marks the one package dist builds, and the archives are named after it:
`stencil-cli-<target>.tar.xz`, each holding the `stencil` binary and the README, with a
`.sha256` beside it. The `dist` profile in `Cargo.toml` is the release profile with thin LTO.
`cargo-dist-version` there and the `cargo-dist` pin in `mise.toml` are the same version, read
by `dist_runs_at_the_version_mise_pins`.

## The release pull request's token

release-plz writes the release pull request's commit through the GitHub API, so GitHub signs
it and the `main` rules accept it. A pull request opened with the workflow's own
`GITHUB_TOKEN` starts no workflow runs, so it could never pass `ci`; the `release-pr` job
opens it with a GitHub App installation token that the xtask mints for each run:

- The xtask signs an RS256 JWT with the App's private key (`iat` sixty seconds in the past,
  `exp` ten minutes after `iat`, `iss` the App's client id from `STENCIL_RELEASE_CLIENT_ID`,
  or its numeric App id from `STENCIL_RELEASE_APP_ID` when no client id is set), looks up the
  App's installation on this repository, and requests an installation token limited to this
  repository and to Contents and Pull requests write. A token whose grant is wider, or whose
  grant the response does not state, it refuses and revokes. The token reaches release-plz as
  `GIT_TOKEN` in its environment and is revoked when release-plz exits. The xtask makes the
  lookup, the token request and the revocation itself, over HTTPS, as the review does
  (`xtask/src/app_token.rs`). The key, the JWT and the token never appear in arguments,
  output, errors or child environments: release-plz and the cargo and git processes it
  starts get the job's environment without the App's ids and key
  (`release_plz_never_receives_the_app_credentials`).
- The workflow builds the xtask in a step without the key, then runs the built binary in the
  step that holds it, so no build script or proc-macro inherits the key
  (`the_step_holding_the_app_key_runs_a_built_binary_not_a_build`). A build script in the
  build step can still write `$GITHUB_ENV` or replace `target/debug/xtask`; closing that takes
  a separate build job that passes the binary as an artifact checked against its digest, which
  the workflow does not do.
- A failed revocation fails the job. A run cancelled between minting and revoking leaves the
  token valid until GitHub expires it, one hour after it was issued.
- The release pull request's author is the App's bot account, `<app-name>[bot]`. The `main`
  rules require no approving review, so `mise run land` merges it once `ci` passes.
- The job runs in the `release` environment, whose secret is the App key. The environment
  admits only the default branch, so no other branch's workflow can read it. The `release` and
  `binaries` jobs use the workflow's own `GITHUB_TOKEN`.

## Setup, once

Create a GitHub App owned by the repository's owner with webhooks off, repository permissions
Contents and Pull requests set to read and write and nothing else, installable only on this
account. Install it on this repository only. Generate a private key, then store the App's
client id (shown on the App's settings page) and the key in the `release` environment and
delete the downloaded file:

```sh
gh api -X PUT repos/frits-v/stencil/environments/release \
  -F 'deployment_branch_policy[protected_branches]=false' \
  -F 'deployment_branch_policy[custom_branch_policies]=true'
gh api -X POST repos/frits-v/stencil/environments/release/deployment-branch-policies \
  -f name=main -f type=branch
gh variable set STENCIL_RELEASE_CLIENT_ID -R frits-v/stencil --env release --body <client-id>
gh secret set STENCIL_RELEASE_APP_PRIVATE_KEY -R frits-v/stencil --env release < key.pem
rm key.pem
```

`STENCIL_RELEASE_APP_ID` with the numeric App id is accepted in place of the client id; when
both are set the client id is used. Until a client id or App id variable and the secret exist
in the environment, the `release-pr` job fails with a message naming them and this document;
the `release` and `binaries` jobs are unaffected. A key that does not belong to the App the
id names, or an App not installed on this repository, fails the installation lookup with
GitHub's message.

Rotation: generate a new private key in the App settings, store it with the same
`gh secret set` command, delete the local file, then delete the old key in the App settings.
The next `release-pr` run uses the new key.
