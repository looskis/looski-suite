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
    /// Shared task operations. Mutations read JSON from stdin.
    Tasks {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: TasksCommand,
    },
    /// CRM definitions, opportunities, deals and module selection. Mutations read JSON from stdin.
    Crm {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: CrmCommand,
    },
    /// Staff roles and memberships. Mutations read JSON from stdin.
    Access {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: AccessCommand,
    },
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
    /// Shared workspace tag definitions. Mutations read JSON from stdin.
    Tags {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: TagCommand,
    },
    /// Retained contact intake and manual review. Mutations read JSON from stdin.
    Intakes {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: std::path::PathBuf,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: IntakeCommand,
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
enum TasksCommand {
    List {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: Option<bool>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        assigned_membership_id: Option<uuid::Uuid>,
        #[arg(long)]
        due_before: Option<String>,
    },
    Get {
        id: uuid::Uuid,
    },
    Create {
        #[arg(long)]
        idempotency_key: String,
    },
    Edit {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Archive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    History {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
}

#[derive(Subcommand)]
enum CrmCommand {
    OpportunityTasks {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: Option<bool>,
    },
    LinkOpportunityTask {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    CreateOpportunityFollowup {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    EndOpportunityTask {
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    OpportunityTaskHistory {
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    DealTasks {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: Option<bool>,
    },
    LinkDealTask {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    CreateDealFollowup {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    EndDealTask {
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    DealTaskHistory {
        id: uuid::Uuid,
        link_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },

    OpportunityContacts {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: bool,
    },
    AddOpportunityContact {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    EditOpportunityContact {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    OpportunityContactHistory {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    OpportunityOrganizations {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: bool,
    },
    AddOpportunityOrganization {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    EditOpportunityOrganization {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    OpportunityOrganizationHistory {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    DealContacts {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: bool,
    },
    AddDealContact {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    EditDealContact {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    DealContactHistory {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    DealOrganizations {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: bool,
    },
    AddDealOrganization {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    EditDealOrganization {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    DealOrganizationHistory {
        id: uuid::Uuid,
        association_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },

    OpportunityTags {
        id: uuid::Uuid,
    },
    SetOpportunityTags {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    DealTags {
        id: uuid::Uuid,
    },
    SetDealTags {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },

    Pipelines {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        parent_id: Option<uuid::Uuid>,
        #[arg(long)]
        archived: bool,
    },
    Pipeline {
        id: uuid::Uuid,
    },
    CreatePipeline {
        #[arg(long)]
        idempotency_key: String,
    },
    EditPipeline {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Stages {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        parent_id: Option<uuid::Uuid>,
        #[arg(long)]
        archived: bool,
    },
    Stage {
        id: uuid::Uuid,
    },
    CreateStage {
        #[arg(long)]
        idempotency_key: String,
    },
    EditStage {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Categories {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        parent_id: Option<uuid::Uuid>,
        #[arg(long)]
        archived: bool,
    },
    Category {
        id: uuid::Uuid,
    },
    CreateCategory {
        #[arg(long)]
        idempotency_key: String,
    },
    EditCategory {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Statuses {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        parent_id: Option<uuid::Uuid>,
        #[arg(long)]
        archived: bool,
    },
    Status {
        id: uuid::Uuid,
    },
    CreateStatus {
        #[arg(long)]
        idempotency_key: String,
    },
    EditStatus {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Opportunities {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        pipeline_id: Option<uuid::Uuid>,
        #[arg(long)]
        stage_id: Option<uuid::Uuid>,
        #[arg(long)]
        tag_id: Option<uuid::Uuid>,
    },
    Opportunity {
        id: uuid::Uuid,
    },
    CreateOpportunity {
        #[arg(long)]
        idempotency_key: String,
    },
    EditOpportunity {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    ArchiveOpportunity {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    OpportunityHistory {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Deals {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        category_id: Option<uuid::Uuid>,
        #[arg(long)]
        status_id: Option<uuid::Uuid>,
        #[arg(long)]
        originating_opportunity_id: Option<uuid::Uuid>,
        #[arg(long)]
        tag_id: Option<uuid::Uuid>,
    },
    Deal {
        id: uuid::Uuid,
    },
    CreateDeal {
        #[arg(long)]
        idempotency_key: String,
    },
    EditDeal {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    ArchiveDeal {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    DealHistory {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Modules,
    SetCrm {
        #[arg(long)]
        idempotency_key: String,
    },
}

#[derive(Subcommand)]
enum AccessCommand {
    Permissions,
    Roles {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Role {
        id: uuid::Uuid,
    },
    CreateRole {
        #[arg(long)]
        idempotency_key: String,
    },
    EditRole {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Members {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Member {
        id: uuid::Uuid,
    },
    EditMember {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
}

#[derive(Subcommand)]
enum ContactCommand {
    Tags {
        id: uuid::Uuid,
    },
    SetTags {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
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
        tag_id: Option<uuid::Uuid>,
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
    Tags {
        id: uuid::Uuid,
    },
    SetTags {
        id: uuid::Uuid,
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
        tag_id: Option<uuid::Uuid>,
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
enum TagCommand {
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
}
#[derive(Subcommand)]
enum IntakeCommand {
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long, default_value = "pending")]
        state: String,
    },
    Get {
        id: uuid::Uuid,
    },
    Submit {
        #[arg(long)]
        idempotency_key: String,
    },
    Resolve {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Candidates {
        id: uuid::Uuid,
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
                ContactCommand::Tags { id } => {
                    serde_json::to_value(client.contacts_tags(workspace, id).await?)?
                }
                ContactCommand::SetTags {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .set_contacts_tags(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                ContactCommand::List {
                    limit,
                    after,
                    search,
                    tag_id,
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
                                tag_id,
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
                OrganizationCommand::Tags { id } => {
                    serde_json::to_value(client.organizations_tags(workspace, id).await?)?
                }
                OrganizationCommand::SetTags {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .set_organizations_tags(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                OrganizationCommand::List {
                    limit,
                    after,
                    search,
                    tag_id,
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
                                tag_id,
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
        Command::Access {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            match action {
                AccessCommand::Permissions => println!(
                    "{}",
                    serde_json::to_string(&client.access_permissions(workspace).await?)?
                ),
                AccessCommand::Roles { limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .access_roles(
                                workspace,
                                &looski::workspace_access::AccessQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                AccessCommand::Role { id } => println!(
                    "{}",
                    serde_json::to_string(&client.access_role(workspace, id).await?)?
                ),
                AccessCommand::CreateRole { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .access_create_role(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                AccessCommand::EditRole {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .access_edit_role(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                AccessCommand::Members { limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .access_members(
                                workspace,
                                &looski::workspace_access::AccessQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                AccessCommand::Member { id } => println!(
                    "{}",
                    serde_json::to_string(&client.access_member(workspace, id).await?)?
                ),
                AccessCommand::EditMember {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .access_edit_member(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
            }
        }
        Command::Tasks {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            let result = match action {
                TasksCommand::List {
                    limit,
                    after,
                    search,
                    archived,
                    status,
                    assigned_membership_id,
                    due_before,
                } => serde_json::to_value(
                    client
                        .tasks_list(
                            workspace,
                            &looski::tasks::TaskQuery {
                                limit,
                                after,
                                search,
                                archived,
                                status: status
                                    .map(|s| serde_json::from_value(serde_json::Value::String(s)))
                                    .transpose()?,
                                assigned_membership_id,
                                due_before,
                            },
                        )
                        .await?,
                )?,
                TasksCommand::Get { id } => {
                    serde_json::to_value(client.tasks_get(workspace, id).await?)?
                }
                TasksCommand::Create { idempotency_key } => serde_json::to_value(
                    client
                        .tasks_create(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::Edit {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .tasks_edit(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::Archive {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .tasks_archive(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::History { id, limit, after } => serde_json::to_value(
                    client
                        .tasks_history(
                            workspace,
                            id,
                            &looski::tasks::TaskHistoryQuery { limit, after },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string(&result)?);
        }
        Command::Crm {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            match action {
                CrmCommand::OpportunityTasks {
                    id,
                    limit,
                    after,
                    ended,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_tasks(
                                workspace,
                                id,
                                &looski::crm::task_links::TaskLinkQuery {
                                    limit,
                                    after,
                                    ended
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::LinkOpportunityTask {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_link_opportunity_task(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::CreateOpportunityFollowup {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_opportunity_followup(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EndOpportunityTask {
                    id,
                    link_id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_end_opportunity_task(
                                workspace,
                                id,
                                link_id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityTaskHistory {
                    id,
                    link_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_task_history(
                                workspace,
                                id,
                                link_id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealTasks {
                    id,
                    limit,
                    after,
                    ended,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_tasks(
                                workspace,
                                id,
                                &looski::crm::task_links::TaskLinkQuery {
                                    limit,
                                    after,
                                    ended
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::LinkDealTask {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_link_deal_task(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::CreateDealFollowup {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_deal_followup(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EndDealTask {
                    id,
                    link_id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_end_deal_task(
                                workspace,
                                id,
                                link_id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealTaskHistory {
                    id,
                    link_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_task_history(
                                workspace,
                                id,
                                link_id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityTags { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_opportunity_tags(workspace, id).await?)?
                ),
                CrmCommand::SetOpportunityTags {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_set_opportunity_tags(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealTags { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_deal_tags(workspace, id).await?)?
                ),
                CrmCommand::SetDealTags {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_set_deal_tags(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityContacts {
                    id,
                    limit,
                    after,
                    ended,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_contacts(
                                workspace,
                                id,
                                &looski::crm::participants::ParticipantQuery {
                                    limit,
                                    after,
                                    ended: Some(ended)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::AddOpportunityContact {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_add_opportunity_contact(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditOpportunityContact {
                    id,
                    association_id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_opportunity_contact(
                                workspace,
                                id,
                                association_id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityContactHistory {
                    id,
                    association_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_contact_history(
                                workspace,
                                id,
                                association_id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityOrganizations {
                    id,
                    limit,
                    after,
                    ended,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_organizations(
                                workspace,
                                id,
                                &looski::crm::participants::ParticipantQuery {
                                    limit,
                                    after,
                                    ended: Some(ended)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::AddOpportunityOrganization {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_add_opportunity_organization(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditOpportunityOrganization {
                    id,
                    association_id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_opportunity_organization(
                                workspace,
                                id,
                                association_id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityOrganizationHistory {
                    id,
                    association_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_organization_history(
                                workspace,
                                id,
                                association_id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealContacts {
                    id,
                    limit,
                    after,
                    ended,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_contacts(
                                workspace,
                                id,
                                &looski::crm::participants::ParticipantQuery {
                                    limit,
                                    after,
                                    ended: Some(ended)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::AddDealContact {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_add_deal_contact(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditDealContact {
                    id,
                    association_id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_deal_contact(
                                workspace,
                                id,
                                association_id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealContactHistory {
                    id,
                    association_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_contact_history(
                                workspace,
                                id,
                                association_id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealOrganizations {
                    id,
                    limit,
                    after,
                    ended,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_organizations(
                                workspace,
                                id,
                                &looski::crm::participants::ParticipantQuery {
                                    limit,
                                    after,
                                    ended: Some(ended)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::AddDealOrganization {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_add_deal_organization(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditDealOrganization {
                    id,
                    association_id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_deal_organization(
                                workspace,
                                id,
                                association_id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealOrganizationHistory {
                    id,
                    association_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_organization_history(
                                workspace,
                                id,
                                association_id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Pipelines {
                    limit,
                    after,
                    parent_id,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_pipelines(
                                workspace,
                                &looski::crm::configuration::DefinitionQuery {
                                    limit,
                                    after,
                                    parent_id,
                                    archived: Some(archived)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Pipeline { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_pipeline(workspace, id).await?)?
                ),
                CrmCommand::CreatePipeline { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_pipeline(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditPipeline {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_pipeline(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::Stages {
                    limit,
                    after,
                    parent_id,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_stages(
                                workspace,
                                &looski::crm::configuration::DefinitionQuery {
                                    limit,
                                    after,
                                    parent_id,
                                    archived: Some(archived)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Stage { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_stage(workspace, id).await?)?
                ),
                CrmCommand::CreateStage { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_stage(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditStage {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_stage(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::Categories {
                    limit,
                    after,
                    parent_id,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_categories(
                                workspace,
                                &looski::crm::configuration::DefinitionQuery {
                                    limit,
                                    after,
                                    parent_id,
                                    archived: Some(archived)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Category { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_category(workspace, id).await?)?
                ),
                CrmCommand::CreateCategory { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_category(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditCategory {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_category(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::Statuses {
                    limit,
                    after,
                    parent_id,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_statuses(
                                workspace,
                                &looski::crm::configuration::DefinitionQuery {
                                    limit,
                                    after,
                                    parent_id,
                                    archived: Some(archived)
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Status { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_status(workspace, id).await?)?
                ),
                CrmCommand::CreateStatus { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_status(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditStatus {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_status(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::Opportunities {
                    limit,
                    after,
                    search,
                    archived,
                    pipeline_id,
                    stage_id,
                    tag_id,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunities(
                                workspace,
                                &looski::crm::OpportunityQuery {
                                    limit,
                                    after,
                                    search,
                                    archived: Some(archived),
                                    pipeline_id,
                                    stage_id,
                                    tag_id,
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Opportunity { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_opportunity(workspace, id).await?)?
                ),
                CrmCommand::CreateOpportunity { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_opportunity(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditOpportunity {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_opportunity(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::ArchiveOpportunity {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_archive_opportunity(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::OpportunityHistory { id, limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_opportunity_history(
                                workspace,
                                id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Deals {
                    limit,
                    after,
                    search,
                    archived,
                    category_id,
                    status_id,
                    originating_opportunity_id,
                    tag_id,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deals(
                                workspace,
                                &looski::crm::DealQuery {
                                    limit,
                                    after,
                                    search,
                                    archived: Some(archived),
                                    category_id,
                                    status_id,
                                    originating_opportunity_id,
                                    tag_id,
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Deal { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_deal(workspace, id).await?)?
                ),
                CrmCommand::CreateDeal { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_deal(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditDeal {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_deal(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::ArchiveDeal {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_archive_deal(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealHistory { id, limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_history(
                                workspace,
                                id,
                                &looski::crm::HistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Modules => println!(
                    "{}",
                    serde_json::to_string(&client.crm_modules(workspace).await?)?
                ),
                CrmCommand::SetCrm { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_set_crm(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
            }
        }
        Command::Tags {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            let result = match action {
                TagCommand::List {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .tags(
                            workspace,
                            &looski::tags::ListTags {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                TagCommand::Get { id } => serde_json::to_value(client.tag(workspace, id).await?)?,
                TagCommand::Create { idempotency_key } => serde_json::to_value(
                    client
                        .create_tag(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TagCommand::Update {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .edit_tag(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Intakes {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_token_file(token_file);
            let result = match action {
                IntakeCommand::List {
                    limit,
                    after,
                    state,
                } => serde_json::to_value(
                    client
                        .intakes(
                            workspace,
                            &looski::contacts::intake::ListIntakes {
                                limit: Some(limit),
                                after,
                                state: Some(state),
                            },
                        )
                        .await?,
                )?,
                IntakeCommand::Get { id } => {
                    serde_json::to_value(client.intake(workspace, id).await?)?
                }
                IntakeCommand::Candidates { id } => {
                    serde_json::to_value(client.intake_candidates(workspace, id).await?)?
                }
                IntakeCommand::Submit { idempotency_key } => serde_json::to_value(
                    client
                        .submit_intake(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                IntakeCommand::Resolve {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .resolve_intake(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
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
