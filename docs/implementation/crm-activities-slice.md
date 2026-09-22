# CRM activities checkpoint

Schemas 16–17 add workspace-owned activity types and logged activities with exactly one workspace-qualified contact, opportunity or deal reference. Types have explicit immutable note/interaction behavior, editable labels/order and archival. Interactions support bounded duration; labels cannot add capabilities. Type changes do not rewrite recorded activity labels or history.

Activity operations enforce CRM enablement, current action and read scopes, active permitted responsibility, and independent anchor visibility. Contact merges retain original references; both alias and canonical contact must remain visible. Contacts provides authorized family IDs before CRM reads indexed candidates, and bounded projections recheck aliases before public pagination. Hidden or unrelated cursors are unavailable.

Save, archive/restore, immutable snapshots, audit and replay commit together. Current authority is checked on retries; optimistic versions reject concurrent edits. Creating requires an active anchor and type. Existing records can retain a retired type, and archival preserves history. Restoring requires an active anchor. There is no automatic deletion.

The HTTP/OpenAPI, CLI, MCP and SSR interfaces cover types, activity lists and filters, creation/editing, archival and history. Web forms retain invalid/conflicting drafts, versions and retry keys; type order and filters survive page navigation. Occurrence timestamps use explicit offsets, normalize to UTC/microseconds, and reject invalid dates or leap seconds.

The expanded isolated activity journey passes type ordering, snapshot retention, transaction rollback, responsibility and anchor scopes, merged aliases, concurrency, CLI/MCP and web draft/filter behavior. Static checks/unit tests and primary gateway smoke pass. Standards/spec re-review reports no remaining findings. Full fresh-install and upgraded-database regression suites pass. The combined timeline, coordination templates and queued follow-up remain separate outstanding increments.
