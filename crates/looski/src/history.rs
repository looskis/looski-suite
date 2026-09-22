// SPDX-License-Identifier: AGPL-3.0-only
//! Ordering shared by independently owned event streams.
use serde::Deserialize;
use uuid::Uuid;
#[derive(Clone, Deserialize)]
pub(crate) struct Position {
    pub recorded_at: String,
    pub id: Uuid,
    pub source: String,
}
impl Position {
    pub fn key(&self) -> (&str, &str, Uuid) {
        (&self.recorded_at, &self.source, self.id)
    }
}
