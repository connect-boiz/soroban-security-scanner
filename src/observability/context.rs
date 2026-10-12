//! Correlation context: the IDs that tie a request's logs and spans together.
//!
//! Uses W3C Trace Context identifiers — a 16-byte trace id (32 hex chars) shared
//! by every span/log of a request, and an 8-byte span id (16 hex chars) for the
//! current operation — plus an application-level request id. The context is
//! propagated across service boundaries via the `traceparent` header.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Correlation identifiers carried with every log record and span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationContext {
    /// 32-hex-char trace id, constant across the whole request.
    pub trace_id: String,
    /// 16-hex-char id of the current span.
    pub span_id: String,
    /// Optional parent span id (set for child spans).
    pub parent_span_id: Option<String>,
    /// Human-facing request id (e.g. surfaced in API responses).
    pub request_id: String,
}

impl CorrelationContext {
    /// Starts a fresh root context with new ids.
    pub fn new_root() -> Self {
        Self {
            trace_id: new_trace_id(),
            span_id: new_span_id(),
            parent_span_id: None,
            request_id: Uuid::new_v4().to_string(),
        }
    }

    /// Derives a child context: same trace and request id, a new span id, and
    /// the current span recorded as parent.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: new_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            request_id: self.request_id.clone(),
        }
    }

    /// Validates that the ids are well-formed W3C identifiers.
    pub fn is_valid(&self) -> bool {
        is_hex_len(&self.trace_id, 32)
            && self.trace_id.bytes().any(|b| b != b'0')
            && is_hex_len(&self.span_id, 16)
            && self.span_id.bytes().any(|b| b != b'0')
    }

    /// Returns the active correlation context for the current async task or thread.
    pub fn current() -> Option<Self> {
        CURRENT_TASK_CONTEXT
            .try_with(|ctx| ctx.clone())
            .ok()
            .or_else(|| CURRENT_THREAD_CONTEXT.with(|cell| cell.borrow().clone()))
    }

    /// Sets the thread-local fallback context (used in sync code and unit tests).
    pub fn set_current(ctx: Option<Self>) {
        CURRENT_THREAD_CONTEXT.with(|cell| {
            *cell.borrow_mut() = ctx;
        });
    }

    /// Executes an async future within the scope of this correlation context.
    pub async fn scope<F: std::future::Future>(ctx: Self, future: F) -> F::Output {
        CURRENT_TASK_CONTEXT.scope(ctx, future).await
    }

    /// Executes a synchronous closure within the scope of this correlation context.
    pub fn scope_sync<R>(ctx: Self, f: impl FnOnce() -> R) -> R {
        let prev = CURRENT_THREAD_CONTEXT.with(|cell| {
            let mut b = cell.borrow_mut();
            let old = b.clone();
            *b = Some(ctx.clone());
            old
        });
        struct Guard(Option<CorrelationContext>);
        impl Drop for Guard {
            fn drop(&mut self) {
                CURRENT_THREAD_CONTEXT.with(|cell| {
                    *cell.borrow_mut() = self.0.take();
                });
            }
        }
        let _guard = Guard(prev);
        f()
    }
}

tokio::task_local! {
    /// Task-local storage for the active correlation context in Tokio tasks.
    pub static CURRENT_TASK_CONTEXT: CorrelationContext;
}

std::thread_local! {
    /// Thread-local storage fallback for synchronous threads.
    static CURRENT_THREAD_CONTEXT: std::cell::RefCell<Option<CorrelationContext>> = const { std::cell::RefCell::new(None) };
}

/// A future that executes with an attached correlation context.
pub struct CorrelatedFuture<F> {
    inner: F,
    context: Option<CorrelationContext>,
}

impl<F: std::future::Future> std::future::Future for CorrelatedFuture<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let (inner, context) = unsafe {
            let this = self.get_unchecked_mut();
            (std::pin::Pin::new_unchecked(&mut this.inner), &this.context)
        };
        match context {
            Some(ctx) => CURRENT_TASK_CONTEXT.sync_scope(ctx.clone(), || {
                CorrelationContext::scope_sync(ctx.clone(), || inner.poll(cx))
            }),
            None => inner.poll(cx),
        }
    }
}

/// Extension trait enabling futures to carry and propagate the correlation context.
pub trait CorrelatedFutureExt: Sized {
    /// Captures the current correlation context and attaches it to the future.
    fn with_current_context(self) -> CorrelatedFuture<Self> {
        self.with_context(CorrelationContext::current())
    }

    /// Attaches an explicit correlation context to the future.
    fn with_context(self, ctx: Option<CorrelationContext>) -> CorrelatedFuture<Self>;
}

impl<F: std::future::Future> CorrelatedFutureExt for F {
    fn with_context(self, ctx: Option<CorrelationContext>) -> CorrelatedFuture<Self> {
        CorrelatedFuture {
            inner: self,
            context: ctx,
        }
    }
}

/// Spawns a Tokio task propagating the current `CorrelationContext`.
pub fn spawn_correlated<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let ctx = CorrelationContext::current();
    tokio::spawn(async move {
        match ctx {
            Some(c) => CorrelationContext::scope(c, future).await,
            None => future.await,
        }
    })
}

/// Spawns a blocking task propagating the current `CorrelationContext`.
pub fn spawn_blocking_correlated<F, R>(f: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let ctx = CorrelationContext::current();
    tokio::task::spawn_blocking(move || match ctx {
        Some(c) => CorrelationContext::scope_sync(c, f),
        None => f(),
    })
}

/// Generates a 32-hex-char trace id.
pub fn new_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Generates a 16-hex-char span id.
pub fn new_span_id() -> String {
    // Use the first 16 hex chars of a fresh UUID.
    Uuid::new_v4().simple().to_string()[..16].to_string()
}

fn is_hex_len(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_context_is_valid_and_unique() {
        let a = CorrelationContext::new_root();
        let b = CorrelationContext::new_root();
        assert!(a.is_valid());
        assert_eq!(a.trace_id.len(), 32);
        assert_eq!(a.span_id.len(), 16);
        assert!(a.parent_span_id.is_none());
        assert_ne!(a.trace_id, b.trace_id);
    }

    #[test]
    fn child_shares_trace_and_request_but_new_span() {
        let root = CorrelationContext::new_root();
        let child = root.child();
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.request_id, root.request_id);
        assert_ne!(child.span_id, root.span_id);
        assert_eq!(child.parent_span_id.as_deref(), Some(root.span_id.as_str()));
    }

    #[test]
    fn invalid_ids_are_detected() {
        let bad = CorrelationContext {
            trace_id: "xyz".to_string(),
            span_id: "0000000000000000".to_string(),
            parent_span_id: None,
            request_id: "r".to_string(),
        };
        assert!(!bad.is_valid());
    }

    #[test]
    fn scope_sync_sets_and_cleans_up_context() {
        assert!(CorrelationContext::current().is_none());
        let ctx = CorrelationContext::new_root();
        let expected_trace = ctx.trace_id.clone();
        CorrelationContext::scope_sync(ctx, || {
            let cur = CorrelationContext::current().expect("context should be present");
            assert_eq!(cur.trace_id, expected_trace);
        });
        assert!(CorrelationContext::current().is_none());
    }

    #[tokio::test]
    async fn scope_async_propagates_in_task() {
        assert!(CorrelationContext::current().is_none());
        let ctx = CorrelationContext::new_root();
        let expected_trace = ctx.trace_id.clone();
        CorrelationContext::scope(ctx, async {
            let cur = CorrelationContext::current().expect("context should be present");
            assert_eq!(cur.trace_id, expected_trace);
        })
        .await;
        assert!(CorrelationContext::current().is_none());
    }

    #[tokio::test]
    async fn future_ext_and_spawn_correlated_propagate_context() {
        let ctx = CorrelationContext::new_root();
        let expected_trace = ctx.trace_id.clone();

        CorrelationContext::scope(ctx.clone(), async {
            // Task spawn using spawn_correlated
            let handle1 = spawn_correlated(async {
                let cur = CorrelationContext::current().expect("context in spawned task");
                cur.trace_id
            });
            let trace1 = handle1.await.unwrap();
            assert_eq!(trace1, expected_trace);

            // Task spawn using with_current_context()
            let fut = async {
                let cur = CorrelationContext::current().expect("context in extension future");
                cur.trace_id
            };
            let handle2 = tokio::spawn(fut.with_current_context());
            let trace2 = handle2.await.unwrap();
            assert_eq!(trace2, expected_trace);

            // Blocking spawn
            let handle3 = spawn_blocking_correlated(|| {
                let cur = CorrelationContext::current().expect("context in blocking task");
                cur.trace_id
            });
            let trace3 = handle3.await.unwrap();
            assert_eq!(trace3, expected_trace);
        })
        .await;
    }
}
