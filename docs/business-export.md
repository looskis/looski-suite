# Business export

Export preserves permitted business records as versioned JSON. It requires `workspace.data.export` and the selected collection's ordinary read permissions. The catalog lists dependent read permissions for relationships. Workspace/assigned read scope still applies to every record and related contact, organization or task. Revoked authority stops subsequent pages; disabled CRM cannot be exported through its ordinary routes.

In a workspace, open **Export data**, choose a collection and lifecycle, and supply its parent ID when requested. Preview and download each page; follow **Next page** until the end. Each file holds at most 25 top-level records. The JSON manifest identifies the workspace, selected collection/parent/lifecycle, application and format versions, and live-read consistency. `items` contains the documented public business projections. `next` is either the complete next request or null. Preserve manifests when combining pages.

For an agent or script, use `GET /api/v1/workspaces/{workspace_id}/exports/collections` and `POST /api/v1/workspaces/{workspace_id}/exports/page`. The POST is read-only and does not need an idempotency key. The CLI exposes `looski export ... collections` and `looski export ... page` (selection JSON on stdin); MCP exposes `export_collections` and `export_page`. All use the same business boundary.

For example, select `{"collection":"contacts","archived":false}`. To build a complete private file for that selection, save the selection in a file and run:

```sh
python3 scripts/export_collection.py --api-url http://localhost:58888 \
  --workspace WORKSPACE_UUID --token-file /private/path/access-token \
  --output /private/path/contacts.json < selection.json
```

The collector invokes the supported CLI, follows all continuations and writes a mode-0600 temporary file. It publishes a new output path only after completion and never replaces an existing file. Fetch, validation or page-limit failures remove the temporary file. The package has `format: looski-business-package`, version 1, the original pages, counts and `complete: true`. Use `--binary` for an installed CLI and `--max-pages` to adjust the default 10,000-page ceiling. Protect exported files like the underlying business records.

Collections cover Contacts/Organizations and their current or ended relationships, tag definitions and assignments, opportunities/deals and their stage/status histories, custom pipelines/stages/categories/statuses, participants, shared Tasks/history/types/templates/revisions, CRM task links, logged activities/history/types, and coordination templates/revisions/rules/instances with checklist item-to-task mappings. Parent-specific collections preserve stable parent IDs in the manifest and stable record IDs in rows. Ended contact organization history includes independently visible merged aliases with their original contact IDs. Export current and archived/ended selections separately, and collect related collections explicitly; downloading Contacts does not silently include CRM or Tasks.

This format represents supported public projections, not every internal database row. It excludes authentication credentials/sessions, staff invitation links, administrative audit/replay payloads, intake evidence, queue internals, diagnostic logs and internal starter-package bookkeeping. Participant/task-link change-event history remains available through its ordinary history APIs; export collections contain their current and ended records. General custom fields, communication/message export and file bytes are not yet implemented. Do not claim full installation portability or recovery from these files. Use coordinated backups for that purpose.

JSON retains text and decimal strings exactly, including values beginning with `=`, `+`, `-` or `@`; it is not a spreadsheet-oriented CSV. A later spreadsheet conversion must apply its own formula-injection protection. Each page is a live authorized read, not a consistent database snapshot. Concurrent edits may change later pages; `complete` means all pages of the selected live collection were received, not that the entire workspace was captured at one instant.

## File content

The `files` collection exports authorized metadata, including byte length and SHA-256, with explicit active/archived selection. It does not embed bytes. Retrieve each authorized `files/{id}/content` response separately and verify its digest, or use the private CLI download command described in [Files](files.md). This is a live read; typed business attachments are not implemented.
