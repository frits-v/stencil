# CUE

Figures can be authored in CUE against cue/stencil.cue, which carries rules the Rust side does not evaluate. The pinned CUE version is 0.17.1. From the repository root, in order, each step stopping on failure:

    cue vet -c ./cue
    cue export ./cue -e customer --out json -o fig.json
    stencil check fig.json && stencil render fig.json --out-dir out

-c reports each non-concrete field by path. A plain JSON figure vets against the schema directly: `cd cue && cue vet -c -d '#Page' . ../fig.json`.

Rules CUE adds beyond stencil vet:

| Rule | Error names |
|---|---|
| every Pcard carries pn, fact or ask | _pcardsWithoutPnFactOrAsk.FN |
| region tint: region-a and onprem-a are tint a, region-b and onprem-b tint b, blue pipes a, pink pipes b; a tinted zone holds no pipe, arm or zone of the other tint at any depth | _otherTintInsideZone |
| a pipe between two sibling zones touches no zone of the other tint | _pipeBesideZoneOfOtherTint |
| legend kinds equal the kinds used by pipes, tee spines and arms, and links | _pipeKindsMissingFromLegend.K, _linkKindsMissingFromLegend.K, _legendKindsUnusedInBody.K |
| legend kinds unique, so at most five entries | |
| ids unique, link endpoints known and distinct | _idsUsedTwice.ID, _linkEndpointsUnknown.ID, _linkToItself.ID |
| via x within the canvas width, y at least 0 | _viaOutsidePage |
| remembered constants rejected in every text field | .FIELD: invalid value |
| canvas concrete: each exported figure has exactly one | |
| grow has one weight per child | _growLengthMatchesChildren |

In the g7 view, gutter pipe p leaves card p of the left column; a blue pipe leaves an onprem-a card and lands in region-a, pink pairs onprem-b with region-b; the gutter has one pipe per card and one half per on-prem zone (_leavesCardInItsMetro, _onePipePerCard, _oneHalfPerMetro, _halfBesideItsMetro).

Two canvases from one source: customer is concrete except canvas and the hidden _workshop labels; internal is customer & {canvas: "internal", _workshop: {...}}. The internal canvas can add labels, and any change to a pipe, card or legend entry is a unification conflict, so workshop text cannot reach the customer canvas.

When one node fails, the per-page sets built from it come up empty, so a single error can also report every legend kind as unused. The first error on a body path is the cause.

cue/check.sh vets the package, exports both views, compares the customer export with examples/g7.json, vets examples/onepager.json against #Page, then runs each negative case: a copy of g7.cue with one edit that vet must reject with the named error. It fails when cue is missing, an edit does not apply, vet passes, or vet fails for another reason. Set CUE to the binary if the shim is not active: `CUE="$(mise which cue)" ./cue/check.sh`.
