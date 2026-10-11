// SPDX-License-Identifier: AGPL-3.0-only
//! Forms and application-selection calls over the canonical HTTP API.
use super::{ApiClient, ApiFailure};
use crate::forms::{
    self,
    recipes::{ConfigureRecipe, Recipe, RetryRun, RunPage, RunQuery, RunSummary, SetRecipeActive},
};
use reqwest::Method;
use uuid::Uuid;

impl ApiClient {
    pub async fn set_module(
        &self,
        workspace: Uuid,
        module: &str,
        key: &str,
        input: &crate::modules::SetModule,
    ) -> anyhow::Result<crate::modules::Module> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/modules/{module}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn forms(
        &self,
        workspace: Uuid,
        query: &forms::ListForms,
    ) -> anyhow::Result<forms::FormPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/forms"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn form(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<forms::Form> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/forms/{id}"),
        )?)
        .await
    }
    pub async fn create_form(
        &self,
        workspace: Uuid,
        key: &str,
        input: &forms::SaveForm,
    ) -> anyhow::Result<forms::Form> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/forms"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_form(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &forms::SaveForm,
    ) -> anyhow::Result<forms::Form> {
        self.form_change(workspace, id, Method::PATCH, "", key, input)
            .await
    }
    pub async fn publish_form(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &forms::PublishForm,
    ) -> anyhow::Result<forms::Form> {
        self.form_change(workspace, id, Method::POST, "/publish", key, input)
            .await
    }
    pub async fn set_form_accepting(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &forms::SetAccepting,
    ) -> anyhow::Result<forms::Form> {
        self.form_change(workspace, id, Method::POST, "/accepting", key, input)
            .await
    }
    pub async fn archive_form(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &forms::ArchiveForm,
    ) -> anyhow::Result<forms::Form> {
        self.form_change(workspace, id, Method::POST, "/archive", key, input)
            .await
    }
    pub async fn set_form_notifications(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &forms::SetNotifications,
    ) -> anyhow::Result<forms::Form> {
        self.form_change(workspace, id, Method::PUT, "/notifications", key, input)
            .await
    }
    async fn form_change<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        workspace: Uuid,
        id: Uuid,
        method: Method,
        suffix: &str,
        key: &str,
        input: &T,
    ) -> anyhow::Result<R> {
        self.business(
            self.authenticated(
                method,
                &format!("/api/v1/workspaces/{workspace}/forms/{id}{suffix}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn form_versions(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<Vec<forms::FormVersion>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/forms/{id}/versions"),
        )?)
        .await
    }
    pub async fn form_submissions(
        &self,
        workspace: Uuid,
        id: Uuid,
        query: &forms::SubmissionQuery,
    ) -> anyhow::Result<forms::SubmissionPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/forms/{id}/submissions"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn form_submission(
        &self,
        workspace: Uuid,
        id: Uuid,
        submission: Uuid,
    ) -> anyhow::Result<forms::Submission> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/forms/{id}/submissions/{submission}"),
        )?)
        .await
    }
    pub async fn form_summary(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<forms::FormSummaryReport> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/forms/{id}/summary"),
        )?)
        .await
    }
    pub async fn form_recipe(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<Recipe> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/forms/{id}/recipe"),
        )?)
        .await
    }
    pub async fn configure_form_recipe(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &ConfigureRecipe,
    ) -> anyhow::Result<Recipe> {
        self.form_change(workspace, id, Method::PUT, "/recipe", key, input)
            .await
    }
    pub async fn set_form_recipe_active(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &SetRecipeActive,
    ) -> anyhow::Result<Recipe> {
        self.form_change(
            workspace,
            id,
            Method::POST,
            "/recipe/activation",
            key,
            input,
        )
        .await
    }
    pub async fn form_recipe_runs(
        &self,
        workspace: Uuid,
        id: Uuid,
        query: &RunQuery,
    ) -> anyhow::Result<RunPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/forms/{id}/recipe/runs"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn retry_form_recipe_run(
        &self,
        workspace: Uuid,
        id: Uuid,
        run: Uuid,
        key: &str,
        input: &RetryRun,
    ) -> anyhow::Result<RunSummary> {
        self.form_change(
            workspace,
            id,
            Method::POST,
            &format!("/recipe/runs/{run}/retry"),
            key,
            input,
        )
        .await
    }
    /// Public endpoints carry no credentials; the form key is the only right.
    pub(super) async fn anonymous<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<T> {
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let request_id = response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let error = response.json::<crate::api::ApiError>().await?;
            return Err(ApiFailure {
                status,
                code: error.code,
                message: error.message,
                request_id,
            }
            .into());
        }
        Ok(response.json().await?)
    }
    pub async fn public_form(&self, public_key: &str) -> anyhow::Result<forms::PublicForm> {
        self.anonymous(
            self.http.get(
                self.base_url
                    .join(&format!("/api/v1/public/forms/{public_key}"))?,
            ),
        )
        .await
    }
    pub async fn submit_public_form(
        &self,
        public_key: &str,
        input: &forms::PublicSubmission,
    ) -> anyhow::Result<forms::Receipt> {
        self.anonymous(
            self.http
                .post(
                    self.base_url
                        .join(&format!("/api/v1/public/forms/{public_key}/submissions"))?,
                )
                .json(input),
        )
        .await
    }
}
