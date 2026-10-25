# CUE

Figures can be authored in CUE, which carries rules the Rust side does not evaluate. cue/core.cue holds the core vocabulary, cue/grammar.cue the #Grammar definition, and cue/grammars/aws.cue, gcp.cue and plain.cue each grammar's data, kinds and rules as packages aws, gcp and plain. cue/ is the module root, so every command runs from inside it. The pinned CUE version is 0.17.1. In order, each step stopping on failure:

    cd cue && cue vet -c ./figures:gcp
    cue export ./figures:gcp -e <field> --out json -o ../fig.json
    cd .. && stencil check fig.json && stencil render fig.json --out-dir out

<field> is the path of the field your figure file binds its page to, named after the figure: a file that writes `topology: gcp.#Page & {...}` exports with `-e topology`, one that writes `figures: topology: ...` with `-e figures.topology`. No name is fixed. Outside this repository, ./figures:gcp is your own figure package, importing github.com/frits-v/stencil/cue/grammars:gcp.

-c reports each non-concrete field by path. A plain JSON figure vets against a grammar's #Page directly: `cd cue && cue vet -c -d '#Page' ./grammars:gcp ../fig.json`.

Rules CUE adds beyond stencil vet, under the gcp grammar:

| Rule | Error names |
|---|---|
| every product Item carries a subtitle or a fact with source doc or ask | _itemsWithoutSubtitleOrFact.TITLE |
| tint pairing: a region (tint 1 when absent) or a tinted onprem holds no solid pipe, arm or Box of another slot at any depth | _otherTintInsideZone |
| a solid pipe between two sibling Boxes touches no tinted Box of another slot | _pipeBesideZoneOfOtherTint |
| no product naming or drawn as Cloud Storage, BigQuery, Pub/Sub, Artifact Registry or Cloud Logging sits inside a vpc | _productInsideVpc.TITLE |
| every apis Box has a gcp Box above it | _apisOutsideGcp.LABEL |
| under aws: no regional service (S3, DynamoDB, SQS and the rest) inside a vpc, no global service inside a region | _regionalServiceInsideVpc.TITLE, _globalInsideRegion.TITLE |
| pipe targets name node ids, two different ones, and never on a Tee arm | _pipeTargetsUnknown.ID, _pipeTargetsEqual, _targetOnTeeArm |
| a tint on a gray or deny line, or on a Box whose kind is not tintable | _tintWithoutEffect |
| Box and Item kinds are the grammar's, and each sits in one of its kind's parents | _kindParentNotAllowed.KIND |
| legend keys (line and tint) equal the keys used by pipes, tee spines and arms, and links | _pipeKeysMissingFromLegend.K, _linkKeysMissingFromLegend.K, _legendKeysUnusedInBody.K |
| legend keys unique | _legendKeysAreUnique |
| ids unique, link endpoints known and distinct | _idsUsedTwice.ID, _linkEndpointsUnknown.ID, _linkToItself.ID |
| via x within the canvas width, y at least 0 | _viaOutsidePage |
| remembered constants rejected in every text field | .FIELD: invalid value |
| grow has one weight per child | _growLengthMatchesChildren |

In the g7 view (cue/figures/g7.cue), gutter pipe p leaves item p of the left column; a tint 1 pipe leaves a tint 1 onprem item and lands in the tint 1 region, tint 2 pairs the same way; the gutter has one pipe per item and one half per on-prem Box (_leavesCardInItsMetro, _onePipePerCard, _oneHalfPerMetro, _halfBesideItsMetro).

Two versions from one source, as cue/figures/g7.cue does with its fields customer and internal: the first page field is concrete except a hidden audience, which sets its badge, and some hidden labels; the second is the first & {_audience: "internal", <labels>}. Any change to a pipe, item or legend entry is a unification conflict, so internal text cannot reach the customer version.

When one node fails, the per-page sets built from it come up empty, so a single error can also report every legend key as unused. The first error on a body path is the cause.

cue/check.sh vets the packages, re-exports every grammar and fails when one differs from crates/stencil-model/grammars, compares g7's customer field, exported, with examples/g7.json, vets every example against the #Page of the grammar it names (plain when it names none), then runs each negative case. Set CUE to the binary if the shim is not active: `CUE="$(mise which cue)" ./cue/check.sh`.
