# stencil

Run `stencil prime` before authoring a figure; it is the briefing. Work in a worktree off origin/main and land through a pull request with `mise run land -- <number>`, which squash-merges only after CI (fmt, clippy, workspace tests, CUE gate) passes. Commit subjects and PR titles follow conventional commits.

@notes/ledger.md
