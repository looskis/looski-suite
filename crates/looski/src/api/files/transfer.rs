// SPDX-License-Identifier: AGPL-3.0-only
//! Bound active file buffers until body completion/drop, including slow clients.
use super::{ApiError, Failure};
use axum::{
    Json,
    body::{Body, Bytes, HttpBody, to_bytes},
    http::StatusCode,
    response::Response,
};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::{Semaphore, SemaphorePermit};
static TRANSFERS: Semaphore = Semaphore::const_new(2);
pub(super) fn acquire() -> Result<SemaphorePermit<'static>, crate::application::OperationError> {
    TRANSFERS
        .try_acquire()
        .map_err(|_| crate::application::OperationError::Unavailable)
}
fn failure(status: StatusCode, code: &str, message: &str) -> Failure {
    (
        status,
        Json(ApiError {
            code: code.into(),
            message: message.into(),
        }),
    )
}
pub(super) async fn upload(body: Body) -> Result<Bytes, Failure> {
    read(body, Duration::from_secs(30)).await
}
async fn read(body: Body, deadline: Duration) -> Result<Bytes, Failure> {
    tokio::time::timeout(deadline, to_bytes(body, 12 * 1024 * 1024))
        .await
        .map_err(|_| {
            failure(
                StatusCode::REQUEST_TIMEOUT,
                "transfer_timeout",
                "File upload timed out",
            )
        })?
        .map_err(|_| {
            failure(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "File request exceeds the transfer limit",
            )
        })
}
pub(super) fn download<T: serde::Serialize>(
    value: &T,
    permit: SemaphorePermit<'static>,
) -> Result<Response, Failure> {
    let bytes = serde_json::to_vec(value).map_err(|_| {
        failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            "File response unavailable",
        )
    })?;
    let mut response = Response::new(Body::new(TransferBody::new(
        bytes.into(),
        permit,
        Duration::from_secs(120),
    )));
    response.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("static header"),
    );
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().expect("static header"));
    Ok(response)
}
struct Buffer {
    bytes: Bytes,
    permit: Option<SemaphorePermit<'static>>,
    expired: bool,
}
struct TransferBody {
    buffer: Arc<Mutex<Buffer>>,
    deadline: tokio::task::JoinHandle<()>,
}
impl TransferBody {
    fn new(bytes: Bytes, permit: SemaphorePermit<'static>, duration: Duration) -> Self {
        let buffer = Arc::new(Mutex::new(Buffer {
            bytes,
            permit: Some(permit),
            expired: false,
        }));
        let weak = Arc::downgrade(&buffer);
        // Hyper may stop polling a body entirely when a peer stops reading.
        // Cancel its large buffer independently of HTTP polling/backpressure.
        let deadline = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            if let Some(buffer) = weak.upgrade() {
                let mut buffer = buffer.lock().expect("transfer buffer mutex");
                buffer.bytes = Bytes::new();
                buffer.permit.take();
                buffer.expired = true;
            }
        });
        Self { buffer, deadline }
    }
}
impl Drop for TransferBody {
    fn drop(&mut self) {
        self.deadline.abort();
    }
}
impl HttpBody for TransferBody {
    type Data = Bytes;
    type Error = std::io::Error;
    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
        let mut buffer = self.buffer.lock().expect("transfer buffer mutex");
        if buffer.expired {
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "File download timed out",
            ))));
        }
        if buffer.bytes.is_empty() {
            buffer.bytes = Bytes::new();
            buffer.permit.take();
            self.deadline.abort();
            return Poll::Ready(None);
        }
        let size = buffer.bytes.len().min(64 * 1024);
        // Copy this small chunk: a zero-copy slice would keep the entire
        // original allocation alive after the deadline clears the buffer.
        let chunk = Bytes::copy_from_slice(&buffer.bytes.split_to(size));
        Poll::Ready(Some(Ok(http_body::Frame::data(chunk))))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct PendingBody;
    impl HttpBody for PendingBody {
        type Data = Bytes;
        type Error = std::io::Error;
        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
            Poll::Pending
        }
    }
    #[tokio::test]
    async fn transfer_permits_bound_outstanding_bodies_and_timeouts_release_them() {
        let semaphore: &'static Semaphore = Box::leak(Box::new(Semaphore::new(1)));
        let mut body = TransferBody::new(
            Bytes::from(vec![0; 131072]),
            semaphore.try_acquire().unwrap(),
            Duration::from_secs(1),
        );
        assert!(semaphore.try_acquire().is_err());
        let chunk = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        assert_eq!(chunk.len(), 65536);
        assert!(semaphore.try_acquire().is_err());
        drop(body);
        assert!(semaphore.try_acquire().is_ok());
        let mut expired = TransferBody::new(
            Bytes::from(vec![0; 131072]),
            semaphore.try_acquire().unwrap(),
            Duration::from_millis(5),
        );
        // Keep the body alive without polling at all. The watchdog must
        // reclaim bytes/permit even though the HTTP consumer does no work.
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(semaphore.try_acquire().is_ok());
        assert!(expired.buffer.lock().unwrap().bytes.is_empty());
        assert!(
            std::future::poll_fn(|cx| Pin::new(&mut expired).poll_frame(cx))
                .await
                .unwrap()
                .is_err()
        );
        assert!(semaphore.try_acquire().is_ok());
        let permit = semaphore.try_acquire().unwrap();
        let error = read(Body::new(PendingBody), Duration::from_millis(5))
            .await
            .unwrap_err();
        assert_eq!(error.0, StatusCode::REQUEST_TIMEOUT);
        drop(permit);
        assert!(semaphore.try_acquire().is_ok());
    }
}
