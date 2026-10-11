// SPDX-License-Identifier: AGPL-3.0-only
//! CMS calls over the canonical HTTP API.
use super::ApiClient;
use crate::cms;
use reqwest::Method;
use uuid::Uuid;

impl ApiClient {
    pub async fn cms_types(
        &self,
        workspace: Uuid,
        query: &cms::ListContentTypes,
    ) -> anyhow::Result<cms::ContentTypePage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/cms/types"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn cms_type(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<cms::ContentType> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/cms/types/{id}"),
        )?)
        .await
    }
    pub async fn create_cms_type(
        &self,
        workspace: Uuid,
        key: &str,
        input: &cms::SaveContentType,
    ) -> anyhow::Result<cms::ContentType> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/types"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn update_cms_type(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::SaveContentType,
    ) -> anyhow::Result<cms::ContentType> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/cms/types/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_cms_type(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::ArchiveContent,
    ) -> anyhow::Result<cms::ContentType> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/types/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn cms_entries(
        &self,
        workspace: Uuid,
        query: &cms::ListEntries,
    ) -> anyhow::Result<cms::EntryPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/cms/entries"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn cms_entry(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<cms::ContentEntry> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}"),
        )?)
        .await
    }
    pub async fn create_cms_entry(
        &self,
        workspace: Uuid,
        key: &str,
        input: &cms::CreateEntry,
    ) -> anyhow::Result<cms::ContentEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/entries"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn update_cms_entry(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::SaveEntry,
    ) -> anyhow::Result<cms::ContentEntry> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn publish_cms_entry(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::ExpectedVersion,
    ) -> anyhow::Result<cms::ContentEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}/publish"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn unpublish_cms_entry(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::ExpectedVersion,
    ) -> anyhow::Result<cms::ContentEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}/unpublish"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_cms_entry(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::ArchiveContent,
    ) -> anyhow::Result<cms::ContentEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn preview_cms_entry(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<cms::PublicPage> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}/preview"),
        )?)
        .await
    }
    pub async fn cms_revisions(
        &self,
        workspace: Uuid,
        id: Uuid,
        query: &cms::RevisionQuery,
    ) -> anyhow::Result<cms::RevisionPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}/revisions"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn cms_revision(
        &self,
        workspace: Uuid,
        id: Uuid,
        revision: Uuid,
    ) -> anyhow::Result<cms::ContentRevision> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/cms/entries/{id}/revisions/{revision}"),
        )?)
        .await
    }
    pub async fn restore_cms_revision(
        &self,
        workspace: Uuid,
        id: Uuid,
        revision: Uuid,
        key: &str,
        input: &cms::ExpectedVersion,
    ) -> anyhow::Result<cms::RestoredEntry> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/cms/entries/{id}/revisions/{revision}/restore"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn cms_media_page(
        &self,
        workspace: Uuid,
        query: &cms::ListMedia,
    ) -> anyhow::Result<cms::MediaPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/cms/media"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn cms_media(&self, workspace: Uuid, id: Uuid) -> anyhow::Result<cms::ContentMedia> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/cms/media/{id}"),
        )?)
        .await
    }
    pub async fn cms_media_content(
        &self,
        workspace: Uuid,
        id: Uuid,
    ) -> anyhow::Result<cms::MediaContent> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/cms/media/{id}/content"),
        )?)
        .await
    }
    pub async fn register_cms_media(
        &self,
        workspace: Uuid,
        key: &str,
        input: &cms::RegisterMedia,
    ) -> anyhow::Result<cms::ContentMedia> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/media"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn update_cms_media(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::SaveMedia,
    ) -> anyhow::Result<cms::ContentMedia> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/cms/media/{id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn archive_cms_media(
        &self,
        workspace: Uuid,
        id: Uuid,
        key: &str,
        input: &cms::ArchiveContent,
    ) -> anyhow::Result<cms::ContentMedia> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/cms/media/{id}/archive"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn public_content_index(
        &self,
        key: &str,
        query: &cms::PublicIndexQuery,
    ) -> anyhow::Result<cms::PublicIndex> {
        self.anonymous(self.http.get(self.public_url(&[key])?).query(query))
            .await
    }
    pub async fn public_content_entry(
        &self,
        key: &str,
        slug: &str,
    ) -> anyhow::Result<cms::PublicPage> {
        self.anonymous(self.http.get(self.public_url(&[key, "entries", slug])?))
            .await
    }
    /// Segments are encoded, so an address cannot step out of the public CMS path.
    fn public_url(&self, segments: &[&str]) -> anyhow::Result<reqwest::Url> {
        let mut url = self.base_url.join("/api/v1/public/cms")?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("The API URL cannot carry a path"))?
            .extend(segments);
        Ok(url)
    }
}
