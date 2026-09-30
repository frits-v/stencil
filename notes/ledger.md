# Ledger

Lessons for sessions in this repository. Every rule names what enforces it.

| Date | Rule | Enforced by |
|---|---|---|
| 2026-09-30 | A change under examples/ regenerates the render fixtures under crates/stencil-render/tests/fixtures and runs cargo test --workspace before the PR opens. stencil check on the changed figure is not verification of the repository. | .github/workflows/ci.yml on every pull request; `mise run land -- <pr>` waits for the checks and squash-merges, so the conventional PR title becomes the commit subject (the repository allows squash merges only; branch protection is unavailable on this private repository) |
