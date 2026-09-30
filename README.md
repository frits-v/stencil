# stencil

Stencil turns a JSON description of an architecture figure into an SVG, a PNG and a measured JSON. The document describes structure only: zones, product cards, facts, notes and the labeled pipes between them. It carries no coordinates. taffy computes every box, cosmic-text measures every string with the Inter font files bundled in this repository, and resvg renders with the same files, so a measured width is the rendered width. The visual grammar is the Google Cloud Architecture Center stencil. The contract is `docs/SPEC.md`.

Build the binary with `cargo build --release`; it lands at `target/release/stencil`.

## Commands

An agent runs `stencil prime` first. It prints a briefing of about 1,500 tokens: the authoring loop, the vocabulary with every field and enum value taken from the model, the layout rules, the checks and how to fix each defect. `stencil prime <topic>` prints one deeper section: `themes`, `links`, `blocks`, `layout`, `checks`, `cue`, or `example` (the g7 document).

```bash
stencil prime
stencil prime links
```

`stencil vet <json>` parses the document, applies the vet rules (field types, text limits, container sizes, nesting depth) and then runs the two checks that need no layout: remembered constants and legend consistency.

```bash
stencil vet examples/g7.json
```

`stencil render <json> --out-dir <dir> [--scale <1-4>]` lays out the page and writes `<stem>.svg`, `<stem>.png` and `<stem>.measured.json` into the output directory, creating it when missing. It prints the three absolute paths. The PNG scale defaults to 2.

```bash
stencil render examples/g7.json --out-dir out
```

`stencil check <json>` does everything `render` does in memory, writes nothing, and runs all eight checks: child inside container, siblings do not overlap, text fits box, remembered constants, legend consistency, links routed, links avoid boxes and pipes land.

```bash
stencil check examples/g7.json
```

`stencil schema` prints the JSON Schema of the document, the same text as `schema/stencil.schema.json`.

```bash
stencil schema > stencil.schema.json
```

Every check line reports how many units it examined, and a check that examined nothing fails. The exit code is 0 when everything passed, 1 when the document has a defect (invalid JSON, a vet violation, a failing check, a character Inter lacks) and 2 when stencil could not run (bad arguments, an unreadable input, an unwritable output directory, a font or renderer fault).

## The CUE gate

Figures are authored in CUE against `cue/stencil.cue`, which carries rules the Rust side does not evaluate, such as region tint pairing and the requirement that every product card has a product name, fact or ask. A figure passes three steps in order, and each one stops the pipeline on failure:

```bash
cue vet -c ./cue
cue export ./cue -e customer --out json -o figure.json
stencil render figure.json --out-dir out
```

`cue vet` enforces the authoring rules, `cue export` writes the structural JSON, and `stencil render` vets that JSON again before it lays out and renders. Run `stencil check figure.json` between the export and the render to see every check result before any file is written. `cue/README.md` describes the CUE package and `cue/check.sh` runs its negative cases.

## Fonts and icons

The four Inter faces in `assets/fonts/` are unmodified files from the Inter 4.1 release under the SIL Open Font License 1.1, whose text ships next to them as `OFL.txt`. The Google Cloud icons in `assets/icons/` are unaltered copies from the official icon library at cloud.google.com/icons, used on labeled product cards in technical figures under the terms recorded in `assets/icons/PROVENANCE.md`.
