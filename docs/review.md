# Code review

Claude reviews a pull request and the review App `frits-code-review` posts a verdict on it:
`APPROVE`, `REQUEST_CHANGES` or `COMMENT`. `xtask review` does all of it; the workflows only
check out, build and run it.

## When a review runs

- A pull request opens as a draft (`gh pr create --draft`) and is marked ready with
  `gh pr ready <number>`. GitHub sends `ready_for_review` only when a draft is marked ready,
  so a pull request opened ready starts no review.
- The `review` workflow starts on `pull_request_target` when a pull request whose author is
  an owner, member or collaborator is marked ready, and on `issue_comment` when a comment on a
  pull request mentions `@frits-code-review` or `@claude` as a whole word, from an account of
  type `User` whose association is `OWNER`, `MEMBER` or `COLLABORATOR`. Opening a pull
  request, pushing to it or editing a comment starts nothing; the workflow's condition and the
  xtask's event decoder both enforce this.
- A pull request whose head is in another repository is refused before its head is fetched
  (`refuse_fork` in `xtask/src/review/setup.rs`), so every head the review reads was pushed by
  someone with write access. This is the boundary the job relies on: the Claude CLI that
  reads the head runs as the same user as the xtask that holds the App's key. The
  repository's pull request creation policy is `collaborators_only`, so a stranger cannot
  open the pull request that refusal would end.
- The review's own text mentions nobody and references nothing: model text is posted on one
  line, its code spans inside fences the review builds, markdown characters outside them as
  entities, and a word joiner after `@` and between `#`, `/` or `-` and a digit. A review
  never asks for another one.
- A pull request takes at most 20 reviews from the App; past that the run fails before
  calling the model.
- Runs of one pull request share a concurrency group on the job, after its condition: a new
  request cancels the run in progress, and a comment that starts no review cancels nothing.

## What runs where

- The trusted `review` job runs the base branch's workflow and xtask. It checks out the base
  branch with its history, builds the base branch's xtask before any secret is mapped, and
  runs `xtask review --post`. The workflow checks out nothing of the pull request.
- The xtask resolves the head commit (from the `ready_for_review` event, or from the pull
  requests API for a comment), fetches exactly that commit from `origin` and checks it out as
  a detached worktree under
  `RUNNER_TEMP`, with hooks off and Git LFS not smudging. It checks the worktree is at that
  commit and posts the review on it, so a push after the commit was resolved changes nothing
  about what is reviewed.
- Nothing from the worktree is built or run. git runs in it with `--no-ext-diff
  --no-textconv`; the Claude binary is the one `mise which claude` names from the base
  checkout, since a mise shim run in the worktree would take the pull request's pin; the
  model gets only read tools. The worktree is removed when the run ends.
- Every process the review starts begins from an empty environment and gets back only the
  variables `xtask/src/child.rs` names: the path, home, user, locale, temporary directory and
  mise's directories. The App key, the Langfuse secret key, the Actions OIDC request pair and
  any job token stay in the xtask's own environment and are in no child's.
  `no_secret_reaches_a_child` in that file starts the test binary with every secret set and
  reads the environment of the child it starts, and the opengrep rule
  `rust-review-child-environment` refuses `Command::new` in `xtask/src/review.rs` and
  `xtask/src/review/`. This keeps a secret out of what a child inherits, not out of reach of
  the user the job runs as: `/proc/<pid>/environ` of the xtask holds the secrets, and would
  hold them even after `unsetenv`, which leaves the original strings in place. The xtask
  never changes its own environment, which Rust marks unsafe once a second thread may read
  it.
- The job runs in the `review` environment, whose secrets are the App key and the Langfuse
  key. `pull_request_target` and `issue_comment` runs carry the default branch's ref, so an
  environment that admits only the default branch serves them and refuses every branch's own
  workflow.
- `.github/workflow-policy.cue`, checked by `mise run workflows` over every workflow file,
  holds a workflow a pull request can start (`pull_request_target`, `issue_comment`,
  `workflow_run`) to checking out only the base branch, with no `ref` or `repository`, and
  to no workflow-level `concurrency`. Its fixtures are under `xtask/fixtures/workflow-policy`.
- The `review-dry-run` workflow runs by hand from any branch:
  `gh workflow run review-dry-run.yml --ref <branch>`. It runs that branch's xtask with
  `xtask review --dry-run` against `origin/main`, prints the verdict it would give and the
  findings, writes them to the step summary and posts nothing; it holds no App key. It is how
  a change to the reviewer is tried before it reaches `main`. On `main` itself the diff is
  empty and the run fails. `mise run review` does the same locally with the developer's
  Claude login and refuses a tree with uncommitted changes, since the model reads the
  working tree.

## The review

- The xtask diffs the merge base against the head, leaving out `Cargo.lock`, the render
  fixtures under `crates/stencil-render/tests/fixtures`, `docs/gallery` and `docs/themes`.
  A diff over 512 KiB, or one with no head-side lines once those are left out, fails the run
  instead of being cut.
- A find pass runs `claude -p` (model alias `opus`) with the diff, `CLAUDE.md`, the ledger
  and `docs/SPEC.md` to read, and returns at most 20 candidates in the JSON Schema
  `xtask/schemas/review_findings.json`. The App's earlier findings on the pull request are
  listed for it; one that still holds is reported again.
- Each of the 12 most severe candidates gets a verify pass in a fresh session, four at a
  time, that tries to refute it and answers `confirmed` or `refuted` in
  `xtask/schemas/review_verdict.json`; the job log shows each verdict's reasoning.
- The model runs with the tools Read, Grep and Glob under permission mode `dontAsk`, which
  refuses any read outside the reviewed checkout and the run's own 0700 directory, passed
  with `--add-dir`; that directory holds the diff, the earlier findings and the instructions.
  `--setting-sources ""` keeps the pull request's `.claude/settings.json` from adding a hook
  or a permission. Its environment is `PATH`, `HOME`, `LANG`, `TMPDIR`, the user name and
  the Anthropic access token.
- Each find pass is capped at 80 turns and $10, each verify pass at 30 turns and $2.

## The verdict

- A rule decides it from the confirmed findings, never the model: a confirmed high or medium
  finding requests changes; without one, a high or medium candidate nobody judged (over the
  cap of 12, or a verify pass that ended without a verdict) gets `COMMENT`; otherwise the App
  approves. Low findings never block.
- After the passes, the xtask reads the check runs and commit statuses on the head commit
  every 30 seconds, leaving out the review's own jobs, until none is pending or 45 minutes
  after the start, and lists each with its outcome in the review body. They never change the
  verdict.
- A finding on a line the diff shows becomes an inline comment; any other goes into the body.
  Each carries a fingerprint of its path, category and summary in a hidden
  `<!-- stencil-review … -->` marker, and a finding the App posted before is not posted again.
  Only markers in the App's own comments and reviews count.
- A finding that holds the run's own token fails the run and posts nothing; one that holds
  another credential prefix is withheld and counted.
- The run fails when it cannot post, when a verify pass gives no verdict, when the trace
  cannot be sent, or when revoking the App's token fails; the verdict, when there is one, is
  posted first.

## Credentials

- The App's installation token: an RS256 JWT from `FRITS_CODE_REVIEW_CLIENT_ID` and
  `FRITS_CODE_REVIEW_APP_PRIVATE_KEY`, exchanged for a token limited to this repository and
  to Checks read, Pull requests write and Commit statuses read, whatever else the App is
  granted; a wider grant is refused and revoked. The token is revoked when the run ends.
- Anthropic: the xtask asks the Actions OIDC endpoint for a token with audience
  `https://api.anthropic.com` and trades it at `https://api.anthropic.com/v1/oauth/token`
  with a JWT bearer grant naming `ANTHROPIC_FEDERATION_RULE_ID`, `ANTHROPIC_ORGANIZATION_ID`
  and, when set, `ANTHROPIC_SERVICE_ACCOUNT_ID` and `ANTHROPIC_WORKSPACE_ID`. GitHub's OIDC
  token lasts ten minutes and Anthropic caps an exchanged token at it, so each pass mints its
  own. A first exchange before any pass checks the rule. Without
  `ANTHROPIC_FEDERATION_RULE_ID` a dry run uses the developer's own Claude login.
- This repository's OIDC tokens carry GitHub's immutable-id subject,
  `repo:frits-v@4488681/stencil@1397217802:environment:<name>`, so the federation rule
  matches stable claims in a CEL condition:

  ```text
  claims.repository_id == "1397217802" && (
    (claims.environment == "review" &&
     claims.job_workflow_ref == "frits-v/stencil/.github/workflows/review.yml@refs/heads/main") ||
    (claims.environment == "review-dry-run" &&
     claims.job_workflow_ref.startsWith("frits-v/stencil/.github/workflows/review-dry-run.yml@"))
  )
  ```

  Before each exchange the xtask prints the claims a rule matches on, never the token. A
  refused exchange answers an opaque `401`; the Console's authentication history names the
  failed check. The rule's token lifetime is at least four minutes, or the run fails before
  any pass.
- All exchanges are made in process over HTTPS (rustls on aws-lc-rs, the webpki roots
  compiled in, no proxy, no redirects); no token appears in arguments, output or errors.

## Tracing

When `LANGFUSE_BASE_URL`, `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY` are all set, the
xtask sends one trace per run to `<base>/api/public/otel/v1/traces` as OTLP/JSON under the
service name `stencil-review`. Runs of one pull request share the session
`owner/name#<number>`; a dry run's session is `dry-run:<head>`. Each trace is tagged `post`
or `dry-run`. None of the three set means no trace; some but not all is an error.

- The root span is the review: the pull request, the commit range and the diff in, the
  verdict and its reason out, the counts as metadata.
- Each pass is an agent span, each assistant message a generation linked to the prompt
  version it ran, with its model, token counts, a share of the pass's cost and its output,
  and each tool call a span with its input and the first 16 KiB of its result.
- The templates `xtask/prompts/review-find.md` and `review-verify.md` are registered in
  Langfuse's prompt management as `stencil-review-find` and `stencil-review-verify`, with
  the model, the limits and the output schema as configuration. Each run creates a new
  version when the text or configuration differs from the one labelled `latest`. A
  registration or export that fails fails the run's exit code.

## Setup, once

- Install the App on this repository.
- Create the `review` environment admitting only the default branch, with the App key and
  the Langfuse secret key; create `review-dry-run` with only the Langfuse secret key:

  ```sh
  gh api -X PUT repos/frits-v/stencil/environments/review \
    -F 'deployment_branch_policy[protected_branches]=false' \
    -F 'deployment_branch_policy[custom_branch_policies]=true'
  gh api -X POST repos/frits-v/stencil/environments/review/deployment-branch-policies \
    -f name=main -f type=branch
  gh secret set FRITS_CODE_REVIEW_APP_PRIVATE_KEY -R frits-v/stencil --env review < key.pem
  gh secret set LANGFUSE_SECRET_KEY -R frits-v/stencil --env review
  gh api -X PUT repos/frits-v/stencil/environments/review-dry-run
  gh secret set LANGFUSE_SECRET_KEY -R frits-v/stencil --env review-dry-run
  ```

- Set the repository variables `FRITS_CODE_REVIEW_CLIENT_ID`, `ANTHROPIC_FEDERATION_RULE_ID`,
  `ANTHROPIC_ORGANIZATION_ID`, `ANTHROPIC_SERVICE_ACCOUNT_ID`, `ANTHROPIC_WORKSPACE_ID`,
  `LANGFUSE_BASE_URL` and `LANGFUSE_PUBLIC_KEY`, and create the federation rule above in the
  Anthropic Console.
- Whether the App's approval counts toward a required review is the repository ruleset's
  setting. When it does, a run that fails before posting leaves the pull request without that
  approval: the review fails closed.
