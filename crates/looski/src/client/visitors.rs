// SPDX-License-Identifier: AGPL-3.0-only
use super::ApiClient;
use crate::visitors::*;
use reqwest::Method;
use uuid::Uuid;
impl ApiClient {
    pub async fn visitors_sites(&self, workspace: Uuid) -> anyhow::Result<Vec<Site>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/visitors/sites"),
        )?)
        .await
    }
    pub async fn visitors_save_site(
        &self,
        workspace: Uuid,
        key: &str,
        input: &SaveSite,
    ) -> anyhow::Result<Site> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/visitors/sites"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn visitors_list(&self, workspace: Uuid, query: &Query) -> anyhow::Result<Page> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/visitors"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn visitors_get(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<Detail> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/visitors/{id}"),
        )?)
        .await
    }
    pub async fn visitors_events(
        &self,
        workspace: Uuid,
        id: Uuid,
        query: &EventsQuery,
    ) -> anyhow::Result<EventsPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/visitors/{id}/events"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn visitors_resolve(
        &self,
        workspace: Uuid,
        key: &str,
        input: &Resolve,
    ) -> anyhow::Result<Visitor> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/visitors/resolve"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn visitors_review(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &Review,
    ) -> anyhow::Result<Receipt> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/visitors/{id}/matches"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn visitors_link(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &Link,
    ) -> anyhow::Result<Visitor> {
        self.business(
            self.authenticated(
                Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/visitors/{id}/contact"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn visitors_delete(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
    ) -> anyhow::Result<Receipt> {
        self.business(
            self.authenticated(
                Method::DELETE,
                &format!("/api/v1/workspaces/{workspace}/visitors/{id}"),
            )?
            .header("Idempotency-Key", key),
        )
        .await
    }
}
