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
    /// Provision the first owner locally; reads credentials as JSON from stdin.
    BootstrapOwner,
    /// List permitted workspaces using an access-token file.
    Workspaces {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
    },
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
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
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
        Command::Workspaces {
            api_url,
            token_file,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            println!(
                "{}",
                serde_json::to_string_pretty(&client.workspaces().await?)?
            );
        }
        Command::Mcp {
            api_url,
            token_file,
        } => {
            let mut client = ApiClient::new(&api_url)?;
            if let Some(file) = token_file {
                client = client.with_token_file(file);
            }
            McpServer::new(client)
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
                verifier: looski::identity::Verifier::new(
                    read_jwt_secret()?.as_bytes(),
                    std::env::var("LOOSKI_AUTH_ISSUER")?,
                ),
            };
            let listener = tokio::net::TcpListener::bind(bind).await?;
            tracing::info!(%bind, "API listening");
            axum::serve(listener, api::router(state))
                .with_graceful_shutdown(shutdown())
                .await?;
        }
        Command::BootstrapOwner => {
            let input = serde_json::from_reader(std::io::stdin())?;
            let database_url = std::fs::read_to_string(std::env::var("LOOSKI_DATABASE_URL_FILE")?)?;
            let db = PgPoolOptions::new()
                .max_connections(1)
                .connect(database_url.trim())
                .await?;
            let http = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?;
            looski::bootstrap::provision(
                &db,
                &http,
                &std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
                &std::env::var("LOOSKI_AUTH_ISSUER")?,
                read_jwt_secret()?.as_bytes(),
                input,
            )
            .await?;
            println!("Owner and workspace created. You can now sign in.");
        }
    }
    Ok(())
}

fn read_jwt_secret() -> anyhow::Result<String> {
    let secret = std::fs::read_to_string(std::env::var("LOOSKI_JWT_SECRET_FILE")?)?;
    anyhow::ensure!(
        secret.trim().len() >= 32,
        "JWT secret must have at least 32 bytes"
    );
    Ok(secret.trim().to_owned())
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
