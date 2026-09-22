# Authentication email checkpoint

Implemented provider-neutral SMTP configuration, private local Mailpit capture, magic-link sign-in, explicit callback confirmation and authenticated password replacement. Supabase owns credentials, tokens, expiry and sessions. Ordinary sign-in does not create accounts or memberships. Configured external SMTP requires verified implicit TLS on port 465; the pinned provider cannot enforce STARTTLS on 587, which setup rejects.

Private configuration validates strict fields and permissions, mounts credentials only into Auth, and preserves the core secret set. Setup recreates Auth when file-secret content changes. Local capture is restricted to loopback origins and has no published ports. The primary installation keeps mail disabled until configured.

Callbacks stage tokens in short-lived HttpOnly cookies, clear the query string and verify only on explicit same-origin POST. Arbitrary redirect parameters are ignored. A GET-only compatibility handler processes default-template links after template-server failure through the same browser flow; raw provider authentication/admin endpoints stay private. Requesting a link returns identical responses for existing/missing accounts, throttling and delivery failure, while fixed operator diagnostics exclude recipient/provider body/credentials.

The real SMTP/browser journey passed magic-link delivery, clean callbacks, safe previews, one-time and expired tokens, current workspace membership, password recovery and replacement login, CSRF, unknown accounts, SMTP outage and cold-cache template outage for both email types. Setup tests cover credential permissions, validation, mode changes and public-origin rejection for local capture. Static/unit checks and targeted standards/spec re-review passed. Primary deployment and disabled-mail behavior passed. Full acceptance passed on both fresh and upgraded installations. External provider deliverability, Cloudflare callback and hosted-mode validation remain open.

Per the user's latest instruction, this work remains uncommitted with subsequent slices for one grouped signing approval.
