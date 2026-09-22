// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded, lossy operational telemetry. Never an audit or authorization store.
use serde::Serialize;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use uuid::Uuid;

const CAPACITY: usize = 1024;
const BATCH: usize = 64;
const QUERY: &str = "INSERT INTO looski_diagnostics.events FORMAT JSONEachRow";

#[derive(Clone, Default)]
pub struct Diagnostics {
    sender: Option<mpsc::Sender<Event>>,
    dropped: Arc<AtomicU64>,
}

// There is deliberately no arbitrary message, URL, header, body or SQL field.
#[derive(Serialize)]
struct Event {
    timestamp_ms: u64,
    event_id: Uuid,
    request_id: Option<Uuid>,
    service: &'static str,
    kind: &'static str,
    operation: String,
    status: u16,
    duration_ms: u64,
}

impl Diagnostics {
    pub fn from_environment() -> Self {
        if std::env::var("LOOSKI_DIAGNOSTICS_MODE").as_deref() != Ok("local") {
            return Self::default();
        }
        let key = std::env::var("LOOSKI_DIAGNOSTICS_KEY_FILE")
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok());
        let Some(key) = key.filter(|key| {
            key.trim().len() == 64 && key.trim().bytes().all(|b| b.is_ascii_hexdigit())
        }) else {
            tracing::warn!("Diagnostics disabled: installation credential unavailable");
            return Self::default();
        };
        Self::start("http://clickhouse:8123", key.trim().to_owned())
    }

    fn start(endpoint: &str, key: String) -> Self {
        let (sender, receiver) = mpsc::channel(CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let diagnostics = Self {
            sender: Some(sender),
            dropped: dropped.clone(),
        };
        let endpoint = endpoint.to_owned();
        tokio::spawn(export(receiver, endpoint, key, dropped));
        diagnostics
    }

    pub fn request(
        &self,
        id: Uuid,
        method: &str,
        route: Option<&str>,
        status: u16,
        elapsed: Duration,
    ) {
        // Only framework-matched route templates reach this boundary, never the
        // original path (which may contain identifiers or attacker input).
        let method = match method {
            "GET" => "GET",
            "POST" => "POST",
            "PATCH" => "PATCH",
            "DELETE" => "DELETE",
            "PUT" => "PUT",
            "HEAD" => "HEAD",
            "OPTIONS" => "OPTIONS",
            _ => "OTHER",
        };
        let route = route
            .filter(|v| v.len() <= 240 && v.is_ascii())
            .unwrap_or("unmatched");
        self.emit(Event::new(
            "api",
            "request",
            format!("{method} {route}"),
            Some(id),
            status,
            elapsed,
        ));
    }

    pub fn worker(&self, processed: bool, failed: bool, elapsed: Duration) {
        if processed || failed {
            self.emit(Event::new(
                "worker",
                "poll",
                if failed { "failed" } else { "processed" }.into(),
                None,
                if failed { 500 } else { 200 },
                elapsed,
            ));
        }
    }

    fn emit(&self, event: Event) {
        if let Some(sender) = &self.sender
            && sender.try_send(event).is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Event {
    fn new(
        service: &'static str,
        kind: &'static str,
        operation: String,
        request_id: Option<Uuid>,
        status: u16,
        elapsed: Duration,
    ) -> Self {
        Self {
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u64::MAX as u128) as u64,
            event_id: Uuid::new_v4(),
            request_id,
            service,
            kind,
            operation,
            status,
            duration_ms: elapsed.as_millis().min(u64::MAX as u128) as u64,
        }
    }
}

async fn export(
    mut receiver: mpsc::Receiver<Event>,
    endpoint: String,
    key: String,
    dropped: Arc<AtomicU64>,
) {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    else {
        return;
    };
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut batch = Vec::with_capacity(BATCH);
    loop {
        let flush = tokio::select! {
            event = receiver.recv(), if batch.len() < BATCH => match event {
                Some(event) => { batch.push(event); batch.len() == BATCH },
                None => break,
            },
            _ = interval.tick() => true,
        };
        if !flush {
            continue;
        }
        if batch.is_empty() {
            continue;
        }
        // Send at most one bounded batch at a time. One failed send is dropped;
        // there is no unbounded retry queue, spool, request wait or readiness gate.
        let mut body = String::new();
        for event in &batch {
            if let Ok(encoded) = serde_json::to_string(event) {
                body.push_str(&encoded);
                body.push('\n');
            }
        }
        let successful = client
            .post(&endpoint)
            .query(&[("query", QUERY)])
            .header("X-ClickHouse-User", "looski_ingest")
            .header("X-ClickHouse-Key", &key)
            .body(body)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success());
        if !successful {
            dropped.fetch_add(batch.len() as u64, Ordering::Relaxed);
        }
        batch.clear();
        let lost = dropped.swap(0, Ordering::Relaxed);
        if lost > 0 {
            tracing::warn!(
                dropped_events = lost,
                "Diagnostic events dropped; business operations unaffected"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn full_queue_drops_without_waiting_and_payload_excludes_raw_input() {
        let (sender, mut receiver) = mpsc::channel(1);
        let diagnostics = Diagnostics {
            sender: Some(sender),
            ..Default::default()
        };
        diagnostics.request(
            Uuid::new_v4(),
            "SECRET-METHOD",
            Some("/api/v1/workspaces/{workspace}/contacts/{id}"),
            403,
            Duration::ZERO,
        );
        diagnostics.request(Uuid::new_v4(), "GET", None, 404, Duration::ZERO);
        assert_eq!(diagnostics.dropped.load(Ordering::Relaxed), 1);
        let serialized = serde_json::to_value(receiver.recv().await.unwrap()).unwrap();
        assert_eq!(
            serialized["operation"],
            "OTHER /api/v1/workspaces/{workspace}/contacts/{id}"
        );
        assert_eq!(serialized.as_object().unwrap().len(), 8);
        assert!(!serialized.to_string().contains("SECRET"));
    }
}
