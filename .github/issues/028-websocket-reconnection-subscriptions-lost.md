# Issue 28: [Frontend] WebSocket Reconnection Does Not Preserve In-Flight Scan Subscriptions

## Description

The frontend `useWebSocket` hook in `frontend/hooks/useWebSocket.ts` manages a WebSocket connection to receive real-time scan progress updates. When the WebSocket connection drops (e.g., due to network interruption, server restart, or idle timeout), the hook attempts to reconnect. However, during reconnection, any active scan subscriptions (the list of scan IDs the user was monitoring) are lost. The user must manually navigate away and back to the scan page to re-subscribe. Furthermore, when the connection re-establishes, the user misses any scan progress events that were sent during the disconnect period. For long-running scans (30+ minutes), a brief network interruption can cause the user to lose all progress visibility for the remaining scan duration, leading them to believe the scan is stuck or has failed.

## Acceptance Criteria

- [ ] Persist active scan subscriptions in browser `sessionStorage` so they survive WebSocket reconnections
- [ ] On WebSocket reconnect, automatically re-subscribe to all persisted scan IDs
- [ ] Implement a "catch-up" mechanism: query the server for scan state for the duration of the disconnection
- [ ] Display a "Reconnecting..." indicator in the UI when the WebSocket is down, and a "Connected" indicator when re-established
- [ ] Show missed scan progress as a toast notification when the connection is restored
- [ ] Write integration tests that simulate WebSocket disconnection and verify subscription recovery

## Additional Context

Key files: `frontend/hooks/useWebSocket.ts`, `frontend/lib/notifications/utils.ts`.
