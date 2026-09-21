// SPDX-License-Identifier: AGPL-3.0-only
//! Workspace operation: current authorization and result use one DB snapshot.
mod dao;
use crate::identity::Identity;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}

pub async fn list(pool: &PgPool, identity: &Identity) -> Result<Vec<Workspace>, sqlx::Error> {
    dao::visible_to(pool, identity).await
}
