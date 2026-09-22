# Shared Files

Files belongs to the workspace foundation and remains usable when CRM is disabled. Upload immutable content up to 8 MiB; edit its display filename, media type, notes and responsible membership; archive and restore it. Each operation checks current membership plus its own `files.read`, `files.create`, `files.update` or `files.archive` permission. Workspace and assigned visibility are supported. File ownership never grants access by itself.

Bytes and metadata live in separate PostgreSQL tables, committed with the audit and retry records in the same transaction. The database checks content length and SHA-256; reads verify them again. Only metadata can be updated by the application database role. Archive retains bytes, and there is no automatic purge or file hard-delete action. PostgreSQL storage is intended for these bounded initial documents; no large-media-library capacity claim is made.

## Interfaces

- Web: open **Files** from the workspace. Upload a file and notes, open its details, download, edit or archive/restore. Downloads are private attachments, never inline previews. Failed forms keep notes and retry/version evidence; reselect the same file to retry an upload. The explicit discard link starts a new attempt.
- API: `/api/v1/workspaces/{workspace_id}/files` provides GET pages and POST uploads. `/{id}` provides metadata GET/PATCH; `/{id}/content` returns metadata and canonical base64; `/{id}/archive` applies archive/restore. Mutations require `Idempotency-Key`; edits/archive require `expected_version`. See the generated OpenAPI contract.
- CLI: `looski files --workspace <id> --token-file <private-token-file> list|get|content|upload|edit|archive`. Upload/edit/archive read the same JSON contract on stdin. `content <id> --output <path>` verifies integrity and creates a private new file atomically without overwriting an existing destination. Without `--output`, content is returned as JSON.
- MCP: `files_list`, `files_get`, `files_content`, `files_upload`, `files_edit`, `files_archive` use the same HTTP business contract and current permissions. Content is canonical base64, including on upload; MCP does not implicitly read local paths.

An upload includes `filename`, `media_type`, optional `notes`, optional `owner_membership_id`, and `content_base64`. Media types are `type/subtype` without parameters. Filenames are display metadata and cannot contain paths or control characters. Raw bytes are capped at 8 MiB; the JSON transfer is capped at 12 MiB. Two API transfer slots bound simultaneous file buffers; uploads have a 30-second read deadline. Download bodies retain their slot until completion/drop and are emitted in bounded chunks with an independent 120-second buffer deadline. Capacity exhaustion returns 503 and an expired upload returns 408; retry the same request/key.

File metadata is an authorized `files` export collection with explicit archived/current selection. Export every metadata page and fetch each file's content separately, verifying its digest. Metadata export alone does not contain uploaded bytes. The planned coordinated installation backup must include the PostgreSQL content table and verify it after restore.

The initial release provides an independently scoped file library. Typed attachments to contacts, tasks, opportunities and deals are not implemented; they require owning-module relationships that check both parent and file access. No generic attachment relationship or attachment-based access inheritance is implied by a file ID.
