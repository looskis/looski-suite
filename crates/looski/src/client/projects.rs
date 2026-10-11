// SPDX-License-Identifier: AGPL-3.0-only
//! Projects calls over the canonical HTTP API.
use super::ApiClient;
use crate::projects;
use reqwest::Method;
use uuid::Uuid;

impl ApiClient {
    pub async fn list_projects(
        &self,
        workspace: Uuid,
        query: &projects::ListProjects,
    ) -> anyhow::Result<projects::ProjectPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/projects"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn create_project(
        &self,
        workspace: Uuid,
        key: &str,
        input: &projects::SaveProject,
    ) -> anyhow::Result<projects::Project> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn my_project_work(
        &self,
        workspace: Uuid,
        query: &projects::MyWorkQuery,
    ) -> anyhow::Result<projects::MyWork> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/projects/my-work"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn project_directory(
        &self,
        workspace: Uuid,
    ) -> anyhow::Result<Vec<projects::ProjectMember>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/directory"),
        )?)
        .await
    }
    pub async fn project_recipes(
        &self,
        workspace: Uuid,
        query: &projects::recipes::DealConversionQuery,
    ) -> anyhow::Result<projects::recipes::DealRecipePage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/projects/recipes"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn create_project_recipe(
        &self,
        workspace: Uuid,
        key: &str,
        input: &projects::recipes::SaveDealRecipe,
    ) -> anyhow::Result<projects::recipes::DealRecipe> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects/recipes"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn project_recipe(
        &self,
        workspace: Uuid,
        recipe_id: Uuid,
    ) -> anyhow::Result<projects::recipes::DealRecipe> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/recipes/{recipe_id}"),
        )?)
        .await
    }
    pub async fn update_project_recipe(
        &self,
        workspace: Uuid,
        recipe_id: Uuid,
        key: &str,
        input: &projects::recipes::SaveDealRecipe,
    ) -> anyhow::Result<projects::recipes::DealRecipe> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/projects/recipes/{recipe_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn set_project_recipe_active(
        &self,
        workspace: Uuid,
        recipe_id: Uuid,
        key: &str,
        input: &projects::recipes::SetDealRecipeActive,
    ) -> anyhow::Result<projects::recipes::DealRecipe> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects/recipes/{recipe_id}/active"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn project_recipe_runs(
        &self,
        workspace: Uuid,
        recipe_id: Uuid,
        query: &projects::recipes::DealConversionQuery,
    ) -> anyhow::Result<projects::recipes::DealConversionPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/projects/recipes/{recipe_id}/runs"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn retry_project_recipe_run(
        &self,
        workspace: Uuid,
        recipe_id: Uuid,
        run_id: Uuid,
        key: &str,
        input: &projects::recipes::RetryDealConversion,
    ) -> anyhow::Result<projects::recipes::DealConversion> {
        self.business(self.authenticated(Method::POST,&format!("/api/v1/workspaces/{workspace}/projects/recipes/{recipe_id}/runs/{run_id}/retry"))?.header("Idempotency-Key",key).json(input)).await
    }
    pub async fn project(
        &self,
        workspace: Uuid,
        project_id: Uuid,
    ) -> anyhow::Result<projects::Project> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/{project_id}"),
        )?)
        .await
    }
    pub async fn update_project(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        key: &str,
        input: &projects::SaveProject,
    ) -> anyhow::Result<projects::Project> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_project(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        key: &str,
        input: &projects::ArchiveProject,
    ) -> anyhow::Result<projects::Project> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn project_members(
        &self,
        workspace: Uuid,
        project_id: Uuid,
    ) -> anyhow::Result<Vec<projects::ProjectMember>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/members"),
        )?)
        .await
    }
    pub async fn add_project_member(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        key: &str,
        input: &projects::AddMember,
    ) -> anyhow::Result<projects::ProjectMember> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/members"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn remove_project_member(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        membership_id: Uuid,
        key: &str,
    ) -> anyhow::Result<projects::ProjectMember> {
        self.business(self.authenticated(Method::POST,&format!("/api/v1/workspaces/{workspace}/projects/{project_id}/members/{membership_id}/remove"))?.header("Idempotency-Key",key)).await
    }
    pub async fn project_columns(
        &self,
        workspace: Uuid,
        project_id: Uuid,
    ) -> anyhow::Result<Vec<projects::Column>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/columns"),
        )?)
        .await
    }
    pub async fn create_project_column(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        key: &str,
        input: &projects::SaveColumn,
    ) -> anyhow::Result<projects::Column> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/columns"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn update_project_column(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        column_id: Uuid,
        key: &str,
        input: &projects::SaveColumn,
    ) -> anyhow::Result<projects::Column> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/columns/{column_id}"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn remove_project_column(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        column_id: Uuid,
        key: &str,
        input: &projects::RemoveColumn,
    ) -> anyhow::Result<projects::Column> {
        self.business(self.authenticated(Method::POST,&format!("/api/v1/workspaces/{workspace}/projects/{project_id}/columns/{column_id}/remove"))?.header("Idempotency-Key",key).json(input)).await
    }
    pub async fn project_board(
        &self,
        workspace: Uuid,
        project_id: Uuid,
    ) -> anyhow::Result<projects::Board> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/board"),
        )?)
        .await
    }
    pub async fn project_delivery(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        query: &projects::DeliveryQuery,
    ) -> anyhow::Result<projects::Delivery> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/delivery"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn project_cards(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        query: &projects::CardQuery,
    ) -> anyhow::Result<projects::CardPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/cards"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn add_project_card(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        key: &str,
        input: &projects::AddCard,
    ) -> anyhow::Result<projects::Card> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/cards"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn project_card(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        card_id: Uuid,
    ) -> anyhow::Result<projects::Card> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/projects/{project_id}/cards/{card_id}"),
        )?)
        .await
    }
    pub async fn move_project_card(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        card_id: Uuid,
        key: &str,
        input: &projects::MoveCard,
    ) -> anyhow::Result<projects::Card> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/cards/{card_id}/move"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn schedule_project_card(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        card_id: Uuid,
        key: &str,
        input: &projects::ScheduleCard,
    ) -> anyhow::Result<projects::Card> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/cards/{card_id}/schedule"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn remove_project_card(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        card_id: Uuid,
        key: &str,
        input: &projects::RemoveCard,
    ) -> anyhow::Result<projects::Card> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/cards/{card_id}/remove"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn project_comments(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        card_id: Uuid,
        query: &projects::PageQuery,
    ) -> anyhow::Result<projects::CommentPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/cards/{card_id}/comments"
                ),
            )?
            .query(query),
        )
        .await
    }
    pub async fn add_project_comment(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        card_id: Uuid,
        key: &str,
        input: &projects::NewComment,
    ) -> anyhow::Result<projects::Comment> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/cards/{card_id}/comments"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_project_comment(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        comment_id: Uuid,
        key: &str,
        input: &projects::EditComment,
    ) -> anyhow::Result<projects::Comment> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!(
                    "/api/v1/workspaces/{workspace}/projects/{project_id}/comments/{comment_id}"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn remove_project_comment(
        &self,
        workspace: Uuid,
        project_id: Uuid,
        comment_id: Uuid,
        key: &str,
        input: &projects::RemoveComment,
    ) -> anyhow::Result<projects::Comment> {
        self.business(self.authenticated(Method::POST,&format!("/api/v1/workspaces/{workspace}/projects/{project_id}/comments/{comment_id}/remove"))?.header("Idempotency-Key",key).json(input)).await
    }
}
