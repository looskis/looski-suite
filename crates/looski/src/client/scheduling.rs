// SPDX-License-Identifier: AGPL-3.0-only
use super::ApiClient;
use crate::scheduling::*;
use reqwest::Method;
use uuid::Uuid;
impl ApiClient {
    pub async fn scheduling_types(
        &self,
        workspace: Uuid,
        query: &SchedulingQuery,
    ) -> anyhow::Result<AppointmentTypePage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/scheduling/types"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn scheduling_create_type(
        &self,
        workspace: Uuid,
        key: &str,
        input: &SaveAppointmentType,
    ) -> anyhow::Result<AppointmentType> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/scheduling/types"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn scheduling_get_type(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<AppointmentType> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/scheduling/types/{id}"),
        )?)
        .await
    }
    pub async fn scheduling_update_type(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &SaveAppointmentType,
    ) -> anyhow::Result<AppointmentType> {
        self.business(
            self.authenticated(
                Method::PUT,
                &format!("/api/v1/workspaces/{workspace}/scheduling/types/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn scheduling_list(
        &self,
        workspace: Uuid,
        query: &SchedulingQuery,
    ) -> anyhow::Result<BookingPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/scheduling/appointments"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn scheduling_get(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<Booking> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/scheduling/appointments/{id}"),
        )?)
        .await
    }
    pub async fn scheduling_change(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &ChangeBooking,
    ) -> anyhow::Result<Booking> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/scheduling/appointments/{id}/change"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn scheduling_retry(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &RetryBooking,
    ) -> anyhow::Result<Booking> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/scheduling/appointments/{id}/retry"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn scheduling_events(
        &self,
        workspace: Uuid,
        id: Uuid,
        query: &SchedulingQuery,
    ) -> anyhow::Result<BookingEventPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/scheduling/appointments/{id}/events"),
            )?
            .query(query),
        )
        .await
    }
}
