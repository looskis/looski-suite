# Google sign-in

Google sign-in uses the installation's existing Supabase Auth identities and Looski workspace permissions. It is optional and disabled by default. Public signup remains disabled. A verified Google email can link to an eligible existing identity through Supabase's automatic identity linking; using an unrelated Google account does not create an account or workspace membership. Invite and bootstrap users through the existing onboarding flow first.

In Google Cloud Console, select the intended project, configure Google Auth Platform branding/audience, and create an OAuth client of type **Web application**. Register the exact `LOOSKI_PUBLIC_URL` followed by `/auth/v1/callback` as an authorized redirect URI. For example, `https://preview-suite.loo.ski/auth/v1/callback`. Use separate clients for preview and production. Basic sign-in requests email and profile; it does not request Gmail, Drive, or offline access. If the app is in testing mode, configure the intended test users as needed.

Download the client JSON to a private location, then run:

```sh
python3 scripts/configure_google.py --dev --client-file /private/path/client.json
make dev
```

Omit `--dev` and use `make up` for the primary installation. Import validates the callback URI, stores only `client_id` and `client_secret` in `.local/<installation>/google-oauth.json` with mode `0600`, and enables `LOOSKI_GOOGLE_AUTH_ENABLED` in the selected settings file. Keep the downloaded original private too. Do not put the secret in `.env`, shell arguments, tracked files, or support messages. Setup derives the separate `auth-google.json` Compose secret; only Auth receives it. Disabling the setting clears the derived secret and hides the button without deleting the original credential or changing other sign-in methods. Recreate Auth and web after configuration changes; reload/recreate the gateway when deploying its new callback route.

The login button POSTs to `/auth/google` with same-origin enforcement and always opens Google’s account chooser. The server creates a random PKCE verifier in a ten-minute HttpOnly, SameSite=Lax cookie (`__Host-` prefixed on HTTPS), and calls Auth's private `/authorize` endpoint. Google returns to the GET-only `/auth/v1/callback`, where Auth validates provider state and identity. Auth redirects to the allowlisted `/auth/callback`, which exchanges the one-time code server-side using the cookie's verifier, clears the pending cookie, and creates the existing HttpOnly session cookies. The validated return path preserves CLI and invitation journeys. Raw authorize, token, admin, and password endpoints stay private. Failure redirects clear provider error fragments. Callback responses disable caching/referrers; do not add query-string logging upstream.

Validate a real Google account matching an existing user, logout, subsequent sign-in, cancellation, and an unprovisioned account before considering a deployment verified. Automated boundary tests use a mock Auth server and do not prove Google's client configuration or actual account linking. Back up the private Google client separately; existing recovery bundles do not include this optional credential, and restored installations default to Google sign-in disabled.

References: [Supabase social provider configuration](https://supabase.com/docs/guides/self-hosting/self-hosted-oauth), [PKCE](https://supabase.com/docs/guides/auth/sessions/pkce-flow), [identity linking](https://supabase.com/docs/guides/auth/auth-identity-linking).
