// SPDX-License-Identifier: AGPL-3.0-only
use super::{Error, adapter::Capabilities};
use serde::Deserialize;
use uuid::Uuid;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connector {
    pub key: String,
    pub workspace_id: Uuid,
    pub provider: String,
    pub installation_id: String,
    pub endpoint: String,
    pub api_token: String,
    pub webhook_secret: String,
    #[serde(default)]
    pub capabilities: Capabilities,
    /// Optional exact receiving-address allowlist; absent destination fails closed.
    #[serde(default)]
    pub destinations: Vec<String>,
}
pub fn load(key: &str, workspace: Uuid) -> Result<Connector, Error> {
    let path = std::env::var("LOOSKI_COMMUNICATIONS_CONFIG_FILE")
        .map_err(|_| Error::Conflict("Communications connectors are not configured"))?;
    let file = std::fs::read(path).map_err(|_| Error::Unavailable)?;
    let configs: Vec<Connector> = serde_json::from_slice(&file)
        .map_err(|_| Error::Conflict("Invalid communications configuration"))?;
    if configs.len() > 100 {
        return Err(Error::Conflict("Too many connectors"));
    }
    let mut found = configs
        .into_iter()
        .filter(|c| c.key == key && c.workspace_id == workspace);
    let config = found.next().ok_or(Error::NotFound)?;
    if found.next().is_some() {
        return Err(Error::Conflict("Duplicate connector key"));
    }
    validate(&config)?;
    Ok(config)
}
fn validate(c: &Connector) -> Result<(), Error> {
    let url =
        reqwest::Url::parse(&c.endpoint).map_err(|_| Error::Invalid("Invalid connector URL"))?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || c.endpoint.ends_with('/')
    {
        return Err(Error::Invalid(
            "Connector requires HTTPS or loopback HTTP, without URL credentials or trailing slash",
        ));
    }
    if !["blueski", "http_v1"].contains(&c.provider.as_str())
        || c.webhook_secret.len() < 32
        || c.installation_id.is_empty()
        || (!local && c.api_token.is_empty())
    {
        return Err(Error::Invalid("Invalid connector credentials or provider"));
    }
    if c.provider == "http_v1"
        && (c.capabilities.channels.is_empty()
            || c.capabilities.attachments
            || c.capabilities.calls)
    {
        return Err(Error::Invalid(
            "HTTP v1 currently supports text messaging channels only",
        ));
    }
    Ok(())
}
