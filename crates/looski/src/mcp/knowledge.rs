// SPDX-License-Identifier: AGPL-3.0-only
//! Knowledge base tools. Each calls the HTTP API with the configured user's session.
use super::{McpServer, business_result};
use crate::knowledge::*;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
pub(super) struct ListCollectionsArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ListKnowledgeCollections,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateCollectionArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: SaveKnowledgeCollection,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetCollectionArgs {
    workspace_id: Uuid,
    collection_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateCollectionArgs {
    workspace_id: Uuid,
    collection_id: Uuid,
    idempotency_key: String,
    input: SaveKnowledgeCollection,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ArchiveCollectionArgs {
    workspace_id: Uuid,
    collection_id: Uuid,
    idempotency_key: String,
    input: ArchiveKnowledgeRecord,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListMembersArgs {
    workspace_id: Uuid,
    collection_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct AddMemberArgs {
    workspace_id: Uuid,
    collection_id: Uuid,
    idempotency_key: String,
    input: AddKnowledgeMember,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RemoveMemberArgs {
    workspace_id: Uuid,
    collection_id: Uuid,
    membership_id: Uuid,
    idempotency_key: String,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct DirectoryArgs {
    workspace_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListDocumentsArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ListKnowledgeDocuments,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct CreateDocumentArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    input: CreateKnowledgeDocument,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetDocumentArgs {
    workspace_id: Uuid,
    document_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct UpdateDocumentArgs {
    workspace_id: Uuid,
    document_id: Uuid,
    idempotency_key: String,
    input: SaveKnowledgeDocument,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ArchiveDocumentArgs {
    workspace_id: Uuid,
    document_id: Uuid,
    idempotency_key: String,
    input: ArchiveKnowledgeRecord,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MoveDocumentArgs {
    workspace_id: Uuid,
    document_id: Uuid,
    idempotency_key: String,
    input: MoveKnowledgeDocument,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ListRevisionsArgs {
    workspace_id: Uuid,
    document_id: Uuid,
    #[serde(flatten)]
    query: KnowledgePageQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct GetRevisionArgs {
    workspace_id: Uuid,
    document_id: Uuid,
    revision_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RestoreRevisionArgs {
    workspace_id: Uuid,
    document_id: Uuid,
    revision_id: Uuid,
    idempotency_key: String,
    input: RestoreKnowledgeRevision,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct SearchArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: KnowledgeSearchQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct SuggestArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: KnowledgeSuggestQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct ActivityArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: KnowledgePageQuery,
}

#[tool_router(router = knowledge_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(description = "List knowledge collections this member reaches.")]
    async fn list_knowledge_collections(
        &self,
        Parameters(a): Parameters<ListCollectionsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_list_collections(a.workspace_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Create a collection, open to workspace readers or restricted to its member list. The creator is listed. Needs workspace-wide knowledge.collections.manage."
    )]
    async fn create_knowledge_collection(
        &self,
        Parameters(a): Parameters<CreateCollectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_create_collection(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Read a collection. A collection this member cannot reach answers as not found."
    )]
    async fn get_knowledge_collection(
        &self,
        Parameters(a): Parameters<GetCollectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_get_collection(a.workspace_id, a.collection_id)
                .await,
        )
    }
    #[tool(
        description = "Edit a collection, including whether it is restricted. Applies to the next request. Requires expected_version."
    )]
    async fn update_knowledge_collection(
        &self,
        Parameters(a): Parameters<UpdateCollectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_update_collection(
                    a.workspace_id,
                    a.collection_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive (archived=true) or restore a collection. Archiving removes its documents from search and freezes them."
    )]
    async fn archive_knowledge_collection(
        &self,
        Parameters(a): Parameters<ArchiveCollectionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_archive_collection(
                    a.workspace_id,
                    a.collection_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(description = "List the members of a collection.")]
    async fn list_knowledge_collection_members(
        &self,
        Parameters(a): Parameters<ListMembersArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_list_members(a.workspace_id, a.collection_id)
                .await,
        )
    }
    #[tool(description = "List a workspace member on a collection.")]
    async fn add_knowledge_collection_member(
        &self,
        Parameters(a): Parameters<AddMemberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_add_member(
                    a.workspace_id,
                    a.collection_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(
        description = "Remove a member from a collection. They lose a restricted collection on their next request."
    )]
    async fn remove_knowledge_collection_member(
        &self,
        Parameters(a): Parameters<RemoveMemberArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_remove_member(
                    a.workspace_id,
                    a.collection_id,
                    a.membership_id,
                    &a.idempotency_key,
                )
                .await,
        )
    }
    #[tool(
        description = "Active workspace members who can be listed on a collection. Requires knowledge.collections.manage."
    )]
    async fn list_knowledge_directory(
        &self,
        Parameters(a): Parameters<DirectoryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.knowledge_directory(a.workspace_id).await)
    }
    #[tool(
        description = "List documents in collections this member reaches, optionally in one collection or of one kind."
    )]
    async fn list_knowledge_documents(
        &self,
        Parameters(a): Parameters<ListDocumentsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_list_documents(a.workspace_id, &a.query)
                .await,
        )
    }
    #[tool(
        description = "Create an article or SOP. The body is plain text in the rich text subset; HTML stays literal."
    )]
    async fn create_knowledge_document(
        &self,
        Parameters(a): Parameters<CreateDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_create_document(a.workspace_id, &a.idempotency_key, &a.input)
                .await,
        )
    }
    #[tool(
        description = "Read a document with its typed blocks and previews of the documents it links to, as far as this member may see them."
    )]
    async fn get_knowledge_document(
        &self,
        Parameters(a): Parameters<GetDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_get_document(a.workspace_id, a.document_id)
                .await,
        )
    }
    #[tool(
        description = "Edit a document, adding a revision. A stale expected_version conflicts rather than overwrites."
    )]
    async fn update_knowledge_document(
        &self,
        Parameters(a): Parameters<UpdateDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_update_document(
                    a.workspace_id,
                    a.document_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive (archived=true) or restore a document. Archiving removes it from search and freezes it."
    )]
    async fn archive_knowledge_document(
        &self,
        Parameters(a): Parameters<ArchiveDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_archive_document(
                    a.workspace_id,
                    a.document_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(
        description = "Move a document to another collection. Access follows at once; earlier revisions keep needing the collection they were written in."
    )]
    async fn move_knowledge_document(
        &self,
        Parameters(a): Parameters<MoveDocumentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_move_document(
                    a.workspace_id,
                    a.document_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(description = "List the revisions of a document this member may read, newest first.")]
    async fn list_knowledge_revisions(
        &self,
        Parameters(a): Parameters<ListRevisionsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_list_revisions(a.workspace_id, a.document_id, &a.query)
                .await,
        )
    }
    #[tool(description = "Read one immutable revision.")]
    async fn get_knowledge_revision(
        &self,
        Parameters(a): Parameters<GetRevisionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_get_revision(a.workspace_id, a.document_id, a.revision_id)
                .await,
        )
    }
    #[tool(
        description = "Copy a revision into a new current revision. History is never rewritten. Requires expected_version."
    )]
    async fn restore_knowledge_revision(
        &self,
        Parameters(a): Parameters<RestoreRevisionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_restore_revision(
                    a.workspace_id,
                    a.document_id,
                    a.revision_id,
                    &a.idempotency_key,
                    &a.input,
                )
                .await,
        )
    }
    #[tool(
        description = "Full-text search over current documents this member may read, with typed snippets and a count of permitted matches."
    )]
    async fn search_knowledge(
        &self,
        Parameters(a): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.knowledge_search(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Document titles matching typed text, among documents this member may read."
    )]
    async fn suggest_knowledge_documents(
        &self,
        Parameters(a): Parameters<SuggestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_suggest(a.workspace_id, &a.query)
                .await,
        )
    }
    #[tool(description = "Recent document changes this member may see, newest first.")]
    async fn list_knowledge_activity(
        &self,
        Parameters(a): Parameters<ActivityArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .knowledge_activity(a.workspace_id, &a.query)
                .await,
        )
    }
}
