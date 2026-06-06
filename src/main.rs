use anyhow::{anyhow, Context, Result};
use fred::{interfaces::TrackingInterface, prelude::*, types::RespVersion};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    env,
    io::{self, Write},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncBufReadExt, BufReader};

// The stdin/stdout mode accepts one line-delimited JSON command per line.
// Examples:
// {"command":"setup","redis_url":"redis://127.0.0.1:6381/0","listener_count":100,"tracking_prefix":"bench:"}
// {"command":"start_round","round_id":"r1","round_prefix":"bench:r1:","expected_keys":100}
// {"command":"shutdown"}
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Command {
    Setup {
        redis_url: String,
        listener_count: usize,
        tracking_prefix: String,
    },
    StartRound {
        round_id: String,
        round_prefix: String,
        expected_keys: usize,
    },
    Shutdown,
}

#[derive(Default)]
struct RuntimeState {
    clients: Vec<Client>,
    startup_duration_ms: Option<f64>,
    rounds_started: u64,
    invalidations_drained: Arc<AtomicU64>,
}

#[derive(Serialize)]
struct Response<'a> {
    status: &'a str,
    #[serde(flatten)]
    payload: serde_json::Value,
}

struct ManualModeConfig {
    redis_url: String,
    listener_count: usize,
    tracking_prefix: String,
}

impl RuntimeState {
    async fn handle_command(&mut self, command: Command) -> Result<Response<'static>> {
        match command {
            Command::Setup {
                redis_url,
                listener_count,
                tracking_prefix,
            } => self.setup(redis_url, listener_count, tracking_prefix).await,
            Command::StartRound {
                round_id,
                round_prefix,
                expected_keys,
            } => self.start_round(round_id, round_prefix, expected_keys).await,
            Command::Shutdown => self.shutdown().await,
        }
    }

    async fn setup(
        &mut self,
        redis_url: String,
        listener_count: usize,
        _tracking_prefix: String,
    ) -> Result<Response<'static>> {
        if listener_count == 0 {
            return Err(anyhow!("listener_count must be greater than 0"));
        }
        if !self.clients.is_empty() {
            return Err(anyhow!("listener helper was already configured"));
        }

        let startup_start = current_unix_nanos();
        let mut clients = Vec::with_capacity(listener_count);

        for _listener_index in 0..listener_count {
            let mut config = Config::from_url(&redis_url)
                .with_context(|| format!("failed to parse redis url: {redis_url}"))?;
            config.version = RespVersion::RESP3;

            let client = Builder::from_config(config)
                .with_performance_config(|config| {
                    config.broadcast_channel_capacity = 1024;
                })
                .build()
                .context("failed to build fred client")?;
            client
                .init()
                .await
                .context("failed to initialize fred client")?;

            let invalidations_drained = Arc::clone(&self.invalidations_drained);
            client.on_invalidation(move |_invalidation| {
                // Counts invalidation *messages*, not keys: a single BCAST push can
                // carry multiple keys (_invalidation.keys). Counter is cumulative for
                // the process lifetime. See follow-up issue for per-key / per-round
                // semantics.
                invalidations_drained.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });

            let prefixes = vec![_tracking_prefix.clone()];
            println!("Subscribing for the following prefixes: {prefixes:?}");
            client
                .start_tracking(prefixes, true, false, false, false)
                .await
                .context("failed to enable CLIENT TRACKING ON BCAST")?;
            clients.push(client);
        }

        let startup_end = current_unix_nanos();
        self.startup_duration_ms = Some((startup_end - startup_start) as f64 / 1_000_000.0);
        self.clients = clients;

        Ok(Response {
            status: "ok",
            payload: json!({
                "listener_count": listener_count,
                "startup_duration_ms": self.startup_duration_ms.unwrap_or(0.0),
            }),
        })
    }

    async fn start_round(
        &mut self,
        round_id: String,
        _round_prefix: String,
        _expected_keys: usize,
    ) -> Result<Response<'static>> {
        if self.clients.is_empty() {
            return Err(anyhow!("listener helper is not configured"));
        }

        self.rounds_started += 1;
        Ok(Response {
            status: "ok",
            payload: json!({
                "round_id": round_id,
                "rounds_started": self.rounds_started,
                "invalidations_drained": self.invalidations_drained.load(Ordering::Relaxed),
            }),
        })
    }

    async fn shutdown(&mut self) -> Result<Response<'static>> {
        for client in self.clients.drain(..) {
            let _ = client.quit().await;
        }

        Ok(Response {
            status: "ok",
            payload: json!({
                "message": "shutdown complete",
                "rounds_started": self.rounds_started,
                "invalidations_drained": self.invalidations_drained.load(Ordering::Relaxed),
            }),
        })
    }
}

fn current_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock went backwards")
        .as_nanos()
}

fn emit_response(response: &Response<'_>) -> Result<()> {
    let encoded = serde_json::to_string(response)?;
    println!("{encoded}");
    io::stdout().flush().context("failed to flush stdout")?;
    Ok(())
}

fn emit_error(error: &anyhow::Error) -> Result<()> {
    emit_response(&Response {
        status: "error",
        payload: json!({
            "message": format!("{error:#}"),
        }),
    })
}

fn print_usage() {
    let program = env::args()
        .next()
        .unwrap_or_else(|| "bcast-listener".to_string());
    eprintln!(
        "\
{program} - Redis client-side caching (CLIENT TRACKING BCAST) listener helper

USAGE:
    {program} [manual] [OPTIONS]
    {program} --help

MODES:
    (default)   Stdin/stdout mode. Reads one line-delimited JSON command per
                line from stdin and writes a JSON response per line to stdout.

                Commands:
                  {{\"command\":\"setup\",\"redis_url\":\"redis://127.0.0.1:6379/0\",\"listener_count\":100,\"tracking_prefix\":\"bench:\"}}
                  {{\"command\":\"start_round\",\"round_id\":\"r1\",\"round_prefix\":\"bench:r1:\",\"expected_keys\":100}}
                  {{\"command\":\"shutdown\"}}

    manual      Spins up the listeners directly from CLI flags, then waits for
                Ctrl-C before shutting down. Useful for ad-hoc testing.

MANUAL MODE OPTIONS:
    --redis-url <url>            Redis connection URL (required)
    --listener-count <n>         Number of tracking clients to create (required)
    --tracking-prefix <prefix>   Key prefix to track [default: bench:]
    -h, --help                   Print manual mode usage

GLOBAL OPTIONS:
    -h, --help                   Print this help and exit

NOTE:
    The helper currently tracks all keys in BCAST mode, so --tracking-prefix is
    accepted for CLI compatibility but does not filter invalidations.

EXAMPLE:
    {program} manual --redis-url redis://127.0.0.1:6379/0 --listener-count 100"
    );
}

fn print_manual_usage() {
    // Example:
    // bcast-listener manual --redis-url redis://127.0.0.1:6381/0 --listener-count 100 --tracking-prefix bench:
    // Note: the helper currently tracks all keys in BCAST mode, so --tracking-prefix
    // is accepted for CLI compatibility but is not used to filter invalidations.
    eprintln!(
        "Usage: bcast-listener manual --redis-url <url> --listener-count <n> [--tracking-prefix <prefix>]"
    );
}

fn parse_manual_mode_config(args: &[String]) -> Result<ManualModeConfig> {
    let mut redis_url = None;
    let mut listener_count = None;
    let mut tracking_prefix = String::from("bench:");
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" => {
                print_manual_usage();
                std::process::exit(0);
            }
            "--redis-url" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| anyhow!("missing value for --redis-url"))?;
                redis_url = Some(value.clone());
            }
            "--listener-count" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| anyhow!("missing value for --listener-count"))?;
                listener_count = Some(
                    value
                        .parse::<usize>()
                        .with_context(|| format!("invalid listener count: {value}"))?,
                );
            }
            "--tracking-prefix" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| anyhow!("missing value for --tracking-prefix"))?;
                tracking_prefix = value.clone();
            }
            unknown => {
                print_manual_usage();
                return Err(anyhow!("unknown argument: {unknown}"));
            }
        }
        index += 1;
    }

    Ok(ManualModeConfig {
        redis_url: redis_url.ok_or_else(|| anyhow!("--redis-url is required"))?,
        listener_count: listener_count
            .ok_or_else(|| anyhow!("--listener-count is required"))?,
        tracking_prefix,
    })
}

async fn run_manual_mode(args: &[String]) -> Result<()> {
    let config = parse_manual_mode_config(args)?;
    let mut runtime = RuntimeState::default();
    let setup_response = runtime
        .setup(
            config.redis_url,
            config.listener_count,
            config.tracking_prefix.clone(),
        )
        .await?;
    emit_response(&Response {
        status: "ok",
        payload: json!({
            "mode": "manual",
            "message": "listeners are ready; press Ctrl-C to stop",
            "tracking_prefix": config.tracking_prefix,
            "listener_count": setup_response.payload["listener_count"],
            "startup_duration_ms": setup_response.payload["startup_duration_ms"],
        }),
    })?;

    tokio::signal::ctrl_c()
        .await
        .context("failed while waiting for Ctrl-C")?;

    let shutdown_response = runtime.shutdown().await?;
    emit_response(&Response {
        status: "ok",
        payload: json!({
            "mode": "manual",
            "message": "shutdown complete",
            "rounds_started": shutdown_response.payload["rounds_started"],
            "invalidations_drained": shutdown_response.payload["invalidations_drained"],
        }),
    })?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        print_usage();
        return Ok(());
    }
    if matches!(args.first().map(String::as_str), Some("manual")) {
        return run_manual_mode(&args[1..]).await;
    }

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut runtime = RuntimeState::default();

    while let Some(line) = lines.next_line().await? {
        println!("line: {}", line);
        if line.trim().is_empty() {
            continue;
        }

        let command: Command = match serde_json::from_str(&line) {
            Ok(command) => command,
            Err(error) => {
                emit_error(&anyhow!(error).context("failed to decode command json"))?;
                continue;
            }
        };
        let should_shutdown = matches!(command, Command::Shutdown);

        match runtime.handle_command(command).await {
            Ok(response) => emit_response(&response)?,
            Err(error) => emit_error(&error)?,
        }

        if should_shutdown {
            break;
        }
    }

    Ok(())
}
