---
name: bcast-tracking-bench-maintainer-review
description: Review a redis-performance/bcast-tracking-bench pull request, branch, or diff, grounded in this repo's actual (thin) GitHub history and its written AGENTS.md/CONTRIBUTING.md conventions — not generic Rust code-review advice. Use this whenever the user asks to review a bcast-tracking-bench PR "like a maintainer would", wants a repo-specific pre-merge check, or is deciding accept/reject on a redis-performance/bcast-tracking-bench PR. Prefer this over a generic code-review skill for anything touching this repo — the generic skill doesn't know this project's real (small) history or the one substantive technical precedent it does have.
---

# bcast-tracking-bench maintainer-style review

## Honesty warning: read this before writing anything

This repo's review history is **very thin, and you should say so rather than perform a richer one.**
As of the mining behind this skill, `redis-performance/bcast-tracking-bench` had exactly **4 merged
PRs**, all authored *and* self-merged by the same person (**fcostaoliveira**, Filipe Oliveira), all
generated with Claude Code (each PR body ends with "🤖 Generated with Claude Code"). The GitHub
Reviews API returns **zero reviews and zero inline review comments** on every one of those PRs — there
is no second human voice in this repo's PR history to imitate, and no per-person "voice profile" to
mine (unlike a larger repo where a real reviewer's phrasing can be cited).

The one genuinely substantive technical artifact in this repo's history is **issue #4**
("invalidations_drained metric: semantics & robustness follow-ups"), which is *not* a human maintainer
comment — it's fcostaoliveira's writeup of findings from an **AI multi-model review pass** ("Raised by
reviewers; opus/sonnet consensus = non-blocking for #3") run against PR #3, captured as a non-blocking
follow-up issue rather than blocking that PR's merge. Treat it as a real, evidenced example of the kind
of thing worth catching in this codebase — not as a maintainer's personal review style, since no human
wrote it as PR commentary. `references/history-notes.md` has the full mined history; read it before
reviewing anything. `references/technical-checklist.md` has the concrete, evidenced checklist derived
from it, plus this repo's own written (CONTRIBUTING.md/AGENTS.md) rules and what its CI actually
enforces vs. only asks for.

**Do not invent a maintainer personality, a "real reviewers say..." quote, or a richer institutional
voice than four self-merged, zero-comment PRs and one AI-review-derived issue support.** If you don't
have a real, on-point precedent for something, say so plainly and reason about the issue on its
technical merits instead of fabricating a citation.

## Process

1. **Get the material.** `gh pr view <n> --repo redis-performance/bcast-tracking-bench
   --json body,commits,files,author` and `gh pr diff <n> --repo redis-performance/bcast-tracking-bench`.
   Read the PR description in full first — every real merged PR here uses a consistent structured
   template (Problem/Change, Fix, Verification, sometimes "Note for reviewer"); if the author already
   named a tradeoff or a verification step there, acknowledge it rather than re-deriving it as new.

2. **Scope gate.** This is a small, single-file Rust binary (`src/main.rs`, ~500 lines, async on
   tokio, using the `fred` Redis client) with two entry points: a manual CLI mode and a
   line-delimited-JSON stdin/stdout mode driven by an external benchmark harness. If the PR's content
   falls entirely outside that surface — e.g., a pure docs/CI/Dockerfile change with no Rust source
   touched — say so in one sentence and apply only the parts of the checklist that are actually
   relevant (a Dockerfile change still gets the Docker-registry-freeze check; a docs-only change gets
   essentially nothing).

3. **Work the checklist** in `references/technical-checklist.md`. It is short and heavily weighted
   toward the one real evidenced incident (issue #4, on the invalidation-counting logic) plus this
   repo's own written rules. Give real weight to:
   - Counter/metric semantics done end-to-end for every code path they claim to cover (issue #4 item
     1: messages vs. keys), not just the common path.
   - Any parameter or config value that's accepted but never actually wired into behavior (issue #4
     item 2: `expected_keys` bound to `_expected_keys` and never used).
   - State that needs resetting between logical rounds/iterations vs. silently accumulating forever
     (issue #4 item 3).
   - Silent failure modes in async/broadcast consumption under load — a `while let Ok(...)` loop over
     a channel that just stops on the first error, dropping the rest of a run without surfacing
     anything (issue #4 item 4, on fred's `broadcast::error::RecvError::Lagged`).
   - Whether a new benchmark-affecting flag is actually *measurable* — PR #5's own description is a
     real, good example of this reasoning (it explains why the feature would have produced ~0% delta
     without per-listener ACL segregation) — cite it as a good example already in this repo, not as a
     maintainer mandate.
   - This repo's written-but-CI-unenforced rule that "all new behaviour must be covered by tests"
     (CONTRIBUTING.md) — be accurate that, as of this mining, `src/main.rs` has **zero** `#[test]`
     functions despite 4 merged PRs, so naming a coverage gap is honest and worth doing, but claiming
     CI will block the PR over it is not (CI runs `cargo test --verbose`, but there's nothing to run).
   - What CI *does* actually enforce as a hard gate: `cargo fmt --all -- --check` and
     `cargo clippy --all-targets -- -D warnings` (`.github/workflows/rust.yml`) — don't re-flag a
     formatting/lint nit that tooling already blocks on; do flag something clippy's default lint set
     wouldn't catch (the issue #4 items above are all in that category).
   - The Docker-registry-freeze and new-dependency rules from `AGENTS.md` — flag a PR that changes
     the Docker Hub `redis/` namespace, or adds a new crate dependency, without the maintainer having
     signed off, per the repo's own written rules.

4. **Write the review.** Since there's no real per-person voice to imitate, default to a plain,
   terse register consistent with what this repo's own PR descriptions look like (structured,
   specific, no filler) rather than inventing a personality:
   - Lead with anything genuinely substantive; stay silent on things that check out rather than
     padding with generic praise.
   - Hedge honestly when uncertain ("worth double-checking", "not sure this is exercised anywhere") —
     don't manufacture false confidence the thin record doesn't support.
   - Do not manufacture whitespace/formatting nits — `cargo fmt`/`clippy -D warnings` already gate
     those in CI.
   - Never literally `@`-mention any GitHub username.
   - Do not write a formal-headers essay ("Correctness", "Security", "Performance") — nothing in this
     repo's real history looks like that; short, specific, numbered points (as issue #4 itself is
     formatted) fit the evidence better.

5. **Land on a verdict.** This repo's only real resolution pattern on record is "self-merge, no
   blocking human review" — so don't claim a PR would get a specific named human's sign-off. State
   plainly whether you'd flag anything as blocking (evidenced standard: something as concrete as issue
   #4's items) or whether it's clean, without inventing a maintainer quote to hang the verdict on.
   Never write the literal word "Verdict", and never close with a bolded label line or a "TL;DR" — let
   the last sentence of the prose carry it.

## What NOT to do

- Don't invent a "maintainer voice" or fabricate a quote — this repo's PR history has zero human
  review comments to draw one from.
- Don't cite issue #4 as if it were a human maintainer's personal review standard — it's an AI
  multi-model review's findings, real and evidenced, but attribute it accurately.
- Don't apply a Python-project checklist (this is Rust/tokio/async) or assume patterns from other
  redis-performance repos apply here without checking this repo's own AGENTS.md/CONTRIBUTING.md first.
- Don't claim CI blocks on test coverage — it doesn't (there are no tests to run yet); do say so
  honestly rather than pretending otherwise or pretending the written rule doesn't exist.
- Don't close with a labeled, bolded verdict block — end in plain prose.
