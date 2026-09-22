// SPDX-License-Identifier: AGPL-3.0-only
use crate::api::SystemResponse;

#[derive(Debug, serde::Serialize)]
pub struct ApiFailure {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}
impl std::fmt::Display for ApiFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).map_err(|_| std::fmt::Error)?
        )
    }
}
impl std::error::Error for ApiFailure {}

/// CLI and MCP use the HTTP boundary, never database credentials.
#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    base_url: reqwest::Url,
    token_file: Option<std::path::PathBuf>,
    source_interface: &'static str,
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
            source_interface: "cli",
        })
    }

    pub fn for_mcp(mut self) -> Self {
        self.source_interface = "mcp";
        self
    }
    pub fn with_token_file(mut self, file: std::path::PathBuf) -> Self {
        self.token_file = Some(file);
        self
    }

    fn authenticated(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
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
            .request(method, self.base_url.join(path)?)
            .bearer_auth(token.trim())
            .header("x-looski-interface", self.source_interface))
    }

    pub async fn workspaces(&self) -> anyhow::Result<Vec<crate::workspaces::Workspace>> {
        Ok(self
            .authenticated(reqwest::Method::GET, "/api/v1/workspaces")?
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn contacts(
        &self,
        workspace: uuid::Uuid,
        query: &crate::contacts::ListContacts,
    ) -> anyhow::Result<crate::contacts::ContactPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/contacts"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn contact(
        &self,
        workspace: uuid::Uuid,
        contact: uuid::Uuid,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contacts/{contact}"),
        )?)
        .await
    }
    pub async fn create_contact(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::contacts::ContactInput,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contacts"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_contact(
        &self,
        workspace: uuid::Uuid,
        contact: uuid::Uuid,
        key: &str,
        input: &crate::contacts::EditContact,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/contacts/{contact}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_contact(
        &self,
        workspace: uuid::Uuid,
        contact: uuid::Uuid,
        key: &str,
        input: &crate::contacts::ArchiveContact,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contacts/{contact}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn merge_contacts(
        &self,
        workspace: uuid::Uuid,
        survivor: uuid::Uuid,
        key: &str,
        input: &crate::contacts::MergeContacts,
    ) -> anyhow::Result<crate::contacts::Contact> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/contacts/{survivor}/merge"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn organizations(
        &self,
        workspace: uuid::Uuid,
        query: &crate::contacts::ListContacts,
    ) -> anyhow::Result<crate::contacts::organizations::OrganizationPage> {
        self.business(
            self.authenticated(
                reqwest::Method::GET,
                &format!("/api/v1/workspaces/{workspace}/organizations"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/organizations/{id}"),
        )?)
        .await
    }
    pub async fn create_organization(
        &self,
        workspace: uuid::Uuid,
        key: &str,
        input: &crate::contacts::organizations::OrganizationInput,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/organizations"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn edit_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::organizations::EditOrganization,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(
            self.authenticated(
                reqwest::Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/organizations/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        key: &str,
        input: &crate::contacts::ArchiveContact,
    ) -> anyhow::Result<crate::contacts::organizations::Organization> {
        self.business(
            self.authenticated(
                reqwest::Method::POST,
                &format!("/api/v1/workspaces/{workspace}/organizations/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn contact_organizations(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
    ) -> anyhow::Result<Vec<crate::contacts::organizations::Relationship>> {
        self.business(self.authenticated(
            reqwest::Method::GET,
            &format!("/api/v1/workspaces/{workspace}/contacts/{id}/organizations"),
        )?)
        .await
    }
    pub async fn set_contact_organization(
        &self,
        workspace: uuid::Uuid,
        id: uuid::Uuid,
        organization: uuid::Uuid,
        key: &str,
        input: &crate::contacts::organizations::RelationshipInput,
    ) -> anyhow::Result<crate::contacts::organizations::Relationship> {
        self.business(
            self.authenticated(
                reqwest::Method::PUT,
                &format!(
                    "/api/v1/workspaces/{workspace}/contacts/{id}/organizations/{organization}"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    async fn business<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<T> {
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let request_id = response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let error = response.json::<crate::api::ApiError>().await?;
            return Err(ApiFailure {
                status,
                code: error.code,
                message: error.message,
                request_id,
            }
            .into());
        }
        Ok(response.json().await?)
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
