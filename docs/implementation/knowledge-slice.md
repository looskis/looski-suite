# Knowledge base checkpoint

November 1, 2026; schema version 33. The knowledge base is the optional application from Milestone 5: articles and standard operating procedures in collections, full-text search, immutable revisions and collection-level permissions. It works with CMS, CRM and every other optional application disabled.

## Behavior and contract

Staff manage collections and documents through web, API, CLI and MCP. A document has a kind (`article` or `sop`), a title and a body, with optimistic versions, idempotent retries and transactional audit. Audit records hold identifiers and numbers, never text. A collection is open to the workspace or restricted to its member list. Documents and collections archive and restore; archived material stays readable to those who reach it, leaves search, autocomplete and the activity feed, and accepts no edits.

Titles and collection names are not unique and documents have no slugs, deliberately: a refused duplicate would reveal that a hidden record exists.

### Permissions and reach

| Permission | Grants | Scopes |
| --- | --- | --- |
| `knowledge.read` | Find and read documents, revisions, collections and member lists | workspace, assigned |
| `knowledge.documents.edit` | Create, edit, move, archive and restore documents and revisions | workspace, assigned |
| `knowledge.collections.manage` | Edit and archive collections, change restriction, manage members. Creating a collection needs workspace scope; the directory of active members needs this permission | workspace, assigned |
| `knowledge.restricted.access` | Reach every restricted collection without being listed, for whichever knowledge permissions the member also holds workspace-wide. It grants no action by itself | workspace |

A member reaches a collection for an action if they are listed on it, or hold the action workspace-wide and it is open, or hold the action workspace-wide with the override. Assigned scope therefore means listed collections only. Listing grants nothing without a role. Every write also needs read. Owners hold all four permissions, so **an owner sees restricted collections they are not listed on**; a role without the override does not.

`knowledge.reaches` in the migration is the one definition of this rule, and every read names it in the statement that selects its rows. Authority is read from the database on each request, so removal from a collection, loss of a role, a move, or a change of restriction applies to the next request on the same token.

A hidden document or collection answers exactly as one that does not exist, including through `after` cursors, filters, member lists, revisions, exports and every mutation. Visibility is checked before version or archive state, so a stale version reveals nothing. Retries remember only the ID of what they changed, plus SHA-256 digests of a document's title and body, and answer from current state under current authority. A retry after a removal is a 404, and the retry table holds no document text. A retried call therefore returns the document as it is now, not as it was.

### Search

Search is PostgreSQL full-text search with the `english` configuration, over a generated `tsvector` of the title (weight A) and the words a reader sees (weight B): the typed blocks' text, with no markup or link targets. Only current text of active documents in active collections is indexed for results; revisions are not searchable. Queries use `websearch_to_tsquery`, so quotes, `or` and `-word` work and malformed input cannot fail.

- **Results, counts and snippets** come from statements that apply `knowledge.reaches`; `total` counts only permitted matches.
- **Ranking** uses `ts_rank`, which reads only the row in question and keeps no corpus statistics, so hidden documents cannot change a visible document's rank or position. The smoke test compares results before and after adding documents stuffed with the search term.
- **Cursors** are an opaque encoding of the last result's own rank and ID, compared by value with visible rows. Nothing is looked up, so a forged cursor naming a hidden document reveals nothing.
- **Snippets** are typed parts (`text`, `matched`). `ts_headline` marks matches with control characters that document text is refused for containing, and the web view renders parts with `<mark>`, never HTML.
- **Autocomplete** (`/knowledge/suggest`) is a title substring match with the same visibility.

The default parser treats text shaped like HTML tags as a tag rather than words: `<script>` in a body is not searchable and is dropped from snippets, though it still renders as literal text on the document's page.

### Revisions

Every change that alters the text appends an immutable revision; the runtime role has no `UPDATE` or `DELETE` on the table. Saving unchanged text adds nothing. Restoring copies a revision into a new one (`restored_from_number`), and earlier rows do not change. Edits, moves and restores all name the expected version: a stale one is a `409`, and concurrent writers cannot overwrite each other. A conflict in the web editor keeps the draft and shows the newer text.

**A revision records the collection it was written in, and reading it needs that collection as well as the document's current one.** Moving a document from a restricted collection into an open one releases its current text to the new readers and nothing older: the move is itself a new revision written in the target, and earlier revisions stay hidden from those who cannot reach their collection, in the history, the activity feed, exports and restore. A hidden revision answers 404, so restore cannot be used to read one. Someone who can read it may restore it, which deliberately releases that text to the document's current readers. Hidden revisions leave gaps in the numbers shown; a gap says only that a revision existed. Opening a restricted collection releases its history, which the settings page says.

### Linked previews and citing documents

A body links another document with a same-site path, `/workspaces/{id}/knowledge/documents/{id}`; each document's page shows the text to paste. Reading a document returns `linked`, one entry per target in reading order (at most 50), with a preview (title, collection name, kind, excerpt) only if the reader may see it now. A hidden target, a missing one and one in another workspace are indistinguishable, and saving a link never checks its target, so it is no oracle. The ID and any words the author wrote in the link's own text are part of the body and visible to its readers.

`knowledge::previews(db, actor, ids)` is the interface for other applications, Helpdesk's article links among them: store only a document ID, and ask each time under the viewing member's own authority. With the knowledge base disabled or no read permission every preview is absent, so the caller needs no dependency on it being enabled. `knowledge::suggest` and `knowledge::search` serve article pickers the same way. Helpdesk now uses this interface for staff reference links; customers receive no knowledge previews.

### Stored markup

The body is plain source in the rich text subset, parsed by the same pure parser as CMS and Projects. It moved from `cms::rich_text` to a neutral `rich_text` module and depends on no application being enabled. HTML in the source stays literal text, unsafe links are refused when saved and render as plain text if they reached storage another way, and the web view renders only typed blocks through `RichText.svelte`. While moving it, a parse of 100,000 characters of unmatched markers was found to take 24 seconds in a debug build, on every read, CMS public pages included. Failed searches are now remembered so parsing stays linear; behavior is otherwise unchanged and a unit test bounds it.

### Module, export and recovery

Disabling the knowledge base closes every route, export, CLI and MCP with `403`. Every record is retained and comes back on re-enabling. It has no public surface, background job or worker. CRM timelines compose nothing from it. Export adds `knowledge_collections` and `knowledge_documents` (with archive selection), and for a given parent `knowledge_collection_members` and `knowledge_revisions`; see [business export](../business-export.md). Recovery needs no specific step; see [recovery](../recovery.md).

## Evidence and limits

`scripts/knowledge_smoke.py` runs on real Postgres with real signed-in second users and every other optional application disabled except for one CRM timeline check. It covers validation and retries; restricted documents absent from results, counts, snippets, cursors, autocomplete, lists, activity, linked previews, exports and CRM timelines; hidden records answering byte-for-byte like missing ones; no uniqueness oracle; scopes, roles and the override; removal on the same token, including a retry after removal and no text in the retry or audit tables; moves in both directions and restriction changes applying at once; history staying with its collection and restore refusing to read it; index contents, and rank, count and cursor equality before and after hidden documents; immutable revisions and table privileges; concurrent edits and racing retries; markup and refused or smuggled links; archive; cross-workspace isolation; CLI and MCP parity; and every route closed when disabled, with data retained.

`scripts/knowledge_browser_smoke.py` drives real HTTP through the gateway with a scripting-free client: application selection, rejected forms keeping drafts, inert stored markup, search with structural highlights, a stale edit keeping its draft, revision restore, a second signed-in person seeing only what they may through members and moves, forged origins, session gates and framing headers on every page, and disabled pages. Both run in `scripts/acceptance.py`.

A one-off headless Chrome run on November 1, 2026 confirmed what those clients cannot: with `onerror`, `<script>` and `<svg onload>` payloads in a collection name and description, document titles and bodies, no payload ran and no dialog opened on the home, search, collection, document and revisions pages, and no element was injected; matches were highlighted; typing filled the suggestion list through the page's own handler; and real form posts for an edit and a restore passed the origin check. This check is not automated.

`scripts/recovery_smoke.py` seeds a restricted collection and a document with an edit and a restore, and checks them, the revision history, the member list and a search hit after an isolated restore. It is not part of `acceptance.py`.

Search was measured once, in a rolled-back transaction on the local fixture (Colima on Apple Silicon) with 20,000 documents of about 2 KB in 20 collections: a query matching 400 documents with snippets took about 10 ms, and one matching all of them about 61 ms without snippets. No whole-stack footprint was measured; the knowledge base adds no service or worker.

Not supported: attachments or images in documents (the rich text subset has no image block; link to Files, which keep their own permissions); nested pages or trees, comments, templates, approval flows, read acknowledgements and review dates for SOPs; drafts separate from what readers see (draft in a restricted collection and move the document when ready); read-only versus edit access per collection, since a collection's list grants whatever the member's role allows; backlinks; collaborative or live editing and diff views; external, customer or public access; search languages other than English, fuzzy matching and searching historical revisions; importing from Confluence, Notion or other products; and permanent deletion. Member lists stop at 500 per collection. The web pickers for a new document's collection and a move target show the first 1000 collections; the API takes any. Helpdesk article links are the interface above only.
