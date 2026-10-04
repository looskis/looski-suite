// SPDX-License-Identifier: AGPL-3.0-only
//! CLI/MCP login storage and refresh, keyed per installation origin.
//!
//! Sessions live in the macOS Keychain where available, otherwise in a 0600
//! file under the user's configuration directory. Supabase refresh tokens
//! rotate and reuse revokes the session, so every refresh happens under an
//! exclusive cross-process file lock and rereads the store before rotating.
use crate::{
    cli_login::{CliSession, SessionUser},
    client::ApiFailure,
};
use anyhow::Context;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Refresh this long before expiry so a request never carries a dying token.
const REFRESH_MARGIN_SECONDS: u64 = 120;
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    File,
    Keychain,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    origin: String,
    access_token: String,
    refresh_token: String,
    expires_at: u64,
    user: SessionUser,
}

/// Token-free description of a stored login.
#[derive(Serialize)]
pub struct Status {
    pub origin: String,
    pub user: SessionUser,
    pub access_expires_at: u64,
    pub credential_store: Backend,
}

pub struct Login {
    base: reqwest::Url,
    origin: String,
    directory: PathBuf,
    backend: Backend,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
fn failure(message: String) -> anyhow::Error {
    ApiFailure {
        status: 401,
        code: "unauthenticated".into(),
        message,
        request_id: None,
    }
    .into()
}

fn configuration_directory() -> anyhow::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("LOOSKI_CONFIG_DIR") {
        let dir = PathBuf::from(dir);
        anyhow::ensure!(dir.is_absolute(), "LOOSKI_CONFIG_DIR must be absolute");
        return Ok(dir);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|h| h.is_absolute())
        .context("HOME is not set; set LOOSKI_CONFIG_DIR")?;
    if cfg!(target_os = "macos") {
        return Ok(home.join("Library/Application Support/Looski Suite"));
    }
    Ok(std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"))
        .join("looski-suite"))
}

#[cfg(unix)]
fn private_directory(path: &std::path::Path) -> anyhow::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    let metadata = std::fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_dir() && metadata.permissions().mode() & 0o077 == 0,
        "Credential directory {} must be a private directory (mode 0700)",
        path.display()
    );
    Ok(())
}
#[cfg(not(unix))]
fn private_directory(path: &std::path::Path) -> anyhow::Result<()> {
    Ok(std::fs::create_dir_all(path)?)
}

#[cfg(target_os = "macos")]
mod keychain {
    use security_framework::passwords;
    const SERVICE: &str = "Looski Suite CLI";
    const NOT_FOUND: i32 = -25300;
    pub fn get(origin: &str) -> anyhow::Result<Option<Vec<u8>>> {
        match passwords::get_generic_password(SERVICE, origin) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.code() == NOT_FOUND => Ok(None),
            Err(error) => anyhow::bail!("Keychain read failed: {error}"),
        }
    }
    pub fn set(origin: &str, value: &[u8]) -> anyhow::Result<()> {
        passwords::set_generic_password(SERVICE, origin, value)
            .map_err(|error| anyhow::anyhow!("Keychain write failed: {error}"))
    }
    pub fn delete(origin: &str) -> anyhow::Result<()> {
        match passwords::delete_generic_password(SERVICE, origin) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == NOT_FOUND => Ok(()),
            Err(error) => anyhow::bail!("Keychain delete failed: {error}"),
        }
    }
}
#[cfg(not(target_os = "macos"))]
mod keychain {
    pub fn get(_: &str) -> anyhow::Result<Option<Vec<u8>>> {
        anyhow::bail!("Keychain storage is only available on macOS")
    }
    pub fn set(_: &str, _: &[u8]) -> anyhow::Result<()> {
        anyhow::bail!("Keychain storage is only available on macOS")
    }
    pub fn delete(_: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Login {
    pub fn for_installation(base: &reqwest::Url) -> anyhow::Result<Self> {
        let backend = match std::env::var("LOOSKI_CREDENTIAL_STORE").as_deref() {
            Ok("file") => Backend::File,
            Ok("keychain") if cfg!(target_os = "macos") => Backend::Keychain,
            Ok("") | Err(_) if cfg!(target_os = "macos") => Backend::Keychain,
            Ok("") | Err(_) => Backend::File,
            Ok(_) => {
                anyhow::bail!("LOOSKI_CREDENTIAL_STORE must be `file` or, on macOS, `keychain`")
            }
        };
        Ok(Self {
            base: base.clone(),
            origin: base.origin().ascii_serialization(),
            directory: configuration_directory()?.join("sessions"),
            backend,
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    fn path(&self, extension: &str) -> PathBuf {
        let key = Sha256::digest(self.origin.as_bytes());
        let name: String = key[..12].iter().map(|b| format!("{b:02x}")).collect();
        self.directory.join(format!("{name}.{extension}"))
    }
    fn not_signed_in(&self) -> anyhow::Error {
        failure(format!(
            "Not signed in to {}. Run `looski login` or set LOOSKI_ACCESS_TOKEN_FILE.",
            self.origin
        ))
    }

    fn load(&self) -> anyhow::Result<Option<Stored>> {
        let bytes = match self.backend {
            Backend::Keychain => keychain::get(&self.origin)?,
            Backend::File => match std::fs::read(self.path("json")) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            },
        };
        let Some(bytes) = bytes else { return Ok(None) };
        let stored: Stored = serde_json::from_slice(&bytes)
            .context("Stored login is unreadable; run `looski login`")?;
        // Never present one installation's tokens to another.
        Ok((stored.origin == self.origin).then_some(stored))
    }
    fn save(&self, stored: &Stored) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(stored)?;
        match self.backend {
            Backend::Keychain => keychain::set(&self.origin, &bytes),
            Backend::File => {
                private_directory(&self.directory)?;
                // Write a private sibling and rename so readers never see a
                // partial file and the mode is set before any secret is written.
                let target = self.path("json");
                let temporary = self.path(&format!("tmp-{}", uuid::Uuid::new_v4()));
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
                let written = (|| {
                    use std::io::Write;
                    let mut file = options.open(&temporary)?;
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                    std::fs::rename(&temporary, &target)
                })();
                if written.is_err() {
                    let _ = std::fs::remove_file(&temporary);
                }
                Ok(written?)
            }
        }
    }
    fn delete(&self) -> anyhow::Result<()> {
        match self.backend {
            Backend::Keychain => keychain::delete(&self.origin),
            Backend::File => match std::fs::remove_file(self.path("json")) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
                _ => Ok(()),
            },
        }
    }

    /// Exclusive per-origin lock shared by every CLI and MCP process.
    async fn lock(&self) -> anyhow::Result<std::fs::File> {
        private_directory(&self.directory)?;
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(self.path("lock"))?;
        let deadline = std::time::Instant::now() + LOCK_WAIT;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(std::fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    anyhow::bail!(
                        "Timed out waiting for another Looski process to refresh the login"
                    )
                }
                Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
            }
        }
    }

    fn store(&self, session: CliSession) -> anyhow::Result<Stored> {
        let stored = Stored {
            origin: self.origin.clone(),
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            expires_at: now() + session.expires_in.max(0) as u64,
            user: session.user,
        };
        self.save(&stored)?;
        Ok(stored)
    }

    /// Rotate the refresh token. The caller holds the lock.
    async fn rotate(&self, http: &reqwest::Client, current: &Stored) -> anyhow::Result<Stored> {
        let response = http
            .post(self.base.join("/api/v1/cli/sessions/refresh")?)
            .json(&serde_json::json!({"refresh_token": current.refresh_token}))
            .send()
            .await
            .context("Looski API is unavailable for login refresh")?;
        let status = response.status();
        if status.is_success() {
            return self.store(response.json().await?);
        }
        let error = response.json::<crate::api::ApiError>().await.ok();
        if status.as_u16() == 400 && error.as_ref().is_some_and(|e| e.code == "invalid_grant") {
            self.delete()?;
            return Err(failure(format!(
                "The Looski login for {} expired or was revoked. Run `looski login`.",
                self.origin
            )));
        }
        Err(ApiFailure {
            status: status.as_u16(),
            code: error
                .as_ref()
                .map_or("unavailable".into(), |e| e.code.clone()),
            message: error.map_or("Login refresh failed".into(), |e| e.message),
            request_id: None,
        }
        .into())
    }

    /// A current access token, refreshing first when it is near expiry or the
    /// API rejected `rejected`. Another process may already have refreshed.
    pub async fn access_token(
        &self,
        http: &reqwest::Client,
        rejected: Option<&str>,
    ) -> anyhow::Result<String> {
        let fresh = |s: &Stored| s.expires_at > now() + REFRESH_MARGIN_SECONDS;
        if rejected.is_none()
            && let Some(current) = self.load()?.filter(fresh)
        {
            return Ok(current.access_token);
        }
        let _lock = self.lock().await?;
        let current = self.load()?.ok_or_else(|| self.not_signed_in())?;
        let stale = match rejected {
            Some(token) => token == current.access_token,
            None => !fresh(&current),
        };
        if !stale {
            return Ok(current.access_token);
        }
        Ok(self.rotate(http, &current).await?.access_token)
    }

    pub fn status(&self) -> anyhow::Result<Option<Status>> {
        Ok(self.load()?.map(|s| Status {
            origin: s.origin,
            user: s.user,
            access_expires_at: s.expires_at,
            credential_store: self.backend,
        }))
    }

    /// Revoke this installation's CLI session, then forget it locally. Returns
    /// whether the server confirmed revocation; local state is always removed.
    pub async fn logout(&self, http: &reqwest::Client) -> anyhow::Result<bool> {
        let _lock = self.lock().await?;
        let Some(mut current) = self.load()? else {
            return Ok(false);
        };
        let mut revoked = false;
        if current.expires_at <= now() + REFRESH_MARGIN_SECONDS {
            current = match self.rotate(http, &current).await {
                Ok(next) => next,
                Err(_) => {
                    self.delete()?;
                    return Ok(false);
                }
            };
        }
        if let Ok(response) = http
            .post(self.base.join("/api/v1/cli/sessions/logout")?)
            .bearer_auth(&current.access_token)
            .header("x-looski-interface", "cli")
            .send()
            .await
        {
            revoked = response.status().as_u16() == 204;
        }
        self.delete()?;
        Ok(revoked)
    }

    /// Interactive authorization-code + PKCE sign-in with a pasted code.
    pub async fn login(
        &self,
        http: &reqwest::Client,
        device_label: Option<String>,
        open_browser: bool,
    ) -> anyhow::Result<Status> {
        let random = |n: usize| -> anyhow::Result<String> {
            let mut bytes = vec![0u8; n];
            getrandom::fill(&mut bytes).map_err(|_| anyhow::anyhow!("No secure randomness"))?;
            Ok(URL_SAFE_NO_PAD.encode(bytes))
        };
        let verifier = random(32)?;
        let state = random(16)?;
        let mut url = self.base.join("/cli/authorize")?;
        url.query_pairs_mut()
            .append_pair("code_challenge", &crate::cli_login::challenge(&verifier))
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state);
        if let Some(label) = device_label.as_deref().filter(|l| !l.trim().is_empty()) {
            url.query_pairs_mut()
                .append_pair("device", &label.trim().chars().take(64).collect::<String>());
        }
        eprintln!(
            "Open this link in a browser signed in to {}:\n\n  {url}\n",
            self.origin
        );
        if open_browser {
            let opener = if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            let _ = std::process::Command::new(opener)
                .arg(url.as_str())
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        eprint!("Paste the authorization code: ");
        let pasted = tokio::task::spawn_blocking(|| {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).map(|_| line)
        })
        .await??;
        let pasted = pasted.trim();
        anyhow::ensure!(!pasted.is_empty(), "No authorization code was entered");
        let (code, returned) = pasted
            .split_once('#')
            .context("Paste the complete code shown in the browser, including the part after #")?;
        anyhow::ensure!(
            returned == state,
            "This code belongs to a different sign-in request. Run `looski login` again."
        );
        let response = http
            .post(self.base.join("/api/v1/cli/sessions")?)
            .header("x-looski-interface", "cli")
            .json(&serde_json::json!({"code": code, "code_verifier": verifier}))
            .send()
            .await
            .context("Looski API is unavailable")?;
        let status = response.status();
        if !status.is_success() {
            let error = response.json::<crate::api::ApiError>().await.ok();
            return Err(ApiFailure {
                status: status.as_u16(),
                code: error
                    .as_ref()
                    .map_or("unavailable".into(), |e| e.code.clone()),
                message: error.map_or("Sign-in failed".into(), |e| e.message),
                request_id: None,
            }
            .into());
        }
        let session: CliSession = response.json().await?;
        let _lock = self.lock().await?;
        let previous = self.load().ok().flatten();
        let stored = self.store(session)?;
        // Best effort: a replaced login should not stay usable server side.
        if let Some(previous) = previous.filter(|p| p.expires_at > now()) {
            let _ = http
                .post(self.base.join("/api/v1/cli/sessions/logout")?)
                .bearer_auth(&previous.access_token)
                .header("x-looski-interface", "cli")
                .send()
                .await;
        }
        Ok(Status {
            origin: stored.origin,
            user: stored.user,
            access_expires_at: stored.expires_at,
            credential_store: self.backend,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(directory: &std::path::Path, origin: &str) -> Login {
        let base = reqwest::Url::parse(origin).unwrap();
        Login {
            origin: base.origin().ascii_serialization(),
            base,
            directory: directory.join("sessions"),
            backend: Backend::File,
        }
    }
    fn session(expires_in: i64) -> CliSession {
        CliSession {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            token_type: "bearer".into(),
            expires_in,
            user: SessionUser {
                id: uuid::Uuid::nil(),
                email: Some("person@example.invalid".into()),
            },
        }
    }

    #[tokio::test]
    async fn file_store_is_private_per_origin_and_skips_refresh_while_fresh() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("looski-login-test-{}", uuid::Uuid::new_v4()));
        let first = login(&root, "http://localhost:58897/api");
        first.store(session(3600)).unwrap();
        let file = std::fs::metadata(first.path("json")).unwrap();
        assert_eq!(file.permissions().mode() & 0o777, 0o600);
        let directory = std::fs::metadata(&first.directory).unwrap();
        assert_eq!(directory.permissions().mode() & 0o777, 0o700);
        // No network is reachable; a fresh token must not trigger refresh.
        let http = reqwest::Client::new();
        assert_eq!(first.access_token(&http, None).await.unwrap(), "access");
        let status = first.status().unwrap().unwrap();
        assert_eq!(status.origin, "http://localhost:58897");
        // Another installation never sees this session.
        assert!(
            login(&root, "http://localhost:58898")
                .status()
                .unwrap()
                .is_none()
        );
        first.delete().unwrap();
        let error = first.access_token(&http, None).await.err().unwrap();
        assert_eq!(error.downcast_ref::<ApiFailure>().unwrap().status, 401);
        assert!(error.to_string().contains("looski login"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn refresh_lock_excludes_other_holders() {
        let root = std::env::temp_dir().join(format!("looski-login-test-{}", uuid::Uuid::new_v4()));
        let login = login(&root, "http://localhost:58897");
        let held = login.lock().await.unwrap();
        let other = std::fs::OpenOptions::new()
            .write(true)
            .open(login.path("lock"))
            .unwrap();
        assert!(matches!(
            other.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        drop(held);
        other.try_lock().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
