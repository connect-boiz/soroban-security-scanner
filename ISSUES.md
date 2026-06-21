# Soroban Security Scanner — 30 Quality Issues

---

## Issue 1: [Frontend] Incomplete Error Boundary Coverage Leaves Unhandled Component Crashes Silent

**Description:**
Currently, the frontend has a basic `ErrorBoundary` component (`frontend/components/ui/ErrorBoundary.tsx`) that catches runtime errors in child components, but it is only applied at the top-level layout. Many deeply nested components such as `ScannerInterface`, `VulnerabilityReport`, `MultiSigWizard`, and `AnalyticsDashboard` are not individually wrapped with error boundaries. When one of these components crashes, the entire page falls back to a single generic error screen, losing user session state and scan progress. Additionally, the current error boundary does not provide detailed crash information to developers (e.g., stack traces, component name, props at time of crash) nor does it offer a "retry" mechanism that could restore just the failed section without reloading the entire page. This creates a poor user experience for security researchers running long-duration scans who might lose all progress due to a transient error in a non-critical visualization component.

**Acceptance Criteria:**
- [ ] Wrap each major page-level component (ScannerInterface, VulnerabilityReport, MultiSigWizard, AnalyticsDashboard, SettingsPanel, BatchOperations) with its own `ErrorBoundary` instance
- [ ] Each error boundary should render a component-specific fallback UI (e.g., "Scan results failed to load — retry") rather than a generic page-level message
- [ ] Add a "retry" button to each fallback that attempts to re-render the errored component
- [ ] Implement logging to `errorReporting.ts` with component name, stack trace, and timestamp on each caught error
- [ ] Write unit tests for each new error boundary instance using `@testing-library/react`
- [ ] Verify that an error in the ScannerInterface does not crash the VulnerabilityReport or navigation sidebar

---

## Issue 2: [Core Scanner] Weak Access Control Detection Produces False Positives for Internal Helper Functions

**Description:**
The `SecurityScanner` in `src/scanners.rs` uses regex-based pattern matching (`require_auth|has_auth`) to detect missing access control on public functions. However, this approach produces a significant number of false positives for public helper functions that are only called internally or are explicitly marked as `pub(crate)` — the regex only checks the `pub fn` visibility modifier at the text level, not the actual Rust visibility from the parsed AST. Functions with `pub(crate)` visibility, utility getters, and read-only query methods are incorrectly flagged as `MissingAccessControl`. This noise reduces the signal-to-noise ratio of scan reports and undermines developer trust in the scanning tool. The AST analysis in `analyze_function` does check for `syn::Visibility::Public`, but the regex patterns in `initialize_patterns` scan the raw file content and don't distinguish between different levels of visibility or function semantics.

**Acceptance Criteria:**
- [ ] Refactor `MissingAccessControl` detection to rely primarily on AST-level visibility analysis rather than raw regex matching
- [ ] Exclude `pub(crate)` and read-only query functions (no state mutations) from access control checks
- [ ] Add a configuration option `ignore_internal_helpers` to `VulnerabilityConfig` that allows users to suppress false positives for internal utility functions
- [ ] Reduce false positive rate for `MissingAccessControl` detection by at least 80% (measured against a benchmark suite of 50 contracts)
- [ ] Update the test suite in `tests/integration_tests.rs` with known false positive cases to ensure they are no longer flagged
- [ ] Document the change in vulnerability detection behavior in `docs/UPGRADE_MECHANISM.md` or a new migration note

---

## Issue 3: [Authentication] Missing Account Lockout Notification — Users Not Informed When Account Is Locked

**Description:**
The authentication system in `src/auth/` implements account lockout after a configurable number of failed login attempts (handled in `account_lockout.rs`). However, when a user's account becomes locked, the system silently rejects further login attempts with a generic "invalid credentials" error message rather than explicitly informing the user that their account has been temporarily locked due to excessive failed attempts. This creates confusion: legitimate users who may have forgotten their password or mistyped it repeatedly receive no clear indication that the lockout policy has been triggered, making them think the system is broken. Furthermore, there is no mechanism to notify the user via email or in-app notification that their account has been locked, nor is there a "forgot password" flow that automatically unlocks the account after successful password reset. The frontend login form also lacks the ability to display a lockout-specific error message with estimated remaining lockout time.

**Acceptance Criteria:**
- [ ] Return a distinct `AccountLocked` error response from the login endpoint when account is locked, including `locked_until` timestamp
- [ ] Update the frontend `LoginForm` component to display a clear lockout message: "Your account has been temporarily locked due to too many failed attempts. Please try again after [time]."
- [ ] Add email notification via `NotificationService` when an account becomes locked, including the lockout duration and instructions for password reset
- [ ] Implement automatic unlock on successful password reset via `PasswordResetForm`
- [ ] Add a `remaining_attempts` field to the login error response so the frontend can warn users before their account gets locked
- [ ] Write integration tests in `tests/auth_integration_tests.rs` covering the lockout notification flow

---

## Issue 4: [Wallet Management] Wallet Import from Ledger Hardware Device Fails Silently on Connection Timeout

**Description:**
The `WalletService` in `src/wallet/service.rs` supports importing wallets from hardware devices like Ledger via the `import_wallet` method. However, when a hardware device connection times out (e.g., device not connected, USB cable unplugged, or the Stellar app not open on the device), the import process fails with a generic `WalletError::ConnectionFailed` error that does not distinguish between a device-not-found scenario and a communication timeout. This leaves users confused about whether they need to reconnect the device, install the Stellar app, or try a different USB port. Furthermore, the frontend `AuthContainer` component does not display a meaningful progress indicator during the hardware import process, so users see a spinner indefinitely when the device is not actually connected. The import operation also lacks a configurable timeout, so it can block the UI thread for up to 120 seconds in some scenarios before failing.

**Acceptance Criteria:**
- [ ] Add specific error variants for `DeviceNotFound`, `ConnectionTimeout`, and `AppNotOpen` to `WalletError` in `src/wallet/types.rs`
- [ ] Implement a configurable timeout (default 30 seconds) for hardware wallet connection attempts
- [ ] Update the import UI to show a stepper/status indicator: "Connecting to device..." → "Opening Stellar app..." → "Importing keys..."
- [ ] Add retry logic (up to 3 attempts) with exponential backoff for transient connection failures
- [ ] Log detailed hardware interaction telemetry to `event_logging.rs` for debugging
- [ ] Write unit tests simulating device timeout, missing app, and successful import scenarios using mocked hardware interfaces

---

## Issue 5: [Multi-Sig] Proposal Execution Does Not Validate Signer Weight Thresholds Before Marking as Executable

**Description:**
The `MultiSigService` in `src/multisig/service.rs` tracks signatures collected for multi-signature proposals and marks a proposal as ready for execution once `collected_signatures.len() >= required_signatures`. However, the implementation does not account for per-signer weight values that should be defined in a `SignerSpec`. In real multi-signature configurations, different signers may have different weight levels (e.g., one "admin" signer might count as 3 votes, while a regular "team member" counts as 1). The current check using a simple count of signatures means that a proposal could be executed with enough low-weight signers while high-weight signers are still missing, violating the intended weighted voting model. Additionally, there is no validation that the collected signatures are from distinct, currently authorized signers — if a signer address was revoked between signing and execution, the stale signature should be rejected.

**Acceptance Criteria:**
- [ ] Add a `weight` field to `MultiSigSigner` and `SignerSpec` in `src/multisig/types.rs`
- [ ] Change proposal readiness check from counting signatures to summing weights: `sum(weights) >= threshold`
- [ ] Add execution-time validation that all signers are still active (not revoked) at the moment of execution
- [ ] Emit an event via event_logging when a signer's weight is not sufficient to contribute to the threshold (e.g., "signature from revoked signer rejected")
- [ ] Update `MultiSigWizard.tsx` frontend component to display individual signer weights and current accumulated weight
- [ ] Write comprehensive tests in `tests/` covering weighted proposals, revoked signer rejection, and threshold boundary cases

---

## Issue 6: [Notification Service] Channel Delivery Status Not Persisted Across Service Restarts

**Description:**
The `NotificationService` in `src/notification_service/service.rs` tracks delivery status for each notification channel (email, SMS, push, in-app) using an in-memory `DeliveryTracker`. When the service restarts (e.g., during deployment, scaling events, or crashes), all delivery tracking data is lost. This means undelivered notifications are silently dropped without retry, and there is no way to audit past notification delivery success rates after a restart. The `DeliveryStats` report that users expect to query for the last 30 days becomes empty after every service restart. The `get_delivery_tracking()` method returns `None` for all previously tracked notifications, even if their delivery was still pending. This is especially critical for security vulnerability alerts where guaranteed delivery is essential.

**Acceptance Criteria:**
- [ ] Persist delivery tracking records to a database table (see `DATABASE_SCHEMA.md` for notification-related schema)
- [ ] Implement a delivery status recovery process on service startup that re-queues any notifications with `Pending` or `InProgress` status
- [ ] Add a configurable delivery retry policy (max 3 retries with exponential backoff: 5min, 30min, 2h)
- [ ] Create a database migration (`006_add_notification_delivery_tracking.sql`) with indexes on `notification_id`, `channel`, `status`, and `created_at`
- [ ] Update the frontend's `InAppNotification` component to show delivery status for each notification channel
- [ ] Add a new `/api/v1/notifications/delivery-stats` REST endpoint backed by persistent storage

---

## Issue 7: [Rate Limiting] IP-Based Rate Limiting Does Not Account for Reverse Proxy Headers

**Description:**
The `RateLimiter` in `src/rate_limiting/limiter.rs` identifies clients by their connection IP address (`std::net::SocketAddr`), which is obtained from the TCP connection. When the application runs behind a reverse proxy (e.g., Nginx, AWS ALB, or Kubernetes Ingress), the connection IP is always the proxy's IP address, not the actual end-user IP. This means that all traffic from different users behind the same proxy is rate-limited as if it came from a single client, causing legitimate users to be unfairly blocked. The `rate_limiting/middleware.rs` and its Axum middleware integration do not inspect `X-Forwarded-For` or `X-Real-IP` headers, nor do they allow configuration of trusted proxy CIDR ranges. This makes the rate-limiting feature ineffective for production deployments behind any proxy infrastructure.

**Acceptance Criteria:**
- [ ] Add a `trusted_proxies` configuration field to `RateLimitConfig` accepting a list of CIDR ranges (e.g., `["10.0.0.0/8", "172.16.0.0/12"]`)
- [ ] Implement `X-Forwarded-For` header parsing that uses the rightmost untrusted IP (per common reverse proxy conventions)
- [ ] Add `X-Real-IP` header support as a fallback when `X-Forwarded-For` is not present
- [ ] Update the middleware to use the resolved end-user IP for rate limit key calculation
- [ ] Log the resolved IP and the original connection IP for debugging purposes
- [ ] Write tests in `src/rate_limiting/tests.rs` that simulate requests through multiple proxy layers

---

## Issue 8: [Time Travel Debugger] Forked State Not Compatible with Upgraded Contract WASM Versions

**Description:**
The `TimeTravelDebugger` in `src/time_travel_debugger.rs` creates a `ForkedState` by snapshotting the Stellar ledger at a specific sequence number and replaying contract state. When a user then attempts to test a new contract WASM against this historical state (via `simulate_contract_upgrade`), the forked state often fails to deserialize correctly because the new WASM version may define different storage schema, data structures, or key formats. The current error handling in `orphaned_state.rs` and `contract_upgrade.rs` genericizes these failures as "state incompatibility" without providing developers with specific information about which storage keys could not be migrated, what the expected vs. actual types are, or whether the incompatibility is critical or ignorable. Without detailed migration guidance, developers must manually diff the two contract versions to understand storage changes, which is error-prone and time-consuming.

**Acceptance Criteria:**
- [ ] Implement per-key storage compatibility analysis that reports for each key: key name, expected type, actual type, and migration status (compatible / incompatible / missing)
- [ ] Add a `StorageDiff` structure to `time_travel_debugger/types.rs` with old vs. new key-value pairs
- [ ] Categorize incompatibilities as `Critical` (data loss risk), `Warning` (type coercion possible), or `Info` (key added/removed)
- [ ] Generate a human-readable "Storage Migration Report" with recommendations for each incompatible storage key
- [ ] Update the time travel CLI command (`stellar-scanner time-travel upgrade`) to display the migration report
- [ ] Write tests that simulate storage schema changes between two contract versions and verify the diagnostic output

---

## Issue 9: [Differential Fuzzing] Input Generator Misses Critical Edge Cases for Soroban `i128` Arithmetic

**Description:**
The `InputGenerator` in `src/differential_fuzzing/input_generator.rs` generates test inputs for contract functions, including edge cases like `MaxI128`, `MinI128`, `ZeroValue`, and `LargeVector`. However, it does not generate composite edge cases that combine multiple extreme values in a single function call — for example, invoking a `transfer(from, to, amount)` function with both `from` and `to` set to the same address, or combining `MaxI128` for amount with an empty `from` address, or passing a `LargeVector` as the `to` parameter. Many real-world Soroban vulnerabilities arise precisely from these combinations of edge conditions (e.g., self-transfer with maximum amount, or overflow when summing multiple extreme values). The current generator treats each edge case independently, producing test inputs that exercise only one boundary condition at a time.

**Acceptance Criteria:**
- [ ] Add a combinatorial edge case generation mode that creates Cartesian products of individual edge case types for multi-parameter functions
- [ ] Include specific composite scenarios: self-transfer, transfer to zero address, simultaneous max input and max output, overflow through multiple accumulative operations
- [ ] Implement a configurable `combinatorial_depth` parameter (default 2) in `DifferentialFuzzingConfig` that limits combinatorial explosion
- [ ] Add a deduplication step to avoid running identical test inputs multiple times
- [ ] Verify in tests that at least 50 distinct composite edge cases are generated from a configuration with 5 basic edge case types
- [ ] Document the composite edge case strategy in `docs/DIFFERENTIAL_FUZZING.md`

---

## Issue 10: [Batch Operations] Gas Estimation Not Available Before Batch Execution

**Description:**
The `BatchOperations` module in `src/batch_operations.rs` allows users to create and execute batch escrow releases and vulnerability verifications. However, there is no mechanism to estimate the gas cost of a batch operation before execution. Users submit batch requests without knowing whether the gas limit they provide is sufficient, leading to frequent "out of gas" transaction failures on Stellar. The `GasLimitManager` in `src/gas_limits.rs` already provides gas estimation infrastructure (`estimate_gas` and `validate_gas_limit` methods), but these are not integrated with the batch operations pipeline. Each item in a batch can have different gas costs depending on its complexity (e.g., verifying a critical vulnerability vs. a low-severity one), and the total gas cost can vary significantly based on the number and types of items batched together.

**Acceptance Criteria:**
- [ ] Add a `estimate_batch_gas()` method to `BatchOperations` that iterates over each item, calls `GasLimitManager::estimate_gas()` per item, and sums the results with overhead
- [ ] Add a CLI subcommand `stellar-scanner batch estimate-gas --batch-id <id>` that displays estimated, recommended, and maximum gas for the batch
- [ ] Update the frontend `BatchOperations.tsx` component to show a gas estimation summary before the user confirms execution
- [ ] If estimated gas exceeds 90% of the Stellar transaction limit, warn the user and suggest splitting the batch
- [ ] Write unit tests verifying gas estimation accuracy against known operation profiles
- [ ] Log gas estimation data in the `BatchOperationSummary` structure for post-execution analysis

---

## Issue 11: [Emergency Stop] Stuck Scans Not Automatically Detected or Cleaned Up

**Description:**
The `EmergencyStop` mechanism in `src/emergency_stop.rs` relies on external triggers (user-initiated, critical vulnerability detection, or manual signal handlers) to stop scanning operations. There is no watchdog or health-check system that can detect when a scan has become stuck (e.g., hung on a large contract file, deadlocked in a regex operation, or waiting indefinitely on a network call). The `K8sScanManager` has a `cleanup_stuck_scans()` method, but this only cleans up Kubernetes pods — it does not handle stuck scans in the core Rust scanner. In practice, a single hung scan can block the entire scanning pipeline (since the scanner processes files sequentially in `scan_directory`), causing timeouts for all subsequent scan requests and degrading the overall platform user experience.

**Acceptance Criteria:**
- [ ] Implement a `ScanWatchdog` that monitors scan progress via a heartbeat mechanism (each file update resets a timer)
- [ ] If no heartbeat is received for `watchdog_timeout_seconds` (configurable, default 120s), automatically trigger emergency stop via `trigger_stop()`
- [ ] Add a per-file processing timeout in `scan_file()` that aborts processing of individual files exceeding the time limit
- [ ] Expose watchdog status via a new API endpoint `GET /api/v1/scanner/watchdog-status`
- [ ] Log watchdog events (heartbeat, timeout, automatic stop) to the event logging system
- [ ] Write tests simulating a stuck operation and verifying automatic cleanup within the timeout period

---

## Issue 12: [Event Logging] No Export or Query API for Compliance Auditing

**Description:**
The `EventLogger` in `src/event_logging.rs` stores critical events in memory with optional persistence, and provides query methods to retrieve events by operation type, time range, or actor. However, there is no pagination support, no CSV/JSON export capability, and no SQL-backed query API for compliance auditors who need to review historical events. The in-memory storage is bounded by `max_events_in_memory` (default 10,000), which means older events are silently dropped without any archival strategy. For SOC 2 Type II and GDPR compliance, the platform needs to support querying events over multi-month periods, exporting audit logs in machine-readable formats, and ensuring tamper-evident event storage (e.g., hash chains).

**Acceptance Criteria:**
- [ ] Add pagination (offset/limit) to all event query methods (`get_events_by_operation`, `get_events_by_time_range`, `get_events_by_actor`)
- [ ] Implement CSV and JSON export endpoints: `GET /api/v1/events/export?format=csv&start=...&end=...`
- [ ] Add an optional SQL-backed event store (using `src/database/models.rs` and `src/database/queries.rs`) for production deployments
- [ ] Implement event hash chaining: each event includes the hash of the previous event, forming a tamper-evident chain
- [ ] Add a `verify_chain(from_event, to_event)` method that validates the integrity of a range of events
- [ ] Create a database migration (`007_add_event_log_store.sql`) for the SQL-backed event persistence

---

## Issue 13: [Gas Limits] No Dynamic Gas Adjustment Based on Historical Usage Patterns

**Description:**
The `GasLimitManager` in `src/gas_limits.rs` uses static operation profiles and a fixed safety margin (10%) to estimate gas consumption. It does not learn from actual execution results to improve its estimation accuracy over time. If the base profile for `escrow_release` assumes 5,000 gas per transfer but on Stellar mainnet each transfer consistently costs 6,200 gas, the estimator will always under-report, causing users to set insufficient gas limits. Conversely, if a profile is too conservative, users may overpay in gas fees. Without adaptive gas estimation, the platform cannot optimize gas costs for users, and the `InsufficientGasLimitConsiderations` vulnerability detection produces stale recommendations.

**Acceptance Criteria:**
- [ ] Add a `GasUsageHistory` storage that records actual gas consumption per operation type per execution
- [ ] Implement a moving-average gas estimator that uses the last 100 executions to adjust profile base costs
- [ ] Add a `learning_rate` parameter (default 0.3) to control how quickly estimates adapt to new data
- [ ] Expose historical gas trends via a new API endpoint: `GET /api/v1/gas/trends?operation=escrow_release`
- [ ] Add a frontend visualization in `AnalyticsDashboard.tsx` showing gas estimation accuracy over time
- [ ] Write tests verifying that the adaptive estimator converges to within +/-5% of actual costs after 50+ data points

---

## Issue 14: [Secure ID Generation] DefaultHasher Used Instead of Cryptographic Hash, Breaking Security Guarantees

**Description:**
The `SecureIdGenerator` in `src/secure_id_generation.rs` is designed to generate cryptographically secure IDs for bounties, sessions, transactions, and nonces. However, the actual hashing implementation uses Rust's `std::collections::hash_map::DefaultHasher` in multiple critical methods (`hash_string`, `hash_entropy_to_id`, `hash_entropy_to_bytes`, `get_random_entropy`). `DefaultHasher` is explicitly documented as not cryptographically secure — it uses SipHash-1-3 with a fixed key (not the keyed variant), making it vulnerable to hash collision attacks and preimage attacks. An attacker who observes a generated ID can determine the internal hasher state, predict future IDs, and forge session tokens or nonces. This defeats the entire purpose of the secure ID generation module, which was created specifically to address issue #114 (predictable ledger sequence IDs).

**Acceptance Criteria:**
- [ ] Replace all `DefaultHasher` usages with a proper cryptographic hash function from the `ring` or `sha2` crate (already in `Cargo.toml` dependencies)
- [ ] Use `ring::digest::SHA256` for ID generation and `ring::hkdf` for key derivation where applicable
- [ ] Use `ring::rand::SecureRandom` for all random number generation instead of the PRNG approach
- [ ] Remove the `get_random_entropy` function that uses `DefaultHasher` and replace with `ring::rand::SystemRandom`
- [ ] Conduct a security review of the rewritten module to confirm no cryptographic shortcuts remain
- [ ] Write cryptographic tests that verify generated IDs are indistinguishable from random (e.g., chi-squared test on bit distribution)

---

## Issue 15: [Frontend] Accessibility Violations in Scan Results Table — Missing ARIA Labels and Keyboard Navigation

**Description:**
The `VulnerabilityReport` component in `frontend/components/VulnerabilityReport.tsx` renders a data table displaying scan results with columns for severity, vulnerability type, file path, and recommendation. This table has several accessibility violations detected by `@axe-core/playwright` in the automated accessibility test suite (`tests/accessibility/a11y.spec.ts`). Specifically: (1) the table lacks a `<caption>` element describing its purpose, (2) sort buttons in column headers do not have `aria-label` attributes indicating the sort direction, (3) the severity color-coding (red for Critical, yellow for High, etc.) relies solely on color without accompanying text or icons, (4) the expandable rows for vulnerability details are not keyboard-accessible (missing `role="button"`, `tabindex`, and keyboard event handlers), and (5) the "Copy to Clipboard" button does not announce the copy action to screen readers. These violations prevent security researchers who rely on assistive technologies from effectively reviewing scan results.

**Acceptance Criteria:**
- [ ] Add a `<caption>` element to the results table: "Security Scan Results — showing {{count}} vulnerabilities"
- [ ] Add `aria-label` attributes to all column sort buttons with the format "Sort by {{column}} (currently {{direction}})"
- [ ] Add descriptive text labels alongside severity color indicators (e.g., a "Critical" badge with red background + text)
- [ ] Make expandable rows keyboard-navigable with `role="button"`, `tabindex="0"`, and `onKeyDown` handlers for Enter/Space
- [ ] Add `aria-live="polite"` announcement region for copy-to-clipboard actions
- [ ] Update automated accessibility tests to verify all new ARIA attributes are present
- [ ] Run axe-core against the vulnerability report page and confirm zero critical/serious violations

---

## Issue 16: [Performance] Frontend Bundle Includes Unused Translations from All Locales, Doubling Initial Load Size

**Description:**
The frontend uses `i18next` with `i18next-fs-backend` (configured in `src/i18n/config.js` and `component-library/src/i18n/config.ts`) to load translation resources. The current configuration loads all locale JSON files at application startup, regardless of the user's selected language. With 15 supported locales and growing, the initial JavaScript bundle includes approximately 1.2MB of unused translation strings for the average user. This contributes to a poor Largest Contentful Paint (LCP) score, particularly on mobile devices with slower network connections. The `PerformanceOptimizations.md` document targets an LCP of 2.5 seconds, but loading all translations on the first request pushes this to over 3.5 seconds on 3G connections.

**Acceptance Criteria:**
- [ ] Implement lazy locale loading using `i18next`'s `backend` option to load only the user's current language on initial page load
- [ ] Preload the default locale (English) in the critical rendering path to avoid Flash of Untranslated Content (FUTC)
- [ ] Queue loading of additional locales in the background after the page becomes interactive
- [ ] Add a `LanguageSelector` component in `component-library/src/components/LanguageSelector.tsx` that pre-fetches the locale data when hovered
- [ ] Reduce initial bundle size by at least 500KB on the first request for non-English users
- [ ] Update Lighthouse CI thresholds in `lighthouserc.js` to reflect the improved LCP metric
- [ ] Write a performance regression test that asserts initial bundle size stays below 2MB

---

## Issue 17: [Database] Missing Database Index on `transactions.created_at` Causes Slow Dashboard Queries

**Description:**
The `transactions` table defined in `DATABASE_SCHEMA.md` has indexes on `transaction_hash`, `from_wallet_id`, `to_wallet_id`, and `user_id`, but does not have a compound index on `(user_id, created_at)` — the most common query pattern for the user dashboard. When a user with thousands of transactions (e.g., a high-frequency security researcher) loads their transaction history, the database performs a sequential scan filtered by `user_id` then sorts by `created_at`, which degrades as transaction volume grows. The `queries.rs` module in `src/database/queries.rs` implements `get_user_transactions_paginated()` which queries `WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3` — this query currently performs a full sort on every paginated page load. Users with 50,000+ transactions experience dashboard load times exceeding 8 seconds.

**Acceptance Criteria:**
- [ ] Create a new migration (`008_add_transaction_user_time_index.sql`) that adds a B-tree compound index on `transactions(user_id, created_at DESC)`
- [ ] Also add an index on `transactions.status` for filtering by transaction status (pending, confirmed, failed)
- [ ] Verify the index is used by running `EXPLAIN ANALYZE` on the paginated query with 100,000 test rows
- [ ] Measure and document the query performance improvement (target: sub-100ms for paginated queries with 100k+ rows)
- [ ] Update `src/database/queries.rs` to add index hints or query hints if needed for the PostgreSQL query planner
- [ ] Write a database benchmark test in `src/database/tests.rs` that measures query latency before and after the index

---

## Issue 18: [CI/CD] Automated Accessibility Tests Not Running in CI Pipeline

**Description:**
The accessibility test suite (`tests/e2e/accessibility.spec.js` and `tests/accessibility/a11y.spec.ts`) uses `@axe-core/playwright` to scan key frontend pages for WCAG 2.1 AA violations. These tests are defined in `playwright.config.js` and can be run locally with `npm run test:a11y`. However, they are not integrated into the GitHub Actions CI pipeline (no `.github/workflows/` file is present in the repository). This means that new PRs introducing accessibility regressions are merged without automated checking. The `docs/ACCESSIBILITY_TESTING.md` documentation states that these tests "run on every push and PR via the Accessibility (axe-core) GitHub Actions workflow," but this workflow file does not actually exist, creating a documentation-to-implementation gap.

**Acceptance Criteria:**
- [ ] Create a `.github/workflows/accessibility.yml` workflow file that triggers on push and pull_request events
- [ ] The workflow should: install dependencies, build the frontend, start the dev server, run Playwright accessibility tests
- [ ] Configure the workflow to fail if any critical or serious axe-core violations are detected (configurable threshold)
- [ ] Add a status badge to the `README.md` showing accessibility test pass/fail status
- [ ] Add a comment to the PR when accessibility violations are detected, listing the specific violations and affected components
- [ ] Set up Playwright test artifact retention (screenshots, trace files) for debugging failed tests

---

## Issue 19: [Multi-Sig Wizard] Proposal Creation Form Lacks Real-Time Validation for Stellar Addresses

**Description:**
The `MultiSigWizard.tsx` component in `frontend/components/MultiSigWizard.tsx` provides a multi-step form for creating multi-signature proposals, including fields for signer addresses, signature thresholds, and execution delays. The signer address fields accept freeform text input but do not validate that the entered address is a valid Stellar public key (G... or X... format, 56 characters, base32-encoded with a version byte and checksum). Users can submit a proposal with an invalid address, which only fails at the backend API call (after form submission) with a cryptic error. This wastes user time and API resources. The `utils/validation.ts` file already contains a `validateStellarAddress()` function, but it is not integrated into the wizard form. Additionally, the form does not prevent users from adding duplicate signer addresses, which would inflate the required signature threshold without adding meaningful security.

**Acceptance Criteria:**
- [ ] Integrate `validateStellarAddress()` from `utils/validation.ts` into all signer address input fields in the wizard
- [ ] Show real-time validation feedback (green checkmark for valid, red error message for invalid) as the user types
- [ ] Prevent duplicate signer addresses — show a warning "Address {{address}} is already a signer" and disable the "Add" button
- [ ] Add input masking/formatting to make Stellar addresses more readable (group characters every 4 characters)
- [ ] Validate that the signature threshold is between 1 and the total number of unique signers
- [ ] Write unit tests for the validation functions covering edge cases like empty input, invalid characters, and checksum mismatch

---

## Issue 20: [Scanner Registry] No Version Comparison Logic — Cannot Determine If Scanner Is Up-to-Date

**Description:**
The `ScannerRegistry` in `src/scanner_registry.rs` manages registered security scanners and their versions with `ScannerVersion`, `VersionStatus` enums, and a registry that maps `(name, version)` pairs. However, the `VersionStatus` uses a simple string comparison (`current == latest`) to determine if a scanner is up to date. There is no semantic version comparison that can handle `>=`, `<=`, `~>` (pessimistic), or range constraints. The `ScannerVersion` struct stores version as a `String` and provides no `compare()` method. This means that when `scanner_registry_usage.py` checks if the installed scanner version meets the minimum requirement, the comparison is lexicographic rather than semantic, leading to incorrect results. For example, version `"9.0.0"` would be considered less than `"10.0.0"` in string comparison (because '9' > '1'), but would be correct in semantic comparison.

**Acceptance Criteria:**
- [ ] Add a `SemanticVersion` struct (major, minor, patch) that can be parsed from strings
- [ ] Replace the `version: String` in `ScannerVersion` with `version: SemanticVersion`
- [ ] Impl `PartialOrd`, `Ord`, and comparison operators for `SemanticVersion` that follow semver 2.0 rules
- [ ] Add support for pre-release tags (`-alpha.1`, `-beta.2`) and build metadata (`+build.1234`)
- [ ] Add a `satisfies_requirement(version, constraint)` function that supports operators: `>=`, `<=`, `~>`, `^`, `=`
- [ ] Update `VersionStatus` to use semantic comparison instead of string equality
- [ ] Write tests covering all comparison operators, pre-release precedence, and edge cases like `0.0.0` and `999.999.999`

---

## Issue 21: [Audit Proof of Scan] Certificate Revocation List Not Implemented, Compromised Certificates Remain Active

**Description:**
The `AuditProofOfScan` module in `src/audit_proof_of_scan.rs` generates `SecurityCertificate` objects that cryptographically prove that a scan was performed at a specific time with specific results. These certificates can be used for compliance and insurance purposes. However, the module lacks a Certificate Revocation List (CRL) mechanism. If a scanner's private key is compromised, or if a certificate was issued in error (e.g., for the wrong contract), there is no way to revoke that certificate. The `CertificateStatus` enum has variants like `Active`, `Expired`, `Revoked`, but the `Revoked` status is never set anywhere in the codebase — there is no `revoke_certificate(id, reason)` function. This means that compromised certificates continue to be treated as valid in downstream verification and insurance claims, creating a serious security and legal liability.

**Acceptance Criteria:**
- [ ] Add a `revoke_certificate(certificate_id: &str, reason: RevocationReason, revoked_by: &str)` function to `AuditProofOfScan`
- [ ] Define a `RevocationReason` enum with variants: `KeyCompromise`, `CertificateAuthorityCompromise`, `AffiliationChanged`, `Superseded`, `CessationOfOperation`, `Unspecified`
- [ ] Implement a CRL data structure (in-memory with optional database persistence) that stores revoked certificate IDs and timestamps
- [ ] Add a `verify_certificate_not_revoked(certificate_id)` check that is called before any certificate verification
- [ ] Publish a `crl.json` endpoint at `GET /api/v1/certificates/revocation-list` for external verifiers
- [ ] Write tests covering certificate revocation, double-revocation (should error), and verification of revoked certificates

---

## Issue 22: [i18n] Translation Strings Missing for Error Messages — Non-English Users See Raw Error Codes

**Description:**
The internationalization system configured in `src/i18n/config.js` and `component-library/src/i18n/config.ts` provides translations for UI labels and content strings. However, error messages returned from the backend API (`src/config.rs`, `src/auth/`, `src/wallet/`, etc.) are not internationalized. When an error occurs, the frontend displays the raw English error string or numeric error code directly to the user, regardless of their selected locale. For example, a user with `es` locale sees "Insufficient balance" instead of "Saldo insuficiente", and a user with `ja` locale sees "Invalid signature" instead of "無効な署名". The `I18N_README.md` and `test-i18n.js` files document translation coverage for UI components but do not address backend error responses. The backend uses `thiserror` and `anyhow` for error handling, but the error messages are constructed in English only.

**Acceptance Criteria:**
- [ ] Define error code constants for all backend errors (e.g., `ERR_INSUFFICIENT_BALANCE`, `ERR_INVALID_SIGNATURE`)
- [ ] Add error message translations in all locale files under `frontend/public/locales/{lang}/errors.json`
- [ ] Create an `ErrorCode` enum in `src/lib.rs` that maps to i18n keys
- [ ] Update the frontend API error handler to map error codes to translated messages using `i18next`
- [ ] Add a fallback mechanism: if a translation for the error is not found in the user's locale, fall back to English
- [ ] Write an i18n coverage test that verifies every error code has a translation in at least the top 5 locales (en, es, ja, fr, zh)

---

## Issue 23: [Offline Support] Service Worker Caches Scan Results but Does Not Provide Offline Access to Cached Data

**Description:**
The service worker registered in `public/sw.js` implements a caching strategy for static assets and API responses using `public/offline-storage.js`, `public/offline-integration.js`, and `public/offline-sync.js`. However, scan results cached by the service worker cannot be accessed by the user when they are offline. The `VulnerabilityReport` component does not check for cached data before making an API call, and there is no "offline mode" indicator in the UI. When the browser goes offline, the scan results page shows an error message instead of displaying the last-cached results. The `offline.html` page is only shown when the navigation itself fails, not when cached data exists but the network request for fresh data fails. This means users who have previously viewed scan results cannot access them during network outages, which is a common scenario for security researchers working in remote or air-gapped environments.

**Acceptance Criteria:**
- [ ] Implement an "offline-first" data fetching pattern: show cached data immediately, then update with fresh data when online
- [ ] Add a banner/indicator in the UI when the app is in offline mode: "You are viewing cached data — last synced [time]"
- [ ] Update the `VulnerabilityReport` component to fall back to cached scan results when the API is unreachable
- [ ] Add an IndexedDB-backed cache for scan results in `public/offline-storage.js` that persists beyond browser restarts
- [ ] Implement a background sync mechanism in `public/offline-sync.js` that queues user actions (like submitting a scan) and retries when online
- [ ] Write offline-mode tests using Playwright that simulate network disconnection and verify cached content is displayed

---

## Issue 24: [Authentication] OAuth2 State Parameter Not Validated, Allowing CSRF on Social Login

**Description:**
The OAuth2 implementation in `src/auth/oauth.rs` handles social login via the `oauth2` crate. The `OAuth2Config` stores client credentials, authorization URL, token URL, and redirect URL. During the OAuth2 authorization code flow, the backend generates a `state` parameter and sends it in the initial redirect to the provider. However, the callback endpoint (`/auth/oauth/callback`) does not validate that the `state` parameter returned by the provider matches the one originally sent. According to the OAuth 2.0 spec (RFC 6749, Section 10.12), this validation is mandatory to prevent CSRF attacks on the redirect flow. Without `state` validation, an attacker can initiate an OAuth2 flow with their own provider session and trick the victim into completing it, linking the attacker's social account to the victim's local account and gaining unauthorized access.

**Acceptance Criteria:**
- [ ] Store the `state` parameter in the user's HTTP session when initiating the OAuth2 redirect
- [ ] Validate the returned `state` parameter against the stored value in the callback handler
- [ ] If `state` does not match, reject the authentication and return a 403 error with a clear message
- [ ] Implement state parameter expiry: state tokens should expire after 10 minutes
- [ ] Use cryptographically secure random generation for the `state` parameter (using `ring::rand`)
- [ ] Write integration tests that simulate a CSRF attack and verify the callback is rejected

---

## Issue 25: [Performance] Skeleton Components Flash on Fast Connections Due to Missing Minimum Display Time

**Description:**
The frontend implements skeleton loading states via `SkeletonTable`, `SkeletonCard`, `SkeletonLoader`, and `LoadingSpinner` in `frontend/components/ui/`. These components appear while data is being fetched and are replaced when the data arrives. On fast connections (e.g., localhost development, fast WiFi), the data arrives so quickly (under 100ms) that the skeleton flashes briefly before being replaced. This creates a distracting "flash of loading content" (FOLC) that worsens the perceived performance experience. Research from the NN Group shows that showing a skeleton for less than 300ms feels more disruptive than showing nothing at all. The current implementation replaces the skeleton immediately when the async operation completes, with no minimum display time or smooth transition.

**Acceptance Criteria:**
- [ ] Implement a `minDisplayTime` prop (default 400ms) on all skeleton components that ensures they are visible for at least this duration
- [ ] Add CSS transitions (`opacity` fade with `300ms` duration) when transitioning from skeleton to content to avoid abrupt replacement
- [ ] Add a `skipSkeleton` prop that can be set to `true` for extremely fast expected queries (e.g., local validation)
- [ ] In the `useLoadingStates.ts` hook, implement the minimum display time logic so that the loading state persists for at least the configured duration
- [ ] Create a demo page in `LoadingStatesDemo.tsx` that demonstrates the skeleton display timing behavior
- [ ] Add a unit test that verifies the skeleton is visible for at least the minimum time even if data resolves immediately

---

## Issue 26: [Contract Upgrade] Timelock Bypass in Emergency Upgrade Allows Admin to Skip Waiting Period Without Justification

**Description:**
The contract upgrade mechanism in `docs/UPGRADE_MECHANISM.md` and the associated Rust code allows an admin to perform an "emergency upgrade" that bypasses the standard 7-day timelock delay. This is necessary for critical security patches. However, the current implementation does not require any justification or proof that the upgrade is genuinely an emergency. An admin can call `emergency_upgrade()` with any reason string, including an arbitrary or misleading reason, and the upgrade proceeds immediately. There is no on-chain audit trail that distinguishes genuine emergency upgrades from malicious ones, and no mechanism for other stakeholders (multi-sig signers, community members) to challenge or halt a suspicious emergency upgrade. The contract also does not enforce a maximum frequency of emergency upgrades — a malicious admin could perform emergency upgrades repeatedly, effectively disabling the timelock entirely.

**Acceptance Criteria:**
- [ ] Add a `MAX_EMERGENCY_UPGRADES_PER_MONTH` constant (default 2) to limit emergency upgrade frequency
- [ ] Require the emergency upgrade reason to be at least 50 characters and include specifics about the vulnerability being patched
- [ ] Add a cooling-off period of 24 hours between emergency upgrades (even with admin privileges)
- [ ] Publish a forced `EMERGENCY_UPGRADE_NOTIFICATION` event with full details (reason, code diff hash, affected functions) that cannot be suppressed
- [ ] Implement a "Challenge Period" (6 hours) during which multi-sig signers can vote to halt the upgrade
- [ ] Write tests verifying: frequency cap enforcement, insufficient reason rejection, cooling-off period, and challenge period resolution

---

## Issue 27: [Differential Fuzzing] Cross-Contract Simulator Does Not Handle Recursive Call Patterns, Missing Reentrancy Bugs

**Description:**
The `CrossContractSimulator` in `src/differential_fuzzing/cross_contract_simulator.rs` simulates cross-contract calls to detect reentrancy vulnerabilities. It currently supports a single level of call depth (`A → B`) but does not handle recursive or multi-level call patterns (`A → B → A` or `A → B → C → A`). In real Soroban contracts, reentrancy attacks often exploit multi-contract callback chains. The `ReentrancyPattern` enum has variants like `Simple`, `CrossContract`, `CallbackBased`, but the simulation engine only tests `Simple` patterns. This means that sophisticated reentrancy vulnerabilities that involve three or more contracts are not detected by the differential fuzzer, leaving a significant class of reentrancy bugs unaddressed.

**Acceptance Criteria:**
- [ ] Extend `CrossContractSimulator` to support recursive call patterns up to a configurable `max_depth` (default 5)
- [ ] Add call graph analysis that identifies cyclic dependencies between contract functions
- [ ] Implement state rollback after each simulated execution to ensure test isolation
- [ ] Add a `detect_reentrancy_patterns()` method that classifies detected patterns by type and severity
- [ ] Add a new `ReentrancyDepth` parameter to `DifferentialFuzzingConfig` to control the maximum call chain length
- [ ] Write tests with mock contracts that form 3-level and 4-level recursive call chains and verify detection

---

## Issue 28: [Frontend] WebSocket Reconnection Does Not Preserve In-Flight Scan Subscriptions

**Description:**
The frontend `useWebSocket` hook in `frontend/hooks/useWebSocket.ts` manages a WebSocket connection to receive real-time scan progress updates. When the WebSocket connection drops (e.g., due to network interruption, server restart, or idle timeout), the hook attempts to reconnect. However, during reconnection, any active scan subscriptions (the list of scan IDs the user was monitoring) are lost. The user must manually navigate away and back to the scan page to re-subscribe. Furthermore, when the connection re-establishes, the user misses any scan progress events that were sent during the disconnect period. For long-running scans (30+ minutes), a brief network interruption can cause the user to lose all progress visibility for the remaining scan duration, leading them to believe the scan is stuck or has failed.

**Acceptance Criteria:**
- [ ] Persist active scan subscriptions in browser `sessionStorage` so they survive WebSocket reconnections
- [ ] On WebSocket reconnect, automatically re-subscribe to all persisted scan IDs
- [ ] Implement a "catch-up" mechanism: query the server for scan state for the duration of the disconnection
- [ ] Display a "Reconnecting..." indicator in the UI when the WebSocket is down, and a "Connected" indicator when re-established
- [ ] Show missed scan progress as a toast notification when the connection is restored
- [ ] Write integration tests that simulate WebSocket disconnection and verify subscription recovery

---

## Issue 29: [Bounty Marketplace] Payout Function Does Not Verify Escrow Balance Before Emitting `payout_ready`

**Description:**
The `BountyMarketplace` contract in `src/bounty_marketplace.rs` has a `claim_reward` function that calculates the bounty payout amount based on severity and emits a `payout_ready` event. However, it does not verify that the contract's escrow balance is sufficient to cover the payout before emitting the event. The `emit_payout_ready` function checks that `amount > 0` (positive), but does not check that the contract holds at least `amount` in XLM. If multiple bounties are approved but the escrow balance is insufficient (e.g., due to a prior payout that drained the contract), `payout_ready` is emitted even though the payout cannot be fulfilled. This creates a "phantom payout" scenario where researchers believe they will receive a reward, attempt to claim it, and fail because the funds do not exist — leading to a poor user experience and potential legal disputes.

**Acceptance Criteria:**
- [ ] Add a `get_contract_balance(env)` helper that queries the contract's actual XLM balance
- [ ] Before emitting `payout_ready` in `claim_reward()`, verify `contract_balance >= reward_amount`
- [ ] If the escrow balance is insufficient, emit an `insufficient_escrow_funds` error event and provide details about the shortfall
- [ ] Add a `replenish_escrow(amount, funder)` function that allows the admin to top up the escrow balance
- [ ] Expose the current escrow balance via a `GET /api/v1/bounty/escrow-balance` endpoint
- [ ] Write contract tests that simulate an escrow deficit scenario and verify that `payout_ready` is not emitted

---

## Issue 30: [Documentation] No API Reference or Usage Examples for the Differential Fuzzing Module

**Description:**
The differential fuzzing module is one of the most technically sophisticated features of the Soroban Security Scanner, supporting multi-SDK-version execution, cross-contract simulation, ledger snapshot integration, and deterministic behavior detection. However, the module has no user-facing documentation beyond the code comments. The `docs/` directory contains documentation for time-based attacks, upgrade mechanisms, web fonts, balance checks, and accessibility, but no `DIFFERENTIAL_FUZZING.md` file. There are no usage examples showing how to configure and run differential fuzzing for a real contract, how to interpret the `DifferentialFuzzingReport` output, or how to fix discrepancies found by the fuzzer. The `examples/` directory contains example files for vulnerable contracts, secure contracts, Kubernetes scanning, scanner registry usage, and auth server, but no example contract for differential fuzzing. A new user cannot determine what `sdk_versions`, `edge_case_types`, or `gas_threshold_percentage` configurations mean for their use case without reading the source code.

**Acceptance Criteria:**
- [ ] Create `docs/DIFFERENTIAL_FUZZING.md` with sections: Overview, Quick Start, Configuration Guide, Interpreting Results, Common Issues, and Best Practices
- [ ] Add a commented example contract at `examples/differential_fuzzing_example.rs` with known discrepancies to demonstrate fuzzing output
- [ ] Create a configuration example at `examples/differential_fuzzing_config.toml` with annotated fields
- [ ] Include CLI usage examples for all differential fuzzing subcommands (run, generate-inputs, compare-versions, validate-deterministic, test-with-network-state, analyze-reentrancy)
- [ ] Add a troubleshooting section for common issues: "No discrepancies found when there should be", "Cross-contract simulation times out", "Ledger snapshot integration fails"
- [ ] Link the new documentation from the main `README.md` under a "Differential Fuzzing" heading

---

## Summary

These 30 issues cover the following areas of the Soroban Security Scanner platform:

| Area | Issues |
|------|--------|
| Frontend / UI / UX | 1, 6, 15, 16, 19, 25, 28 |
| Core Scanner / Detection | 2, 7, 10, 11, 20 |
| Authentication / Authorization | 3, 24 |
| Wallet / Multi-Sig | 4, 5, 19 |
| Smart Contracts | 26, 29 |
| Security / Cryptography | 14, 21 |
| Performance / Infra | 16, 25, 17, 18 |
| Data / Persistence | 6, 12, 17 |
| i18n / L10n | 22 |
| Documentation | 30 |
| CI/CD / Testing | 18, 23 |
| Differential Fuzzing | 9, 27 |
| Gas / Events | 10, 13 |
| Offline / Connectivity | 23, 28 |

Each issue includes a detailed description (6-12 lines), 4-7 actionable acceptance criteria, and references to specific files in the codebase.
