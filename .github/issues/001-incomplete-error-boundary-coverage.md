# Issue 1: [Frontend] Incomplete Error Boundary Coverage Leaves Unhandled Component Crashes Silent

## Description

Currently, the frontend has a basic `ErrorBoundary` component (`frontend/components/ui/ErrorBoundary.tsx`) that catches runtime errors in child components, but it is only applied at the top-level layout. Many deeply nested components such as `ScannerInterface`, `VulnerabilityReport`, `MultiSigWizard`, and `AnalyticsDashboard` are not individually wrapped with error boundaries. When one of these components crashes, the entire page falls back to a single generic error screen, losing user session state and scan progress. Additionally, the current error boundary does not provide detailed crash information to developers (e.g., stack traces, component name, props at time of crash) nor does it offer a "retry" mechanism that could restore just the failed section without reloading the entire page. This creates a poor user experience for security researchers running long-duration scans who might lose all progress due to a transient error in a non-critical visualization component.

## Acceptance Criteria

- [ ] Wrap each major page-level component (ScannerInterface, VulnerabilityReport, MultiSigWizard, AnalyticsDashboard, SettingsPanel, BatchOperations) with its own `ErrorBoundary` instance
- [ ] Each error boundary should render a component-specific fallback UI (e.g., "Scan results failed to load — retry") rather than a generic page-level message
- [ ] Add a "retry" button to each fallback that attempts to re-render the errored component
- [ ] Implement logging to `errorReporting.ts` with component name, stack trace, and timestamp on each caught error
- [ ] Write unit tests for each new error boundary instance using `@testing-library/react`
- [ ] Verify that an error in the ScannerInterface does not crash the VulnerabilityReport or navigation sidebar

## Additional Context

Affected components are located in the `frontend/components/` directory. The existing `ErrorBoundary.tsx` is at `frontend/components/ui/ErrorBoundary.tsx`.
