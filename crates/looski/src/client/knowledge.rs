// SPDX-License-Identifier: AGPL-3.0-only
//! Knowledge base calls over the canonical HTTP API.
use super::ApiClient;
use crate::knowledge::*;
use reqwest::Method;
use uuid::Uuid;

impl ApiClient {
    pub async fn knowledge_list_collections(
        &self,
        workspace: Uuid,
        query: &ListKnowledgeCollections,
    ) -> anyhow::Result<KnowledgeCollectionPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/knowledge/collections"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn knowledge_create_collection(
        &self,
        workspace: Uuid,
        key: &str,
        input: &SaveKnowledgeCollection,
    ) -> anyhow::Result<KnowledgeCollection> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/knowledge/collections"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_get_collection(
        &self,
        workspace: Uuid,
        collection_id: Uuid,
    ) -> anyhow::Result<KnowledgeCollection> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/knowledge/collections/{collection_id}"),
        )?)
        .await
    }
    pub async fn knowledge_update_collection(
        &self,
        workspace: Uuid,
        collection_id: Uuid,
        key: &str,
        input: &SaveKnowledgeCollection,
    ) -> anyhow::Result<KnowledgeCollection> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/knowledge/collections/{collection_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_archive_collection(
        &self,
        workspace: Uuid,
        collection_id: Uuid,
        key: &str,
        input: &ArchiveKnowledgeRecord,
    ) -> anyhow::Result<KnowledgeCollection> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/knowledge/collections/{collection_id}/archive"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_list_members(
        &self,
        workspace: Uuid,
        collection_id: Uuid,
    ) -> anyhow::Result<Vec<KnowledgeMember>> {
        self.business(self.authenticated(
            Method::GET,
            &format!(
                "/api/v1/workspaces/{workspace}/knowledge/collections/{collection_id}/members"
            ),
        )?)
        .await
    }
    pub async fn knowledge_add_member(
        &self,
        workspace: Uuid,
        collection_id: Uuid,
        key: &str,
        input: &AddKnowledgeMember,
    ) -> anyhow::Result<KnowledgeMember> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/knowledge/collections/{collection_id}/members"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_remove_member(
        &self,
        workspace: Uuid,
        collection_id: Uuid,
        membership_id: Uuid,
        key: &str,
    ) -> anyhow::Result<KnowledgeMemberRemoved> {
        self.business(self.authenticated(Method::POST,&format!("/api/v1/workspaces/{workspace}/knowledge/collections/{collection_id}/members/{membership_id}/remove"))?.header("Idempotency-Key", key)).await
    }
    pub async fn knowledge_directory(
        &self,
        workspace: Uuid,
    ) -> anyhow::Result<Vec<KnowledgeMember>> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/knowledge/directory"),
        )?)
        .await
    }
    pub async fn knowledge_list_documents(
        &self,
        workspace: Uuid,
        query: &ListKnowledgeDocuments,
    ) -> anyhow::Result<KnowledgeDocumentPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/knowledge/documents"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn knowledge_create_document(
        &self,
        workspace: Uuid,
        key: &str,
        input: &CreateKnowledgeDocument,
    ) -> anyhow::Result<KnowledgeDocument> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/knowledge/documents"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_get_document(
        &self,
        workspace: Uuid,
        document_id: Uuid,
    ) -> anyhow::Result<KnowledgeDocument> {
        self.business(self.authenticated(
            Method::GET,
            &format!("/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}"),
        )?)
        .await
    }
    pub async fn knowledge_update_document(
        &self,
        workspace: Uuid,
        document_id: Uuid,
        key: &str,
        input: &SaveKnowledgeDocument,
    ) -> anyhow::Result<KnowledgeDocument> {
        self.business(
            self.authenticated(
                Method::PATCH,
                &format!("/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_archive_document(
        &self,
        workspace: Uuid,
        document_id: Uuid,
        key: &str,
        input: &ArchiveKnowledgeRecord,
    ) -> anyhow::Result<KnowledgeDocument> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!(
                    "/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}/archive"
                ),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_move_document(
        &self,
        workspace: Uuid,
        document_id: Uuid,
        key: &str,
        input: &MoveKnowledgeDocument,
    ) -> anyhow::Result<KnowledgeDocument> {
        self.business(
            self.authenticated(
                Method::POST,
                &format!("/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}/move"),
            )?
            .header("Idempotency-Key", key)
            .json(input),
        )
        .await
    }
    pub async fn knowledge_list_revisions(
        &self,
        workspace: Uuid,
        document_id: Uuid,
        query: &KnowledgePageQuery,
    ) -> anyhow::Result<KnowledgeRevisionPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!(
                    "/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}/revisions"
                ),
            )?
            .query(query),
        )
        .await
    }
    pub async fn knowledge_get_revision(
        &self,
        workspace: Uuid,
        document_id: Uuid,
        revision_id: Uuid,
    ) -> anyhow::Result<KnowledgeRevision> {
        self.business(self.authenticated(Method::GET,&format!("/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}/revisions/{revision_id}"))?).await
    }
    pub async fn knowledge_restore_revision(
        &self,
        workspace: Uuid,
        document_id: Uuid,
        revision_id: Uuid,
        key: &str,
        input: &RestoreKnowledgeRevision,
    ) -> anyhow::Result<KnowledgeDocument> {
        self.business(self.authenticated(Method::POST,&format!("/api/v1/workspaces/{workspace}/knowledge/documents/{document_id}/revisions/{revision_id}/restore"))?.header("Idempotency-Key", key).json(input)).await
    }
    pub async fn knowledge_search(
        &self,
        workspace: Uuid,
        query: &KnowledgeSearchQuery,
    ) -> anyhow::Result<KnowledgeSearchPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/knowledge/search"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn knowledge_suggest(
        &self,
        workspace: Uuid,
        query: &KnowledgeSuggestQuery,
    ) -> anyhow::Result<KnowledgeSuggestions> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/knowledge/suggest"),
            )?
            .query(query),
        )
        .await
    }
    pub async fn knowledge_activity(
        &self,
        workspace: Uuid,
        query: &KnowledgePageQuery,
    ) -> anyhow::Result<KnowledgeActivityPage> {
        self.business(
            self.authenticated(
                Method::GET,
                &format!("/api/v1/workspaces/{workspace}/knowledge/activity"),
            )?
            .query(query),
        )
        .await
    }
}
