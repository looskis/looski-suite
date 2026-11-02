// SPDX-License-Identifier: AGPL-3.0-only
use super::ApiClient;
use crate::communications::*;
use reqwest::Method;
use uuid::Uuid;
impl ApiClient {
    pub async fn communications_accounts(&self, workspace: Uuid) -> anyhow::Result<Vec<Account>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/communications/accounts"),
        )?)
        .await
    }
    pub async fn communications_connect(
        &self,
        workspace: Uuid,
        key: &str,
        input: &ConnectAccount,
    ) -> anyhow::Result<Account> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/communications/accounts"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn communications_update_account(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &UpdateAccount,
    ) -> anyhow::Result<Account> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/communications/accounts/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn communications_sync(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<SyncResult> {
        self.business(self.authenticated(
            Method::POST,
            &format!("/api/v1/workspaces/{workspace}/communications/accounts/{id}/sync"),
        )?)
        .await
    }
    pub async fn communications_list(
        &self,
        workspace: Uuid,
        query: &ConversationQuery,
    ) -> anyhow::Result<Conversations> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/communications/conversations"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn communications_get(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<Conversation> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/communications/conversations/{id}"),
        )?)
        .await
    }
    pub async fn communications_update(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &UpdateConversation,
    ) -> anyhow::Result<Conversation> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/communications/conversations/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn communications_messages(
        &self,
        workspace: Uuid,
        id: Uuid,
        query: &MessageQuery,
    ) -> anyhow::Result<Messages> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!(
                    "/api/v1/workspaces/{workspace}/communications/conversations/{id}/messages"
                ),
            )?
            .query(query),
        )
        .await
    }
    pub async fn communications_send(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &SendMessage,
    ) -> anyhow::Result<Message> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/communications/conversations/{id}/messages"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
}
