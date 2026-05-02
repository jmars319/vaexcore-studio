use std::{env, io::Read, net::SocketAddr, path::PathBuf};

use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use vaexcore_core::{ApiResponse, EngineMode, EngineStatus, APP_NAME};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_true")]
    pub dry_run: bool,
    pub status_addr: Option<SocketAddr>,
    #[serde(default)]
    pub pipeline_name: Option<String>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            dry_run: true,
            status_addr: None,
            pipeline_name: Some("dry-run".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunnerStatus {
    pub service: String,
    pub ready: bool,
    pub dry_run: bool,
    pub pipeline_name: Option<String>,
    pub engine_status: EngineStatus,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "media_runner=info,vaexcore=info".into()),
        )
        .init();

    let cli = CliArgs::parse(env::args().skip(1))?;
    let mut config = read_config(cli.config_path)?;

    if cli.dry_run {
        config.dry_run = true;
    }

    if let Some(status_addr) = cli.status_addr {
        config.status_addr = Some(status_addr);
    }

    let status = status_from_config(&config);

    if let Some(addr) = config.status_addr {
        let app = Router::new()
            .route(
                "/health",
                get(|| async {
                    Json(ApiResponse::ok(serde_json::json!({
                        "service": "vaexcore-media-runner",
                        "ok": true
                    })))
                }),
            )
            .route(
                "/status",
                get({
                    let status = status.clone();
                    move || async move { Json(ApiResponse::ok(status.clone())) }
                }),
            );

        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, dry_run = config.dry_run, "media-runner status endpoint listening");
        axum::serve(listener, app).await?;
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&ApiResponse::ok(status))?
        );
    }

    Ok(())
}

#[derive(Debug, Default)]
struct CliArgs {
    config_path: Option<PathBuf>,
    status_addr: Option<SocketAddr>,
    dry_run: bool,
}

impl CliArgs {
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut parsed = Self::default();
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--config requires a path"))?;
                    parsed.config_path = Some(PathBuf::from(value));
                }
                "--status-addr" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--status-addr requires host:port"))?;
                    parsed.status_addr = Some(value.parse()?);
                }
                "--dry-run" => parsed.dry_run = true,
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => anyhow::bail!("unknown argument '{other}'"),
            }
        }

        Ok(parsed)
    }
}

fn read_config(path: Option<PathBuf>) -> anyhow::Result<RunnerConfig> {
    let content = match path {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut content = String::new();
            std::io::stdin().read_to_string(&mut content)?;
            content
        }
    };

    if content.trim().is_empty() {
        return Ok(RunnerConfig::default());
    }

    Ok(serde_json::from_str(&content)?)
}

fn status_from_config(config: &RunnerConfig) -> RunnerStatus {
    RunnerStatus {
        service: "vaexcore-media-runner".to_string(),
        ready: true,
        dry_run: config.dry_run,
        pipeline_name: config.pipeline_name.clone(),
        engine_status: EngineStatus::idle(
            "media-runner",
            if config.dry_run {
                EngineMode::DryRun
            } else {
                EngineMode::ExternalSidecar
            },
        ),
    }
}

fn default_true() -> bool {
    true
}

fn print_help() {
    println!(
        "{APP_NAME} media-runner\n\nUSAGE:\n  media-runner [--config path] [--status-addr 127.0.0.1:51387] [--dry-run]\n\nJSON config can be passed through --config or stdin."
    );
}
