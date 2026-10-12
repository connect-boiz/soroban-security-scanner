# Soroban Security Scanner — 20 New Standard Issues (#41–60)

> **Created:** July 20, 2026
> **Format:** Follows `.github/ISSUE_TEMPLATE.md` with detailed description + actionable acceptance criteria
> **Pre-existing issues:** #1–30 (ISSUES.md) + #31–40 (CRITICAL_ISSUES.md)

---

## Issue 41: [Observability] Structured Logs Missing Correlation IDs — Impossible to Trace Requests Across Microservices

**Priority:** 🟠 High
**Area:** Observability / Debugging
**Files:** `src/observability/logger.rs`, `src/observability/context.rs`, `src/observability/tracing.rs`

**Description:**
The observability layer in `src/observability/` provides structured logging via `Logger` (`logger.rs`), trace context propagation through `Span` and W3C `traceparent` headers (`tracing.rs`), and a `CorrelationContext` struct (`context.rs`) designed to tie logs and spans together. However, the `CorrelationContext` is not automatically injected into every log record emitted through the `Logger`. Individual call sites must manually call `CorrelationContext::current()` and pass trace IDs, span IDs, and request IDs into each `LogRecord`. In practice, this manual step is frequently omitted — especially in error paths, background workers, and async tasks — resulting in logs that cannot be correlated to a specific user request. When a user reports a failed scan, operators must grep logs by timestamp range and guess which log lines belong to the user's session. The `LogRecord` struct (`record.rs`) has optional `trace_id`, `span_id`, and `request_id` fields, but these are `None` in the majority of production log entries because there is no middleware or wrapper that automatically populates them from the current correlation context. This defeats the purpose of structured logging and makes distributed debugging across the API server, scanner engine, and notification service nearly impossible.

**Acceptance Criteria:**
- [ ] Implement a `Logger::with_context()` method that automatically populates `trace_id`, `span_id`, and `request_id` from `CorrelationContext::current()` on every `LogRecord`
- [ ] Add an Axum middleware layer (or equivalent) that initializes `CorrelationContext` at the start of each HTTP request with a newly generated `request_id` and extracts `traceparent` from incoming headers
- [ ] Ensure that async task spawning (`tokio::spawn`, `tokio::spawn_blocking`) propagates the parent's `CorrelationContext` using `tracing::Span` or a custom `Future` extension trait
- [ ] Add a `GET /api/v1/observability/correlation-logs?request_id=<id>` endpoint that returns all log records associated with a given request, ordered by timestamp
- [ ] Log a WARNING when a `LogRecord` is emitted without correlation IDs, identifying the call site (file + line) for remediation
- [ ] Write integration tests in `src/observability/tests.rs` that verify: (a) correlation IDs are present on log records emitted within an HTTP request handler, (b) logs from spawned async tasks inherit parent correlation context, and (c) the correlation log endpoint returns matching records
- [ ] Update developer documentation with a guide on how to ensure logs are always correlated

---

## Issue 42: [Caching] Cache Warming Strategy Ignores Hot-Key Stampede Protection During Cold Start

**Priority:** 🟠 High
**Area:** Performance / Caching
**Files:** `src/caching/warming.rs`, `src/caching/cache.rs`, `src/caching/monitoring.rs`

**Description:**
The `CacheWarmer` in `src/caching/warming.rs` pre-populates the cache with critical data at application startup, and the `Cache` struct in `cache.rs` implements single-flight stampede protection via `get_or_load()`. However, these two systems operate independently with a dangerous gap: during the cache warming window (the period between application start and completion of `CacheWarmer::warm_all()`), concurrent requests for not-yet-warmed keys bypass the single-flight protection because the warming process populates entries directly via `Cache::put()` rather than through `get_or_load()`. If 100 requests arrive for the same expensive-to-compute scan result during this window, each request independently triggers a database query and computation because the cache key is not yet populated. The `tests.rs` file includes a test `expensive_scan_is_computed_once_under_load` that validates single-flight behavior for a warm cache, but there is no test that simulates concurrent requests arriving during the cold-start warming phase. On application restart in a production environment with hundreds of concurrent users, this can cause a thundering herd that overwhelms the database connection pool, creating a cascading failure where the very thing meant to speed up the application (caching) makes it slower on restart.

**Acceptance Criteria:**
- [ ] Integrate `get_or_load()` semantics into `CacheWarmer::warm_key()` so that concurrent requests for a key being warmed are coalesced into a single computation
- [ ] Add a `warming_in_progress` set to `CacheWarmer` that tracks keys currently being warmed; requests for these keys should await the warming completion via a `tokio::sync::Notify` or similar mechanism
- [ ] Implement a configurable `warm_timeout` (default 30 seconds) after which warming for a key is abandoned and the key is served from the database directly
- [ ] Add a `GET /api/v1/cache/warm-status` endpoint that reports: total keys to warm, keys warmed so far, keys in progress, and ETA for completion
- [ ] Write a concurrency stress test in `src/caching/tests.rs` that spawns 200 simultaneous "requests" during the cold-start window and verifies that an expensive computation is executed at most once per key
- [ ] Update `src/caching/monitoring.rs` to track a `cache_warm_stampede_events` counter

---

## Issue 43: [Query Optimization] N+1 Detection Is Read-Only — No Automatic Prevention or Circuit Breaker

**Priority:** 🟡 Medium
**Area:** Performance / Database
**Files:** `src/query_optimization/nplus1.rs`, `src/query_optimization/engine.rs`, `src/query_optimization/benchmark.rs`

**Description:**
The `NPlusOneDetector` in `src/query_optimization/nplus1.rs` identifies N+1 query patterns by normalizing SQL statements (`normalize.rs`) and tracking repeated queries with varying parameters over a window. It generates suggestions for remediation (e.g., "use eager loading with JOIN" or "batch these queries with WHERE id IN (...)"), but it operates purely in advisory mode — it does not prevent N+1 queries from executing, does not automatically batch or optimize them, and does not have a circuit breaker that blocks the request handler when a dangerously high number of repeated queries is detected. This means that during a code change that accidentally introduces an N+1 pattern (e.g., a developer adds a `for` loop around a database call), the system will silently degrade with no alerts, latency spikes, or automated protection. The `QueryOptimizationEngine` in `engine.rs` provides a facade that integrates N+1 detection, index advisement, and slow query logging, but the `optimize()` method only returns recommendations — it never takes action. In production, a single poorly written query in a hot endpoint can take down the database by multiplying the query load 1000x.

**Acceptance Criteria:**
- [ ] Add a `QueryCircuitBreaker` that tracks the N+1 ratio (total queries / unique queries) per request; if the ratio exceeds `max_n_plus_one_ratio` (default 20:1), abort the request with HTTP 429 and log a CRITICAL alert
- [ ] Implement `auto_batch()` in `nplus1.rs` that automatically rewrites detected N+1 patterns into batch queries using `WHERE id = ANY($1)` when the query shape matches a known pattern
- [ ] Add a `NPlusOnePolicy` enum with variants `Allow`, `Warn`, `Block` configurable per endpoint or globally
- [ ] Integrate the circuit breaker into the database connection layer (`src/database/connection.rs`) so it applies to all queries regardless of call site
- [ ] Emit a Prometheus counter `n_plus_one_queries_blocked_total` and a gauge `n_plus_one_ratio_current`
- [ ] Write a benchmark in `src/query_optimization/benchmark.rs` that demonstrates a 1000x N+1 pattern and verifies the circuit breaker triggers before the 21st repeated query
- [ ] Document the circuit breaker configuration and rollout strategy in `PERFORMANCE_OPTIMIZATIONS.md`

---

## Issue 44: [Invariants] Invariant Engine Checks Only Point-in-Time State Equality — Temporal Invariants Not Verified

**Priority:** 🟡 Medium
**Area:** Core Scanner / Correctness
**Files:** `src/invariants.rs`, `src/security_analyzer.rs`

**Description:**
The `InvariantRule` enum in `src/invariants.rs` defines 24 invariants covering token economics (TotalSupplyConsistency, BalanceNonNegative, TransferConservation), access control (AdminAuthorization, OwnershipConsistency), mathematics (OverflowProtection), and Stellar-specific rules. Each rule is checked by regex pattern matching (`check_pattern()`) against contract source code, verifying that the code contains specific patterns expected to maintain the invariant. However, all invariants are **point-in-time** checks — they verify that a contract's source code at a single snapshot appears to satisfy the invariant. They do not verify **temporal invariants** that must hold across sequences of operations. For example, `TimestampMonotonicity` checks that the code contains `timestamp > prev_timestamp`, but it cannot verify that no execution path could ever decrease the timestamp. `ConservationOfValue` checks for `value.*conservation` in the source, but it cannot verify that total value is actually conserved across all possible transaction sequences. Real Soroban vulnerabilities often arise from temporal property violations (e.g., a reentrancy attack that causes the `SumOfBalancesEqualsSupply` invariant to be temporarily violated between a balance decrease and a supply decrease). The invariant engine has no symbolic execution, model checking, or temporal logic capability — it is fundamentally a static pattern matcher.

**Acceptance Criteria:**
- [ ] Add a `TemporalInvariant` enum with variants: `AlwaysHolds`, `EventuallyHolds`, `NeverViolated`, `MonotonicIncrease`, `MonotonicDecrease`
- [ ] Implement a `SequenceDiagram` generator that extracts all possible state transitions from a contract's public function call graph and represents them as a directed graph
- [ ] For each `InvariantRule`, define a `temporal_check()` method that traverses the state transition graph and verifies the temporal property holds on all paths up to `max_depth` (configurable, default 5 transitions)
- [ ] Add a `--temporal-depth` CLI flag to `stellar-scanner scan` controlling how deep the path exploration goes
- [ ] Write conformance tests: provide a contract known to violate `ConservationOfValue` over a 3-step transfer → mint → burn sequence and verify the temporal check catches it
- [ ] Document temporal checking limitations (path explosion, state space coverage) in a new `docs/TEMPORAL_INVARIANTS.md`

---

## Issue 45: [Upload Sanitization] Deep Inspection Does Not Validate WASM Function Signatures Against Stellar Environment Interface

**Priority:** 🟠 High
**Area:** Security / Input Validation
**Files:** `src/upload_sanitization/wasm.rs`, `src/upload_sanitization/deep_inspection.rs`, `src/upload_sanitization/sanitize.rs`

**Description:**
The upload sanitization pipeline in `src/upload_sanitization/` validates uploaded WASM binaries through multiple stages: magic byte verification (`magic.rs`), malware signature scanning (`malware.rs`), content type checks (`content_type.rs`), and a deep inspection module (`deep_inspection.rs`) that examines WASM structure. However, the `deep_inspection.rs` module checks for structural validity of the WASM module but does **not validate that the module's exported function signatures conform to the expected Soroban contract interface** (the Stellar Environment Interface / SEI). A malicious actor can upload a WASM module that is structurally valid (passes all current checks) but whose exported functions have intentionally mismatched signatures — for example, a function named `transfer` that takes a `u64` instead of the expected `(Address, Address, i128)`, or a function named `init` that takes no parameters when the scanner expects initialization parameters. During scanning, calling these functions with the expected arguments causes the WASM runtime to trap with an opaque error, which the scanner misinterprets as a vulnerability or silently ignores. Worse, a contract designed to exploit specific scanner behavior could implement function signatures that trigger edge cases in the analysis engine itself.

**Acceptance Criteria:**
- [ ] Define a `SorobanContractInterface` struct specifying the expected function signatures for standard Soroban contract entry points (`init`, `transfer`, `allowance`, `approve`, `balance`, `mint`, `burn`, `upgrade`)
- [ ] Add `validate_function_signatures()` to `deep_inspection.rs` that parses the WASM export section, extracts each exported function's parameter and return types, and compares them against `SorobanContractInterface`
- [ ] For functions not matching any known interface, emit a `WARNING`-level event with the function name and mismatched signature details
- [ ] Add a `--strict-signature-check` CLI flag that rejects uploads with any signature mismatches
- [ ] Implement `SorobanContractInterface` as a configurable JSON file (`config/sei-interface.json`) that can be updated when Stellar updates the SEI specification
- [ ] Write adversarial tests: upload a WASM module with a `transfer` function that takes `(Address, i64)` instead of `(Address, Address, i128)` and verify the mismatch is detected and reported
- [ ] Document the signature validation in `docs/BUSINESS_CONTINUITY.md` under the security hardening section

---

## Issue 46: [Frontend] Form Components Lack Accessible Error Announcements for Screen Reader Users

**Priority:** 🟡 Medium
**Area:** Accessibility / Frontend
**Files:** `frontend/components/form/Form.tsx`, `frontend/components/form/FormField.tsx`, `frontend/components/form/FormErrorSummary.tsx`

**Description:**
The form system in `frontend/components/form/` provides `Form`, `FormField`, `FormErrorSummary`, and `FormProgress` components for building accessible forms. `FormErrorSummary` collects validation errors and displays them at the top of the form, which is a good accessibility pattern. However, when a form submission fails client-side validation (e.g., an invalid Stellar address, a missing required field), the error messages are only rendered visually — they are not announced to screen readers via an ARIA live region. The `aria-live` attribute is not set on the error summary container, and individual field errors do not use `aria-describedby` to associate error messages with their input fields. A screen reader user submitting an invalid form hears no feedback indicating what went wrong, making them believe the form simply "did nothing." The `FormField` component renders an error `<span>` below each input, but this span has no `role="alert"` and is not referenced by the input's `aria-describedby`. For a platform whose documentation claims WCAG 2.1 AA compliance (as stated in `docs/ACCESSIBILITY_TESTING.md`), this is a fundamental gap that would be flagged by any manual screen reader audit.

**Acceptance Criteria:**
- [ ] Add `role="alert"` and `aria-live="assertive"` to the `FormErrorSummary` container so that errors are announced immediately when they appear
- [ ] Generate a unique `aria-describedby` ID for each `FormField`'s error message and wire it to the input element's `aria-describedby` attribute
- [ ] When validation errors change (e.g., user corrects one field but another still has an error), announce the updated error summary text via a visually hidden live region
- [ ] Add a `formAnnouncements` live region to `Form.tsx` that reads out success messages (e.g., "Form submitted successfully") after a successful submission
- [ ] Ensure `FormProgress` (multi-step forms) announces the current step and total steps when the step changes
- [ ] Write accessibility tests using `@axe-core/playwright` that verify `aria-describedby` associations exist for all form fields with errors
- [ ] Add a manual testing checklist item to `docs/ACCESSIBILITY_TESTING.md` for screen reader testing of form validation flows

---

## Issue 47: [Analytics Dashboard] Chart Components Render Blank or Broken States When Data Is Empty

**Priority:** 🟢 Low
**Area:** Frontend / User Experience
**Files:** `frontend/components/charts/PortfolioChart.tsx`, `frontend/components/charts/PerformanceChart.tsx`, `frontend/components/charts/TransactionChart.tsx`, `frontend/components/AnalyticsDashboard.tsx`

**Description:**
The chart components in `frontend/components/charts/` — `PortfolioChart`, `PerformanceChart`, and `TransactionChart` — render data visualizations for the `AnalyticsDashboard`. These components receive data arrays as props and pass them to a charting library for rendering. However, when a user has no historical data (e.g., a new user who hasn't performed any scans yet, or a researcher browsing a time range with no activity), the chart components render either an empty plot area with axes but no data, a cryptic error from the charting library about "min ≥ max" on scales, or in some cases a completely blank white rectangle. None of the chart components implement an `EmptyState` sub-component that displays a helpful message like "No scan data for this period — run your first scan to see analytics here" with an illustration or call-to-action button. This creates a confusing and broken-looking experience for new users, who may interpret the blank charts as a malfunction rather than an expected empty state. The `AnalyticsDashboard` parent component also does not provide a fallback when all child charts are empty.

**Acceptance Criteria:**
- [ ] Add an `isEmpty` check at the start of each chart component's render logic; if `true`, render an `EmptyChartState` sub-component instead of the chart
- [ ] The `EmptyChartState` should display: a relevant icon/illustration, a context-specific message (e.g., "No transactions yet" vs. "No vulnerability trends — run a scan to populate"), and a CTA button when applicable
- [ ] Implement a `hasAnyData()` utility in `AnalyticsDashboard.tsx` that checks all chart data sources; if all are empty, render a single "Welcome to Analytics" empty state with a link to the scan page
- [ ] Ensure empty states pass WCAG 2.1 AA contrast requirements and have appropriate `aria-label` attributes
- [ ] Write a Storybook story in `components/charts/` (create if needed) that demonstrates each chart's empty state
- [ ] Write a unit test verifying that each chart component renders the empty state when passed an empty data array

---

## Issue 48: [Session Management] Stateless JWT Sessions Cannot Be Force-Invalidated on Password Change or Account Compromise

**Priority:** 🟠 High
**Area:** Security / Authentication
**Files:** `src/auth/jwt.rs`, `src/auth/session_manager.rs`, `src/session/session.rs`

**Description:**
The authentication system in `src/auth/` uses stateless JWT tokens for session management. The `JwtManager` in `jwt.rs` generates and validates tokens, and the `SessionManager` in `session_manager.rs` tracks active sessions. The `src/session/` module provides both stateful (`session.rs`) and stateless (`stateless.rs`) session implementations. When a user changes their password, or when an account compromise is detected, there is no mechanism to invalidate **all existing JWTs** issued to that user before the password change. Because JWTs are stateless (they are validated by checking the signature, not by a server-side lookup), any token issued before the password change remains valid until its natural expiration. This means an attacker who obtained a valid JWT before the victim changed their password can continue to access the account for the remaining lifetime of the token (potentially hours or days depending on the configured TTL). The `SessionManager` has a `revoke_session()` method for stateful sessions, but this is not integrated with the JWT validation path — `validate_token()` in `jwt.rs` only checks signature and expiry, with no call to `is_session_revoked()`. The `stateless.rs` session implementation explicitly documents this limitation but provides no solution.

**Acceptance Criteria:**
- [ ] Implement a `TokenRevocationList` (backed by Redis or database) that stores revoked JWT IDs (`jti` claims) with their expiry time
- [ ] Add a unique `jti` (JWT ID) claim to every issued token
- [ ] Update `validate_token()` in `jwt.rs` to check the `jti` against the `TokenRevocationList` after signature verification and before accepting the token
- [ ] When a user changes their password, call `revoke_all_user_tokens(user_id)` which adds all active `jti` values for that user to the revocation list
- [ ] Implement a background job that periodically prunes expired entries from the revocation list (entries past their original token expiry)
- [ ] Add a `POST /api/v1/auth/revoke-all-sessions` endpoint (requires current password for confirmation) so users can self-service revoke all active sessions
- [ ] Write integration tests: (a) issue a JWT, change password, verify the old JWT is rejected, (b) issue multiple JWTs, revoke all, verify all are rejected
- [ ] Document the revocation mechanism and its impact on stateless token semantics in `AUTHENTICATION_README.md`

---

## Issue 49: [DB Pool] Connection Pool Health Metrics Not Exposed to Prometheus/Grafana for Operational Monitoring

**Priority:** 🟡 Medium
**Area:** Infrastructure / Observability
**Files:** `src/db_pool/pool.rs`, `src/db_pool/monitoring.rs`, `src/db_pool/config.rs`

**Description:**
The database connection pool in `src/db_pool/` provides secure connection pooling with monitoring (`monitoring.rs`), SSL support (`ssl.rs`), replica routing (`replica.rs`), and retry logic (`retry.rs`). The `PoolMonitor` in `monitoring.rs` tracks internal pool metrics including active connections, idle connections, wait queue depth, and connection acquisition latency. However, these metrics are only logged to the application log stream — they are **not exposed as Prometheus metrics** for ingestion by the Grafana monitoring stack documented in `README.md` (Prometheus + Grafana). The `src/observability/metrics.rs` module provides infrastructure for exposing application metrics, but there is no bridge between `PoolMonitor` and `LogMetricsCollector`. Operations teams cannot set up Grafana alerts for critical conditions like "connection pool at 90% capacity" or "connection acquisition latency exceeding 500ms," which means they learn about pool exhaustion only after users report timeouts. For the 99.9% uptime target stated in `docs/DISASTER_RECOVERY.md`, automated pool monitoring with alerting thresholds is essential.

**Acceptance Criteria:**
- [ ] Register pool metrics as Prometheus gauges: `db_pool_active_connections`, `db_pool_idle_connections`, `db_pool_wait_queue_depth`, `db_pool_total_connections`
- [ ] Register a Prometheus histogram: `db_pool_connection_acquire_duration_seconds` with buckets `[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]`
- [ ] Update `PoolMonitor` to push metrics to the `LogMetricsCollector` on each `tick()` interval (default every 15 seconds)
- [ ] Add a `GET /api/v1/db-pool/metrics` endpoint returning JSON with current pool stats for ad-hoc debugging
- [ ] Create a Grafana dashboard JSON template (`grafana/db-pool-dashboard.json`) with panels for each metric, including a "Pool Utilization %" gauge and a "Connection Wait Time" time series
- [ ] Add alert thresholds to `PoolMonitor`: WARN at 80% utilization, CRITICAL at 95% utilization, with configurable hysteresis
- [ ] Write a test that simulates pool exhaustion and verifies the WARN and CRITICAL metrics are emitted

---

## Issue 50: [Address Filter] No Integration with External Threat Intelligence Feeds for Known Malicious Addresses

**Priority:** 🟡 Medium
**Area:** Security / Intelligence
**Files:** `src/address_filter.rs`, `src/scanner_registry.rs`

**Description:**
The `AddressFilter` in `src/address_filter.rs` provides whitelist/blacklist management for Stellar addresses with category tagging, file-based import/export (CSV and JSON), regex pattern matching, and expiration support. However, the blacklist is populated entirely through manual addition — either via individual `add_to_blacklist()` calls or bulk import from local files. There is no integration with any external threat intelligence feed that tracks known malicious Stellar addresses (phishing contracts, rug-pull deployers, exploit contract addresses). The `ScannerRegistry` in `src/scanner_registry.rs` manages scanner versions and plugins, suggesting a plugin architecture exists, but no `AddressIntelPlugin` trait is defined that would allow subscribing to external feeds. Security researchers relying on the scanner must manually curate and maintain their own blacklists, which quickly become stale as new malicious addresses appear daily. The `AddressFilterConfig` has no `threat_intel_feeds` configuration section. For a security scanning platform, automated threat intelligence integration is a key differentiator that would set it apart from local static analysis tools.

**Acceptance Criteria:**
- [ ] Define a `ThreatIntelFeed` trait with methods: `fetch_indicators() -> Vec<AddressEntry>`, `source_name() -> &str`, and `update_interval() -> Duration`
- [ ] Implement built-in feeds: `StellarExpertKnownScams` (scrapes or queries StellarExpert's known scam list), `StellarGuardBlacklist`, and a generic `JsonHttpsFeed` for custom URLs returning JSON arrays of addresses
- [ ] Add a `threat_intel_feeds` configuration section to `AddressFilterConfig` with per-feed enable/disable, update interval, and API key fields
- [ ] Implement a background refresh loop in `AddressFilter` that periodically fetches indicators from enabled feeds and merges them into the blacklist
- [ ] Add deduplication logic: if a manually-added entry and a feed entry have the same address, the manual entry takes precedence and is not overwritten
- [ ] Add a `GET /api/v1/address-filter/threat-feeds` endpoint listing configured feeds, last sync time, and indicator counts per feed
- [ ] Write tests with a mock `ThreatIntelFeed` that returns known test vectors and verify the auto-merge, deduplication, and expiration behavior
- [ ] Document the threat intelligence integration in a new `docs/THREAT_INTELLIGENCE.md`

---

## Issue 51: [Security Headers] CSP Report-Only Mode Has No Dashboard — Violation Reports Are Logged But Not Actionable

**Priority:** 🟡 Medium
**Area:** Security / Monitoring
**Files:** `src/security_headers/csp.rs`, `src/security_headers/monitoring.rs`, `src/security_headers/builder.rs`

**Description:**
The Content Security Policy (CSP) module in `src/security_headers/csp.rs` generates CSP headers for the application, with support for `Content-Security-Policy` (enforcing) and `Content-Security-Policy-Report-Only` (monitoring) modes. The `report-uri` and `report-to` directives are configured to send violation reports to an internal endpoint. The `SecurityHeadersMonitor` in `monitoring.rs` logs each CSP violation report as a structured log event. However, there is no dashboard, aggregation UI, or alerting mechanism that makes these violation reports actionable. An operator would need to manually grep application logs to find CSP violations, correlate them by directive, and determine whether they represent a real security issue or a false positive from a new feature deployment. The `csp.rs` module has a `CspBuilder` that constructs policies, but there is no `CspViolationAggregator` that groups violations, tracks trends, or alerts on spikes. For a platform that ships security headers as a feature (per commit `feat(security): comprehensive security headers and CSP (#341)`), the lack of violation observability means operators deploy CSP in report-only mode and then never graduate to enforcing mode because they have no confidence in the violation data.

**Acceptance Criteria:**
- [ ] Implement a `CspViolationStore` that persists violation reports to the database (create migration `009_add_csp_violations.sql`) with columns: `directive`, `blocked_uri`, `document_uri`, `source_file`, `line_number`, `violated_at`, `user_agent`
- [ ] Add a `GET /api/v1/security/csp-violations` endpoint with filtering by directive, date range, and blocked URI; support pagination
- [ ] Add a `GET /api/v1/security/csp-dashboard` endpoint returning aggregations: violations by directive (count, trend), top blocked URIs, violations over time
- [ ] Create a frontend `CspDashboard` component at `frontend/components/CspDashboard.tsx` accessible from the admin panel, displaying: violation trend chart, directive breakdown pie chart, and a table of recent violations
- [ ] Implement a `CspPolicyGraduationCheck` that analyzes 30 days of violation data and recommends whether the policy is safe to move from report-only to enforcing mode
- [ ] Add alerting: if a directive has zero violations for 7 consecutive days, suggest tightening it; if a new directive (from a deployment) spikes violations, alert via `LogAlerter`
- [ ] Write integration tests that submit synthetic CSP violation reports and verify they appear in the dashboard aggregations

---

## Issue 52: [Frontend] File Upload Zone Accepts Files Based on Extension Only — No Client-Side Magic Byte Validation

**Priority:** 🟡 Medium
**Area:** Security / Frontend
**Files:** `frontend/components/FileUploadZone.tsx`, `frontend/hooks/useFileUpload.ts`, `src/upload_sanitization/magic.rs`

**Description:**
The `FileUploadZone` component in `frontend/components/FileUploadZone.tsx` provides drag-and-drop file upload for contract source code and WASM binaries. The backend `upload_sanitization/magic.rs` validates uploaded files by checking magic bytes (file signatures) server-side, rejecting files whose content does not match their claimed type. However, the **frontend** `useFileUpload` hook (`frontend/hooks/useFileUpload.ts`) validates files based solely on file extension (`.rs`, `.wasm`) and MIME type from the browser's `File.type` property — both of which are client-controlled and can be trivially spoofed. A user can rename `malware.exe` to `contract.wasm`, and the frontend will happily accept it and begin uploading. The upload only fails when the backend `magic.rs` rejects it, wasting bandwidth and server resources on invalid files. Additionally, there is no client-side file size validation, so a user could attempt to upload a 500MB file (masquerading as a `.wasm`), saturating their upload bandwidth and the server's request body parser. The `FileUploadZone` component has no progress feedback beyond a basic spinner, so users don't know if an upload is progressing or stuck.

**Acceptance Criteria:**
- [ ] Implement client-side magic byte detection in `useFileUpload.ts`: read the first 4 bytes of the file using `FileReader.readAsArrayBuffer()` and validate against known WASM magic bytes (`\0asm`) and Rust source (`use`, `//!`, `#!`, `pub`, `mod`, `fn`, `impl`, `trait` — check first non-whitespace line)
- [ ] Add a `maxFileSize` prop to `FileUploadZone` (default 10MB for `.wasm`, 5MB for `.rs`) and reject oversized files with an inline error message before upload begins
- [ ] Add a `FileUploadProgress` sub-component showing: file name, upload percentage bar, upload speed (KB/s), and estimated time remaining
- [ ] Implement upload cancellation: a "Cancel" button that calls `XMLHttpRequest.abort()` or `AbortController.abort()` on the in-flight upload
- [ ] For files rejected client-side (wrong magic bytes, too large), show a specific error message: "This file does not appear to be a valid WASM binary" vs. "File exceeds the 10MB size limit"
- [ ] Write unit tests for the magic byte detection covering: valid `.wasm` with correct header, `.wasm` renamed from `.exe`, empty file, truncated file, and valid `.rs` file
- [ ] Add a Playwright end-to-end test that attempts to upload a non-WASM file and verifies the client-side error is displayed without a network request being made

---

## Issue 53: [Escrow] Multi-Party Escrow Lacks Timeout Auto-Refund — Funds Can Be Locked Indefinitely

**Priority:** 🟠 High
**Area:** Smart Contracts / Fund Safety
**Files:** `src/escrow.rs`, `contracts/`

**Description:**
The `Escrow` contract in `src/escrow.rs` implements a simple two-party escrow with `create_escrow()` and `release()` functions. Funds are deposited by the contract deployer and can be released to the beneficiary via the `release()` function, which includes reentrancy protection (state updated before external call). However, the contract has **no timeout mechanism**. If the depositor creates an escrow and then loses access to their keys, or if the beneficiary address is invalid, or if the release conditions are never met, the funds remain locked in the escrow contract **forever**. There is no `refund()` function, no `cancel()` function, and no time-based automatic refund that returns funds to the depositor after a deadline. The `EscrowData` struct has no `created_at`, `expires_at`, or `timeout` fields. For a platform that handles real user funds via the bounty marketplace (`bounty_marketplace.rs`), this is a critical fund-safety issue. A user who accidentally creates an escrow with a typo in the beneficiary address has no recourse to recover their funds. The `docs/UPGRADE_MECHANISM.md` mentions emergency procedures but these only apply to contract upgrades, not to stuck escrow funds.

**Acceptance Criteria:**
- [ ] Add `created_at: u64` (ledger timestamp), `timeout_seconds: u64`, and `depositor: Address` fields to `EscrowData`
- [ ] Add a `create_escrow_with_timeout(env, beneficiary, amount, timeout_seconds)` function that records the creation timestamp and timeout
- [ ] Implement a `refund(env)` function callable by the depositor after `timeout_seconds` has elapsed, which returns the escrowed funds to the depositor
- [ ] Add a `cancel(env)` function callable by the depositor **before** the escrow is released, which returns the funds (separate from refund — no timeout requirement, but can only be used before any release attempt)
- [ ] Add a configurable minimum timeout (default 1 hour / 3600 ledger closes, minimum 5 minutes) to prevent accidental zero-timeout escrows
- [ ] Emit `EscrowRefunded` and `EscrowCancelled` events via event logging for audit trail
- [ ] Write tests: (a) create escrow with timeout, wait past timeout, verify depositor can refund, (b) verify beneficiary cannot refund, (c) verify refund fails before timeout, (d) verify cancel works before release, (e) verify cancel fails after release
- [ ] Document the timeout/refund mechanism in `BATCH_OPERATIONS.md` (or a new `ESCROW.md`)

---

## Issue 54: [Performance] Web Vitals Monitoring Lacks Alerting Thresholds — Degradation Goes Unnoticed

**Priority:** 🟡 Medium
**Area:** Performance / Monitoring
**Files:** `frontend/hooks/usePerformanceMonitoring.ts`, `src/performance/web-vitals.js`, `src/observability/alerting.rs`

**Description:**
The frontend `usePerformanceMonitoring` hook (`frontend/hooks/usePerformanceMonitoring.ts`) and the backend web vitals collector (`src/performance/web-vitals.js`) track Core Web Vitals metrics: Largest Contentful Paint (LCP), First Input Delay (FID), Cumulative Layout Shift (CLS), Interaction to Next Paint (INP), and Time to First Byte (TTFB). These metrics are collected, logged, and exposed through the `web-vitals.test.js` test suite which validates the collection infrastructure. However, there are **no alerting thresholds** configured that would notify the operations team when web vitals degrade. If a new deployment accidentally increases LCP from 2.1s to 4.5s on mobile, no one is alerted — the degradation is only discoverable if someone manually checks a Grafana dashboard or if users begin complaining. The `LogAlerter` in `src/observability/alerting.rs` detects error patterns and rate anomalies in application logs, but web vitals metrics are not routed through this alerting pipeline. The `lighthouserc.js` configuration sets thresholds for Lighthouse CI, but Lighthouse CI only runs on PR builds, not continuously in production. Real users on slow connections or older devices could experience severe performance degradation with no automated detection.

**Acceptance Criteria:**
- [ ] Define web vitals alert thresholds in a configuration file: LCP > 4000ms (CRITICAL), LCP > 2500ms (WARNING); CLS > 0.25 (CRITICAL), CLS > 0.1 (WARNING); INP > 500ms (CRITICAL), INP > 200ms (WARNING); TTFB > 3000ms (CRITICAL), TTFB > 800ms (WARNING)
- [ ] Implement a `VitalsAlerter` in `src/performance/` that evaluates the p75 of each metric over a rolling 5-minute window and triggers alerts via `LogAlerter` when thresholds are exceeded
- [ ] Add Prometheus gauges for each web vital metric's p75 value over the rolling window
- [ ] Add a `GET /api/v1/performance/web-vitals/alerts` endpoint returning any currently active web vitals alerts
- [ ] Create a `PerformanceAlertBanner` component in the frontend admin panel that displays when a web vitals alert is active
- [ ] Integrate with the incident response playbook in `docs/INCIDENT_RESPONSE.md`: add a "Performance Degradation" runbook entry
- [ ] Write a test that simulates a spike in LCP and verifies the alert is triggered within the 5-minute evaluation window

---

## Issue 55: [Security Monitoring] Anomaly Detection Has No Baseline Learning Period — Normal Behavior Flagged as Suspicious on Day One

**Priority:** 🟡 Medium
**Area:** Security / Monitoring
**Files:** `src/security_monitoring/anomaly.rs`, `src/security_monitoring/detection.rs`, `src/security_monitoring/engine.rs`

**Description:**
The `AnomalyDetector` in `src/security_monitoring/anomaly.rs` identifies anomalous behavior in the platform, and the `DetectionEngine` in `detection.rs` correlates detection events into security incidents. However, the anomaly detection algorithm begins evaluating behavior **immediately** on startup with no baseline learning period. When the system is first deployed (or after a configuration change, or after a major version upgrade), all normal behavior patterns are flagged as anomalous because the model has no historical data to compare against. For example: a new deployment where 50 users perform their first-ever scans — the anomaly detector sees "50 scans in 5 minutes" with no prior data, flags it as a potential DDoS, and triggers a `SEV2` incident. This creates alert fatigue for operations teams during the most critical period (post-deployment) when they need to be focused on genuine issues. The `SecurityMonitoringEngine` in `engine.rs` starts all detection modules in parallel without coordinating a learning phase. There is no `BaselineLearner` that observes traffic for a configurable period and establishes normal ranges before anomaly detection becomes active.

**Acceptance Criteria:**
- [ ] Implement a `BaselineLearner` that collects metrics for a configurable `learning_period_seconds` (default 3600 seconds / 1 hour) on initial deployment or after a reset
- [ ] During the learning period, anomaly detection runs in `observation` mode: it computes anomaly scores but does not generate alerts or incidents
- [ ] After the learning period, the `BaselineLearner` computes baseline statistics (mean, standard deviation, p95, p99) for each metric and activates anomaly detection with these baselines
- [ ] Add a `baseline_status` field to `SecurityMonitoringEngine` with states: `Learning`, `Active`, `Resetting`, `Degraded` (when baseline is stale due to missing data)
- [ ] Add a `POST /api/v1/security-monitoring/reset-baseline` endpoint (admin-only) for manually resetting the baseline after a major infrastructure change
- [ ] Implement baseline expiry: if baseline data is more than 30 days old, transition to `Degraded` state and log a WARNING recommending a reset
- [ ] Write tests that verify anomaly alerts are suppressed during the learning period and activate correctly after the learning period ends
- [ ] Document the baseline learning process in `docs/INCIDENT_RESPONSE.md`

---

## Issue 56: [i18n] RTL Language Support (Arabic, Hebrew) Not Implemented in Frontend Layout

**Priority:** 🟢 Low
**Area:** Internationalization / Accessibility
**Files:** `frontend/app/layout.tsx`, `component-library/src/i18n/config.ts`, `frontend/tailwind.config.a11y.js`

**Description:**
The internationalization system (`src/i18n/config.js`, `component-library/src/i18n/config.ts`) supports 15 locales with translation files for UI labels and content. However, the frontend layout (`frontend/app/layout.tsx`) does not support right-to-left (RTL) text direction for Arabic (`ar`), Hebrew (`he`), or other RTL languages. The HTML root element always uses `dir="ltr"`, and the Tailwind CSS configuration (`tailwind.config.a11y.js`) does not enable RTL variants (`rtl:` prefix) for margin, padding, text alignment, and flexbox direction utilities. When a user selects Arabic as their locale, the translated text renders correctly from right to left (because the browser handles Unicode bidirectional text), but the entire page layout remains left-to-right — the sidebar is on the left, navigation flows left-to-right, form labels are left-aligned, and icons point in the wrong direction. This creates a disjointed and unprofessional experience for RTL users where text flows right-to-left but the visual hierarchy still assumes LTR reading patterns. The `I18N_README.md` documents translation coverage but does not address layout direction.

**Acceptance Criteria:**
- [x] Add `dir` attribute to the `<html>` element in `layout.tsx` that changes to `"rtl"` when the active locale is Arabic or Hebrew, based on a `RTL_LOCALES` constant
- [x] Enable `rtl:` variants in Tailwind CSS configuration (using `tailwindcss-rtl` plugin or manual `safelist`)
- [x] Implement a `useTextDirection()` hook in `frontend/hooks/` that returns `"ltr"` or `"rtl"` based on the active locale
- [x] Create RTL-aware wrapper components in `component-library/src/components/` for layout primitives: `RtlContainer` (applies `dir` and swaps flex direction), `RtlAwareIcon` (mirrors directional icons like arrows)
- [x] Update `A11yPrimitives.tsx` to swap `left`/`right` ARIA attributes based on text direction
- [x] Verify with a visual test: Arabic locale with sidebar on the right, content flowing RTL, form labels right-aligned, and arrow icons pointing correctly
- [x] Document RTL support setup in `I18N_README.md` with a section on adding new RTL locales

---

## Issue 57: [Supply Chain] SBOM Generation Excludes Transitive Dependency Licenses — Incomplete Compliance Reporting

**Priority:** 🟡 Medium
**Area:** Security / Compliance
**Files:** `src/supply_chain/sbom.rs`, `src/supply_chain/inventory.rs`, `src/supply_chain/policy.rs`

**Description:**
The Software Bill of Materials (SBOM) generator in `src/supply_chain/sbom.rs` produces SBOM documents in SPDX and CycloneDX formats, enumerating direct dependencies with their versions, ecosystems, and declared licenses. The `DependencyInventory` in `inventory.rs` tracks package metadata including license information. However, the SBOM generator only includes **direct dependencies** (those listed in `Cargo.toml` and `package.json`) and does **not** recursively resolve and include **transitive dependencies** with their licenses. For compliance with open-source license policies (e.g., ensuring no GPL-licensed transitive dependency is pulled in), transitive license information is essential. A direct dependency with an MIT license could pull in a transitive dependency with a GPL or AGPL license that requires disclosure or imposes copyleft obligations. The `PolicyEngine` in `policy.rs` evaluates license compliance rules, but it only evaluates direct dependencies because that's all the inventory contains. For enterprise customers requiring SOC 2 Type II or FedRAMP compliance, incomplete SBOM data with missing transitive license information is a blocker for procurement.

**Acceptance Criteria:**
- [ ] Implement `resolve_transitive_dependencies()` in `inventory.rs` that walks the full dependency tree using `cargo metadata` (Rust) and `npm ls --json` (Node.js) and populates a `DependencyGraph` with parent-child relationships
- [ ] Add a `transitive` boolean field to `Dependency` to distinguish direct vs. transitive dependencies
- [ ] Update `sbom.rs` to include transitive dependencies in SPDX and CycloneDX output, with a `DEPENDENCY_OF` relationship linking each transitive dependency to its parent
- [ ] Add a `--include-transitive` flag to the SBOM generation CLI (default `true`) and a `--max-depth` flag to limit recursion depth
- [ ] Update `PolicyEngine` license checks to evaluate transitive dependencies, with a `license_policy.scope` config option: `direct_only` (current behavior), `direct_and_transitive` (new default), or `transitive_ignore_list` (allow specific GPL packages)
- [ ] Write tests: (a) create a mock dependency tree with 3 levels, verify SBOM includes all levels, (b) verify a GPL transitive dependency triggers a policy violation, (c) verify the `transitive_ignore_list` allows specific exceptions
- [ ] Document the transitive dependency resolution in `docs/BUSINESS_CONTINUITY.md` under compliance

---

## Issue 58: [Scanner] No Incremental Scan Support — Full Re-Scan on Every File Change Wastes Resources

**Priority:** 🟡 Medium
**Area:** Core Scanner / Performance
**Files:** `src/scanners.rs`, `src/analysis.rs`, `src/security_analyzer.rs`

**Description:**
The `SecurityScanner` in `src/scanners.rs` performs a full scan of all contract files in a project directory every time a scan is triggered. There is no incremental scan mode that re-scans only files that have changed since the last scan. For large projects with hundreds of contract files (e.g., a DeFi protocol with 50+ contracts), a full scan can take 10-30 minutes. When a developer makes a small change to a single file and wants to re-check it, the entire project is re-scanned, wasting CPU, memory, and time. The scanner has no concept of a "scan session" with persistent state, no file modification timestamp tracking, and no change-detection mechanism (e.g., comparing file hashes against a previous scan manifest). The `analysis.rs` module processes files sequentially with no dependency graph to understand which files might be affected by a change in an imported module. For the continuous integration use case (scanning on every push), incremental scanning would reduce median scan time by 80-95% for typical pull requests.

**Acceptance Criteria:**
- [ ] Implement a `ScanManifest` that records, for each file in the last scan: file path, SHA-256 hash, last scan timestamp, and list of dependencies (imported modules)
- [ ] Add a `--incremental` CLI flag to `stellar-scanner scan` that: (a) loads the previous `ScanManifest`, (b) computes current file hashes, (c) identifies changed files and files that import changed modules, (d) scans only the affected subset
- [ ] Implement `compute_affected_files()` that builds an import dependency graph and transitively includes all files that depend (directly or indirectly) on changed files
- [ ] Add a `--force-full` flag to override incremental mode and force a complete re-scan (useful after dependency updates or scanner version changes)
- [ ] Store the `ScanManifest` in the project's `.stellar-scanner/` directory (similar to `.git`), with manifest versioning for forward/backward compatibility
- [ ] Add scan time reporting: log "Scanned 3/50 files (incremental) in 12.4s — full scan would have taken ~180s"
- [ ] Write tests: (a) scan a 3-file project, change 1 file, verify only 1 file + any dependents are re-scanned, (b) verify unchanged files are skipped, (c) verify `--force-full` scans all files regardless of changes
- [ ] Document incremental scanning in `docs/UPGRADE_MECHANISM.md`

---

## Issue 59: [API Versioning] Deprecation Notification Webhooks Have No Signature Verification — Spoofable Notifications

**Priority:** 🟡 Medium
**Area:** Security / API
**Files:** `src/api_versioning/deprecation.rs`, `src/api_versioning/router.rs`, `src/api_versioning/version.rs`

**Description:**
The API versioning system in `src/api_versioning/` supports deprecation notification subscriptions via `POST /api/v1/admin/notifications/subscribe`, where clients can register webhook URLs to receive programmatic notifications when an API version they depend on is deprecated or approaching sunset. The `deprecation.rs` module implements the `SunsetProcedures` with a 10-step checklist for sunsetting versions and sends urgency notifications at configured thresholds (90, 60, 30, 14, 7, 1 days). However, when webhook notifications are sent to registered subscriber URLs, they are dispatched as a simple HTTP POST with a JSON body — there is **no signature or HMAC verification mechanism** on the webhook payload. A malicious actor who discovers a subscriber's webhook URL can send spoofed deprecation notifications that appear to come from the scanner platform, potentially tricking the subscriber into taking premature migration actions, clicking malicious links, or exposing internal systems. This is a well-known webhook security pattern (Stripe, GitHub, and Slack all sign their webhooks with HMAC-SHA256), and its absence makes the notification system untrustworthy for production use.

**Acceptance Criteria:**
- [ ] Generate a unique `webhook_signing_secret` for each subscriber at subscription time, stored encrypted in the database and returned once to the subscriber (never stored in plaintext in logs)
- [ ] Sign each webhook payload with HMAC-SHA256 using the subscriber's secret; include the signature in the `X-Soroban-Signature` header as `t=timestamp,v1=signature`
- [ ] Include a `X-Soroban-Webhook-Id` header with a unique ID for each delivery to prevent replay attacks
- [ ] Add a `verify_webhook_signature(payload, signature_header, secret) -> bool` function to `api_versioning/deprecation.rs` and document the verification algorithm for subscribers
- [ ] Provide a reference implementation of signature verification in TypeScript (for Node.js subscribers) and Python (as a gist or in `docs/API_VERSIONING.md`)
- [ ] Add a `POST /api/v1/admin/notifications/test-webhook` endpoint that sends a signed test payload to the subscriber's URL so they can verify their verification implementation
- [ ] Write tests: (a) verify a correctly signed payload passes verification, (b) verify a payload with a tampered body fails verification, (c) verify a replayed payload with the same signature fails (via webhook ID uniqueness check)
- [ ] Update `docs/API_VERSIONING.md` with a "Webhook Security" section

---

## Issue 60: [Notifications] Template Rendering Has No Preview Endpoint — Impossible to Test Templates Before Sending

**Priority:** 🟢 Low
**Area:** Developer Experience / Notifications
**Files:** `src/notification_service/templates.rs`, `src/notification_service/service.rs`, `src/notification_service/providers.rs`

**Description:**
The notification service in `src/notification_service/` sends email, SMS, push, and in-app notifications using templates defined in `templates.rs`. The `TemplateManager` renders templates with dynamic data (e.g., user name, scan ID, vulnerability severity, bounty amount) and dispatches them through providers (`providers.rs`). However, there is no way to preview a rendered template before sending it to real users. When a developer or operator needs to update an email template (e.g., changing the wording of a vulnerability alert or updating the branding), they must either: (a) read the raw template source with its placeholder syntax and mentally imagine the final output, or (b) trigger a real notification to themselves, which requires creating actual scan/vulnerability/bounty data in the system. The `NotificationService` in `service.rs` has a `send_notification()` method but no `preview_notification()` or `render_template()` public API. The `templates.rs` module has `Template::render()` but it's only callable internally. This slows down template iteration significantly — a simple wording change requires a full deploy-and-test cycle.

**Acceptance Criteria:**
- [ ] Add a `render_preview(template_name: &str, context: TemplateContext) -> RenderedTemplate` method to `TemplateManager` that renders a template with the provided context and returns the subject + body/content
- [ ] Add a `POST /api/v1/admin/notifications/preview` endpoint (admin-only) that accepts a template name and a JSON context object and returns the rendered subject, plain-text body, and HTML body (for email templates)
- [ ] Add a `GET /api/v1/admin/notifications/templates` endpoint that lists all available templates with their names, descriptions, and expected context variables (schema)
- [ ] Add a frontend `TemplatePreview` component in the admin panel (`frontend/components/notifications/TemplatePreview.tsx`) that provides: a template selector dropdown, a JSON editor for the context object, and a live preview panel that shows the rendered output (HTML rendered in an iframe for email templates)
- [ ] Implement syntax highlighting for template placeholders in the JSON editor to make it obvious which variables are being set
- [ ] Add unit tests that verify `render_preview()` produces correct output for each template type (email, SMS, push, in-app) with a standard test context
- [ ] Document the template preview feature in `NOTIFICATION_SERVICE.md`

---

## Summary of New Issues (#41–60)

| # | Issue Title | Priority | Area |
|---|-------------|----------|------|
| 41 | Structured Logs Missing Correlation IDs | 🟠 High | Observability |
| 42 | Cache Warming Ignores Hot-Key Stampede During Cold Start | 🟠 High | Performance / Caching |
| 43 | N+1 Detection Is Read-Only — No Auto-Prevention | 🟡 Medium | Performance / Database |
| 44 | Invariant Engine Only Checks Point-in-Time — No Temporal Invariants | 🟡 Medium | Core Scanner |
| 45 | WASM Deep Inspection Doesn't Validate Function Signatures | 🟠 High | Security / Input Validation |
| 46 | Form Components Lack Accessible Error Announcements | 🟡 Medium | Accessibility |
| 47 | Chart Components Don't Handle Empty States | 🟢 Low | UX / Frontend |
| 48 | Stateless JWT Sessions Can't Be Force-Invalidated | 🟠 High | Security / Auth |
| 49 | DB Pool Metrics Not Exposed to Prometheus | 🟡 Medium | Infrastructure |
| 50 | No Threat Intelligence Feed Integration for Address Blacklist | 🟡 Medium | Security / Intel |
| 51 | CSP Violation Reports Have No Dashboard | 🟡 Medium | Security / Monitoring |
| 52 | File Upload Zone Accepts Files by Extension Only | 🟡 Medium | Security / Frontend |
| 53 | Multi-Party Escrow Lacks Timeout Auto-Refund | 🟠 High | Smart Contracts |
| 54 | Web Vitals Monitoring Lacks Alerting Thresholds | 🟡 Medium | Performance |
| 55 | Anomaly Detection Has No Baseline Learning Period | 🟡 Medium | Security / Monitoring |
| 56 | RTL Language Support Not Implemented | 🟢 Low | i18n / Accessibility |
| 57 | SBOM Excludes Transitive Dependency Licenses | 🟡 Medium | Compliance |
| 58 | No Incremental Scan Support | 🟡 Medium | Core Scanner |
| 59 | Deprecation Webhooks Have No Signature Verification | 🟡 Medium | Security / API |
| 60 | Template Rendering Has No Preview Endpoint | 🟢 Low | DX / Notifications |

**Priority Breakdown:**
- 🟠 High: 7 issues (#41, #42, #45, #48, #49, #53, #60)
- 🟡 Medium: 10 issues (#43, #44, #46, #50, #51, #52, #54, #55, #57, #58, #59)
- 🟢 Low: 3 issues (#47, #56, #60)

*All issues reference specific files in the codebase and follow the `.github/ISSUE_TEMPLATE.md` format with detailed descriptions and actionable acceptance criteria.*
