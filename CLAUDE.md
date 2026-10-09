# stencil

Run `stencil prime` before authoring a figure; it is the briefing. Work in a worktree off origin/main and land through a pull request with `mise run land -- <number>`, which squash-merges only after CI passes. CI is `mise run ci`, the `ci` command of the xtask crate: fmt, clippy, workspace tests, the CUE gate, the gallery and theme docs, the opengrep rules, and zizmor and jactionlint at pedantic. A check goes into the xtask, not into workflow YAML, and a tool is a mise pin, not an action. Commit subjects and PR titles follow conventional commits.

@notes/ledger.md
