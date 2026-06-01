# bcast-listener

A Redis client-side caching (`CLIENT TRACKING ... BCAST`) listener helper. It
spins up a configurable number of [fred](https://crates.io/crates/fred) clients,
enables RESP3 broadcast invalidation tracking on each, and drains the resulting
invalidation messages. It is intended for benchmarking and ad-hoc testing of
Redis broadcast tracking under many concurrent listeners.

## Requirements

- A recent stable [Rust toolchain](https://rustup.rs/) (edition 2024, so Rust
  1.85+).
- A reachable Redis server with RESP3 / client tracking support (Redis 6+).

## Build

```bash
# Debug build
cargo build

# Optimized release build (recommended for benchmarking)
cargo build --release
```

The resulting binary is written to `target/debug/bcast-listener` or
`target/release/bcast-listener`.

## Run

The tool has two modes.

### Manual mode

Spins up the listeners directly from CLI flags, then waits for `Ctrl-C` before
shutting down. Useful for quick, interactive testing.

```bash
cargo run --release -- manual \
  --redis-url redis://127.0.0.1:6379/0 \
  --listener-count 100 \
  --tracking-prefix bench:
```

Options:

| Flag | Description |
| --- | --- |
| `--redis-url <url>` | Redis connection URL (**required**) |
| `--listener-count <n>` | Number of tracking clients to create (**required**) |
| `--tracking-prefix <prefix>` | Key prefix to track (default: `bench:`) |
| `-h`, `--help` | Print usage |

> **Note:** The helper currently tracks all keys in BCAST mode, so
> `--tracking-prefix` is accepted for CLI compatibility but does not filter
> invalidations.

### Stdin/stdout mode (default)

When no `manual` subcommand is given, the tool reads one line-delimited JSON
command per line from stdin and writes one JSON response per line to stdout.
This is intended to be driven by another process (e.g. a benchmark harness).

```bash
cargo run --release
```

Then send commands on stdin, one JSON object per line:

```json
{"command":"setup","redis_url":"redis://127.0.0.1:6379/0","listener_count":100,"tracking_prefix":"bench:"}
{"command":"start_round","round_id":"r1","round_prefix":"bench:r1:","expected_keys":100}
{"command":"shutdown"}
```

Each command produces a JSON response, for example:

```json
{"status":"ok","listener_count":100,"startup_duration_ms":42.5}
```

## Help

```bash
cargo run -- --help
```
