// SPDX-License-Identifier: AGPL-3.0-only
//! Projects tools. Each calls the HTTP API with the configured user's session.
use super::{McpServer, business_result};
use crate::projects;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectsArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: projects::ListProjects,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateProjectArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: projects::SaveProject,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetMyProjectWorkArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: projects::MyWorkQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectDirectoryArgs {
    workspace_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectRecipesArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: projects::recipes::DealConversionQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateProjectRecipeArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    change: projects::recipes::SaveDealRecipe,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetProjectRecipeArgs {
    workspace_id: Uuid,
    recipe_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateProjectRecipeArgs {
    workspace_id: Uuid,
    recipe_id: Uuid,
    idempotency_key: String,
    change: projects::recipes::SaveDealRecipe,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct SetProjectRecipeActiveArgs {
    workspace_id: Uuid,
    recipe_id: Uuid,
    idempotency_key: String,
    change: projects::recipes::SetDealRecipeActive,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectRecipeRunsArgs {
    workspace_id: Uuid,
    recipe_id: Uuid,
    #[serde(flatten)]
    query: projects::recipes::DealConversionQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RetryProjectRecipeRunArgs {
    workspace_id: Uuid,
    recipe_id: Uuid,
    run_id: Uuid,
    idempotency_key: String,
    change: projects::recipes::RetryDealConversion,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetProjectArgs {
    workspace_id: Uuid,
    project_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateProjectArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    idempotency_key: String,
    change: projects::SaveProject,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ArchiveProjectArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    idempotency_key: String,
    change: projects::ArchiveProject,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectMembersArgs {
    workspace_id: Uuid,
    project_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct AddProjectMemberArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    idempotency_key: String,
    change: projects::AddMember,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RemoveProjectMemberArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    membership_id: Uuid,
    idempotency_key: String,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectColumnsArgs {
    workspace_id: Uuid,
    project_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateProjectColumnArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    idempotency_key: String,
    change: projects::SaveColumn,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateProjectColumnArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    column_id: Uuid,
    idempotency_key: String,
    change: projects::SaveColumn,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RemoveProjectColumnArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    column_id: Uuid,
    idempotency_key: String,
    change: projects::RemoveColumn,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetProjectBoardArgs {
    workspace_id: Uuid,
    project_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetProjectDeliveryArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    #[serde(flatten)]
    query: projects::DeliveryQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectCardsArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    #[serde(flatten)]
    query: projects::CardQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct AddProjectCardArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    idempotency_key: String,
    change: projects::AddCard,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetProjectCardArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    card_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MoveProjectCardArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    card_id: Uuid,
    idempotency_key: String,
    change: projects::MoveCard,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ScheduleProjectCardArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    card_id: Uuid,
    idempotency_key: String,
    change: projects::ScheduleCard,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RemoveProjectCardArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    card_id: Uuid,
    idempotency_key: String,
    change: projects::RemoveCard,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListProjectCommentsArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    card_id: Uuid,
    #[serde(flatten)]
    query: projects::PageQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct AddProjectCommentArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    card_id: Uuid,
    idempotency_key: String,
    change: projects::NewComment,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EditProjectCommentArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    comment_id: Uuid,
    idempotency_key: String,
    change: projects::EditComment,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RemoveProjectCommentArgs {
    workspace_id: Uuid,
    project_id: Uuid,
    comment_id: Uuid,
    idempotency_key: String,
    change: projects::RemoveComment,
}
#[tool_router(router = projects_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(
        description = "List projects this member can read; assigned scope lists those they belong to."
    )]
    async fn list_projects(
        &self,
        Parameters(a): Parameters<ListProjectsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.list_projects(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Create a project with a default board. Works with CRM disabled; the creator becomes a member."
    )]
    async fn create_project(
        &self,
        Parameters(a): Parameters<CreateProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_project(a.workspace_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "Tasks assigned to the caller across projects they may read.")]
    async fn get_my_project_work(
        &self,
        Parameters(a): Parameters<GetMyProjectWorkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.my_project_work(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Active workspace members who can be added to a project. Requires projects.members.manage."
    )]
    async fn list_project_directory(
        &self,
        Parameters(a): Parameters<ListProjectDirectoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.project_directory(a.workspace_id).await)
    }
    #[tool(description = "List deal-to-project recipes.")]
    async fn list_project_recipes(
        &self,
        Parameters(a): Parameters<ListProjectRecipesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.project_recipes(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Create a deal-to-project recipe naming the deal status that marks the milestone. It starts inactive."
    )]
    async fn create_project_recipe(
        &self,
        Parameters(a): Parameters<CreateProjectRecipeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_project_recipe(a.workspace_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "Get a deal-to-project recipe with its current revision.")]
    async fn get_project_recipe(
        &self,
        Parameters(a): Parameters<GetProjectRecipeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_recipe(a.workspace_id, a.recipe_id)
                .await,
        )
    }
    #[tool(
        description = "Save a new immutable revision of a recipe; existing runs keep theirs. Requires expected_version."
    )]
    async fn update_project_recipe(
        &self,
        Parameters(a): Parameters<UpdateProjectRecipeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .update_project_recipe(a.workspace_id, a.recipe_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(
        description = "Activate or deactivate a recipe. Activation records the caller as the authority each run re-checks."
    )]
    async fn set_project_recipe_active(
        &self,
        Parameters(a): Parameters<SetProjectRecipeActiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_project_recipe_active(
                    a.workspace_id,
                    a.recipe_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "List a recipe's conversions with their job state and failure reason.")]
    async fn list_project_recipe_runs(
        &self,
        Parameters(a): Parameters<ListProjectRecipeRunsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_recipe_runs(a.workspace_id, a.recipe_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Resume a blocked or exhausted conversion. It stays the same conversion and cannot create a second project."
    )]
    async fn retry_project_recipe_run(
        &self,
        Parameters(a): Parameters<RetryProjectRecipeRunArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .retry_project_recipe_run(
                    a.workspace_id,
                    a.recipe_id,
                    a.run_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "Get one project.")]
    async fn get_project(
        &self,
        Parameters(a): Parameters<GetProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.project(a.workspace_id, a.project_id).await)
    }
    #[tool(
        description = "Edit a project's name, description, status or deadline. Requires expected_version."
    )]
    async fn update_project(
        &self,
        Parameters(a): Parameters<UpdateProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .update_project(a.workspace_id, a.project_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "Archive or restore a project. Its tasks stay in shared Tasks.")]
    async fn archive_project(
        &self,
        Parameters(a): Parameters<ArchiveProjectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_project(a.workspace_id, a.project_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "List a project's members.")]
    async fn list_project_members(
        &self,
        Parameters(a): Parameters<ListProjectMembersArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_members(a.workspace_id, a.project_id)
                .await,
        )
    }
    #[tool(description = "Add an active workspace member to a project.")]
    async fn add_project_member(
        &self,
        Parameters(a): Parameters<AddProjectMemberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .add_project_member(a.workspace_id, a.project_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "Remove a member; assigned-scope access to the project ends at once.")]
    async fn remove_project_member(
        &self,
        Parameters(a): Parameters<RemoveProjectMemberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .remove_project_member(
                    a.workspace_id,
                    a.project_id,
                    a.membership_id,
                    &a.idempotency_key,
                )
                .await,
        )
    }
    #[tool(description = "List a board's columns in order.")]
    async fn list_project_columns(
        &self,
        Parameters(a): Parameters<ListProjectColumnsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_columns(a.workspace_id, a.project_id)
                .await,
        )
    }
    #[tool(
        description = "Add a board column, optionally with the task status a move into it requests."
    )]
    async fn create_project_column(
        &self,
        Parameters(a): Parameters<CreateProjectColumnArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_project_column(a.workspace_id, a.project_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "Rename, remap or reorder a board column. Requires expected_version.")]
    async fn update_project_column(
        &self,
        Parameters(a): Parameters<UpdateProjectColumnArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .update_project_column(
                    a.workspace_id,
                    a.project_id,
                    a.column_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "Remove a column, moving its cards to another column unchanged.")]
    async fn remove_project_column(
        &self,
        Parameters(a): Parameters<RemoveProjectColumnArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .remove_project_column(
                    a.workspace_id,
                    a.project_id,
                    a.column_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "Read a project board: columns and the cards this member can see.")]
    async fn get_project_board(
        &self,
        Parameters(a): Parameters<GetProjectBoardArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_board(a.workspace_id, a.project_id)
                .await,
        )
    }
    #[tool(
        description = "Delivery view: progress, deadlines and workload over the tasks this member can see."
    )]
    async fn get_project_delivery(
        &self,
        Parameters(a): Parameters<GetProjectDeliveryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_delivery(a.workspace_id, a.project_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "List a project's visible cards; ended=true lists tasks removed from the board."
    )]
    async fn list_project_cards(
        &self,
        Parameters(a): Parameters<ListProjectCardsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_cards(a.workspace_id, a.project_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Put a task on the board: an existing shared task, or a new one created through Tasks."
    )]
    async fn add_project_card(
        &self,
        Parameters(a): Parameters<AddProjectCardArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .add_project_card(a.workspace_id, a.project_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "Get one visible card with its shared task.")]
    async fn get_project_card(
        &self,
        Parameters(a): Parameters<GetProjectCardArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_card(a.workspace_id, a.project_id, a.card_id)
                .await,
        )
    }
    #[tool(
        description = "Move a card to a column and place. Requires expected_version; a stale version conflicts rather than undoing another move."
    )]
    async fn move_project_card(
        &self,
        Parameters(a): Parameters<MoveProjectCardArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .move_project_card(
                    a.workspace_id,
                    a.project_id,
                    a.card_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Change a card's assignee, deadline and status through Tasks. Requires the task's expected version."
    )]
    async fn schedule_project_card(
        &self,
        Parameters(a): Parameters<ScheduleProjectCardArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .schedule_project_card(
                    a.workspace_id,
                    a.project_id,
                    a.card_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "Take a task off the board. The task and its comments are kept.")]
    async fn remove_project_card(
        &self,
        Parameters(a): Parameters<RemoveProjectCardArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .remove_project_card(
                    a.workspace_id,
                    a.project_id,
                    a.card_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "List comments on a visible card, oldest first.")]
    async fn list_project_comments(
        &self,
        Parameters(a): Parameters<ListProjectCommentsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .project_comments(a.workspace_id, a.project_id, a.card_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Comment on a card. Text is a Markdown subset; HTML stays literal and unsafe links are refused."
    )]
    async fn add_project_comment(
        &self,
        Parameters(a): Parameters<AddProjectCommentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .add_project_comment(
                    a.workspace_id,
                    a.project_id,
                    a.card_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(description = "Edit one's own comment. Requires expected_version.")]
    async fn edit_project_comment(
        &self,
        Parameters(a): Parameters<EditProjectCommentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .edit_project_comment(
                    a.workspace_id,
                    a.project_id,
                    a.comment_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
    #[tool(
        description = "Remove a comment, erasing its text. Authors remove their own; moderators remove anyone's."
    )]
    async fn remove_project_comment(
        &self,
        Parameters(a): Parameters<RemoveProjectCommentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .remove_project_comment(
                    a.workspace_id,
                    a.project_id,
                    a.comment_id,
                    &a.idempotency_key,
                    &a.change,
                )
                .await,
        )
    }
}
