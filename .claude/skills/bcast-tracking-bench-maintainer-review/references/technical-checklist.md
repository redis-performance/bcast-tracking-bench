# Technical checklist: redis-performance/bcast-tracking-bench

Every item below is either (a) directly evidenced by this repo's own mined history
(`history-notes.md`), or (b) a written rule from this repo's own `AGENTS.md`/`CONTRIBUTING.md`. Nothing
here is generic Rust-best-practices padding. If a PR doesn't touch the surface an item covers, skip it
rather than force-fitting it.

## 1. Invalidation-counting / metric semantics (evidenced: issue #4, on PR #3)

The single richest evidenced precedent in this repo. If a PR touches `invalidations_drained`, any other
counter, or how `start_round`/`shutdown` report stats, check for these four concrete failure shapes
(all real, all found in this exact codebase):

- **Unit ambiguity**: does the counter count messages or keys? A RESP3 BCAST push can carry multiple
  keys per message (`Invalidation.keys: Vec<Key>`) — `fetch_add(1)` per message and `fetch_add(keys.len())`
  per key are both defensible, but the field name and any consumer expecting `expected_keys` parity
  need to agree on which one it is.
- **Inert parameters**: a value accepted into a command/struct but never actually read anywhere (like
  `start_round`'s `expected_keys` bound to `_expected_keys`) — grep for the parameter's real name, not
  just its presence in the signature, to confirm it's wired into actual control flow.
- **Missing reset between logical units of work**: an `Arc<AtomicU64>` (or similar) created once outside
  a loop of rounds/iterations, only ever incremented, never reset or delta'd — check whether the
  intended semantics are cumulative-for-the-whole-run or per-round, and whether the code matches which
  one it claims.
- **Read-vs-drain races**: a counter read immediately after triggering work, with no wait/gate on the
  async consumers actually having drained everything yet.

## 2. Silent failure in async/broadcast consumption (evidenced: issue #4 item 4)

This tool's whole job is draining broadcast invalidation messages under many concurrent listeners —
i.e., exactly the load profile that trips bounded-channel edge cases. If a PR touches the
`on_invalidation`/listener-loop code:

- Check what happens when a channel read errors out, not just the happy path. fred's tokio broadcast
  consumption can return `RecvError::Lagged` when a listener falls behind the channel's capacity; a bare
  `while let Ok(...)` loop treats that as end-of-stream and exits the loop permanently, silently
  dropping that listener from the rest of the run with no error surfaced anywhere.
  Ask: does an error branch continue, log-and-continue, or silently stop? A benchmark tool going quiet
  under exactly the high-throughput conditions it exists to stress is the worst failure mode for this
  codebase specifically.

## 3. Is the benchmark change actually measurable? (evidenced: PR #5 / issue #6)

For a PR that adds or changes something meant to make a Redis behavior benchmarkable (a new flag, a new
mode, a new metric): does the PR's own description (or your read of the diff) establish that the change
would actually produce a measurable delta, not just add surface area? PR #5 is a real, good example of
this reasoning already present in this repo — it explains why the *prior* single-shared-connection setup
would have made the feature it's meant to benchmark produce ~0% delta. Cite it as "here's a real example
already in this repo," not as an invented maintainer mandate.

## 4. What CI actually enforces vs. what's only written down

`.github/workflows/rust.yml` runs on every push/PR and is a real hard gate on:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --all-targets` and `cargo test --verbose`

Don't re-flag a formatting or default-clippy-lint nit — CI already blocks on those. Do flag anything
clippy's default lint set doesn't reach (the section-1/2 items above are exactly that category —
they're logic/semantics bugs, not lint-shaped).

`CONTRIBUTING.md` also states, in writing, "All new behaviour must be covered by tests" and "Coverage
should not decrease." As of this mining, `src/main.rs` (~500 lines) has **zero** `#[test]` functions
across all 4 merged PRs — `cargo test --verbose` runs in CI but there is nothing for it to run. Naming a
missing-test gap on a non-trivial PR is honest and worth doing; claiming CI will actually block merge
over it is not — nothing in this repo's real history shows that happening.

## 5. Written repo rules worth checking a diff against (AGENTS.md / CONTRIBUTING.md)

- **Docker registry freeze**: images publish to Docker Hub under `redis/bcast-tracking-bench`
  (`.github/workflows/docker.yml`, PR #2). `AGENTS.md` explicitly says not to change the registry
  (e.g., Docker Hub → GHCR) without maintainer approval — flag any PR that touches this without
  clear sign-off.
- **New dependencies**: `AGENTS.md` says not to introduce a new crate dependency without checking with
  the maintainer first. A `Cargo.toml`/`Cargo.lock` diff adding a new external crate (as opposed to
  toggling an existing dependency's feature flags, e.g. PR #5's `i-acl` fred feature) is worth naming.
- **Branch naming / no direct pushes to main**: `<type>/<short-description>`, PR against `main`, never
  a direct push — not something you can verify from a diff alone, but worth a one-line sanity check if
  visible in the PR metadata.
- **Comments should explain *why*, not *what***: `AGENTS.md`'s explicit written rule. This repo's
  binary is a purpose-built benchmark helper with some genuinely non-obvious concurrency/protocol
  behavior (BCAST semantics, ACL-based send-time filtering) — a comment explaining an unusual choice is
  valuable here; a comment restating the next line of code is exactly what the rule says to avoid.
- **No dead code / no commented-out blocks**: `CONTRIBUTING.md`'s explicit written rule, and literally
  the shape of the bug PR #3 fixed (a commented-out increment). Flag any newly-introduced
  commented-out code the same way.

## What this checklist is honestly thin or silent on

- Nothing here is drawn from a real human maintainer review comment — this repo's PR history has zero
  of those. Section 1 and 2 are drawn from an AI multi-model review's output (issue #4), attributed
  accurately in `history-notes.md`.
- No evidenced precedent exists yet for how this repo handles a genuinely contested design disagreement,
  a rejected PR, or review from a second human contributor — because none of those have happened here
  yet. Don't invent one; reason from first principles and the written docs instead.
