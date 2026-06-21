# Issue 23: [Offline Support] Service Worker Caches Scan Results but Does Not Provide Offline Access to Cached Data

## Description

The service worker registered in `public/sw.js` implements a caching strategy for static assets and API responses using `public/offline-storage.js`, `public/offline-integration.js`, and `public/offline-sync.js`. However, scan results cached by the service worker cannot be accessed by the user when they are offline. The `VulnerabilityReport` component does not check for cached data before making an API call, and there is no "offline mode" indicator in the UI. When the browser goes offline, the scan results page shows an error message instead of displaying the last-cached results. The `offline.html` page is only shown when the navigation itself fails, not when cached data exists but the network request for fresh data fails. This means users who have previously viewed scan results cannot access them during network outages, which is a common scenario for security researchers working in remote or air-gapped environments.

## Acceptance Criteria

- [ ] Implement an "offline-first" data fetching pattern: show cached data immediately, then update with fresh data when online
- [ ] Add a banner/indicator in the UI when the app is in offline mode: "You are viewing cached data — last synced [time]"
- [ ] Update the `VulnerabilityReport` component to fall back to cached scan results when the API is unreachable
- [ ] Add an IndexedDB-backed cache for scan results in `public/offline-storage.js` that persists beyond browser restarts
- [ ] Implement a background sync mechanism in `public/offline-sync.js` that queues user actions (like submitting a scan) and retries when online
- [ ] Write offline-mode tests using Playwright that simulate network disconnection and verify cached content is displayed

## Additional Context

Key files: `public/sw.js`, `public/offline-storage.js`, `public/offline-integration.js`, `public/offline-sync.js`, `public/offline.html`, `frontend/components/VulnerabilityReport.tsx`.
