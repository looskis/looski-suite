// SPDX-License-Identifier: AGPL-3.0-only
//! CMS tools. Each calls the HTTP API with the configured user's session.
use super::{McpServer, business_result};
use crate::cms::{
    ArchiveContent, CreateEntry, ExpectedVersion, ListContentTypes, ListEntries, ListMedia,
    RegisterMedia, RevisionQuery, SaveContentType, SaveEntry, SaveMedia,
};
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
pub(super) struct TypeListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ListContentTypes,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct TypeArgs {
    workspace_id: Uuid,
    type_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct TypeCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    content_type: SaveContentType,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct TypeUpdateArgs {
    workspace_id: Uuid,
    type_id: Uuid,
    idempotency_key: String,
    content_type: SaveContentType,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct TypeArchiveArgs {
    workspace_id: Uuid,
    type_id: Uuid,
    idempotency_key: String,
    change: ArchiveContent,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ListEntries,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryCreateArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    entry: CreateEntry,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryUpdateArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
    idempotency_key: String,
    entry: SaveEntry,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryVersionArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
    idempotency_key: String,
    expected_version: i64,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct EntryArchiveArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
    idempotency_key: String,
    change: ArchiveContent,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RevisionListArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
    #[serde(flatten)]
    query: RevisionQuery,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RevisionArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
    revision_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct RevisionRestoreArgs {
    workspace_id: Uuid,
    entry_id: Uuid,
    revision_id: Uuid,
    idempotency_key: String,
    expected_version: i64,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MediaListArgs {
    workspace_id: Uuid,
    #[serde(flatten)]
    query: ListMedia,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MediaArgs {
    workspace_id: Uuid,
    media_id: Uuid,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MediaRegisterArgs {
    workspace_id: Uuid,
    idempotency_key: String,
    media: RegisterMedia,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MediaUpdateArgs {
    workspace_id: Uuid,
    media_id: Uuid,
    idempotency_key: String,
    media: SaveMedia,
}
#[derive(Deserialize, JsonSchema)]
pub(super) struct MediaArchiveArgs {
    workspace_id: Uuid,
    media_id: Uuid,
    idempotency_key: String,
    change: ArchiveContent,
}

#[tool_router(router = cms_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(description = "List content types.")]
    async fn list_cms_types(
        &self,
        Parameters(a): Parameters<TypeListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.cms_types(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Read a content type and its fields.")]
    async fn get_cms_type(
        &self,
        Parameters(a): Parameters<TypeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.cms_type(a.workspace_id, a.type_id).await)
    }
    #[tool(description = "Create a content type with its fields.")]
    async fn create_cms_type(
        &self,
        Parameters(a): Parameters<TypeCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_cms_type(a.workspace_id, &a.idempotency_key, &a.content_type)
                .await,
        )
    }
    #[tool(
        description = "Edit a content type. A field keeps its kind; removing one leaves published revisions unchanged. Requires expected_version."
    )]
    async fn update_cms_type(
        &self,
        Parameters(a): Parameters<TypeUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .update_cms_type(
                    a.workspace_id,
                    a.type_id,
                    &a.idempotency_key,
                    &a.content_type,
                )
                .await,
        )
    }
    #[tool(
        description = "Archive (archived=true) or restore a content type. Archiving takes its public pages offline."
    )]
    async fn archive_cms_type(
        &self,
        Parameters(a): Parameters<TypeArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_cms_type(a.workspace_id, a.type_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(
        description = "List entries. Without draft access only published entries and their published titles are listed."
    )]
    async fn list_cms_entries(
        &self,
        Parameters(a): Parameters<EntryListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.cms_entries(a.workspace_id, &a.query).await)
    }
    #[tool(
        description = "Read an entry: its draft when you may read drafts, and what is published."
    )]
    async fn get_cms_entry(
        &self,
        Parameters(a): Parameters<EntryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.cms_entry(a.workspace_id, a.entry_id).await)
    }
    #[tool(
        description = "Create a draft entry of a content type. Rich text values are plain text in a small Markdown subset, never HTML."
    )]
    async fn create_cms_entry(
        &self,
        Parameters(a): Parameters<EntryCreateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .create_cms_entry(a.workspace_id, &a.idempotency_key, &a.entry)
                .await,
        )
    }
    #[tool(
        description = "Save an entry draft as a new revision; what is live changes only on publish. Requires expected_version."
    )]
    async fn update_cms_entry(
        &self,
        Parameters(a): Parameters<EntryUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .update_cms_entry(a.workspace_id, a.entry_id, &a.idempotency_key, &a.entry)
                .await,
        )
    }
    #[tool(
        description = "Make an entry draft public at its address. Requires publishing permission and expected_version."
    )]
    async fn publish_cms_entry(
        &self,
        Parameters(a): Parameters<EntryVersionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .publish_cms_entry(
                    a.workspace_id,
                    a.entry_id,
                    &a.idempotency_key,
                    &ExpectedVersion {
                        expected_version: a.expected_version,
                    },
                )
                .await,
        )
    }
    #[tool(description = "Take an entry off its public address, keeping its draft and revisions.")]
    async fn unpublish_cms_entry(
        &self,
        Parameters(a): Parameters<EntryVersionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .unpublish_cms_entry(
                    a.workspace_id,
                    a.entry_id,
                    &a.idempotency_key,
                    &ExpectedVersion {
                        expected_version: a.expected_version,
                    },
                )
                .await,
        )
    }
    #[tool(
        description = "Archive (archived=true) or restore an entry. Archiving withdraws it; restoring leaves it unpublished."
    )]
    async fn archive_cms_entry(
        &self,
        Parameters(a): Parameters<EntryArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_cms_entry(a.workspace_id, a.entry_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
    #[tool(
        description = "Render an entry draft as its public page would, for staff who may read drafts."
    )]
    async fn preview_cms_entry(
        &self,
        Parameters(a): Parameters<EntryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .preview_cms_entry(a.workspace_id, a.entry_id)
                .await,
        )
    }
    #[tool(description = "List an entry's immutable revisions, newest first.")]
    async fn list_cms_revisions(
        &self,
        Parameters(a): Parameters<RevisionListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .cms_revisions(a.workspace_id, a.entry_id, &a.query)
                .await,
        )
    }
    #[tool(description = "Read one revision of an entry.")]
    async fn get_cms_revision(
        &self,
        Parameters(a): Parameters<RevisionArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .cms_revision(a.workspace_id, a.entry_id, a.revision_id)
                .await,
        )
    }
    #[tool(
        description = "Copy an earlier revision into the draft as a new revision. Publish afterwards to make it live."
    )]
    async fn restore_cms_revision(
        &self,
        Parameters(a): Parameters<RevisionRestoreArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .restore_cms_revision(
                    a.workspace_id,
                    a.entry_id,
                    a.revision_id,
                    &a.idempotency_key,
                    &ExpectedVersion {
                        expected_version: a.expected_version,
                    },
                )
                .await,
        )
    }
    #[tool(description = "List the media library.")]
    async fn list_cms_media(
        &self,
        Parameters(a): Parameters<MediaListArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.cms_media_page(a.workspace_id, &a.query).await)
    }
    #[tool(description = "Read a media item.")]
    async fn get_cms_media(
        &self,
        Parameters(a): Parameters<MediaArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(self.client.cms_media(a.workspace_id, a.media_id).await)
    }
    #[tool(description = "Read a media item with its bytes as base64.")]
    async fn get_cms_media_content(
        &self,
        Parameters(a): Parameters<MediaArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .cms_media_content(a.workspace_id, a.media_id)
                .await,
        )
    }
    #[tool(
        description = "Add a PNG, JPEG, GIF, WebP or PDF from Files to the media library. Requires reading that file. It becomes public only through published entries."
    )]
    async fn register_cms_media(
        &self,
        Parameters(a): Parameters<MediaRegisterArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .register_cms_media(a.workspace_id, &a.idempotency_key, &a.media)
                .await,
        )
    }
    #[tool(description = "Edit a media item's title and description. Requires expected_version.")]
    async fn update_cms_media(
        &self,
        Parameters(a): Parameters<MediaUpdateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .update_cms_media(a.workspace_id, a.media_id, &a.idempotency_key, &a.media)
                .await,
        )
    }
    #[tool(
        description = "Archive (archived=true) or restore a media item. Archiving removes it from public pages at once."
    )]
    async fn archive_cms_media(
        &self,
        Parameters(a): Parameters<MediaArchiveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        business_result(
            self.client
                .archive_cms_media(a.workspace_id, a.media_id, &a.idempotency_key, &a.change)
                .await,
        )
    }
}
