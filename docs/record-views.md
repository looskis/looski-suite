# Record fields and reusable views

Contacts and Organizations use one field registry across validation, editors, tables, filters and ordering. Standard properties live alongside core record columns; workspace-specific fields use immutable `custom_` keys. Typed values are stored in the record's `properties` JSON object and follow its existing authorization, optimistic version, audit history, export and deletion lifecycle. Omitting `properties` on an older API update preserves it.

Custom definitions support text, number, date, boolean, select, email and URL. Administrators need `records.fields.manage`. Renaming and archiving preserve existing values and saved queries. Types cannot change and existing select options cannot be removed. Archived definitions remain resolvable by older views. Each workspace supports 200 custom fields per entity type.

Addresses contain `address_line1`, `address_line2`, `locality`, `administrative_area`, `postal_code`, and ISO 3166-1 alpha-2 `country_code`. A nonempty address requires a country. Postal codes remain strings, including leading zeroes and international formats. This is structural validation, not postal deliverability verification. See [Google's PostalAddress model](https://docs.cloud.google.com/domains/docs/reference/rest/Shared.Types/PostalAddress) for the international address conventions used here.

Contact identifiers retain submitted order. The first email is primary, the second is secondary; additional addresses remain available. Existing records retain their previous deterministic order until edited. Identifier uniqueness, sharing checks and sorted lock acquisition are unchanged.

## Filters and saved views

The HTTP list endpoints accept a serialized `filter` AST, `sort` field key and `direction` (`asc` or `desc`). Conditions support typed comparisons, text matching, membership and empty checks. Groups support nested `and` and `or` rules. For example:

```json
{"type":"group","operator":"and","rules":[{"type":"condition","field":"status","operator":"eq","value":"customer"},{"type":"group","operator":"or","rules":[{"type":"condition","field":"job_title","operator":"contains","value":"director"},{"type":"condition","field":"custom_score","operator":"gte","value":80}]}]}
```

Queries run in PostgreSQL across authorized records before pagination. Field keys are checked against the registry and paths/values use bound parameters. Queries are limited to 60 nodes, 5 nesting levels and 20 children per group. Membership accepts up to 50 values. Missing values are excluded from ordinary comparisons, including negative comparisons; use an explicit empty condition in an OR group to include them. Empty AND groups match all; empty OR groups match none. Sorting uses nulls last and record ID as the tie-breaker. Standard created/updated date filters compare UTC calendar dates. Lists retain the legacy order when no explicit sort is requested.

Saved views retain ordered columns (up to 40), filter AST, archive state and ordering. They belong to their creator; only that user can edit or delete them. Team views are visible to authorized workspace readers and do not widen record access. Sharing requires workspace-wide read scope. Saves/deletes use idempotency keys, audit events and optimistic versions. Users may store up to 100 views per entity type. Temporary search and tag constraints stay in the URL.

The schema includes workspace/status and JSONB indexes. Arbitrary property comparisons are parameterized but may require a scan within the authorized workspace. Add measured expression indexes for frequently used fields as datasets grow; do not assume a JSONB index accelerates every comparison. See [PostgreSQL JSON indexing](https://www.postgresql.org/docs/current/datatype-json.html).

## Other behavior

Opportunity and deal amounts retain exact decimal storage and their original currency. Presentation uses currency formatting; billing frequency is explicitly one-time, monthly, quarterly or annually. Legacy API updates preserve frequency. No automatic exchange-rate or recurring-revenue conversion is implied.

Inbox supports subject/account filtering and newest-activity ordering. The web thread requests the latest message page and displays that page chronologically, with navigation to earlier messages. API clients retain their previous ordering unless they opt into `newest_first`.

## Validation

Run `python3 scripts/record_views_smoke.py` only against the dedicated `*-identity-check` fixture. It exercises nested custom-field filters, null semantics, duplicate sort values across pages, primary email order, international addresses, backward-compatible updates, view replay/concurrency/deletion, authorization and billing frequency. Existing Contacts and Organizations acceptance suites cover record scopes, merges, history and API/CLI/MCP regressions.

## Reusable table sizing and commands

`components/table/DataTable.svelte` owns sizing, containment and resizing;
`table-layout.ts` is the pure sizing policy. Pass typed column metadata plus header,
cell and empty-state snippets. Automatic widths measure the loaded page's rendered
text, cap long values at 420px, and share unused viewport space. Additional columns
scroll inside the table region without widening the parent. Manual widths can reach
960px. Drag a header separator, use Left/Right (Shift for larger steps), or double-click
/Home to reset. Widths are personal browser preferences scoped by account, workspace,
record kind and saved view; shared view definitions continue to share column selection,
filters and sorting. Full cell values remain available through record details.

`commands.ts` is the common action registry. Cmd/Ctrl+K opens the command palette for
navigation, creation forms, workspace switching and appearance. Creation commands
navigate to existing `?action=create` drawers and never submit forms automatically.
Mobile keeps an icon trigger because touch devices may not have a keyboard.
