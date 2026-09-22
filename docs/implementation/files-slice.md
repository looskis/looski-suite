# Shared Files implementation evidence

Status: implemented and verified on upgraded and fresh schema-28 installations. No commit yet; grouped signing remains pending until the remaining build work is ready.

Implemented: schema 28 metadata/content tables; bounded immutable bytes with database and reader SHA-256/length validation; independently scoped upload/read/update/archive; atomic audit and digest-only replay evidence; workspace serialization with post-lock authorization; OpenAPI/CLI/MCP/browser parity; safe downloads and versioned metadata export. See [Files](../files.md) for constraints and remaining typed-association scope.

Review exposed stale permission decisions across lock waits and browser failure forms replacing draft/retry evidence. Both failures were reproduced against the first Files image, then corrected in source. API transfer review also added upload deadlines and download permit ownership through chunked-body completion/drop.

Evidence so far:

- `/tmp/looski-files-route-red.log`: absent upload endpoint returned 404 before implementation.
- `/tmp/looski-files-checks-2.log`: initial format, lint, typecheck, contract checks, Rust and Python tests passed.
- `/tmp/looski-files-auth-race-red.log`: old edit succeeded with 200 during a committed permission-revocation overlap; expected 403.
- `/tmp/looski-files-browser-red.log`: old upload failure lost notes and retry identity.
- `/tmp/looski-files-reviewed-unit.log`: transfer bounds/deadlines, download integrity/private publication, input limits and SHA-256 known answer pass.

Final evidence:

- `/tmp/looski-files-final-checks.log`: formatting, Clippy, Svelte typecheck, generated contracts, 12 Rust tests and the then-current 12 Python tests pass.
- `/tmp/looski-files-final-unit.log`: independent no-poll download deadline, bounded chunks/permits, private no-overwrite download, validation and checksum checks pass.
- `/tmp/looski-files-auth-race-green.log`: upload, edit and archive all observe revocation committed while waiting for the workspace lock.
- `/tmp/looski-files-final-acceptance.log`: full upgraded-installation regression passes, including actual multipart upload, retained failure/retry state and stale-edit notes/version/key.
- `/tmp/looski-files-fresh-up.log` and `/tmp/looski-files-fresh-acceptance.log`: clean schema-28 installation and full acceptance pass, including production Docker builds.
- Both standards/spec reviewers found no remaining actionable Files issues after recheck.
- `/tmp/looski-files-footprint.log`: measured fixture storage; repeated-byte limit-test payloads compress strongly and are not representative of arbitrary media.

Backup recovery of actual bytes is verified by the [recovery checkpoint](recovery-slice.md), including complete table inventory, every file digest/length, PGMQ data and restored authenticated downloads.
