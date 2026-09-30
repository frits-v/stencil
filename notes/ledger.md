# Ledger

Lessons for sessions in this repository. Every rule names what enforces it.

| Date | Rule | Enforced by |
|---|---|---|
| 2026-09-30 | A change under examples/ regenerates the render fixtures under crates/stencil-render/tests/fixtures and runs cargo test --workspace before the PR opens. stencil check on the changed figure is not verification of the repository. | .github/workflows/ci.yml on every pull request; `mise run land -- <pr>` waits for the checks and squash-merges, so the conventional PR title becomes the commit subject (the repository allows squash merges only; branch protection is unavailable on this private repository) |
| 2026-09-30 | In an iso figure, a zone edge through label text is a check defect, not a note for visual review; labels move to open floor before the check runs. | `iso-labels-clear` slab-edge pairs in crates/stencil-render/src/iso.rs, tested in crates/stencil-render/tests/iso.rs; `stencil gallery` fails on any example whose own projection is iso |
| 2026-09-30 | In an iso figure, a link leg within 24 px of a parallel zone edge, a leg that turns back, or a last leg shorter than two arrowheads is a check defect, not a note for visual review. | `iso-links-clear` in crates/stencil-render/src/iso.rs, tested in crates/stencil-render/tests/iso.rs; `stencil gallery` fails on any example whose own projection is iso |
