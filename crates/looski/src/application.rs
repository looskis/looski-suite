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
