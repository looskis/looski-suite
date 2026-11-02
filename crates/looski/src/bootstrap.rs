// SPDX-License-Identifier: AGPL-3.0-only
//! Owner provisioning, shared by the local operator and expiring setup service.
use anyhow::{Context, ensure};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

mod dao;

#[derive(Deserialize)]
pub struct OwnerInput {
    email: String,
    password: String,
    display_name: String,
    workspace_name: String,
}

pub async fn provision(
    pool: &PgPool,
    http: &reqwest::Client,
    auth_url: &str,
    issuer: &str,
    secret: &[u8],
    input: OwnerInput,
) -> anyhow::Result<()> {
    ensure!(
        (1..=120).contains(&input.display_name.trim().chars().count()),
        "Name must be 1–120 characters"
    );
    ensure!(
        (1..=120).contains(&input.workspace_name.trim().chars().count()),
        "Workspace name must be 1–120 characters"
    );
    ensure!(
        (12..=1024).contains(&input.password.len()),
        "Password must be 12–1024 bytes"
    );
    ensure!(
        input.email.len() <= 254 && input.email.contains('@'),
        "Enter a valid email address"
    );
    let mut tx = pool.begin().await?;
    let completed = dao::lock_installation(&mut tx).await?;
    ensure!(
        !completed,
        "This installation already has an owner. Bootstrap cannot be repeated."
    );

    let token = encode(
        &Header::default(),
        &serde_json::json!({
            "role": "service_role", "iss": issuer,
            "exp": jsonwebtoken::get_current_timestamp() + 60
        }),
        &EncodingKey::from_secret(secret),
    )?;
    let created = http.post(format!("{auth_url}/admin/users")).bearer_auth(token)
        .json(&serde_json::json!({"email": input.email, "password": input.password, "email_confirm": true}))
        .send().await.context("Auth provisioning is unavailable; retry with the same credentials")?;
    #[derive(Deserialize)]
    struct User {
        id: Uuid,
    }
    let user = if created.status().is_success() {
        created
            .json::<User>()
            .await
            .context("Invalid Auth provisioning response")?
    } else if created.status().as_u16() == 422 {
        // If Auth committed before a previous suite transaction failed, proving
        // the same password can resume. Never look up an account by email and
        // silently attach it, change its password, or delete it on failure.
        #[derive(Deserialize)]
        struct Session {
            user: User,
        }
        let response = http
            .post(format!("{auth_url}/token?grant_type=password"))
            .json(&serde_json::json!({"email": input.email, "password": input.password}))
            .send()
            .await
            .context("Auth sign-in unavailable")?;
        ensure!(
            response.status().is_success(),
            "Auth account exists or input was rejected; verify the email and password before retrying"
        );
        response
            .json::<Session>()
            .await
            .context("Invalid Auth session response")?
            .user
    } else {
        anyhow::bail!(
            "Auth provisioning failed (status {}); no workspace was created",
            created.status()
        );
    };
    dao::create_owner(&mut tx, issuer, &user.id.to_string(), &input).await?;
    tx.commit()
        .await
        .context("Suite provisioning did not finish; retry with the same credentials")?;
    Ok(())
}
