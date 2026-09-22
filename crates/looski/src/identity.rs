// SPDX-License-Identifier: AGPL-3.0-only
//! Credential verification owns construction of the trusted identity type.
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct Verifier {
    key: DecodingKey,
    validation: Validation,
    issuer: String,
}

pub struct Identity {
    confirmed_email: Option<String>,
    issuer: String,
    subject: String,
}

impl Identity {
    /// Only Jobs can construct this sealed durable-actor record. No token or
    /// request field can select worker authority; every operation reauthorizes.
    pub(crate) fn resume(record: &crate::jobs::RecordedIdentity) -> Self {
        Self {
            confirmed_email: None,
            issuer: record.issuer().into(),
            subject: record.subject().into(),
        }
    }

    pub(crate) fn bind_confirmed_email(mut self, email: Option<String>) -> Self {
        self.confirmed_email = email;
        self
    }
    pub fn confirmed_email(&self) -> Option<&str> {
        self.confirmed_email.as_deref()
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
}

#[derive(Clone, Deserialize)]
struct Claims {
    sub: Uuid,
    session_id: Uuid,
    role: String,
    #[serde(default)]
    is_anonymous: bool,
    #[serde(default)]
    client_id: Option<serde_json::Value>,
}

impl Verifier {
    pub fn new(secret: &[u8], issuer: String) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&issuer]);
        validation.set_audience(&["authenticated"]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 0;
        validation.validate_nbf = true;
        Self {
            key: DecodingKey::from_secret(secret),
            validation,
            issuer,
        }
    }

    pub fn verify(&self, token: &str) -> Option<Identity> {
        let claims = decode::<Claims>(token, &self.key, &self.validation)
            .ok()?
            .claims;
        // Delegated OAuth requires a future grant/scope path; never promote it
        // to unrestricted first-party authority merely because its JWT is valid.
        if claims.role != "authenticated"
            || claims.is_anonymous
            || claims.client_id.is_some()
            || claims.sub.is_nil()
            || claims.session_id.is_nil()
        {
            return None;
        }
        Some(Identity {
            confirmed_email: None,
            issuer: self.issuer.clone(),
            subject: claims.sub.to_string(),
        })
    }
}
