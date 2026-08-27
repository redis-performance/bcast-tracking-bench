# Mined history: redis-performance/bcast-tracking-bench

Mined 2026-08-27 via `gh pr list --state all --limit 300`, `gh issue list --state all --limit 300`,
`gh pr view <n> --json ...`, and `gh api repos/redis-performance/bcast-tracking-bench/pulls/<n>/reviews`
and `.../pulls/<n>/comments` for every PR. This repo was created 2026-06-03; at mining time it had **4
merged PRs, 0 open/closed-unmerged PRs, and 2 open issues**. This is a genuinely small, young,
single-maintainer repo — the notes below are the complete record, not a sample.

## The PRs

| # | Title | Author | Reviews | Inline comments | Merged by |
|---|-------|--------|---------|------------------|-----------|
| 1 | Add CONTRIBUTING.md and AGENTS.md | fcostaoliveira | 0 | 0 | fcostaoliveira |
| 2 | Add Docker image and multi-platform publish workflow | fcostaoliveira | 0 | 0 | fcostaoliveira |
| 3 | Re-enable invalidations_drained counter | fcostaoliveira | 0 | 0 | fcostaoliveira |
| 5 | feat: per-listener ACL-user authentication for send-time filtering benchmarks | fcostaoliveira | 0 | 0 | fcostaoliveira |

Every PR: same author, same person merging, same day open-to-merge, zero GitHub review events, zero
inline diff comments. Every PR body ends with "🤖 Generated with Claude Code" — these are AI-assisted
PRs by the repo's one maintainer, not multi-contributor traffic. **There is no second voice anywhere in
this repo's PR history.** Do not construct one.

### PR body template (real, consistent across #2, #3, #5)

Each of these three PRs' descriptions follows the same real shape — worth recognizing when reviewing,
since a PR that already explains its own tradeoffs this way doesn't need those tradeoffs re-discovered:

- A `## Problem` or `## What's included` section stating what was broken/added and why.
- A `## Fix` / `## Change` section.
- A `## Verification` section listing exactly what was run locally (`cargo build --release`,
  `cargo clippy`, `cargo fmt --check`) and, for #3, an actual before/after runtime check against a real
  Redis instance.
- Sometimes a `## Note for reviewer` section flagging an ambiguity the author is explicitly unsure
  about (PR #3: whether the counter should count invalidation messages or individual keys — this
  became issue #4 item 1).

### PR #3 and issue #4 — the one real substantive precedent

PR #3 fixed a real, concrete bug: `on_invalidation`'s counter increment had been commented out, so
`invalidations_drained` always reported 0, silently making round/shutdown stats useless. The fix itself
(re-enable a `fetch_add`) was small and merged same-day with no review.

**Issue #4**, opened two days later by the same author, is titled "invalidations_drained metric:
semantics & robustness follow-ups" and explicitly says: *"Follow-ups surfaced by a multi-reviewer pass
on #3 ... Raised by reviewers; opus/sonnet consensus = non-blocking for #3."* This is an **AI
multi-model review's output**, not a human maintainer's PR comment — attribute it that way. Its four
findings are real, specific, and worth treating as this repo's evidenced technical bar even though no
human wrote them into the PR thread itself:

1. **Messages vs. keys**: `fetch_add(1)` counts invalidation *messages*, but one RESP3 BCAST push can
   carry multiple keys (`Invalidation.keys: Vec<Key>`); a consumer comparing against `expected_keys`
   likely wants a per-key count.
2. **`expected_keys` is inert**: `start_round`'s `expected_keys` parameter is bound to `_expected_keys`
   and never read — no gating/waiting logic uses it, so there's a read-vs-drain race.
3. **Counter never reset between rounds**: the `AtomicU64` is created once and only incremented, so
   reported values are cumulative across all rounds, not per-round.
4. **Silent under-count on broadcast `Lagged`**: fred's `on_invalidation` consumes a bounded tokio
   broadcast channel (capacity 1024); if a listener falls more than 1024 messages behind, `recv()`
   returns `RecvError::Lagged` and the consuming `while let Ok(...)` loop exits permanently — exactly
   the high-throughput regime this tool is built to stress-test.

None of these blocked #3's merge; they were captured as a follow-up issue instead. That resolution
pattern — real, concrete findings, captured but non-blocking for a small, already-useful fix — is itself
evidence worth citing when calibrating how hard to block a similarly-scoped PR here.

### Issue #6 / PR #5 — issue-first workflow, and an honest "would this even be measurable" check

Issue #6 is a short "Ask" (add per-listener ACL-user auth so send-time ACL filtering, redis/redis#15122,
is actually measurable) that PR #5 implements the same day, with the issue closed via "Implements #6."
It also names a real downstream consumer: `redis/redis-benchmarks-specification#412` — this repo isn't
purely standalone; other repos in the org consume it. PR #5's description itself reasons carefully about
why the *previous* behavior (every listener using the same default-user connection) would have made the
new Redis feature it's meant to benchmark produce a ~0% measurable delta — a real, good example of a PR
author checking "does this change actually let us measure what we say it measures," worth citing as a
real precedent already in this repo, not as an invented maintainer requirement.

## What this history is honestly thin or silent on

- No human review comments or line comments exist anywhere in this repo's PR history to mine a
  "reviewer voice" from — the closest thing is the AI-review-derived issue #4, attributed accurately
  above.
- No closed-without-merge or rejected PRs exist — every PR opened here to date has merged.
- No second contributor has opened a PR — all traffic is from fcostaoliveira.
- No CI failure or CodeQL/Copilot-bot-caught-bug precedent was found (this repo has no CodeQL/Copilot
  workflow configured, unlike some sibling repos in the org).
- Zero `#[test]` functions exist in `src/main.rs` despite CONTRIBUTING.md's written rule that "all new
  behaviour must be covered by tests" and "coverage should not decrease" — the rule exists on paper and
  is not evidenced in practice across any of the 4 merged PRs.

Do not paper over these gaps with invented precedent. When a review would benefit from a citation this
history doesn't have, say so plainly and reason from the diff and from `AGENTS.md`/`CONTRIBUTING.md`'s
written rules instead.
