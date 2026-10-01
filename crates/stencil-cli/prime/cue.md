# CUE

Figures can be authored in CUE, which carries rules the Rust side does not evaluate. cue/core.cue holds the core vocabulary, cue/grammar.cue the #Grammar definition, and cue/grammars/gcp.cue and plain.cue each grammar's data, kinds and rules as packages gcp and plain. cue/ is the module root, so every command runs from inside it. The pinned CUE version is 0.17.1. In order, each step stopping on failure:

    cd cue && cue vet -c ./figures:gcp
    cue export ./figures:gcp -e customer --out json -o ../fig.json
    cd .. && stencil check fig.json && stencil render fig.json --out-dir out

-c reports each non-concrete field by path. A plain JSON figure vets against a grammar's #Page directly: `cd cue && cue vet -c -d '#Page' ./grammars:gcp ../fig.json`.

Rules CUE adds beyond stencil vet, under the gcp grammar:

| Rule | Error names |
|---|---|
| every product Item carries a subtitle or a fact with source doc or ask | _itemsWithoutSubtitleOrFact.TITLE |
| tint pairing: a region (tint 1 when absent) or a tinted onprem holds no solid pipe, arm or Box of another slot at any depth | _otherTintInsideZone |
| a solid pipe between two sibling Boxes touches no tinted Box of another slot | _pipeBesideZoneOfOtherTint |
| a tint on a gray or deny line, or on a Box whose kind is not tintable | _tintWithoutEffect |
| Box and Item kinds are the grammar's, and each sits in one of its kind's parents | _kindParentNotAllowed.KIND |
| legend keys (line and tint) equal the keys used by pipes, tee spines and arms, and links | _pipeKeysMissingFromLegend.K, _linkKeysMissingFromLegend.K, _legendKeysUnusedInBody.K |
| legend keys unique | _legendKeysAreUnique |
| ids unique, link endpoints known and distinct | _idsUsedTwice.ID, _linkEndpointsUnknown.ID, _linkToItself.ID |
| via x within the canvas width, y at least 0 | _viaOutsidePage |
| remembered constants rejected in every text field | .FIELD: invalid value |
| grow has one weight per child | _growLengthMatchesChildren |

In the g7 view (cue/figures/g7.cue), gutter pipe p leaves item p of the left column; a tint 1 pipe leaves a tint 1 onprem item and lands in the tint 1 region, tint 2 pairs the same way; the gutter has one pipe per item and one half per on-prem Box (_leavesCardInItsMetro, _onePipePerCard, _oneHalfPerMetro, _halfBesideItsMetro).

Two canvases from one source: customer is concrete except canvas and the hidden _workshop labels; internal is customer & {canvas: "internal", _workshop: {...}}. Any change to a pipe, item or legend entry is a unification conflict, so workshop text cannot reach the customer canvas.

When one node fails, the per-page sets built from it come up empty, so a single error can also report every legend key as unused. The first error on a body path is the cause.

cue/check.sh vets the packages, re-exports both grammars and fails when either differs from crates/stencil-model/grammars, compares the customer export with examples/g7.json, vets every example against the gcp #Page, then runs each negative case. Set CUE to the binary if the shim is not active: `CUE="$(mise which cue)" ./cue/check.sh`.
