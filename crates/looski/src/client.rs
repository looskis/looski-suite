// SPDX-License-Identifier: AGPL-3.0-only
use crate::api::SystemResponse;

/// CLI and MCP use the HTTP boundary, never database credentials.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base_url: reqwest::Url,
}

impl ApiClient {
    pub fn new(base_url: &str) -> anyhow::Result<Self> {
        let base_url = reqwest::Url::parse(base_url)?;
        anyhow::ensure!(
            matches!(base_url.scheme(), "http" | "https"),
            "API URL must use HTTP or HTTPS"
        );
        anyhow::ensure!(
            base_url.username().is_empty() && base_url.password().is_none(),
            "Do not put credentials in the API URL"
        );
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()?,
            base_url,
        })
    }

    pub async fn system(&self) -> anyhow::Result<SystemResponse> {
        Ok(self
            .http
            .get(self.base_url.join("/api/v1/system")?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
