// SPDX-License-Identifier: AGPL-3.0-only
//! Transport-independent application operations. Business modules will own their
//! operations and SQLx DAOs; do not put SQL or permission checks in handlers.

/// Public installation metadata, containing no user or business information.
pub struct SystemInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub stage: &'static str,
}

pub fn system_info() -> SystemInfo {
    SystemInfo {
        name: "Looski Suite",
        version: env!("CARGO_PKG_VERSION"),
        stage: "scaffold",
    }
}

/// Server-established operation context; the caller cannot construct an Identity.
pub struct ActorContext<'a> {
    pub identity: &'a crate::identity::Identity,
    pub workspace: uuid::Uuid,
    pub request_id: uuid::Uuid,
    /// Adapter attribution only; it cannot grant authority.
    pub source_interface: &'static str,
}

#[derive(Debug)]
pub enum OperationError {
    Invalid(&'static str),
    Forbidden,
    NotFound,
    Conflict(&'static str),
    Database(sqlx::Error),
}
impl From<sqlx::Error> for OperationError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}
