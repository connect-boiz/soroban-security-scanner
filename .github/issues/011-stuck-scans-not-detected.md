# Issue 11: [Emergency Stop] Stuck Scans Not Automatically Detected or Cleaned Up

## Description

The `EmergencyStop` mechanism in `src/emergency_stop.rs` relies on external triggers (user-initiated, critical vulnerability detection, or manual signal handlers) to stop scanning operations. There is no watchdog or health-check system that can detect when a scan has become stuck (e.g., hung on a large contract file, deadlocked in a regex operation, or waiting indefinitely on a network call). The `K8sScanManager` has a `cleanup_stuck_scans()` method, but this only cleans up Kubernetes pods — it does not handle stuck scans in the core Rust scanner. In practice, a single hung scan can block the entire scanning pipeline (since the scanner processes files sequentially in `scan_directory`), causing timeouts for all subsequent scan requests and degrading the overall platform user experience.

## Acceptance Criteria

- [ ] Implement a `ScanWatchdog` that monitors scan progress via a heartbeat mechanism (each file update resets a timer)
- [ ] If no heartbeat is received for `watchdog_timeout_seconds` (configurable, default 120s), automatically trigger emergency stop via `trigger_stop()`
- [ ] Add a per-file processing timeout in `scan_file()` that aborts processing of individual files exceeding the time limit
- [ ] Expose watchdog status via a new API endpoint `GET /api/v1/scanner/watchdog-status`
- [ ] Log watchdog events (heartbeat, timeout, automatic stop) to the event logging system
- [ ] Write tests simulating a stuck operation and verifying automatic cleanup within the timeout period

## Additional Context

Key files: `src/emergency_stop.rs`, `src/scanners.rs`, `src/security_analyzer.rs`.
