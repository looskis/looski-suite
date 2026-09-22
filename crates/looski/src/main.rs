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
    /// Workspace Contacts operations. Mutations read their JSON body from stdin.
    Contacts {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: ContactCommand,
    },
    /// Workspace organization directory. Mutations read JSON from stdin.
    Organizations {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: OrganizationCommand,
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

#[derive(Subcommand)]
enum ContactCommand {
    Organizations {
        id: uuid::Uuid,
    },
    SetOrganization {
        id: uuid::Uuid,
        organization: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Merge {
        survivor: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    Get {
        id: uuid::Uuid,
    },
    Create {
        #[arg(long)]
        idempotency_key: String,
    },
    Update {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Archive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        restore: bool,
    },
}

#[derive(Subcommand)]
enum OrganizationCommand {
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    Get {
        id: uuid::Uuid,
    },
    Create {
        #[arg(long)]
        idempotency_key: String,
    },
    Update {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Archive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        restore: bool,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let code = if let Some(failure) = error.downcast_ref::<looski::client::ApiFailure>() {
            eprintln!("{failure}");
            match failure.status {
                400 | 404 | 422 => 3,
                401 | 403 => 4,
                409 => 5,
                500..=599 => 6,
                _ => 1,
            }
        } else {
            eprintln!("{error}");
            1
        };
        std::process::exit(code);
    }
}
async fn run() -> anyhow::Result<()> {
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
        Command::Contacts {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            let result = match action {
                ContactCommand::Organizations { id } => {
                    serde_json::to_value(client.contact_organizations(workspace, id).await?)?
                }
                ContactCommand::SetOrganization {
                    id,
                    organization,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .set_contact_organization(
                            workspace,
                            id,
                            organization,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                ContactCommand::Merge {
                    survivor,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .merge_contacts(
                            workspace,
                            survivor,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                ContactCommand::List {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .contacts(
                            workspace,
                            &looski::contacts::ListContacts {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                ContactCommand::Get { id } => {
                    serde_json::to_value(client.contact(workspace, id).await?)?
                }
                ContactCommand::Create { idempotency_key } => serde_json::to_value(
                    client
                        .create_contact(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                ContactCommand::Update {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .edit_contact(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                ContactCommand::Archive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .archive_contact(
                            workspace,
                            id,
                            &idempotency_key,
                            &looski::contacts::ArchiveContact {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Organizations {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            let result = match action {
                OrganizationCommand::List {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .organizations(
                            workspace,
                            &looski::contacts::ListContacts {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                OrganizationCommand::Get { id } => {
                    serde_json::to_value(client.organization(workspace, id).await?)?
                }
                OrganizationCommand::Create { idempotency_key } => serde_json::to_value(
                    client
                        .create_organization(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                OrganizationCommand::Update {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .edit_organization(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                OrganizationCommand::Archive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .archive_organization(
                            workspace,
                            id,
                            &idempotency_key,
                            &looski::contacts::ArchiveContact {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
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
