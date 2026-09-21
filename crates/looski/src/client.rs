// SPDX-License-Identifier: AGPL-3.0-only
use crate::api::SystemResponse;

/// CLI and MCP use the HTTP boundary, never database credentials.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base_url: reqwest::Url,
    token_file: Option<std::path::PathBuf>,
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
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(5))
                .build()?,
            base_url,
            token_file: None,
        })
    }

    pub fn with_token_file(mut self, file: std::path::PathBuf) -> Self {
        self.token_file = Some(file);
        self
    }

    pub async fn workspaces(&self) -> anyhow::Result<Vec<crate::workspaces::Workspace>> {
        let host = self.base_url.host_str().unwrap_or_default();
        let loopback = host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        anyhow::ensure!(
            self.base_url.scheme() == "https" || loopback,
            "Authenticated API calls require HTTPS except on localhost/loopback"
        );
        let file = self.token_file.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Set LOOSKI_ACCESS_TOKEN_FILE to a private session access-token file")
        })?;
        let token = std::fs::read_to_string(file)?;
        Ok(self
            .http
            .get(self.base_url.join("/api/v1/workspaces")?)
            .bearer_auth(token.trim())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn authenticated_client_rejects_remote_http_before_reading_credentials() {
        let error = ApiClient::new("http://business.example")
            .unwrap()
            .with_token_file("/nonexistent/private-token".into())
            .workspaces()
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("require HTTPS"));
    }
}
