# Observability: Correlation IDs & Distributed Tracing Guide

## 1. Overview

In distributed architectures, requests navigate across multiple microservices and asynchronous workers (such as the API gateway, scanner engine, and notification dispatcher). Without end-to-end correlation, diagnosing failures requires manual timestamp filtering and speculation.

The observability subsystem in `soroban-security-scanner` enforces 100% request traceability by tying structured JSON logs ([`LogRecord`](../src/observability/record.rs)) to W3C-compliant distributed tracing contexts ([`CorrelationContext`](../src/observability/context.rs)).

### Core Correlation Identifiers

| Identifier | Type / Spec | Purpose |
|------------|-------------|---------|
| `trace_id` | 16-byte (32 hex characters) W3C trace ID | Global identifier shared across all services and spans processing a request. |
| `span_id` | 8-byte (16 hex characters) W3C span ID | Local identifier for the current operation or execution unit. |
| `parent_span_id` | Optional 8-byte (16 hex characters) | ID of the upstream caller span for building distributed span trees. |
| `request_id` | UUID v4 or client-provided string | Human-readable identifier returned to API clients via `x-request-id`. |
| `traceparent` | W3C Trace Context Header (`00-<trace_id>-<span_id>-01`) | Wire format exchanged across HTTP and RPC service boundaries. |

---

## 2. Automatic Log Correlation with `Logger::with_context()`

Instead of manually extracting IDs at every log call site, invoke `Logger::with_context()`. This produces a contextual logger instance that reads the active [`CorrelationContext::current()`](../src/observability/context.rs) from task-local or thread-local storage:

```rust
use soroban_security_scanner::observability::{Logger, LogLevel};

// Create or inject base logger
let base_logger: Logger = /* ... */;

// Inside an HTTP handler or async task with an active correlation context:
let logger = base_logger.with_context();

// Every log record emitted automatically includes trace_id, span_id, and request_id:
logger.info(1700000000, "Initiating contract bytecode vulnerability scan");
```

Output record in centralized sinks (JSON):
```json
{
  "timestamp": 1700000000,
  "level": "Info",
  "target": "scanner::engine",
  "message": "Initiating contract bytecode vulnerability scan",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "request_id": "req-9c84e1b8-6f6a-4b72-b1b7-d1a1b4d00123"
}
```

---

## 3. HTTP Ingress: Axum Correlation Middleware

Add [`correlation_middleware`](../src/observability/middleware.rs) to your Axum router:

```rust
use axum::{routing::get, Router};
use soroban_security_scanner::observability::correlation_middleware;

let app = Router::new()
    .route("/api/scan", get(handle_scan))
    .layer(axum::middleware::from_fn(correlation_middleware));
```

### Middleware Lifecycle:
1. **Header Inspection**: Checks for `traceparent` (W3C format) and `x-request-id`.
2. **Context Creation**:
   - If incoming `traceparent` exists, creates a child context under the remote parent.
   - If absent, generates a fresh root trace and UUID request ID.
3. **Task-Local Scoping**: Runs downstream handlers inside `CorrelationContext::scope(...)`.
4. **Response Headers**: Automatically appends `x-request-id` and `traceparent` to the outgoing HTTP response.

---

## 4. Context Propagation Across Async Tasks & Threads

When spawning asynchronous Tokio tasks or background worker threads, task-local storage does not automatically cross the spawn boundary without wrapping. Use our provided propagation primitives:

### Option A: `spawn_correlated` (Tokio Tasks)
```rust
use soroban_security_scanner::observability::spawn_correlated;

spawn_correlated(async move {
    // Automatically inherits parent trace_id and request_id
    logger.with_context().info(ts, "Processing background analysis job");
});
```

### Option B: `.with_current_context()` Extension Trait
```rust
use soroban_security_scanner::observability::CorrelatedFutureExt;

let job = async move {
    logger.with_context().info(ts, "Executing async subtask");
};

tokio::spawn(job.with_current_context());
```

### Option C: `spawn_blocking_correlated` (Compute / Blocking IO)
```rust
use soroban_security_scanner::observability::spawn_blocking_correlated;

spawn_blocking_correlated(move || {
    // Thread-local context is automatically populated on the worker thread
    logger.with_context().info(ts, "Running heavy decompilation analysis");
});
```

---

## 5. Querying Correlated Logs via API

The observability API provides an endpoint to fetch all log records tied to a request:

```http
GET /api/v1/observability/correlation-logs?request_id=<request_id>
```

### Router Integration:
```rust
use soroban_security_scanner::observability::{build_observability_routes, AggregatingSink};
use std::sync::Arc;

let sink = Arc::new(AggregatingSink::new());
let observability_router = build_observability_routes(Arc::clone(&sink));
```

### Example Response:
```json
[
  {
    "timestamp": 1700000000,
    "level": "Info",
    "target": "api::handler",
    "message": "Received scan submission",
    "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
    "span_id": "00f067aa0ba902b7",
    "request_id": "req-9c84e1b8-6f6a-4b72-b1b7-d1a1b4d00123"
  },
  {
    "timestamp": 1700000002,
    "level": "Info",
    "target": "scanner::engine",
    "message": "Bytecode disassembled and rules checked",
    "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
    "span_id": "e9b72a91fa816281",
    "request_id": "req-9c84e1b8-6f6a-4b72-b1b7-d1a1b4d00123"
  }
]
```

Records are sorted chronologically by `timestamp` ascending.

---

## 6. Remediating Missing Correlation Warnings

When a log record at `Info`, `Warn`, or `Error` level is emitted without correlation IDs, `Logger` automatically emits a diagnostic warning:

```text
WARN LogRecord emitted without correlation IDs at src/scanner/engine.rs:142
```

Using `#[track_caller]`, the warning pinpoints the exact file and line number of the offending call site.

To fix the call site:
1. Ensure the calling function is covered by `correlation_middleware`, `CorrelationContext::scope`, or `spawn_correlated`.
2. Call `logger.with_context().info(...)` instead of `logger.info(...)`.
