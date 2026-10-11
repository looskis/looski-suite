// SPDX-License-Identifier: AGPL-3.0-only
use super::ApiClient;
use crate::time::*;
use reqwest::Method;
use uuid::Uuid;
impl ApiClient {
    pub async fn time_list(
        &self,
        workspace: Uuid,
        query: &TimeQuery,
    ) -> anyhow::Result<TimeEntryPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/time/entries"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn time_create(
        &self,
        workspace: Uuid,
        key: &str,
        input: &SaveTimeEntry,
    ) -> anyhow::Result<TimeEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/time/entries"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn time_get(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<TimeEntry> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/time/entries/{id}"),
        )?)
        .await
    }
    pub async fn time_update(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &SaveTimeEntry,
    ) -> anyhow::Result<TimeEntry> {
        self.business(
            self.authenticated(
                Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/time/entries/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn time_transition(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &TimerTransition,
    ) -> anyhow::Result<TimeEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/time/entries/{id}/transition"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn time_report(
        &self,
        workspace: Uuid,
        query: &TimeQuery,
    ) -> anyhow::Result<TimeReport> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/time/report"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn time_budgets(
        &self,
        workspace: Uuid,
        query: &TimeQuery,
    ) -> anyhow::Result<TimeBudgetPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/time/budgets"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn time_create_budget(
        &self,
        workspace: Uuid,
        key: &str,
        input: &SaveTimeBudget,
    ) -> anyhow::Result<TimeBudget> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/time/budgets"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn time_update_budget(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &SaveTimeBudget,
    ) -> anyhow::Result<TimeBudget> {
        self.business(
            self.authenticated(
                Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/time/budgets/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
}
