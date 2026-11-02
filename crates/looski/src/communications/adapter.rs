// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned, transport-neutral connector contract. Native adapters may implement
//! this trait; connectors in other languages use the HTTP v1 adapter.
use super::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};
use utoipa::ToSchema;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub channels: Vec<String>,
    pub send_text: bool,
    pub subjects: bool,
    pub delivery_receipts: bool,
    pub read_receipts: bool,
    pub attachments: bool,
    pub calls: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub version: u32,
    pub installation_id: String,
    pub id: i64,
    pub kind: String,
    pub message_id: String,
    pub thread_id: Option<String>,
    pub channel: Option<String>,
    pub sender: Option<String>,
    pub destination: Option<String>,
    pub body: Option<String>,
    pub subject: Option<String>,
    pub client_ref: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}
#[derive(Serialize)]
pub struct Outbound {
    pub version: u32,
    pub client_ref: String,
    pub thread_id: String,
    pub channel: String,
    pub body: String,
    pub subject: String,
}
#[derive(Deserialize)]
pub struct Accepted {
    pub message_id: String,
    pub idempotent: bool,
}
pub type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;
pub trait ChannelAdapter: Send + Sync {
    fn capabilities(&self) -> Capabilities;
    fn decode(&self, body: &[u8]) -> Result<Event, Error>;
    fn send<'a>(&'a self, message: &'a Outbound) -> AdapterFuture<'a, Accepted>;
    fn poll(&self, after: i64) -> AdapterFuture<'_, Vec<Event>>;
}

pub struct HttpAdapter {
    pub config: super::config::Connector,
    http: reqwest::Client,
}
impl HttpAdapter {
    pub fn new(config: super::config::Connector) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Error::Unavailable)?;
        Ok(Self { config, http })
    }
    async fn response(&self, request: reqwest::RequestBuilder) -> Result<Vec<u8>, Error> {
        let mut response = request
            .bearer_auth(&self.config.api_token)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(
                if response.status().is_server_error() || response.status().as_u16() == 429 {
                    Error::Unavailable
                } else {
                    Error::Conflict("Connector rejected request; check connection")
                },
            );
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 2_000_000 {
                return Err(Error::Invalid("Connector response exceeds limit"));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
impl ChannelAdapter for HttpAdapter {
    fn capabilities(&self) -> Capabilities {
        if self.config.provider == "blueski" {
            Capabilities {
                channels: vec!["imessage".into(), "sms".into(), "rcs".into()],
                send_text: true,
                delivery_receipts: true,
                read_receipts: true,
                ..Default::default()
            }
        } else {
            self.config.capabilities.clone()
        }
    }
    fn decode(&self, body: &[u8]) -> Result<Event, Error> {
        let event = if self.config.provider == "blueski" {
            let v: Value = serde_json::from_slice(body)
                .map_err(|_| Error::Invalid("Invalid Blueski event"))?;
            let s = |key: &str| v.get(key).and_then(Value::as_str).map(str::to_owned);
            let kind = s("event").ok_or(Error::Invalid("Missing event kind"))?;
            let state = match kind.as_str() {
                "message.received" => Some("received".into()),
                "message.queued" => Some("accepted".into()),
                "message.sent" => Some("accepted".into()), // AppleScript acceptance is not delivery.
                "message.failed" => Some("failed".into()),
                _ => s("status"),
            };
            Event {
                version: 1,
                installation_id: s("installation_id").unwrap_or_default(),
                id: v["id"].as_i64().unwrap_or(0),
                kind,
                message_id: s("provider_message_id")
                    .or_else(|| s("message_id"))
                    .unwrap_or_default(),
                thread_id: s("chat_id"),
                channel: s("protocol"),
                sender: s("handle"),
                destination: s("destination_caller_id"),
                body: s("text"),
                subject: None,
                client_ref: s("client_ref"),
                state,
                metadata: json!({"thread_kind":v.get("thread_kind"),"membership_complete":v.get("membership_complete"),"classification_conflict":v.get("classification_conflict"),"provider_timestamp":v.get("timestamp")}),
            }
        } else {
            serde_json::from_slice::<Event>(body)
                .map_err(|_| Error::Invalid("Invalid connector v1 event"))?
        };
        if event.version != 1
            || event.installation_id != self.config.installation_id
            || event.id <= 0
            || event.message_id.is_empty()
            || event.message_id.len() > 512
        {
            return Err(Error::Invalid(
                "Event identity or version does not match connection",
            ));
        }
        if ![
            "message.received",
            "message.queued",
            "message.sent",
            "message.failed",
            "message.status",
        ]
        .contains(&event.kind.as_str())
        {
            return Err(Error::Invalid("Unsupported connector event kind"));
        }
        if let Some(state) = &event.state
            && ![
                "received",
                "queued",
                "accepted",
                "sent",
                "delivered",
                "read",
                "failed",
                "unknown",
            ]
            .contains(&state.as_str())
        {
            return Err(Error::Invalid("Unsupported message state"));
        }
        // Excluded destinations must still advance the replay cursor. Legacy
        // journal entries may predate thread classification; validate payloads
        // only after this operator-defined boundary, as ingestion also does.
        if event.kind == "message.received"
            && !self.config.destinations.is_empty()
            && !event
                .destination
                .as_ref()
                .is_some_and(|d| self.config.destinations.contains(d))
        {
            return Ok(event);
        }
        if event.kind == "message.received" {
            let thread = event.thread_id.as_deref().unwrap_or_default();
            let channel = event.channel.as_deref().unwrap_or_default();
            if thread.is_empty()
                || thread.len() > 512
                || !self.capabilities().channels.iter().any(|c| c == channel)
                || event.body.as_ref().is_none_or(|s| s.len() > 100_000)
                || event.subject.as_ref().is_some_and(|s| s.len() > 1000)
            {
                return Err(Error::Invalid("Invalid incoming message"));
            }
        }
        Ok(event)
    }
    fn send<'a>(&'a self, message: &'a Outbound) -> AdapterFuture<'a, Accepted> {
        Box::pin(async move {
            // Verify identity and retry support before creating an external effect.
            // Older Blueski versions did not provide durable client_ref idempotency.
            let blueski = self.config.provider == "blueski";
            let health_path = if blueski {
                "/healthz"
            } else {
                "/v1/capabilities"
            };
            let health: Value = serde_json::from_slice(
                &self
                    .response(
                        self.http
                            .get(format!("{}{health_path}", self.config.endpoint)),
                    )
                    .await?,
            )
            .map_err(|_| Error::Conflict("Invalid connector capability response"))?;
            let compatible = if blueski {
                let version: Vec<u64> = health["version"]
                    .as_str()
                    .unwrap_or("")
                    .split('.')
                    .map(|v| v.parse().unwrap_or(0))
                    .collect();
                health["product"] == "blueski"
                    && version.len() == 3
                    && version.as_slice() >= [0, 2, 0].as_slice()
            } else {
                health["version"] == 1 && health["idempotent_send"] == true
            };
            if !compatible || health["installation_id"] != self.config.installation_id {
                return Err(Error::Conflict(
                    "Connector identity or durable retry support does not match configuration",
                ));
            }
            let request = if self.config.provider == "blueski" {
                self.http.post(format!("{}/messages",self.config.endpoint)).json(&json!({"chat_id":message.thread_id,"protocol":message.channel,"text":message.body,"client_ref":message.client_ref}))
            } else {
                self.http
                    .post(format!("{}/v1/messages", self.config.endpoint))
                    .json(message)
            };
            let accepted: Accepted = serde_json::from_slice(&self.response(request).await?)
                .map_err(|_| {
                    Error::Conflict("Invalid send receipt; reconcile at connector before retrying")
                })?;
            if !accepted.idempotent
                || accepted.message_id.is_empty()
                || accepted.message_id.len() > 512
            {
                return Err(Error::Conflict(
                    "Connector must acknowledge durable idempotent sending",
                ));
            }
            Ok(accepted)
        })
    }
    fn poll(&self, after: i64) -> AdapterFuture<'_, Vec<Event>> {
        Box::pin(async move {
            let path = if self.config.provider == "blueski" {
                "/events"
            } else {
                "/v1/events"
            };
            let bytes = self
                .response(
                    self.http
                        .get(format!("{}{path}", self.config.endpoint))
                        .query(&[("since", after), ("limit", 50)]),
                )
                .await?;
            let values: Vec<Value> = serde_json::from_slice(&bytes)
                .map_err(|_| Error::Invalid("Expected an array of connector events"))?;
            if values.len() > 50 {
                return Err(Error::Invalid("Connector returned too many events"));
            }
            let mut cursor = after;
            let mut events = Vec::new();
            for value in values {
                let event =
                    self.decode(&serde_json::to_vec(&value).map_err(|_| Error::Unavailable)?)?;
                if event.id <= cursor {
                    return Err(Error::Invalid(
                        "Connector events must have increasing cursors",
                    ));
                }
                cursor = event.id;
                events.push(event);
            }
            Ok(events)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connector(provider: &str) -> HttpAdapter {
        HttpAdapter::new(super::super::config::Connector {
            key: "fixture".into(),
            workspace_id: uuid::Uuid::nil(),
            provider: provider.into(),
            installation_id: "fixture-node".into(),
            endpoint: "http://127.0.0.1:1".into(),
            api_token: "test".into(),
            webhook_secret: "fixture-secret-32-characters-long".into(),
            capabilities: Capabilities {
                channels: vec!["email".into()],
                send_text: true,
                subjects: true,
                ..Default::default()
            },
            destinations: vec![],
        })
        .unwrap()
    }
    #[test]
    fn blueski_maps_acceptance_without_claiming_delivery() {
        let adapter = connector("blueski");
        let event = json!({"installation_id":"fixture-node","id":1,"event":"message.sent","message_id":"dispatch-1","client_ref":"our-message"});
        assert_eq!(
            adapter
                .decode(&serde_json::to_vec(&event).unwrap())
                .unwrap()
                .state
                .as_deref(),
            Some("accepted")
        );
        assert!(!adapter.capabilities().attachments);
        let wrong = json!({"installation_id":"other-node","id":1,"event":"message.sent","message_id":"dispatch-1"});
        assert!(
            adapter
                .decode(&serde_json::to_vec(&wrong).unwrap())
                .is_err()
        );
    }
    #[test]
    fn excluded_legacy_events_advance_without_ingesting_invalid_threads() {
        let mut adapter = connector("blueski");
        adapter.config.destinations = vec!["+16469921008".into()];
        let legacy = json!({"installation_id":"fixture-node","id":2,"event":"message.received","message_id":"old-message","protocol":"imessage","text":"Legacy"});
        assert!(
            adapter
                .decode(&serde_json::to_vec(&legacy).unwrap())
                .is_ok()
        );
        let mut eligible = legacy.clone();
        eligible["destination_caller_id"] = json!("+16469921008");
        assert!(
            adapter
                .decode(&serde_json::to_vec(&eligible).unwrap())
                .is_err()
        );
        let mut wrong = legacy;
        wrong["installation_id"] = json!("other-node");
        assert!(
            adapter
                .decode(&serde_json::to_vec(&wrong).unwrap())
                .is_err()
        );
    }
    #[test]
    fn email_contract_preserves_subject_thread_and_metadata() {
        let adapter = connector("http_v1");
        let event = Event {
            version: 1,
            installation_id: "fixture-node".into(),
            id: 1,
            kind: "message.received".into(),
            message_id: "<mail@example.invalid>".into(),
            thread_id: Some("email-thread".into()),
            channel: Some("email".into()),
            sender: Some("customer@example.invalid".into()),
            destination: Some("support@example.invalid".into()),
            body: Some("Hello".into()),
            subject: Some("Order question".into()),
            client_ref: None,
            state: Some("received".into()),
            metadata: json!({"in_reply_to":"<original@example.invalid>"}),
        };
        let decoded = adapter
            .decode(&serde_json::to_vec(&event).unwrap())
            .unwrap();
        assert_eq!(decoded.subject, event.subject);
        assert_eq!(decoded.metadata, event.metadata);
        let mut invalid = event;
        invalid.version = 2;
        assert!(
            adapter
                .decode(&serde_json::to_vec(&invalid).unwrap())
                .is_err()
        );
    }
}
