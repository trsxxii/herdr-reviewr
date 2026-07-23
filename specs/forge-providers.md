---
Status: Current
Created: 2026-07-22
Last edited: 2026-07-23
---

# forge providers

The per-forge contracts behind `forge-host.md`: repository identity, the CLI and its remedies, and how each forge's concepts fill the normalized snapshot.

## Overview

Every forge fills the same snapshot and renders through the same `PR` tab. Only the noun and the reference form differ on screen.

| forge        | CLI                                 | noun          | abbreviation | reference |
| ------------ | ----------------------------------- | ------------- | ------------ | --------- |
| GitHub       | `gh`                                | pull request  | `PR`         | `#226`    |
| GitLab       | `glab`                              | merge request | `MR`         | `!42`     |
| Azure DevOps | `az` + the `azure-devops` extension | pull request  | `PR`         | `#12`     |

User-facing strings use the resolved forge's vocabulary: its name, noun, abbreviation, and reference form (`pr-tab.md`).

Each forge's section covers its repository identity, its CLI, its snapshot mappings, and an `Admission` bullet: which repository its queries address and any admission path of its own, under the neutral resolution rules (`forge-host.md`). A mapping not stated below is the identity.

## GitHub

- Identity: `owner/repository`.
- CLI: `gh`. The login remedy is `gh auth login --hostname <host>`.
- Merge: `CONFLICTING`/`DIRTY` folds to `conflicting`, `BLOCKED` to `blocked`, everything else (`CLEAN`, `BEHIND`, `UNSTABLE`, still-computing `UNKNOWN`) to `clean`. `mergeable=UNKNOWN` is GitHub computing lazily and folds to `clean` unless `mergeStateStatus` is `DIRTY`.
- Checks: check runs and commit statuses normalize into the one list.
- Comments: submitted reviews are `review` rows, review threads are `finding` rows with GitHub's resolved and outdated flags, and conversation comments are `comment` rows.
- Admission: the containment query runs against the `origin` repository, where the commits live — GitHub walks the fork network from there. An `origin` on another host proves nothing, so the target repository stands in.

## GitLab

- Identity: the full namespace path. Groups nest, so the path may hold more than two segments.
- CLI: `glab`. The login remedy is `glab auth login --hostname <host>`.
- State: `opened` maps to `open`, `merged` to `merged`, and `closed` or `locked` to `closed`. A cross-project merge request sets `head_is_fork`.
- Merge: a conflict folds to `conflicting`, a blocked status to `blocked`, and everything else to `clean`. A blocked status is unresolved blocking discussions, missing required approvals, or a denied policy. A still-checking status folds to `clean`.
- Checks: the head pipeline's jobs, one row per job. A job allowed to fail contributes a skipped check, never a failing one. A jobs page past the row cap adds one `pipeline` row carrying the pipeline's own verdict. No pipeline, or a pipeline project the user cannot read, shows an empty checks list.
- Comments: MR-level notes are `comment` rows, diff-position discussions are `finding` rows with GitLab's resolved flag, and an approval is a `review` row. A discussion carries no code context, so a GitLab `finding` has no snippet. An access-token service account (`project_…_bot…`, `group_…_bot…`) counts as a bot, as do the `[bot]` and `-bot` name suffixes. An unavailable approvals surface contributes no `review` rows. A discussion page total past GitLab's counting ceiling (~10,000 rows) serves the oldest page, marked truncated.
- Admission: every query runs against the target project. GitLab scopes merge-request lookup to the target, and an open fork MR's commits reach it through the merge-request refs. A commit the target does not know proves nothing and never fails the fetch. When containment resolves nothing, a publication point's branch name nominates merge requests in any state, and exact head identity admits them.

## Azure DevOps

- Identity: `organization/project/repository`. The accepted URL forms are `dev.azure.com/{organization}/{project}/_git/{repository}`, `ssh.dev.azure.com:v3/{organization}/{project}/{repository}`, and the legacy `{organization}.visualstudio.com` and `vs-ssh.visualstudio.com:v3` equivalents. A `DefaultCollection` filler segment on a legacy host drops, and a repository named after its project may omit the project segment. A project or repository name travels percent-encoded in the URL and is addressed decoded.
- CLI: `az` with the `azure-devops` extension. A missing extension shows its own install step. The login remedy is `az login`, or `az devops login` for a personal access token.
- State: `active` maps to `open`, `completed` to `merged`.
- Merge: a merge status of conflicts folds to `conflicting`. A rejected required policy folds to `blocked`. Everything else, including a still-queued merge check, folds to `clean`.
- Checks: policy evaluations and commit statuses normalize into the one list.
- Comments: PR-level threads are `comment` rows, file-position threads are `finding` rows with the thread's resolved status, and a reviewer vote is a `review` row. A thread carries no code context, so an Azure DevOps `finding` has no snippet. The platform's service identities and build-service accounts count as bots, as do the shared name suffixes.
- Admission: an open pull request admits when its reported source tip is exactly the pinned `HEAD` or a publication point. A completed pull request admits by the same exact source tip, an absorbed one included. The provider finds candidates by enumerating the repository's newest 100 active and newest 100 completed pull requests, and enumeration only nominates. An abandoned pull request never resolves, so Azure DevOps has no closed epilogue. An unreadable enumeration fails the fetch.

## Non-goals

- No forge beyond these three.
- No per-forge rendering. The `PR` tab renders only the normalized snapshot.

## Related specs

- [forge-host](./forge-host.md)
- [configuration](./config.md)
- [pr-tab](./pr-tab.md)
