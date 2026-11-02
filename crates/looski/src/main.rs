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
enum FilesCommand {
    List {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        search: Option<String>,
    },
    Get {
        id: uuid::Uuid,
    },
    Content {
        id: uuid::Uuid,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    Upload {
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
}
#[derive(Subcommand)]
enum ExportCommand {
    Collections,
    Page,
}

#[derive(Subcommand)]
enum Command {
    /// Shared files. Mutations read JSON from stdin; content can write a private, new file.
    Files {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: FilesCommand,
    },
    /// Export authorized business pages as JSON. Page selection is JSON on stdin.
    Export {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: ExportCommand,
    },

    Jobs {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: JobsCommand,
    },

    /// Run the local durable worker; --once handles at most one notification.
    Worker {
        #[arg(long)]
        once: bool,
    },
    /// Check the worker heartbeat without granting business authority.
    WorkerHealth,

    /// Shared task operations. Mutations read JSON from stdin.
    Tasks {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
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
        token_file: Option<std::path::PathBuf>,
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
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: AccessCommand,
    },
    /// Provision the first owner locally; reads credentials as JSON from stdin.
    BootstrapOwner,
    /// Serve the explicitly enabled, expiring first-owner claim flow.
    SetupServer {
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: SocketAddr,
    },
    /// Sign in through the browser and store a CLI/MCP session for this installation.
    Login {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        /// Name shown on the approval page; defaults to this host's name.
        #[arg(long)]
        device_label: Option<String>,
        /// Print the link without trying to open a browser.
        #[arg(long)]
        no_browser: bool,
    },
    /// Revoke and forget the stored session for this installation.
    Logout {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
    },
    /// Show the signed-in identity and the workspaces it can currently use.
    Whoami {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
    },
    /// List permitted workspaces with the stored login or an access-token file.
    Workspaces {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
    },
    /// Workspace Contacts operations. Mutations read their JSON body from stdin.
    Contacts {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
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
        token_file: Option<std::path::PathBuf>,
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
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: TagCommand,
    },
    /// Optional application selection for a workspace.
    Modules {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: ModuleCommand,
    },
    /// Forms, versions, submissions and the CRM inquiry recipe. Mutations read JSON from stdin.
    Forms {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: FormCommand,
    },
    /// Projects, boards, delivery views, comments and the deal-to-project recipe. Mutations read JSON from stdin.
    Projects {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: ProjectCommand,
    },
    /// CMS content types, entries, revisions and media. Mutations read JSON from stdin.
    Cms {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: CmsCommand,
    },
    /// Time records. Mutations read JSON from stdin.
    Time {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: TimeCommand,
    },
    /// Scheduling records. Mutations read JSON from stdin.
    Scheduling {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: SchedulingCommand,
    },
    /// Helpdesk staff tickets and configuration. Mutations read JSON from stdin.
    Helpdesk {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: HelpdeskCommand,
    },
    /// Knowledge base collections, documents, revisions and search. Mutations read JSON from stdin.
    Knowledge {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
        #[arg(long)]
        workspace: uuid::Uuid,
        #[command(subcommand)]
        action: KnowledgeCommand,
    },
    /// Read published content by its content type's public key, without signing in.
    PublicContent {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        public_key: String,
        #[command(subcommand)]
        action: PublicContentCommand,
    },
    /// Read or answer a published form by its public key, without signing in.
    PublicForm {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        public_key: String,
        #[command(subcommand)]
        action: PublicFormCommand,
    },
    /// Retained contact intake and manual review. Mutations read JSON from stdin.
    Intakes {
        #[arg(long, env = "LOOSKI_API_URL", default_value = "http://localhost:58888")]
        api_url: String,
        #[arg(long, env = "LOOSKI_ACCESS_TOKEN_FILE")]
        token_file: Option<std::path::PathBuf>,
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
enum JobsCommand {
    List {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Get {
        id: uuid::Uuid,
    },
    History {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Change {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
}
#[derive(Subcommand)]
enum TasksCommand {
    Types {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        archived: Option<bool>,
    },
    Type {
        id: uuid::Uuid,
    },
    CreateType {
        #[arg(long)]
        idempotency_key: String,
    },
    EditType {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Templates {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        archived: Option<bool>,
    },
    Template {
        id: uuid::Uuid,
    },
    CreateTemplate {
        #[arg(long)]
        idempotency_key: String,
    },
    EditTemplate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    TemplateRevisions {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    InstantiateTemplate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },

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
    Timeline {
        #[arg(long)]
        anchor_kind: String,
        #[arg(long)]
        anchor_id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<String>,
    },

    StarterPackages {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    StarterPackage {
        id: uuid::Uuid,
    },
    InstallStarter {
        #[arg(long)]
        idempotency_key: String,
    },
    RemoveStarter {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },

    CoordinationRules {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        archived: Option<bool>,
    },
    CoordinationRule {
        id: uuid::Uuid,
    },
    CreateCoordinationRule {
        #[arg(long)]
        idempotency_key: String,
    },
    EditCoordinationRule {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },

    CoordinationTemplates {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        archived: Option<bool>,
    },
    CoordinationTemplate {
        id: uuid::Uuid,
    },
    CreateCoordinationTemplate {
        #[arg(long)]
        idempotency_key: String,
    },
    EditCoordinationTemplate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    CoordinationRevisions {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    DealCoordination {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    DealCoordinationInstance {
        id: uuid::Uuid,
        instance_id: uuid::Uuid,
    },
    ApplyCoordination {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },

    ActivityTypes {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        archived: Option<bool>,
    },
    ActivityType {
        id: uuid::Uuid,
    },
    CreateActivityType {
        #[arg(long)]
        idempotency_key: String,
    },
    EditActivityType {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Activities {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        anchor_kind: String,
        #[arg(long)]
        anchor_id: uuid::Uuid,
        #[arg(long)]
        archived: Option<bool>,
        #[arg(long)]
        type_id: Option<uuid::Uuid>,
    },
    Activity {
        id: uuid::Uuid,
    },
    CreateActivity {
        #[arg(long)]
        idempotency_key: String,
    },
    EditActivity {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    ArchiveActivity {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    ActivityHistory {
        id: uuid::Uuid,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },

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
    PreviewInvitation {
        id: uuid::Uuid,
    },
    Invitations {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Invitation {
        id: uuid::Uuid,
    },
    CreateInvitation {
        #[arg(long)]
        idempotency_key: String,
    },
    SendInvitation {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    RevokeInvitation {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    AcceptInvitation {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },

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
    DeletePermanently {
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
enum ModuleCommand {
    List,
    Set {
        module: String,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        disable: bool,
    },
}
#[derive(clap::Subcommand)]
enum ProjectCommand {
    #[command(
        about = "List projects this member can read; assigned scope lists those they belong to."
    )]
    ListProjects {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        status: Option<String>,
    },
    #[command(
        about = "Create a project with a default board. Works with CRM disabled; the creator becomes a member. Reads SaveProject JSON from stdin."
    )]
    CreateProject {
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "Tasks assigned to the caller across projects they may read.")]
    GetMyProjectWork {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        include_finished: bool,
    },
    #[command(
        about = "Active workspace members who can be added to a project. Requires projects.members.manage."
    )]
    ListProjectDirectory {},
    #[command(about = "List deal-to-project recipes.")]
    ListProjectRecipes {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    #[command(
        about = "Create a deal-to-project recipe naming the deal status that marks the milestone. It starts inactive. Reads SaveDealRecipe JSON from stdin."
    )]
    CreateProjectRecipe {
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "Get a deal-to-project recipe with its current revision.")]
    GetProjectRecipe {
        #[arg(long)]
        recipe_id: uuid::Uuid,
    },
    #[command(
        about = "Save a new immutable revision of a recipe; existing runs keep theirs. Requires expected_version. Reads SaveDealRecipe JSON from stdin."
    )]
    UpdateProjectRecipe {
        #[arg(long)]
        recipe_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Activate or deactivate a recipe. Activation records the caller as the authority each run re-checks. Reads SetDealRecipeActive JSON from stdin."
    )]
    SetProjectRecipeActive {
        #[arg(long)]
        recipe_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "List a recipe's conversions with their job state and failure reason.")]
    ListProjectRecipeRuns {
        #[arg(long)]
        recipe_id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    #[command(
        about = "Resume a blocked or exhausted conversion. It stays the same conversion and cannot create a second project. Reads RetryDealConversion JSON from stdin."
    )]
    RetryProjectRecipeRun {
        #[arg(long)]
        recipe_id: uuid::Uuid,
        #[arg(long)]
        run_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "Get one project.")]
    GetProject {
        #[arg(long)]
        project_id: uuid::Uuid,
    },
    #[command(
        about = "Edit a project's name, description, status or deadline. Requires expected_version. Reads SaveProject JSON from stdin."
    )]
    UpdateProject {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Archive or restore a project. Its tasks stay in shared Tasks. Reads ArchiveProject JSON from stdin."
    )]
    ArchiveProject {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "List a project's members.")]
    ListProjectMembers {
        #[arg(long)]
        project_id: uuid::Uuid,
    },
    #[command(
        about = "Add an active workspace member to a project. Reads AddMember JSON from stdin."
    )]
    AddProjectMember {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "Remove a member; assigned-scope access to the project ends at once.")]
    RemoveProjectMember {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        membership_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "List a board's columns in order.")]
    ListProjectColumns {
        #[arg(long)]
        project_id: uuid::Uuid,
    },
    #[command(
        about = "Add a board column, optionally with the task status a move into it requests. Reads SaveColumn JSON from stdin."
    )]
    CreateProjectColumn {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Rename, remap or reorder a board column. Requires expected_version. Reads SaveColumn JSON from stdin."
    )]
    UpdateProjectColumn {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        column_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Remove a column, moving its cards to another column unchanged. Reads RemoveColumn JSON from stdin."
    )]
    RemoveProjectColumn {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        column_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "Read a project board: columns and the cards this member can see.")]
    GetProjectBoard {
        #[arg(long)]
        project_id: uuid::Uuid,
    },
    #[command(
        about = "Delivery view: progress, deadlines and workload over the tasks this member can see."
    )]
    GetProjectDelivery {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        assigned_membership_id: Option<uuid::Uuid>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        due: Option<String>,
    },
    #[command(
        about = "List a project's visible cards; ended=true lists tasks removed from the board."
    )]
    ListProjectCards {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        ended: bool,
    },
    #[command(
        about = "Put a task on the board: an existing shared task, or a new one created through Tasks. Reads AddCard JSON from stdin."
    )]
    AddProjectCard {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "Get one visible card with its shared task.")]
    GetProjectCard {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        card_id: uuid::Uuid,
    },
    #[command(
        about = "Move a card to a column and place. Requires expected_version; a stale version conflicts rather than undoing another move. Reads MoveCard JSON from stdin."
    )]
    MoveProjectCard {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        card_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Change a card's assignee, deadline and status through Tasks. Requires the task's expected version. Reads ScheduleCard JSON from stdin."
    )]
    ScheduleProjectCard {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        card_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Take a task off the board. The task and its comments are kept. Reads RemoveCard JSON from stdin."
    )]
    RemoveProjectCard {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        card_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(about = "List comments on a visible card, oldest first.")]
    ListProjectComments {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        card_id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    #[command(
        about = "Comment on a card. Text is a Markdown subset; HTML stays literal and unsafe links are refused. Reads NewComment JSON from stdin."
    )]
    AddProjectComment {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        card_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Edit one's own comment. Requires expected_version. Reads EditComment JSON from stdin."
    )]
    EditProjectComment {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        comment_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Remove a comment, erasing its text. Authors remove their own; moderators remove anyone's. Reads RemoveComment JSON from stdin."
    )]
    RemoveProjectComment {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        comment_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
}
/// Parse a snake_case option into its API enum.
fn enum_argument<T: serde::de::DeserializeOwned>(
    value: Option<String>,
) -> anyhow::Result<Option<T>> {
    value
        .map(|v| Ok(serde_json::from_value(serde_json::Value::String(v))?))
        .transpose()
}
#[derive(Subcommand)]
enum FormCommand {
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
    /// Reads SaveForm JSON.
    Create {
        #[arg(long)]
        idempotency_key: String,
    },
    /// Reads SaveForm JSON including expected_version.
    Update {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Publish {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
    /// Open or (with --close) stop accepting answers.
    Accepting {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        close: bool,
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
    /// Reads SetNotifications JSON.
    Notifications {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Versions {
        id: uuid::Uuid,
    },
    Submissions {
        id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Submission {
        id: uuid::Uuid,
        submission_id: uuid::Uuid,
    },
    Summary {
        id: uuid::Uuid,
    },
    Recipe {
        id: uuid::Uuid,
    },
    /// Reads ConfigureRecipe JSON.
    ConfigureRecipe {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    /// Activate or (with --deactivate) stop the CRM inquiry recipe.
    ActivateRecipe {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        deactivate: bool,
    },
    Runs {
        id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    RetryRun {
        id: uuid::Uuid,
        run_id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
}
#[derive(Subcommand)]
enum PublicFormCommand {
    Get,
    /// Reads PublicSubmission JSON.
    Submit,
}
#[derive(Subcommand)]
enum TimeCommand {
    List {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        project_id: Option<uuid::Uuid>,
        #[arg(long)]
        from_on: Option<String>,
        #[arg(long)]
        through_on: Option<String>,
        #[arg(long)]
        billable: Option<bool>,
    },
    Create {
        #[arg(long)]
        idempotency_key: String,
    },
    Get {
        id: uuid::Uuid,
    },
    Update {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Transition {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Report {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        project_id: Option<uuid::Uuid>,
        #[arg(long)]
        from_on: Option<String>,
        #[arg(long)]
        through_on: Option<String>,
        #[arg(long)]
        billable: Option<bool>,
    },
    Budgets {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
    },
    CreateBudget {
        #[arg(long)]
        idempotency_key: String,
    },
    UpdateBudget {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
}
#[derive(Subcommand)]
enum SchedulingCommand {
    Types {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
    },
    CreateType {
        #[arg(long)]
        idempotency_key: String,
    },
    GetType {
        id: uuid::Uuid,
    },
    UpdateType {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    List {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
    },
    Get {
        id: uuid::Uuid,
    },
    Change {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Retry {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Events {
        id: uuid::Uuid,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
    },
}
#[derive(Subcommand)]
enum HelpdeskCommand {
    Settings,
    Directory,
    Configure {
        #[arg(long)]
        idempotency_key: String,
    },
    List {
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        contact_id: Option<uuid::Uuid>,
    },
    Get {
        id: uuid::Uuid,
    },
    Messages {
        id: uuid::Uuid,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        limit: Option<i64>,
    },
    Update {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Link {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Reply {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
}
#[derive(Subcommand)]
enum KnowledgeCommand {
    #[command(about = "Collections this member reaches.")]
    CollectionList {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    CollectionGet {
        id: uuid::Uuid,
    },
    /// Reads SaveKnowledgeCollection JSON.
    CollectionCreate {
        #[arg(long)]
        idempotency_key: String,
    },
    /// Reads SaveKnowledgeCollection JSON including expected_version.
    CollectionUpdate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    CollectionArchive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        restore: bool,
    },
    MemberList {
        collection: uuid::Uuid,
    },
    MemberAdd {
        collection: uuid::Uuid,
        membership: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    MemberRemove {
        collection: uuid::Uuid,
        membership: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    #[command(
        about = "Active workspace members who can be listed. Requires knowledge.collections.manage."
    )]
    Directory,
    DocumentList {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        collection: Option<uuid::Uuid>,
        #[arg(long)]
        sop: bool,
        #[arg(long)]
        archived: bool,
    },
    DocumentGet {
        id: uuid::Uuid,
    },
    /// Reads CreateKnowledgeDocument JSON.
    DocumentCreate {
        #[arg(long)]
        idempotency_key: String,
    },
    /// Reads SaveKnowledgeDocument JSON including expected_version.
    DocumentUpdate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    DocumentArchive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        restore: bool,
    },
    DocumentMove {
        id: uuid::Uuid,
        #[arg(long)]
        collection: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
    Revisions {
        id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Revision {
        id: uuid::Uuid,
        revision: uuid::Uuid,
    },
    Restore {
        id: uuid::Uuid,
        revision: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
    #[command(about = "Full-text search over documents this member may read.")]
    Search {
        q: String,
        #[arg(long, default_value_t = 20)]
        limit: i64,
        #[arg(long)]
        after: Option<String>,
        #[arg(long)]
        collection: Option<uuid::Uuid>,
        #[arg(long)]
        sop: bool,
    },
    Suggest {
        q: String,
        #[arg(long, default_value_t = 8)]
        limit: i64,
    },
    Activity {
        #[arg(long, default_value_t = 20)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
}
#[derive(Subcommand)]
enum CmsCommand {
    TypeList {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    TypeGet {
        id: uuid::Uuid,
    },
    /// Reads SaveContentType JSON.
    TypeCreate {
        #[arg(long)]
        idempotency_key: String,
    },
    /// Reads SaveContentType JSON including expected_version.
    TypeUpdate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    TypeArchive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        restore: bool,
    },
    EntryList {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
        #[arg(long)]
        type_id: Option<uuid::Uuid>,
    },
    EntryGet {
        id: uuid::Uuid,
    },
    /// Reads CreateEntry JSON.
    EntryCreate {
        #[arg(long)]
        idempotency_key: String,
    },
    /// Reads SaveEntry JSON including expected_version.
    EntryUpdate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    Publish {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
    Unpublish {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
    EntryArchive {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
        #[arg(long)]
        restore: bool,
    },
    Preview {
        id: uuid::Uuid,
    },
    Revisions {
        id: uuid::Uuid,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
    },
    Revision {
        id: uuid::Uuid,
        revision: uuid::Uuid,
    },
    /// Copy a revision into the draft; publish afterwards to make it live.
    Restore {
        id: uuid::Uuid,
        revision: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        expected_version: i64,
    },
    MediaList {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        after: Option<uuid::Uuid>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    MediaGet {
        id: uuid::Uuid,
    },
    MediaContent {
        id: uuid::Uuid,
    },
    /// Reads RegisterMedia JSON.
    MediaRegister {
        #[arg(long)]
        idempotency_key: String,
    },
    /// Reads SaveMedia JSON including expected_version.
    MediaUpdate {
        id: uuid::Uuid,
        #[arg(long)]
        idempotency_key: String,
    },
    MediaArchive {
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
enum PublicContentCommand {
    Index {
        #[arg(long)]
        after: Option<String>,
    },
    Entry {
        slug: String,
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
        Command::Files {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                FilesCommand::List {
                    after,
                    limit,
                    archived,
                    search,
                } => serde_json::to_value(
                    client
                        .files_list(
                            workspace,
                            &looski::files::FileQuery {
                                after,
                                limit,
                                archived: Some(archived),
                                search,
                            },
                        )
                        .await?,
                )?,
                FilesCommand::Get { id } => {
                    serde_json::to_value(client.files_get(workspace, id).await?)?
                }
                FilesCommand::Content { id, output } => {
                    let content = client.files_content(workspace, id).await?;
                    if let Some(path) = output {
                        looski::files::write_download(&path, &content)?;
                        serde_json::to_value(content.file)?
                    } else {
                        serde_json::to_value(content)?
                    }
                }
                FilesCommand::Upload { idempotency_key } => serde_json::to_value(
                    client
                        .files_upload(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                FilesCommand::Edit {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .files_edit(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                FilesCommand::Archive {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .files_archive(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string(&result)?);
        }
        Command::Export {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            match action {
                ExportCommand::Collections => println!(
                    "{}",
                    serde_json::to_string(&client.export_collections(workspace).await?)?
                ),
                ExportCommand::Page => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .export_page(workspace, &serde_json::from_reader(std::io::stdin())?)
                            .await?
                    )?
                ),
            }
        }

        Command::Openapi => println!("{}", api::openapi().to_pretty_json()?),
        Command::Login {
            api_url,
            device_label,
            no_browser,
        } => {
            let client = ApiClient::new(&api_url)?;
            let device_label = device_label.or_else(|| {
                let host = std::process::Command::new("hostname").output().ok()?;
                let host = String::from_utf8(host.stdout).ok()?;
                Some(format!("Looski CLI on {}", host.trim()))
            });
            let status = client
                .login()?
                .login(client.http()?, device_label, !no_browser)
                .await?;
            println!("{}", serde_json::to_string(&status)?);
        }
        Command::Logout { api_url } => {
            let client = ApiClient::new(&api_url)?;
            let login = client.login()?;
            let signed_in = login.status()?.is_some();
            let revoked = login.logout(client.http()?).await?;
            println!(
                "{}",
                serde_json::json!({"origin": login.origin(), "signed_in": false, "was_signed_in": signed_in, "server_revoked": revoked})
            );
        }
        Command::Whoami { api_url } => {
            let client = ApiClient::new(&api_url)?.with_credentials(None)?;
            // Proves the session still works and reflects current permissions.
            let workspaces = client.workspaces().await?;
            let status = client
                .login()?
                .status()?
                .ok_or_else(|| anyhow::anyhow!("Not signed in. Run `looski login`."))?;
            println!(
                "{}",
                serde_json::json!({"origin": status.origin, "user": status.user, "access_expires_at": status.access_expires_at, "credential_store": status.credential_store, "workspaces": workspaces})
            );
        }
        Command::Status { api_url } => {
            let info = ApiClient::new(&api_url)?.system().await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        Command::Workspaces {
            api_url,
            token_file,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
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
                ContactCommand::DeletePermanently {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .delete_contact_permanently(
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            match action {
                AccessCommand::PreviewInvitation { id } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .invitation_preview(
                                workspace,
                                id,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                AccessCommand::Invitations { limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .invitation_list(
                                workspace,
                                &looski::workspace_access::AccessQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                AccessCommand::Invitation { id } => println!(
                    "{}",
                    serde_json::to_string(&client.invitation_get(workspace, id).await?)?
                ),
                AccessCommand::CreateInvitation { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .invitation_create(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                AccessCommand::SendInvitation {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .invitation_send(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                AccessCommand::RevokeInvitation {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .invitation_revoke(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                AccessCommand::AcceptInvitation {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .invitation_accept(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
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
        Command::Jobs {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            match action {
                JobsCommand::List { limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .jobs_list(workspace, &looski::jobs::JobQuery { limit, after })
                            .await?
                    )?
                ),
                JobsCommand::Get { id } => println!(
                    "{}",
                    serde_json::to_string(&client.jobs_get(workspace, id).await?)?
                ),
                JobsCommand::History { id, limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .jobs_history(workspace, id, &looski::jobs::JobQuery { limit, after })
                            .await?
                    )?
                ),
                JobsCommand::Change {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .jobs_change(
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                TasksCommand::Types {
                    limit,
                    after,
                    archived,
                } => serde_json::to_value(
                    client
                        .tasks_types(
                            workspace,
                            &looski::tasks::configuration::TaskConfigurationQuery {
                                limit,
                                after,
                                archived,
                            },
                        )
                        .await?,
                )?,
                TasksCommand::Type { id } => {
                    serde_json::to_value(client.tasks_type(workspace, id).await?)?
                }
                TasksCommand::CreateType { idempotency_key } => serde_json::to_value(
                    client
                        .tasks_create_type(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::EditType {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .tasks_edit_type(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::Templates {
                    limit,
                    after,
                    archived,
                } => serde_json::to_value(
                    client
                        .tasks_templates(
                            workspace,
                            &looski::tasks::configuration::TaskConfigurationQuery {
                                limit,
                                after,
                                archived,
                            },
                        )
                        .await?,
                )?,
                TasksCommand::Template { id } => {
                    serde_json::to_value(client.tasks_template(workspace, id).await?)?
                }
                TasksCommand::CreateTemplate { idempotency_key } => serde_json::to_value(
                    client
                        .tasks_create_template(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::EditTemplate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .tasks_edit_template(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                TasksCommand::TemplateRevisions { id, limit, after } => serde_json::to_value(
                    client
                        .tasks_template_revisions(
                            workspace,
                            id,
                            &looski::tasks::configuration::TaskRevisionQuery { limit, after },
                        )
                        .await?,
                )?,
                TasksCommand::InstantiateTemplate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .tasks_instantiate_template(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
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
                CrmCommand::ActivityTypes {
                    limit,
                    after,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_activity_types(
                                workspace,
                                &looski::crm::activities::types::ActivityTypeQuery {
                                    limit,
                                    after,
                                    archived
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::ActivityType { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_activity_type(workspace, id).await?)?
                ),
                CrmCommand::CreateActivityType { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_activity_type(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditActivityType {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_activity_type(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::Activities {
                    limit,
                    after,
                    anchor_kind,
                    anchor_id,
                    archived,
                    type_id,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_activities(
                                workspace,
                                &looski::crm::activities::ActivityQuery {
                                    limit,
                                    after,
                                    anchor_kind: serde_json::from_value(
                                        serde_json::Value::String(anchor_kind)
                                    )?,
                                    anchor_id,
                                    archived,
                                    type_id
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::Activity { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_activity(workspace, id).await?)?
                ),
                CrmCommand::CreateActivity { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_activity(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditActivity {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_activity(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::ArchiveActivity {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_archive_activity(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::ActivityHistory { id, limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_activity_history(
                                workspace,
                                id,
                                &looski::crm::activities::ActivityHistoryQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::CoordinationTemplates {
                    limit,
                    after,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_coordination_templates(
                                workspace,
                                &looski::crm::coordination::CoordinationQuery {
                                    limit,
                                    after,
                                    archived
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::CoordinationTemplate { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_coordination_template(workspace, id).await?)?
                ),
                CrmCommand::CreateCoordinationTemplate { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_coordination_template(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditCoordinationTemplate {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_coordination_template(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::CoordinationRevisions { id, limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_coordination_revisions(
                                workspace,
                                id,
                                &looski::crm::coordination::CoordinationPageQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealCoordination { id, limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_coordination(
                                workspace,
                                id,
                                &looski::crm::coordination::CoordinationPageQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::DealCoordinationInstance { id, instance_id } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_deal_coordination_instance(workspace, id, instance_id)
                            .await?
                    )?
                ),
                CrmCommand::ApplyCoordination {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_apply_coordination(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::CoordinationRules {
                    limit,
                    after,
                    archived,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_coordination_rules(
                                workspace,
                                &looski::crm::status_jobs::RuleQuery {
                                    limit,
                                    after,
                                    archived
                                }
                            )
                            .await?
                    )?
                ),
                CrmCommand::CoordinationRule { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_coordination_rule(workspace, id).await?)?
                ),
                CrmCommand::CreateCoordinationRule { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_create_coordination_rule(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::EditCoordinationRule {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_edit_coordination_rule(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::StarterPackages { limit, after } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_starter_packages(
                                workspace,
                                &looski::crm::starter::StarterQuery { limit, after }
                            )
                            .await?
                    )?
                ),
                CrmCommand::StarterPackage { id } => println!(
                    "{}",
                    serde_json::to_string(&client.crm_starter_package(workspace, id).await?)?
                ),
                CrmCommand::InstallStarter { idempotency_key } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_install_starter(
                                workspace,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::RemoveStarter {
                    id,
                    idempotency_key,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_remove_starter(
                                workspace,
                                id,
                                &idempotency_key,
                                &serde_json::from_reader(std::io::stdin())?
                            )
                            .await?
                    )?
                ),
                CrmCommand::Timeline {
                    anchor_kind,
                    anchor_id,
                    limit,
                    after,
                } => println!(
                    "{}",
                    serde_json::to_string(
                        &client
                            .crm_timeline(
                                workspace,
                                &looski::crm::timeline::TimelineQuery {
                                    anchor_kind: serde_json::from_value(
                                        serde_json::Value::String(anchor_kind)
                                    )?,
                                    anchor_id,
                                    limit,
                                    after
                                }
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
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
        Command::Modules {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                ModuleCommand::List => serde_json::to_value(client.crm_modules(workspace).await?)?,
                ModuleCommand::Set {
                    module,
                    idempotency_key,
                    expected_version,
                    disable,
                } => serde_json::to_value(
                    client
                        .set_module(
                            workspace,
                            &module,
                            &idempotency_key,
                            &looski::modules::SetModule {
                                enabled: !disable,
                                expected_version,
                            },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Forms {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                FormCommand::List {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .forms(
                            workspace,
                            &looski::forms::ListForms {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                FormCommand::Get { id } => serde_json::to_value(client.form(workspace, id).await?)?,
                FormCommand::Create { idempotency_key } => serde_json::to_value(
                    client
                        .create_form(
                            workspace,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                FormCommand::Update {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .edit_form(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                FormCommand::Publish {
                    id,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .publish_form(
                            workspace,
                            id,
                            &idempotency_key,
                            &looski::forms::PublishForm { expected_version },
                        )
                        .await?,
                )?,
                FormCommand::Accepting {
                    id,
                    idempotency_key,
                    expected_version,
                    close,
                } => serde_json::to_value(
                    client
                        .set_form_accepting(
                            workspace,
                            id,
                            &idempotency_key,
                            &looski::forms::SetAccepting {
                                expected_version,
                                accepting: !close,
                            },
                        )
                        .await?,
                )?,
                FormCommand::Archive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .archive_form(
                            workspace,
                            id,
                            &idempotency_key,
                            &looski::forms::ArchiveForm {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
                FormCommand::Notifications {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .set_form_notifications(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                FormCommand::Versions { id } => {
                    serde_json::to_value(client.form_versions(workspace, id).await?)?
                }
                FormCommand::Submissions { id, limit, after } => serde_json::to_value(
                    client
                        .form_submissions(
                            workspace,
                            id,
                            &looski::forms::SubmissionQuery {
                                limit: Some(limit),
                                after,
                            },
                        )
                        .await?,
                )?,
                FormCommand::Submission { id, submission_id } => serde_json::to_value(
                    client.form_submission(workspace, id, submission_id).await?,
                )?,
                FormCommand::Summary { id } => {
                    serde_json::to_value(client.form_summary(workspace, id).await?)?
                }
                FormCommand::Recipe { id } => {
                    serde_json::to_value(client.form_recipe(workspace, id).await?)?
                }
                FormCommand::ConfigureRecipe {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .configure_form_recipe(
                            workspace,
                            id,
                            &idempotency_key,
                            &serde_json::from_reader(std::io::stdin())?,
                        )
                        .await?,
                )?,
                FormCommand::ActivateRecipe {
                    id,
                    idempotency_key,
                    expected_version,
                    deactivate,
                } => serde_json::to_value(
                    client
                        .set_form_recipe_active(
                            workspace,
                            id,
                            &idempotency_key,
                            &looski::forms::recipes::SetRecipeActive {
                                expected_version,
                                active: !deactivate,
                            },
                        )
                        .await?,
                )?,
                FormCommand::Runs { id, limit, after } => serde_json::to_value(
                    client
                        .form_recipe_runs(
                            workspace,
                            id,
                            &looski::forms::recipes::RunQuery {
                                limit: Some(limit),
                                after,
                            },
                        )
                        .await?,
                )?,
                FormCommand::RetryRun {
                    id,
                    run_id,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .retry_form_recipe_run(
                            workspace,
                            id,
                            run_id,
                            &idempotency_key,
                            &looski::forms::recipes::RetryRun { expected_version },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Projects {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                ProjectCommand::ListProjects {
                    limit,
                    after,
                    search,
                    archived,
                    status,
                } => {
                    let query = looski::projects::ListProjects {
                        limit: Some(limit),
                        after,
                        search,
                        archived: Some(archived),
                        status: enum_argument(status)?,
                    };
                    serde_json::to_value(client.list_projects(workspace, &query).await?)?
                }
                ProjectCommand::CreateProject { idempotency_key } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .create_project(workspace, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::GetMyProjectWork {
                    limit,
                    after,
                    include_finished,
                } => {
                    let query = looski::projects::MyWorkQuery {
                        limit: Some(limit),
                        after,
                        include_finished: Some(include_finished),
                    };
                    serde_json::to_value(client.my_project_work(workspace, &query).await?)?
                }
                ProjectCommand::ListProjectDirectory {} => {
                    serde_json::to_value(client.project_directory(workspace).await?)?
                }
                ProjectCommand::ListProjectRecipes { limit, after } => {
                    let query = looski::projects::recipes::DealConversionQuery {
                        limit: Some(limit),
                        after,
                    };
                    serde_json::to_value(client.project_recipes(workspace, &query).await?)?
                }
                ProjectCommand::CreateProjectRecipe { idempotency_key } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .create_project_recipe(workspace, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::GetProjectRecipe { recipe_id } => {
                    serde_json::to_value(client.project_recipe(workspace, recipe_id).await?)?
                }
                ProjectCommand::UpdateProjectRecipe {
                    recipe_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .update_project_recipe(workspace, recipe_id, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::SetProjectRecipeActive {
                    recipe_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .set_project_recipe_active(
                                workspace,
                                recipe_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::ListProjectRecipeRuns {
                    recipe_id,
                    limit,
                    after,
                } => {
                    let query = looski::projects::recipes::DealConversionQuery {
                        limit: Some(limit),
                        after,
                    };
                    serde_json::to_value(
                        client
                            .project_recipe_runs(workspace, recipe_id, &query)
                            .await?,
                    )?
                }
                ProjectCommand::RetryProjectRecipeRun {
                    recipe_id,
                    run_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .retry_project_recipe_run(
                                workspace,
                                recipe_id,
                                run_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::GetProject { project_id } => {
                    serde_json::to_value(client.project(workspace, project_id).await?)?
                }
                ProjectCommand::UpdateProject {
                    project_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .update_project(workspace, project_id, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::ArchiveProject {
                    project_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .archive_project(workspace, project_id, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::ListProjectMembers { project_id } => {
                    serde_json::to_value(client.project_members(workspace, project_id).await?)?
                }
                ProjectCommand::AddProjectMember {
                    project_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .add_project_member(workspace, project_id, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::RemoveProjectMember {
                    project_id,
                    membership_id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .remove_project_member(
                            workspace,
                            project_id,
                            membership_id,
                            &idempotency_key,
                        )
                        .await?,
                )?,
                ProjectCommand::ListProjectColumns { project_id } => {
                    serde_json::to_value(client.project_columns(workspace, project_id).await?)?
                }
                ProjectCommand::CreateProjectColumn {
                    project_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .create_project_column(workspace, project_id, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::UpdateProjectColumn {
                    project_id,
                    column_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .update_project_column(
                                workspace,
                                project_id,
                                column_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::RemoveProjectColumn {
                    project_id,
                    column_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .remove_project_column(
                                workspace,
                                project_id,
                                column_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::GetProjectBoard { project_id } => {
                    serde_json::to_value(client.project_board(workspace, project_id).await?)?
                }
                ProjectCommand::GetProjectDelivery {
                    project_id,
                    assigned_membership_id,
                    status,
                    due,
                } => {
                    let query = looski::projects::DeliveryQuery {
                        assigned_membership_id,
                        status: enum_argument(status)?,
                        due: enum_argument(due)?,
                    };
                    serde_json::to_value(
                        client
                            .project_delivery(workspace, project_id, &query)
                            .await?,
                    )?
                }
                ProjectCommand::ListProjectCards {
                    project_id,
                    limit,
                    after,
                    ended,
                } => {
                    let query = looski::projects::CardQuery {
                        limit: Some(limit),
                        after,
                        ended: Some(ended),
                    };
                    serde_json::to_value(
                        client.project_cards(workspace, project_id, &query).await?,
                    )?
                }
                ProjectCommand::AddProjectCard {
                    project_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .add_project_card(workspace, project_id, &idempotency_key, &input)
                            .await?,
                    )?
                }
                ProjectCommand::GetProjectCard {
                    project_id,
                    card_id,
                } => serde_json::to_value(
                    client.project_card(workspace, project_id, card_id).await?,
                )?,
                ProjectCommand::MoveProjectCard {
                    project_id,
                    card_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .move_project_card(
                                workspace,
                                project_id,
                                card_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::ScheduleProjectCard {
                    project_id,
                    card_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .schedule_project_card(
                                workspace,
                                project_id,
                                card_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::RemoveProjectCard {
                    project_id,
                    card_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .remove_project_card(
                                workspace,
                                project_id,
                                card_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::ListProjectComments {
                    project_id,
                    card_id,
                    limit,
                    after,
                } => {
                    let query = looski::projects::PageQuery {
                        limit: Some(limit),
                        after,
                    };
                    serde_json::to_value(
                        client
                            .project_comments(workspace, project_id, card_id, &query)
                            .await?,
                    )?
                }
                ProjectCommand::AddProjectComment {
                    project_id,
                    card_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .add_project_comment(
                                workspace,
                                project_id,
                                card_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::EditProjectComment {
                    project_id,
                    comment_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .edit_project_comment(
                                workspace,
                                project_id,
                                comment_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
                ProjectCommand::RemoveProjectComment {
                    project_id,
                    comment_id,
                    idempotency_key,
                } => {
                    let input = serde_json::from_reader(std::io::stdin())?;
                    serde_json::to_value(
                        client
                            .remove_project_comment(
                                workspace,
                                project_id,
                                comment_id,
                                &idempotency_key,
                                &input,
                            )
                            .await?,
                    )?
                }
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Time {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            use looski::time::*;
            fn body<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
                Ok(serde_json::from_reader(std::io::stdin())?)
            }
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                TimeCommand::List {
                    after,
                    limit,
                    archived,
                    project_id,
                    from_on,
                    through_on,
                    billable,
                } => serde_json::to_value(
                    client
                        .time_list(
                            workspace,
                            &TimeQuery {
                                after,
                                limit,
                                archived: Some(archived),
                                project_id,
                                from_on,
                                through_on,
                                billable,
                            },
                        )
                        .await?,
                )?,
                TimeCommand::Create { idempotency_key } => serde_json::to_value(
                    client
                        .time_create(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                TimeCommand::Get { id } => {
                    serde_json::to_value(client.time_get(workspace, id).await?)?
                }
                TimeCommand::Update {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .time_update(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                TimeCommand::Transition {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .time_transition(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                TimeCommand::Report {
                    after,
                    limit,
                    archived,
                    project_id,
                    from_on,
                    through_on,
                    billable,
                } => serde_json::to_value(
                    client
                        .time_report(
                            workspace,
                            &TimeQuery {
                                after,
                                limit,
                                archived: Some(archived),
                                project_id,
                                from_on,
                                through_on,
                                billable,
                            },
                        )
                        .await?,
                )?,
                TimeCommand::Budgets {
                    after,
                    limit,
                    archived,
                } => serde_json::to_value(
                    client
                        .time_budgets(
                            workspace,
                            &TimeQuery {
                                after,
                                limit,
                                archived: Some(archived),
                                ..Default::default()
                            },
                        )
                        .await?,
                )?,
                TimeCommand::CreateBudget { idempotency_key } => serde_json::to_value(
                    client
                        .time_create_budget(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                TimeCommand::UpdateBudget {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .time_update_budget(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Scheduling {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            use looski::scheduling::*;
            fn body<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
                Ok(serde_json::from_reader(std::io::stdin())?)
            }
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                SchedulingCommand::Types {
                    after,
                    limit,
                    archived,
                } => serde_json::to_value(
                    client
                        .scheduling_types(
                            workspace,
                            &SchedulingQuery {
                                after,
                                limit,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                SchedulingCommand::CreateType { idempotency_key } => serde_json::to_value(
                    client
                        .scheduling_create_type(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                SchedulingCommand::GetType { id } => {
                    serde_json::to_value(client.scheduling_get_type(workspace, id).await?)?
                }
                SchedulingCommand::UpdateType {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .scheduling_update_type(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                SchedulingCommand::List {
                    after,
                    limit,
                    archived,
                } => serde_json::to_value(
                    client
                        .scheduling_list(
                            workspace,
                            &SchedulingQuery {
                                after,
                                limit,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                SchedulingCommand::Get { id } => {
                    serde_json::to_value(client.scheduling_get(workspace, id).await?)?
                }
                SchedulingCommand::Change {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .scheduling_change(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                SchedulingCommand::Retry {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .scheduling_retry(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                SchedulingCommand::Events {
                    id,
                    after,
                    limit,
                    archived,
                } => serde_json::to_value(
                    client
                        .scheduling_events(
                            workspace,
                            id,
                            &SchedulingQuery {
                                after,
                                limit,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Helpdesk {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            use looski::helpdesk::*;
            fn body<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
                Ok(serde_json::from_reader(std::io::stdin())?)
            }
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                HelpdeskCommand::Settings => {
                    serde_json::to_value(client.helpdesk_settings(workspace).await?)?
                }
                HelpdeskCommand::Directory => {
                    serde_json::to_value(client.helpdesk_directory(workspace).await?)?
                }
                HelpdeskCommand::Configure { idempotency_key } => serde_json::to_value(
                    client
                        .helpdesk_configure(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                HelpdeskCommand::List {
                    after,
                    limit,
                    archived,
                    status,
                    contact_id,
                } => serde_json::to_value(
                    client
                        .helpdesk_list(
                            workspace,
                            &TicketQuery {
                                after,
                                limit,
                                archived: Some(archived),
                                status,
                                contact_id,
                            },
                        )
                        .await?,
                )?,
                HelpdeskCommand::Get { id } => {
                    serde_json::to_value(client.helpdesk_get(workspace, id).await?)?
                }
                HelpdeskCommand::Messages { id, after, limit } => serde_json::to_value(
                    client
                        .helpdesk_messages(workspace, id, &MessageQuery { after, limit })
                        .await?,
                )?,
                HelpdeskCommand::Update {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .helpdesk_update(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                HelpdeskCommand::Link {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .helpdesk_link(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                HelpdeskCommand::Reply {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .helpdesk_reply(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Knowledge {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            use looski::knowledge::*;
            fn body<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
                Ok(serde_json::from_reader(std::io::stdin())?)
            }
            let kind = |sop: bool| sop.then_some(KnowledgeDocumentKind::Sop);
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                KnowledgeCommand::CollectionList {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .knowledge_list_collections(
                            workspace,
                            &ListKnowledgeCollections {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::CollectionGet { id } => {
                    serde_json::to_value(client.knowledge_get_collection(workspace, id).await?)?
                }
                KnowledgeCommand::CollectionCreate { idempotency_key } => serde_json::to_value(
                    client
                        .knowledge_create_collection(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                KnowledgeCommand::CollectionUpdate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .knowledge_update_collection(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                KnowledgeCommand::CollectionArchive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .knowledge_archive_collection(
                            workspace,
                            id,
                            &idempotency_key,
                            &ArchiveKnowledgeRecord {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::MemberList { collection } => serde_json::to_value(
                    client.knowledge_list_members(workspace, collection).await?,
                )?,
                KnowledgeCommand::MemberAdd {
                    collection,
                    membership,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .knowledge_add_member(
                            workspace,
                            collection,
                            &idempotency_key,
                            &AddKnowledgeMember {
                                membership_id: membership,
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::MemberRemove {
                    collection,
                    membership,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .knowledge_remove_member(
                            workspace,
                            collection,
                            membership,
                            &idempotency_key,
                        )
                        .await?,
                )?,
                KnowledgeCommand::Directory => {
                    serde_json::to_value(client.knowledge_directory(workspace).await?)?
                }
                KnowledgeCommand::DocumentList {
                    limit,
                    after,
                    collection,
                    sop,
                    archived,
                } => serde_json::to_value(
                    client
                        .knowledge_list_documents(
                            workspace,
                            &ListKnowledgeDocuments {
                                limit: Some(limit),
                                after,
                                collection_id: collection,
                                kind: kind(sop),
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::DocumentGet { id } => {
                    serde_json::to_value(client.knowledge_get_document(workspace, id).await?)?
                }
                KnowledgeCommand::DocumentCreate { idempotency_key } => serde_json::to_value(
                    client
                        .knowledge_create_document(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                KnowledgeCommand::DocumentUpdate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .knowledge_update_document(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                KnowledgeCommand::DocumentArchive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .knowledge_archive_document(
                            workspace,
                            id,
                            &idempotency_key,
                            &ArchiveKnowledgeRecord {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::DocumentMove {
                    id,
                    collection,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .knowledge_move_document(
                            workspace,
                            id,
                            &idempotency_key,
                            &MoveKnowledgeDocument {
                                expected_version,
                                collection_id: collection,
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::Revisions { id, limit, after } => serde_json::to_value(
                    client
                        .knowledge_list_revisions(
                            workspace,
                            id,
                            &KnowledgePageQuery {
                                limit: Some(limit),
                                after,
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::Revision { id, revision } => serde_json::to_value(
                    client
                        .knowledge_get_revision(workspace, id, revision)
                        .await?,
                )?,
                KnowledgeCommand::Restore {
                    id,
                    revision,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .knowledge_restore_revision(
                            workspace,
                            id,
                            revision,
                            &idempotency_key,
                            &RestoreKnowledgeRevision { expected_version },
                        )
                        .await?,
                )?,
                KnowledgeCommand::Search {
                    q,
                    limit,
                    after,
                    collection,
                    sop,
                } => serde_json::to_value(
                    client
                        .knowledge_search(
                            workspace,
                            &KnowledgeSearchQuery {
                                q,
                                limit: Some(limit),
                                after,
                                collection_id: collection,
                                kind: kind(sop),
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::Suggest { q, limit } => serde_json::to_value(
                    client
                        .knowledge_suggest(
                            workspace,
                            &KnowledgeSuggestQuery {
                                q,
                                limit: Some(limit),
                            },
                        )
                        .await?,
                )?,
                KnowledgeCommand::Activity { limit, after } => serde_json::to_value(
                    client
                        .knowledge_activity(
                            workspace,
                            &KnowledgePageQuery {
                                limit: Some(limit),
                                after,
                            },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Cms {
            api_url,
            token_file,
            workspace,
            action,
        } => {
            use looski::cms::{ArchiveContent, ExpectedVersion};
            fn body<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
                Ok(serde_json::from_reader(std::io::stdin())?)
            }
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            let result = match action {
                CmsCommand::TypeList {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .cms_types(
                            workspace,
                            &looski::cms::ListContentTypes {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                CmsCommand::TypeGet { id } => {
                    serde_json::to_value(client.cms_type(workspace, id).await?)?
                }
                CmsCommand::TypeCreate { idempotency_key } => serde_json::to_value(
                    client
                        .create_cms_type(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                CmsCommand::TypeUpdate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .update_cms_type(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                CmsCommand::TypeArchive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .archive_cms_type(
                            workspace,
                            id,
                            &idempotency_key,
                            &ArchiveContent {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
                CmsCommand::EntryList {
                    limit,
                    after,
                    search,
                    archived,
                    type_id,
                } => serde_json::to_value(
                    client
                        .cms_entries(
                            workspace,
                            &looski::cms::ListEntries {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                                type_id,
                            },
                        )
                        .await?,
                )?,
                CmsCommand::EntryGet { id } => {
                    serde_json::to_value(client.cms_entry(workspace, id).await?)?
                }
                CmsCommand::EntryCreate { idempotency_key } => serde_json::to_value(
                    client
                        .create_cms_entry(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                CmsCommand::EntryUpdate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .update_cms_entry(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                CmsCommand::Publish {
                    id,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .publish_cms_entry(
                            workspace,
                            id,
                            &idempotency_key,
                            &ExpectedVersion { expected_version },
                        )
                        .await?,
                )?,
                CmsCommand::Unpublish {
                    id,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .unpublish_cms_entry(
                            workspace,
                            id,
                            &idempotency_key,
                            &ExpectedVersion { expected_version },
                        )
                        .await?,
                )?,
                CmsCommand::EntryArchive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .archive_cms_entry(
                            workspace,
                            id,
                            &idempotency_key,
                            &ArchiveContent {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
                CmsCommand::Preview { id } => {
                    serde_json::to_value(client.preview_cms_entry(workspace, id).await?)?
                }
                CmsCommand::Revisions { id, limit, after } => serde_json::to_value(
                    client
                        .cms_revisions(
                            workspace,
                            id,
                            &looski::cms::RevisionQuery {
                                limit: Some(limit),
                                after,
                            },
                        )
                        .await?,
                )?,
                CmsCommand::Revision { id, revision } => {
                    serde_json::to_value(client.cms_revision(workspace, id, revision).await?)?
                }
                CmsCommand::Restore {
                    id,
                    revision,
                    idempotency_key,
                    expected_version,
                } => serde_json::to_value(
                    client
                        .restore_cms_revision(
                            workspace,
                            id,
                            revision,
                            &idempotency_key,
                            &ExpectedVersion { expected_version },
                        )
                        .await?,
                )?,
                CmsCommand::MediaList {
                    limit,
                    after,
                    search,
                    archived,
                } => serde_json::to_value(
                    client
                        .cms_media_page(
                            workspace,
                            &looski::cms::ListMedia {
                                limit: Some(limit),
                                after,
                                search,
                                archived: Some(archived),
                            },
                        )
                        .await?,
                )?,
                CmsCommand::MediaGet { id } => {
                    serde_json::to_value(client.cms_media(workspace, id).await?)?
                }
                CmsCommand::MediaContent { id } => {
                    serde_json::to_value(client.cms_media_content(workspace, id).await?)?
                }
                CmsCommand::MediaRegister { idempotency_key } => serde_json::to_value(
                    client
                        .register_cms_media(workspace, &idempotency_key, &body()?)
                        .await?,
                )?,
                CmsCommand::MediaUpdate {
                    id,
                    idempotency_key,
                } => serde_json::to_value(
                    client
                        .update_cms_media(workspace, id, &idempotency_key, &body()?)
                        .await?,
                )?,
                CmsCommand::MediaArchive {
                    id,
                    idempotency_key,
                    expected_version,
                    restore,
                } => serde_json::to_value(
                    client
                        .archive_cms_media(
                            workspace,
                            id,
                            &idempotency_key,
                            &ArchiveContent {
                                expected_version,
                                archived: !restore,
                            },
                        )
                        .await?,
                )?,
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::PublicContent {
            api_url,
            public_key,
            action,
        } => {
            let client = ApiClient::new(&api_url)?;
            let result = match action {
                PublicContentCommand::Index { after } => serde_json::to_value(
                    client
                        .public_content_index(&public_key, &looski::cms::PublicIndexQuery { after })
                        .await?,
                )?,
                PublicContentCommand::Entry { slug } => {
                    serde_json::to_value(client.public_content_entry(&public_key, &slug).await?)?
                }
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::PublicForm {
            api_url,
            public_key,
            action,
        } => {
            let client = ApiClient::new(&api_url)?;
            let result = match action {
                PublicFormCommand::Get => {
                    serde_json::to_value(client.public_form(&public_key).await?)?
                }
                PublicFormCommand::Submit => serde_json::to_value(
                    client
                        .submit_public_form(
                            &public_key,
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
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
            let client = ApiClient::new(&api_url)?.with_credentials(token_file)?;
            McpServer::new(client)
                .serve(rmcp::transport::stdio())
                .await?
                .waiting()
                .await?;
        }
        Command::WorkerHealth => {
            let modified = std::fs::metadata("/tmp/looski-worker-heartbeat")?.modified()?;
            anyhow::ensure!(
                modified.elapsed()?.as_secs() < 90,
                "Worker heartbeat is stale"
            );
        }
        Command::Worker { once } => {
            let database_url = std::fs::read_to_string(std::env::var("LOOSKI_DATABASE_URL_FILE")?)?;
            let db = PgPoolOptions::new()
                .max_connections(2)
                .acquire_timeout(Duration::from_secs(3))
                .after_connect(|db, _| {
                    Box::pin(async move {
                        sqlx::query("SET statement_timeout='30s'")
                            .execute(&mut *db)
                            .await?;
                        sqlx::query("SET lock_timeout='10s'").execute(db).await?;
                        Ok(())
                    })
                })
                .connect(database_url.trim())
                .await?;
            let schema = sqlx::query_scalar::<_, i32>(
                "SELECT version FROM suite.schema_version WHERE singleton=true",
            )
            .fetch_one(&db)
            .await?;
            anyhow::ensure!(
                schema == 36,
                "Worker requires schema 36 but the database is at {schema}; apply migrations first"
            );
            let invitation_key =
                std::fs::read_to_string(std::env::var("LOOSKI_INVITATION_KEY_FILE")?)?;
            let mailer = looski::workspace_access::invitations::delivery::Mailer::new(
                std::env::var("LOOSKI_AUTH_MAIL_ENABLED").as_deref() == Ok("true"),
                std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
                std::env::var("LOOSKI_PUBLIC_URL")?,
                std::env::var("LOOSKI_AUTH_ISSUER")?,
                read_jwt_secret()?.as_bytes(),
                invitation_key.trim().as_bytes(),
            )?;
            let mail_enabled = std::env::var("LOOSKI_AUTH_MAIL_ENABLED").as_deref() == Ok("true");
            let smtp = match std::env::var("LOOSKI_SMTP_CONFIG_FILE") {
                Ok(path) if mail_enabled => Some(std::fs::read_to_string(path)?),
                _ => None,
            };
            let outbox = looski::mail::Outbox::new(
                smtp.as_deref(),
                std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
                std::env::var("LOOSKI_PUBLIC_URL")?,
                std::env::var("LOOSKI_AUTH_ISSUER")?,
                read_jwt_secret()?.as_bytes(),
            )?;
            let diagnostics = looski::diagnostics::Diagnostics::from_environment();
            loop {
                let started = std::time::Instant::now();
                let result = looski::jobs::run_once(&db, &mailer, &outbox).await;
                diagnostics.worker(
                    matches!(result, Ok(true)),
                    result.is_err(),
                    started.elapsed(),
                );
                if once {
                    anyhow::ensure!(result.is_ok(), "Worker could not process notification");
                    println!("{{\"processed\":{}}}", result.unwrap());
                    break;
                }
                if result.is_ok() {
                    std::fs::write("/tmp/looski-worker-heartbeat", b"ok")?;
                } else {
                    tracing::warn!("Worker database unavailable; retrying");
                }
                tokio::select! {_=shutdown()=>break,_=tokio::time::sleep(Duration::from_secs(1))=>{}}
            }
        }
        Command::Api { bind } => {
            let database_url = std::fs::read_to_string(std::env::var("LOOSKI_DATABASE_URL_FILE")?)?;
            let invitation_key =
                std::fs::read_to_string(std::env::var("LOOSKI_INVITATION_KEY_FILE")?)?;
            anyhow::ensure!(
                invitation_key.trim().len() >= 32,
                "Invitation key is unavailable"
            );
            let state = AppState {
                auth_mail_enabled: std::env::var("LOOSKI_AUTH_MAIL_ENABLED").as_deref()
                    == Ok("true"),
                invitation_tokens: looski::workspace_access::invitations::TokenIssuer::new(
                    invitation_key.trim().as_bytes(),
                ),
                db: PgPoolOptions::new()
                    .max_connections(5)
                    .acquire_timeout(Duration::from_secs(2))
                    .connect_lazy(database_url.trim())?,
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(2))
                    .build()?,
                auth_url: std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
                public_url: std::env::var("LOOSKI_PUBLIC_URL")?,
                verifier: looski::identity::Verifier::new(
                    read_jwt_secret()?.as_bytes(),
                    std::env::var("LOOSKI_AUTH_ISSUER")?,
                ),
                cli_login: looski::cli_login::Service::new(
                    std::env::var("LOOSKI_AUTH_INTERNAL_URL")?,
                    std::env::var("LOOSKI_AUTH_ISSUER")?,
                    read_jwt_secret()?.as_bytes(),
                )?,
                public: looski::cli_login::Throttle::default(),
            };
            let listener = tokio::net::TcpListener::bind(bind).await?;
            tracing::info!(%bind, "API listening");
            axum::serve(
                listener,
                api::router_with_diagnostics(
                    state,
                    looski::diagnostics::Diagnostics::from_environment(),
                ),
            )
            .with_graceful_shutdown(shutdown())
            .await?;
        }
        Command::SetupServer { bind } => looski::setup::serve(bind).await?,
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
