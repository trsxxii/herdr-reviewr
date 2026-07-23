---
Status: Current
Created: 2026-06-27
Last edited: 2026-07-23
---

# forge host

How reviewr reads one pull request from the repository's forge — identity, state, checks, comments — through that forge's official CLI, for the read-only `PR` tab (`pr-tab.md`). It never writes back.

## Overview

reviewr resolves the worktree's pull request through its commits, then reads a snapshot of it through the forge's CLI on each poll. Every shown PR provably belongs to this worktree's work. The snapshot is the single value the `PR` tab renders.

The remote's hostname picks the forge. Three forges are supported — GitHub, GitLab, and Azure DevOps — each read through its own CLI. The per-forge contracts live in `forge-providers.md`. Everything below holds for every forge.

```
PR #226  open  persiyanov/deep-research-benchmark → main   ⇡ 2 unpushed
  merge      ⚠ conflicts with main
  checks     ✗ failing — ✓ build-main-image · ✓ review · ✗ tests
  comments   5 (newest first) — @you 5m · @codex 2h · @claude 2h · …
```

The snapshot:

| field                  | type   | meaning                                                                     |
| ---------------------- | ------ | --------------------------------------------------------------------------- |
| `number`               | int?   | PR number, `null` when no PR resolves                                       |
| `title`, `url`         | string | identity                                                                    |
| `body`                 | string | the PR description as the forge returns it, empty when none                 |
| `state`                | enum   | `open`, `merged`, or `closed`                                               |
| `is_draft`             | bool   | draft flag                                                                  |
| `head_ref`             | string | the PR's head branch name, which may differ from the local branch           |
| `head_is_fork`         | bool   | the head lives in another repository                                        |
| `base_ref`             | string | the merge target                                                            |
| `merge`                | enum   | `clean`, `conflicting`, or `blocked`                                        |
| `sync`                 | enum   | `in_sync`, `unpushed`, `behind`, or `unknown`, with a count when known      |
| `checks`               | list   | one row per latest check: `name` and `status` (conclusion folded in)        |
| `comments`             | list   | one row per comment, newest first                                           |
| `truncated`            | bool   | a capped surface had a further page, so a list is a prefix                  |

A `comments` row:

| field                        | type         | meaning                                                                  |
| ---------------------------- | ------------ | ------------------------------------------------------------------------ |
| `kind`                       | enum         | `review` (a review's body), `comment` (conversation), `finding` (inline) |
| `author`, `author_is_bot`    | string, bool | the `@login` and whether it is a bot                                     |
| `anchor`                     | string       | `path:line` for a `finding`, the literal kind word otherwise             |
| `body`, `snippet`            | string       | the text as the forge returns it, only a `finding` carries a snippet     |
| `created_at`                 | time         | post time, the newest-first sort key                                     |
| `is_resolved`, `is_outdated` | bool         | thread state for a `finding`, always false otherwise                     |
| `reply_count`                | int          | replies on a `finding`'s thread beyond the root                          |

## Behavior

### Forge hosts

Each forge recognizes its public hosts. One config key per forge adds one self-hosted hostname. The value contracts live in `config.md`.

| forge        | built-in hosts                            | self-hosted key     |
| ------------ | ----------------------------------------- | ------------------- |
| GitHub       | `github.com`                              | `github_host`       |
| GitLab       | `gitlab.com`                              | `gitlab_host`       |
| Azure DevOps | `dev.azure.com`, `*.visualstudio.com`     | `azure_devops_host` |

Host matching is case-insensitive. A missing key adds no self-hosted host. The built-in hosts remain recognized when a key is present. Azure DevOps' ssh hosts, `ssh.dev.azure.com` and `vs-ssh.visualstudio.com`, fold into their https equivalents, so both clone forms name one target (`forge-providers.md`).

A remote is recognized when its hostname matches a forge host and its path carries that forge's repository identity (`forge-providers.md`). The matching forge's CLI performs every read for that repository.

A repository target is the forge, the hostname, and the repository identity together. The same path on a different forge or hostname is a different target.

Both remotes use the primary fetch URL after Git's `url.*.insteadOf` rewrite. A separate push URL does not affect PR reads.

| remote state                                                        | outcome                                                                     |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `upstream` names a recognized forge host with a repository identity | reviewr reads that repository on that forge                                 |
| `upstream` is absent, hostless, unsupported, or malformed           | `origin` determines the repository                                          |
| reading `upstream` fails                                            | reviewr shows the retryable Git error and never falls through               |
| `origin` names a recognized forge host with a repository identity   | reviewr reads that repository on that forge                                 |
| `origin` names another hosted repository                            | reviewr names the unsupported host and points self-hosted users to the keys |
| `origin` is missing or hostless                                     | reviewr says the PR tab needs a recognized forge `upstream` or `origin`     |
| `origin` names a recognized host without a repository identity      | reviewr says the forge origin is malformed                                  |
| reading `origin` fails                                              | reviewr shows the retryable Git error                                       |

A fork clone with `origin` pointed at the fork and a recognized `upstream` pointed at the base
repository resolves the base repository's PR without setup.

Canonical SSH remotes work in scp-style (`git@host:path.git`) and `ssh://` forms. Hosted URL forms
use `http://`, `https://`, or `git://`. File URLs and other schemes are not forge repository
identities. SSH aliases are not inferred as canonical forge hosts. A CLI's own host override
environment variable, like `GH_HOST`, cannot redirect a fetch.

### Resolution

The worktree's commits nominate pull requests. Containment or exact head identity admits them. A branch name never proves identity, so names play no part in resolution.

- Each fetch pins `HEAD` and the base ref to commit OIDs. Ancestry, distance, and sync calculations use those pins while the agent commits beside it.
- The publication points are the nearest ancestors of the pinned `HEAD` present on `origin`. A point that is an ancestor of any resolved base entry proves nothing and is skipped.
- The pinned `HEAD` also nominates by exact identity when it is not an ancestor of any resolved base entry. A PR admits through this path only when its head is exactly that commit. The PR may be open or merged.
- With no publication point and no exact-identity admission, the tab shows the empty state. The worktree has published no provable work.
- With no base resolvable, no nomination is provable and the tab shows the empty state.
- Each publication point is asked of the forge: which pull requests contain this commit. The repository each forge queries, and its admission mechanics, live in `forge-providers.md`. Only PRs based on the resolved repository target count.
- Every resolved PR therefore contains the worktree's published work or carries its parked commit as the exact head. There is no third admission path.
- Exactly one open PR resolves when one contains a publication point.
- Several open PRs disambiguate in order: a head equal to the pinned `HEAD`, a head equal to a publication point, the head named by the recorded upstream. A record naming a configured base is tracking, not publication, and never joins the tiebreak. Failing all three, reviewr surfaces the count, never a silent guess.
- With no open PR, the newest-merged PR containing a publication point shows as historical state. A merged PR whose head is exactly the pinned `HEAD` resolves the same way, even with no publication point.
- A worktree parked on published base history keeps its epilogue: the absorbed tip still nominates, and a merged PR whose head is exactly that commit resolves as history. Containment proves nothing for an absorbed commit.
- A PR closed without merging does not associate. Where the forge lists closed pull requests, it still resolves as history through exact identity: an `origin` branch tip at a publication point names it, and its reported head equals that point. A forge that lists no closed pull requests has no such epilogue (`forge-providers.md`).
- With none at all, the body shows the calm empty state (`pr-tab.md`).
- A fork PR resolves through the same admission rules. Each provider queries the repository where its forge can prove the association (`forge-providers.md`). `pr-tab.md` marks the fork case.
- Local staleness costs recall first: a stale `origin/*` ref can hide a publication point. A stale base ref can also admit mainline history as one, until a fetch heals it.
- A detached `HEAD` has no pin and shows the empty state.

What a user observes:

- A worktree pushed as `git push origin HEAD:<other-name>` resolves its PR. The pushed commits are the publication point, whatever the name.
- A teammate's PR parked at this worktree's exact `HEAD` never beats the PR on the recorded upstream.
- A remote branch extending `HEAD` can be a colleague's continuation of this work. Its PR resolves when no better pick exists. The header names the resolved branch, and `sync` shows `behind`.
- A worktree with no commits beyond the base shows the empty state. A sibling worktree's PR never attaches to it.
- A reused branch name never resurrects an earlier, unrelated PR. Old PRs do not contain this worktree's commits.
- The worktree's own merged PR shows as history while the space stays parked on its branch, even after the base absorbs the merge.
- A squash-merged PR shows as history after the forge deletes its remote branch. The parked tip is exactly the PR's head, and that proves it.
- A rebase discards the old publication points. Between the rebase and its force-push, the tab shows the empty state. The push restores it on the next poll.

### Derived state

- `merge` folds the forge's merge and policy state to the blockers worth surfacing: a conflicting merge is `conflicting`, a rule or policy block is `blocked`, and everything else — including a forge still computing mergeability — is `clean`, which the footer shows as nothing. The per-forge folding lives in `forge-providers.md`.
- `sync` compares the pinned `HEAD` OID to the PR's `head_oid`: equal is `in_sync`, `HEAD` ahead is `unpushed` with a `git rev-list --count` count, and `head_oid` ahead is `behind`. If the PR head object is unavailable locally, the relation is `unknown`; reviewr never guesses `in_sync`.
- `unpushed` means the checks and comments on screen describe an older commit than the local tree.

### Checks

- A check row is the latest run for its name. A passed re-run replaces an earlier failure.
- Each forge's check-like surfaces normalize into one list (`forge-providers.md`).
- A top-level rollup gives the overall pass or fail.

### Comments

- Each forge's comment surfaces merge into one list: submitted reviews, inline threads, and conversation comments (`forge-providers.md`).
- A bot's PR-level posts collapse to its latest. A human's are each kept.
- `is_resolved` and `is_outdated` come from the forge, never recomputed against the worktree.
- Outdated and resolved threads stay in the list with their marker.
- Each surface reads its newest 100 rows, never paged to exhaustion. A further page on any surface — reviews, comments, threads, or checks — sets `truncated`, and `pr-tab.md` marks the capped list, so it is never presented as complete. A forge that cannot identify its newest page serves the oldest page, marked truncated.

### Refresh

- The first fetch starts when the panel opens, so the tab is populated before the user reaches it.
- A refetch fires on entering the tab, on the `refresh` binding (default `r`), and on the agent's turn-end (a `working` → `idle`/`done` edge) on any tab. A turn may have pushed or merged, changing forge state with no other local signal, and one fetch per turn keeps the tab fresh before it is entered.
- A fallback poll refetches every 60 seconds while the tab is active. Off the tab there is no polling.
- The locally derived state — the pinned `HEAD` and base, the publication points, the tiebreak — moves on a mere commit or push, so it is freshness, never identity. Its churn alone never blanks the tab.
- A refresh that observes a different repository target or origin clears the current PR. reviewr cannot prove the snapshot still describes the same pull request (`overview.md` Continuity).
- Every other observed change keeps the snapshot painted and refetches behind it. The same pull request with newer work is stale, not wrong. The in-flight glyph covers the gap (`tui.md`).
- Either observation starts the replacement fetch at once, on or off the tab, so entering the tab finds fresh work already underway.
- One fetch is in flight at a time. The `refresh` binding cancels the fetch in flight and starts fresh. Any other trigger arriving mid-flight rides it: the result paints, then one fresh fetch supersedes it.
- A forge-side change during a fetch can appear on the following fetch.
- Each fetch uses one validated config snapshot for host and base selection (→ CFG-ONE-SNAPSHOT, `config.md`).
- A forge result paints only if the current config, repository target, pinned `HEAD`, pinned base, and
  publication points still match the input that produced it. If reviewr cannot prove that match, the
  result never paints, and one replacement fetch starts against the current input.
- If the repository target is proven unchanged before a later branch-state read fails, the visible
  same-target snapshot stays with a retry notice; the next refresh performs a fresh forge fetch.
- The snapshot re-derives in full each fetch. reviewr keeps no hidden or historical PR cache beyond the visible snapshot.
- Exiting reviewr stops scheduling and restores the terminal immediately. No later PR completion
  can paint.

## Failure semantics

reviewr reads the forge and never writes it, so every failure degrades to a clear state. `Changes` and `All files` are unaffected.

- A same-input failure preserves the visible snapshot and shows its remedy. With no same-input snapshot, the remedy fills the tab.

| failure                                 | remedy shown                                          |
| --------------------------------------- | ----------------------------------------------------- |
| missing forge CLI or required extension | that component's install step (`forge-providers.md`)  |
| unauthenticated fetch                   | that CLI's login command (`forge-providers.md`)       |
| any other fetch error                   | the retry error                                       |

- A failure before the repository target resolves replaces any snapshot with the retryable Git error. reviewr cannot prove that the snapshot still belongs to the current target.
- A branch-state Git failure after the same repository target resolves preserves the visible
  same-target snapshot with the retryable Git error.
- An unsupported origin names the host and points self-hosted users to the per-forge host keys.
- An origin that stops being recognized, for example after its host key is removed, replaces any snapshot with the unsupported-host remedy.
- A host key naming a server that runs a different forge fails as the chosen CLI's fetch error.
- No PR at any lifecycle state shows a calm empty state. The next poll lights the tab up when a PR appears.
- Every read is side-effect-free.
- Two active PR tabs on one worktree converge within one poll interval. An inactive tab catches up when entered.

## Non-goals

- No writes to any forge. reviewr never posts, resolves a thread, re-runs a check, or merges. It never routes PR feedback to the agent.
- No transport of its own. Every forge read goes through that forge's CLI, which owns hosts, credentials, and TLS.
- No repository selector or cross-repository search.
- No different parent repositories across sibling worktrees from one clone. Use a separate clone for each parent.
- No SSH host-alias normalization. An alias-only repository needs a canonical-host remote.
- No discovery of an unrecorded publication name on a non-`origin` remote.
- No event subscription. The snapshot polls the CLI, no webhook or socket.
- No server-version compatibility layer for self-hosted schemas.

## Related specs

- [forge-providers](./forge-providers.md)
- [configuration](./config.md)
- [pr-tab](./pr-tab.md)
- [herdr-host](./herdr-host.md)
- [overview](./overview.md)
