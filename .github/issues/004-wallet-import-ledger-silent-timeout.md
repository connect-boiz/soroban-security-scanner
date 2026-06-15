# Issue 4: [Wallet Management] Wallet Import from Ledger Hardware Device Fails Silently on Connection Timeout

## Description

The `WalletService` in `src/wallet/service.rs` supports importing wallets from hardware devices like Ledger via the `import_wallet` method. However, when a hardware device connection times out (e.g., device not connected, USB cable unplugged, or the Stellar app not open on the device), the import process fails with a generic `WalletError::ConnectionFailed` error that does not distinguish between a device-not-found scenario and a communication timeout. This leaves users confused about whether they need to reconnect the device, install the Stellar app, or try a different USB port. Furthermore, the frontend `AuthContainer` component does not display a meaningful progress indicator during the hardware import process, so users see a spinner indefinitely when the device is not actually connected. The import operation also lacks a configurable timeout, so it can block the UI thread for up to 120 seconds in some scenarios before failing.

## Acceptance Criteria

- [ ] Add specific error variants for `DeviceNotFound`, `ConnectionTimeout`, and `AppNotOpen` to `WalletError` in `src/wallet/types.rs`
- [ ] Implement a configurable timeout (default 30 seconds) for hardware wallet connection attempts
- [ ] Update the import UI to show a stepper/status indicator: "Connecting to device..." → "Opening Stellar app..." → "Importing keys..."
- [ ] Add retry logic (up to 3 attempts) with exponential backoff for transient connection failures
- [ ] Log detailed hardware interaction telemetry to `event_logging.rs` for debugging
- [ ] Write unit tests simulating device timeout, missing app, and successful import scenarios using mocked hardware interfaces

## Additional Context

Key files: `src/wallet/service.rs`, `src/wallet/types.rs`, `src/wallet/mod.rs`, `frontend/components/auth/AuthContainer.tsx`.
