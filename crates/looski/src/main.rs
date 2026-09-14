// SPDX-License-Identifier: AGPL-3.0-only
use clap::{Parser, Subcommand};
use looski::{
    api::{self, AppState},
    client::ApiClient,
    mcp::McpServer,
};
use rmcp::ServiceExt;
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};

#[derive(Parser)]
#[command(version, about = "Looski Suite development scaffold")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the HTTP API. Requires the migrated development database.
    Api {
        #[arg(long, env = "LOOSKI_API_BIND", default_value = "127.0.0.1:58081")]
        bind: SocketAddr,
    },
    /// Emit the generated OpenAPI contract without connecting to a database.
    Openapi,
    /// Read public installation status as JSON.
    Status {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
    },
    /// Run the local stdio MCP adapter. Diagnostics go to stderr.
    Mcp {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    match Cli::parse().command {
        Command::Openapi => println!("{}", api::openapi().to_pretty_json()?),
        Command::Status { api_url } => {
            let info = ApiClient::new(&api_url)?.system().await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        Command::Mcp { api_url } => {
            McpServer::new(ApiClient::new(&api_url)?)
                .serve(rmcp::transport::stdio())
                .await?
                .waiting()
                .await?;
        }
        Command::Api { bind } => {
            let database_url = std::fs::read_to_string(std::env::var("LOOSKI_DATABASE_URL_FILE")?)?;
            let state = AppState {
                db: PgPoolOptions::new()
                    .max_connections(5)
                    .acquire_timeout(Duration::from_secs(2))
                    .connect_lazy(database_url.trim())?,
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(2))
                    .build()?,
                auth_url: std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
            };
            let listener = tokio::net::TcpListener::bind(bind).await?;
            tracing::info!(%bind, "API listening");
            axum::serve(listener, api::router(state))
                .with_graceful_shutdown(shutdown())
                .await?;
        }
    }
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
