# Issue 12: [Event Logging] No Export or Query API for Compliance Auditing

## Description

The `EventLogger` in `src/event_logging.rs` stores critical events in memory with optional persistence, and provides query methods to retrieve events by operation type, time range, or actor. However, there is no pagination support, no CSV/JSON export capability, and no SQL-backed query API for compliance auditors who need to review historical events. The in-memory storage is bounded by `max_events_in_memory` (default 10,000), which means older events are silently dropped without any archival strategy. For SOC 2 Type II and GDPR compliance, the platform needs to support querying events over multi-month periods, exporting audit logs in machine-readable formats, and ensuring tamper-evident event storage (e.g., hash chains).

## Acceptance Criteria

- [ ] Add pagination (offset/limit) to all event query methods (`get_events_by_operation`, `get_events_by_time_range`, `get_events_by_actor`)
- [ ] Implement CSV and JSON export endpoints: `GET /api/v1/events/export?format=csv&start=...&end=...`
- [ ] Add an optional SQL-backed event store (using `src/database/models.rs` and `src/database/queries.rs`) for production deployments
- [ ] Implement event hash chaining: each event includes the hash of the previous event, forming a tamper-evident chain
- [ ] Add a `verify_chain(from_event, to_event)` method that validates the integrity of a range of events
- [ ] Create a database migration (`007_add_event_log_store.sql`) for the SQL-backed event persistence

## Additional Context

Key files: `src/event_logging.rs`, `src/database/models.rs`, `src/database/queries.rs`, `migrations/`.
