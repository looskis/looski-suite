# Authentication email

Password login works with mail disabled. Email sign-in and recovery use Supabase Auth through the suite's server; successful identity verification still requires application membership for workspace access. Ordinary magic links never create accounts. Links expire after one hour and need explicit confirmation before consumption. Recovery replaces the password of the authenticated identity; it does not change roles or memberships.

Set `LOOSKI_MAIL_MODE` in `.env` (or the setup environment), then run `make up`:

- `disabled`: default; no email delivery UI. Auth has an unreachable SMTP endpoint so it cannot fall back to logging verification messages.
- `local`: loopback development only. Private Mailpit captures messages within the Compose network, with no published inbox/SMTP ports and a maximum of 100 messages. Capture is ephemeral; stopping/recreating may discard it. Do not use real customer mail here.
- `smtp`: use a provider-neutral SMTP account. Create `.local/<COMPOSE_PROJECT_NAME>/smtp.json` with file permissions `0600`. All six fields are required: `host`, numeric `port` (465), `user`, `password`, `from`, and `sender_name`. The sender must be allowed by the provider. Use an app-specific SMTP credential when required. Never put credentials in `.env`, shell arguments, issue reports or tracked files.

Example shape (placeholder values only):

```json
{"host":"smtp.example.com","port":465,"user":"smtp-user","password":"replace-privately","from":"accounts@example.com","sender_name":"Looski Suite"}
```

Setup validates the private file and creates a separate private `auth-mail.json` Compose secret; it never rotates existing database or identity secrets. Only Auth receives the SMTP credentials. External SMTP requires implicit TLS on port 465 with certificate verification. The pinned Auth transport does not require STARTTLS on port 587, so setup rejects that port rather than risking plaintext authentication links. A provider that only offers port 587 needs a future relay with enforced STARTTLS; it is not supported by this packaging yet. OAuth-only mail APIs are not SMTP credentials and require a different connector.

`LOOSKI_PUBLIC_URL` must be the canonical origin, without a trailing slash. Public origins require HTTPS. Configure Cloudflare Tunnel to reach the existing suite gateway port, with that hostname; no Auth or SMTP container port needs publishing. Email links use `/auth/confirm` on the configured origin. The private template server starts before Auth. Browser confirmation first removes the token from the address bar, keeps it in a ten-minute HttpOnly pending cookie, and only verifies it on a same-origin form POST. Raw `/auth/v1` authentication/admin paths stay private. A GET-only `/auth/v1/verify` compatibility route runs in the suite web server: when Supabase falls back to its default email template during a template-server outage, this route performs the same pending-cookie/explicit-confirmation flow. It never proxies verification to Auth or consumes tokens on GET. Callback responses have `no-store` and `no-referrer`; do not add query-string access logging in front of them.

Provider sending limits, sender/domain verification and deliverability are operator responsibilities. Request responses do not disclose whether an account exists or was throttled. Delivery failures keep that same account-neutral response, avoiding account discovery during outages. The web server emits a fixed diagnostic without recipient, provider body, link or credentials; the page directs users to their administrator if no email arrives. Public Cloudflare and hosted Supabase validation remain release gates until their real configuration is supplied; passing local capture tests does not satisfy those gates.

Implementation references: [pinned Supabase SMTP client](https://github.com/supabase/auth/blob/v2.196.0/internal/mailer/mailmeclient/mailmeclient.go), [custom email templates](https://supabase.com/docs/guides/self-hosting/custom-email-templates), [Mailpit Docker](https://mailpit.axllent.org/docs/install/docker/).
