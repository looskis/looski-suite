// SPDX-License-Identifier: AGPL-3.0-only
//! Forms tools. Each calls the HTTP API with the configured user's session.
use super::{McpServer, business_result};
use crate::forms::{
    ListForms, SaveForm, SetNotifications, SubmissionQuery,
    recipes::{ConfigureRecipe, RunQuery},
};
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
pub(super) struct ModuleSetArgs {
    workspace_id: Uuid,
    /// Application key, such as `crm` or `forms`.
    module: String,
    idempotency_key: String,
    change: crate::modules::SetModule,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ListForms,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormArgs {
    workspace_id: Uuid,
    form_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    form: SaveForm,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormUpdateArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    idempotency_key: String,
    form: SaveForm,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormVersionArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    idempotency_key: String,
    expected_version: i64,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormToggleArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    idempotency_key: String,
    expected_version: i64,
    enabled: bool,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormNotificationsArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    idempotency_key: String,
    change: SetNotifications,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormSubmissionsArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    #[serde(flatten)]
    query: SubmissionQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormSubmissionArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    submission_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormRecipeArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    idempotency_key: String,
    change: ConfigureRecipe,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormRunsArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    #[serde(flatten)]
    query: RunQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct FormRetryArgs {
    workspace_id: Uuid,
    form_id: Uuid,
    run_id: Uuid,
    idempotency_key: String,
    expected_version: i64,
}

#[tool_router(router = forms_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(
        description = "Enable or disable an optional application (crm or forms). Disabling blocks dependent work but retains records; requires expected_version."
    )]
    async fn set_module(
        &self,
        Parameters(a): Parameters<ModuleSetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_module(a.workspace_id, &a.module, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "List forms in the workspace. Forms must be enabled.")]
    async fn list_forms(
        &self,
        Parameters(a): Parameters<FormListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.forms(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Get a form with its editable draft, published version number, public key and acceptance state."
    )]
    async fn get_form(
        &self,
        Parameters(a): Parameters<FormArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.form(a.workspace_id, a.form_id).await)
    }
    #[tool(
        description = "Create a form draft. Fields use stable lowercase keys; kinds are statement, short_text, long_text, email, phone, number, date, choice, multi_choice and checkbox; show_if refers to an earlier choice or checkbox."
    )]
    async fn create_form(
        &self,
        Parameters(a): Parameters<FormCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_form(a.workspace_id, &a.idempotency_key, &a.form)
                .await,
        )
    }
    #[tool(
        description = "Edit a form's draft and details using expected_version. Published versions never change; publish to make the draft public."
    )]
    async fn update_form(
        &self,
        Parameters(a): Parameters<FormUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .edit_form(a.workspace_id, a.form_id, &a.idempotency_key, &a.form)
                .await,
        )
    }
    #[tool(
        description = "Publish the draft as a new immutable version and start accepting answers."
    )]
    async fn publish_form(
        &self,
        Parameters(a): Parameters<FormVersionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .publish_form(
                    a.workspace_id,
                    a.form_id,
                    &a.idempotency_key,
                    &crate::forms::PublishForm {
                        expected_version: a.expected_version,
                    },
                )
                .await,
        )
    }
    #[tool(description = "Open (enabled=true) or close a published form to public answers.")]
    async fn set_form_accepting(
        &self,
        Parameters(a): Parameters<FormToggleArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_form_accepting(
                    a.workspace_id,
                    a.form_id,
                    &a.idempotency_key,
                    &crate::forms::SetAccepting {
                        expected_version: a.expected_version,
                        accepting: a.enabled,
                    },
                )
                .await,
        )
    }
    #[tool(
        description = "Archive (enabled=true) or restore a form. Archiving closes it; submissions are retained."
    )]
    async fn archive_form(
        &self,
        Parameters(a): Parameters<FormToggleArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_form(
                    a.workspace_id,
                    a.form_id,
                    &a.idempotency_key,
                    &crate::forms::ArchiveForm {
                        expected_version: a.expected_version,
                        archived: a.enabled,
                    },
                )
                .await,
        )
    }
    #[tool(
        description = "Set members notified by email of new responses; each must be able to read submissions."
    )]
    async fn set_form_notifications(
        &self,
        Parameters(a): Parameters<FormNotificationsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_form_notifications(a.workspace_id, a.form_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(description = "List a form's published versions, newest first.")]
    async fn list_form_versions(
        &self,
        Parameters(a): Parameters<FormArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.form_versions(a.workspace_id, a.form_id).await)
    }
    #[tool(
        description = "List submissions newest first with original answers, version and recipe run state. Requires forms.submissions.read."
    )]
    async fn list_form_submissions(
        &self,
        Parameters(a): Parameters<FormSubmissionsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .form_submissions(a.workspace_id, a.form_id, &a.query)
                .await,
        )
    }
    #[tool(description = "Get one submission with its original answers.")]
    async fn get_form_submission(
        &self,
        Parameters(a): Parameters<FormSubmissionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .form_submission(a.workspace_id, a.form_id, a.submission_id)
                .await,
        )
    }
    #[tool(
        description = "Response counts for the published version's choice and checkbox questions."
    )]
    async fn get_form_summary(
        &self,
        Parameters(a): Parameters<FormArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.form_summary(a.workspace_id, a.form_id).await)
    }
    #[tool(
        description = "Get the form's CRM inquiry recipe, its current mapping revision and activation."
    )]
    async fn get_form_recipe(
        &self,
        Parameters(a): Parameters<FormArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.form_recipe(a.workspace_id, a.form_id).await)
    }
    #[tool(
        description = "Save a new CRM inquiry mapping revision: name/email/phone field keys, pipeline, stage, owner and title prefix. Runs keep the revision they started with."
    )]
    async fn configure_form_recipe(
        &self,
        Parameters(a): Parameters<FormRecipeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .configure_form_recipe(a.workspace_id, a.form_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(
        description = "Activate (enabled=true) or deactivate the CRM inquiry recipe. Activation records your authority for future runs and requires the contact and opportunity permissions they use."
    )]
    async fn set_form_recipe_active(
        &self,
        Parameters(a): Parameters<FormToggleArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .set_form_recipe_active(
                    a.workspace_id,
                    a.form_id,
                    &a.idempotency_key,
                    &crate::forms::recipes::SetRecipeActive {
                        expected_version: a.expected_version,
                        active: a.enabled,
                    },
                )
                .await,
        )
    }
    #[tool(description = "List CRM inquiry recipe runs newest first, with job state and failures.")]
    async fn list_form_recipe_runs(
        &self,
        Parameters(a): Parameters<FormRunsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .form_recipe_runs(a.workspace_id, a.form_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Retry a run awaiting contact review (after resolving its intake) or whose job failed. Completed runs never repeat."
    )]
    async fn retry_form_recipe_run(
        &self,
        Parameters(a): Parameters<FormRetryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .retry_form_recipe_run(
                    a.workspace_id,
                    a.form_id,
                    a.run_id,
                    &a.idempotency_key,
                    &crate::forms::recipes::RetryRun {
                        expected_version: a.expected_version,
                    },
                )
                .await,
        )
    }
}
