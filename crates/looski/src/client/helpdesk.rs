// SPDX-License-Identifier: AGPL-3.0-only
use super::ApiClient;
use crate::helpdesk::*;
use reqwest::Method;
use uuid::Uuid;
impl ApiClient {
    pub async fn helpdesk_settings(&self, workspace: Uuid) -> anyhow::Result<HelpdeskSettings> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/helpdesk/settings"),
        )?)
        .await
    }
    pub async fn helpdesk_configure(
        &self,
        workspace: Uuid,
        key: &str,
        input: &ConfigureHelpdesk,
    ) -> anyhow::Result<HelpdeskSettings> {
        self.business(
            self.authenticated(
                Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/helpdesk/settings"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn helpdesk_directory(&self, workspace: Uuid) -> anyhow::Result<Vec<HelpdeskMember>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/helpdesk/directory"),
        )?)
        .await
    }
    pub async fn helpdesk_list(
        &self,
        workspace: Uuid,
        query: &TicketQuery,
    ) -> anyhow::Result<TicketPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/helpdesk/tickets"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn helpdesk_get(&self, workspace: Uuid, ticket: Uuid) -> anyhow::Result<Ticket> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/helpdesk/tickets/{ticket}"),
        )?)
        .await
    }
    pub async fn helpdesk_update(
        &self,
        workspace: Uuid,
        ticket: Uuid,
        key: &str,
        input: &UpdateTicket,
    ) -> anyhow::Result<Ticket> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/helpdesk/tickets/{ticket}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn helpdesk_link(
        &self,
        workspace: Uuid,
        ticket: Uuid,
        key: &str,
        input: &LinkTicket,
    ) -> anyhow::Result<Ticket> {
        self.business(
            self.authenticated(
                Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/helpdesk/tickets/{ticket}/links"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn helpdesk_messages(
        &self,
        workspace: Uuid,
        ticket: Uuid,
        query: &MessageQuery,
    ) -> anyhow::Result<TicketMessages> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/helpdesk/tickets/{ticket}/messages"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn helpdesk_reply(
        &self,
        workspace: Uuid,
        ticket: Uuid,
        key: &str,
        input: &ReplyTicket,
    ) -> anyhow::Result<TicketMessage> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/helpdesk/tickets/{ticket}/messages"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
}
