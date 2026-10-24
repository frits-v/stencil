// The policy every file under .github/workflows meets, checked by `xtask workflows` with
// `cue vet -c`. A workflow that a pull request's author can start while it holds the base
// repository's secrets (`pull_request_target`, `issue_comment`, `workflow_run`) checks out
// only the base branch, never a ref or repository the author controls, and sets no
// workflow-level `concurrency`, which applies before any job's condition. Unlike
// jactionlint's `untrusted-checkout`, an `environment:` on the job exempts nothing.
package workflows

import (
	"list"
	"strings"
)

on: {...}

_privileged: list.Contains([for name, _ in on {name}], "pull_request_target") ||
	list.Contains([for name, _ in on {name}], "issue_comment") ||
	list.Contains([for name, _ in on {name}], "workflow_run")

if _privileged {
	concurrency?: error("a workflow a pull request can start sets concurrency on the job, after its condition")
	jobs: [string]: steps?: [...{
		uses?: string
		if uses != _|_ if strings.HasPrefix(uses, "actions/checkout@") {
			with?: {
				ref?:        error("a privileged workflow checks out only the base branch: no ref")
				repository?: error("a privileged workflow checks out only the base branch: no repository")
				...
			}
		}
		...
	}]
}
