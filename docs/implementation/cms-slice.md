# CMS checkpoint

November 1, 2026; schema version 31. CMS is the optional application from Milestone 3: content types, entries, media, drafts, revisions, a publishing interface and a representative rendered public page with an embedded form. It works with Forms and CRM disabled.

## Behavior and contract

Staff manage content through web, API, CLI and MCP. A content type lists fields of kind text, rich text, number, yes-or-no, date, web address, media or embedded form. A field's key is its stable identity and keeps its kind; labels change freely. Every saved change to an entry appends an immutable revision holding the title, address, values and the type's fields at that moment. An unchanged save adds nothing. Required fields are enforced when publishing, so an unfinished draft can always be saved.

Publishing points the entry at its latest revision, under the current type's fields. If the type changed since the last save, a new revision records exactly what goes public. The public page renders with the fields of the revision it publishes, so later type edits never alter it. Only one entry of a type may be live at an address; drafts may share one, and renaming a draft moves nothing until it is published. Old addresses do not redirect.

Restoring a revision copies it into the draft as a new revision and leaves what is live alone until someone with publishing permission publishes. History is never rewritten. Values whose field no longer exists with the same kind are left out and reported by name, and stay readable in the revision. All mutations use optimistic versions, idempotent retries and transactional audit, like other modules.

### Permissions

All are workspace-wide and assignable in roles.

| Permission | Grants |
| --- | --- |
| `cms.read` | Content types, media and published entries. Unpublished entries do not exist to a member with only this; lists show published titles. |
| `cms.drafts.read` | Drafts, previews and revision history. Editing and publishing also need it. |
| `cms.entries.edit` | Create, edit, restore and archive drafts. |
| `cms.entries.publish` | Publish and unpublish. Also required to archive a published entry, or to archive or restore a type with published entries, since those change what is live. |
| `cms.types.manage` | Create, edit and archive content types. |
| `cms.media.manage` | Register Files as media; edit and archive media. Registering also needs `files.read` on that file, including its record scope. |

### Public access

The mode is recorded in [decisions](../decisions.md): anonymous pages on the installation's own origin, reachable wherever the installation is. Each content type has a 128-bit public key, shown in its builder. `/c/{key}` lists its published entries and `/c/{key}/{slug}` renders one. The API equivalents sit under `/api/v1/public/cms/{key}`, and the CLI reads them with `looski public-content`. The gateway strips `Cookie` on `/c/*`, and the page's API call carries no credential, so a key reaches only what is published under that type. Reads are throttled per client. Disabling CMS or archiving a type answers 404 and retains every record.

### Stored markup

Rich text is plain source in a Markdown subset: `#`–`###` headings, paragraphs, `-` and `1.` lists, `>` quotes, `**bold**`, `*italic*`, `` `code` `` and links. The server parses it into typed blocks; HTML in the source stays literal text. Links must start with `https://`, `http://`, `mailto:` or a single `/`: an unsafe link is refused when saved, and any that reaches storage some other way renders as plain text. The web renderer uses typed components only and has no raw HTML output.

Published pages are rendered on the server with no client scripts and send `default-src 'none'; img-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; frame-src 'self'; base-uri 'none'; form-action 'none'` plus `frame-ancestors 'none'`, including on error pages. The policy keeps `Referrer-Policy: same-origin`; the embedded form posts from its own document on its own origin.

### Media and private Files

Media builds on Files: a media record adopts one file, and the bytes stay in Files, immutable and hash-checked. The web upload creates an ordinary private file, then registers it; a refused file stays private in Files. Registration accepts only PNG, JPEG, GIF, WebP and PDF, and the file's first bytes must match its declared type. SVG and HTML are refused because browsers can execute them. The type is copied at registration, so editing file metadata later cannot change how the bytes are served.

Nothing about a file becomes public by registering it. `/api/v1/public/cms/{key}/media/{media_id}` answers only for active media, whose file is active, named by a revision that is live right now under that key's type. A file id, a draft reference, a superseded revision or another type's key all answer 404. Public values expose the media id, title, description and type, never the file id or filename. Responses send the fixed type, `nosniff` and `default-src 'none'; sandbox`; PDFs download rather than display. Archiving media, or archiving the file in Files, withdraws it at once and pages render without it; publishing a draft that names withdrawn media is refused.

Registering is a deliberate disclosure inside the workspace: members with `cms.read` can then see the item and its filename through the library, without any Files permission.

### Embedded forms

A form field stores a form id with no database dependency on Forms. Choosing a new one requires Forms enabled, workspace-wide `forms.read`, and an unarchived form in this workspace; unknown, foreign and unreadable forms give the same answer. An existing choice survives editing while Forms is off.

The page frames `/f/{form_key}/embed`, the existing public embed, and links to the form's own page for readers who cannot use frames. While Forms is disabled or the form is unpublished, closed or archived, the public value carries no key and the page shows "This form is not currently available" and still serves. Only the key reaches the page, so an unpublished form rename cannot leak. Submissions then follow Forms unchanged, including its CRM recipe.

### Export and recovery

Export adds `cms_content_types`, `cms_entries`, `cms_revisions` and `cms_media`; see [business export](../business-export.md). Media metadata is exported; bytes come from Files. Recovery needs no CMS-specific step; see [recovery](../recovery.md).

## Evidence and limits

`scripts/cms_smoke.py` runs on real Postgres with Forms and CRM disabled until its embed section. It covers type and value validation, retries and stale versions, draft privacy in public reads, staff reads, lists, previews and cursors, publish permission on publish, unpublish and archive, one live entry per address, media adoption and byte checks, every refused route to bytes, withdrawal through CMS and Files, revision restore and its dropped fields, module disablement, an embedded form while unavailable, a submission reaching a CRM opportunity and contact, cross-workspace denial, export pages and permissions, CLI/MCP parity and audit. It also writes an unsafe link directly to a stored revision and confirms it does not render as a link.

`scripts/cms_browser_smoke.py` drives real HTTP through the gateway with a scripting-free client: application selection, type builder, media upload and refusal, draft editing, a rejected save keeping what was typed, preview, save and publish, escaped markup, the content policy, no script tags, the embed frame and a submission inside it, forged origins, session gates, a staff cookie changing nothing, revision restore through the page, and the unavailable-form message. Both run in `scripts/acceptance.py`.

A one-off headless Chrome run on November 1, 2026 confirmed what those clients cannot: on a published page with script, `onerror` and `onload` payloads in its title and body, no payload ran, the page had no script elements, its stylesheets and image loaded under the policy with no violations logged, and a real submission inside the frame reached its confirmation. This check is not automated.

`scripts/recovery_smoke.py` now seeds a published page, its media and a later unpublished draft before the snapshot. After an isolated restore it confirms the public API, the rendered `/c/` address, identical media bytes with their type, the draft and both revisions. The rehearsal passed on this code on November 1, 2026. It is not part of `acceptance.py`. On Colima, set `TMPDIR` to a directory the VM shares, because the encryption helper bind-mounts a temporary key and the default macOS temporary directory is not shared.

Latency, measured once on an Apple M6 through the gateway on the local fixture, 200 sequential requests to one page with rich text, one image and one form: rendered page p50 3.3 ms and p95 4.3 ms; API read p50 1.3 ms and p95 1.6 ms. This excludes image transfer and any tunnel, and is not a load test. CMS adds no background work or service, and its whole-stack footprint was not measured separately.

Not supported: a visual site builder, templates or themes (roadmap 19), custom domains or pretty site paths, scheduled publishing, approval workflows, comments, localization, inline images or forms inside rich text (media and forms are fields, rendered in field order), references between entries, nested Markdown lists or tables, public search, redirects from old addresses, rotating a public key, per-page passwords, image resizing, CDN caching (responses are `no-store`), and video or other media formats. Frames have a fixed minimum height and scroll rather than resizing to the form. Throttles are per client and in memory, as for Forms; a public media response holds up to 8 MiB in memory while it is sent.
