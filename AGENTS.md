# Agent guidelines

Instructions for AI coding agents (Claude Code, Copilot, Cursor, etc.) working in this repo.

## Project overview

A Redis client-side caching (`CLIENT TRACKING ... BCAST`) listener helper. It
spins up a configurable number of [fred](https://crates.io/crates/fred) clients,
enables RESP3 broadcast invalidation tracking on each, and drains the resulting
invalidation messages. It supports a manual CLI mode and a line-delimited JSON
stdin/stdout mode for driving from a benchmark harness. Intended for
benchmarking and ad-hoc testing of Redis broadcast tracking under many
concurrent listeners.

## Local setup

Requires a recent stable Rust toolchain (edition 2024, Rust 1.85+) and a
reachable Redis server with RESP3 / client tracking support (Redis 6+).

```bash
git clone git@github.com:redis-performance/bcast_listeners.git
cd bcast_listeners
cargo build --release
```

## Branch naming

Same as human contributors: `<type>/<short-description>` (e.g. `fix/off-by-one-in-pipeline`).

## Coding standards

- Match the style already in the file you are editing.
- Prefer clear, minimal changes over large refactors unless explicitly asked.
- Do not add comments that describe *what* the code does — only add comments when the *why* is non-obvious.
- Do not introduce new dependencies without checking with the maintainer.

## Running tests

```bash
cargo test
```

Always run tests before declaring a task complete.

## How to submit changes

1. Create a branch: `git checkout -b <type>/<description>`.
2. Commit with a clear message focused on *why*, not *what*.
3. Open a pull request against `main`.
4. Do **not** push directly to `main`.

## Docker images

Several repos in this org publish Docker images to Docker Hub under the `redis/` namespace:

| Repo | Image |
|------|-------|
| go-ycsb | `redis/go-ycsb` |
| redis-benchmark-go | `redis/redis-benchmark-go` |
| pubsub-sub-bench | `redis/pubsub-sub-bench` |
| vector-db-benchmark | `redis/vector-db-benchmark` |
| sidekiq-benchmark | `redis/sidekiq-benchmark` |
| memtier_benchmark | `redislabs/memtier_benchmark` |

Rules when working with Docker:
- Always use `redis:8.6` or newer as the Redis base image in Dockerfiles and CI service containers.
- Do **not** change the Docker registry (e.g. Docker Hub → GHCR) without explicit maintainer approval.
- If a `docker-publish` workflow fails due to stale credentials, report it — do not change the registry.

## What to avoid

- Do not reformat files unrelated to your change.
- Do not remove error handling or tests.
- Do not commit secrets, credentials, or large binary files.
- Do not amend published commits.
