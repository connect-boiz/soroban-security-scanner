# Issue 15: [Frontend] Accessibility Violations in Scan Results Table — Missing ARIA Labels and Keyboard Navigation

## Description

The `VulnerabilityReport` component in `frontend/components/VulnerabilityReport.tsx` renders a data table displaying scan results with columns for severity, vulnerability type, file path, and recommendation. This table has several accessibility violations detected by `@axe-core/playwright` in the automated accessibility test suite (`tests/accessibility/a11y.spec.ts`). Specifically: (1) the table lacks a `<caption>` element describing its purpose, (2) sort buttons in column headers do not have `aria-label` attributes indicating the sort direction, (3) the severity color-coding (red for Critical, yellow for High, etc.) relies solely on color without accompanying text or icons, (4) the expandable rows for vulnerability details are not keyboard-accessible (missing `role="button"`, `tabindex`, and keyboard event handlers), and (5) the "Copy to Clipboard" button does not announce the copy action to screen readers. These violations prevent security researchers who rely on assistive technologies from effectively reviewing scan results.

## Acceptance Criteria

- [ ] Add a `<caption>` element to the results table: "Security Scan Results — showing {{count}} vulnerabilities"
- [ ] Add `aria-label` attributes to all column sort buttons with the format "Sort by {{column}} (currently {{direction}})"
- [ ] Add descriptive text labels alongside severity color indicators (e.g., a "Critical" badge with red background + text)
- [ ] Make expandable rows keyboard-navigable with `role="button"`, `tabindex="0"`, and `onKeyDown` handlers for Enter/Space
- [ ] Add `aria-live="polite"` announcement region for copy-to-clipboard actions
- [ ] Update automated accessibility tests to verify all new ARIA attributes are present
- [ ] Run axe-core against the vulnerability report page and confirm zero critical/serious violations

## Additional Context

Key files: `frontend/components/VulnerabilityReport.tsx`, `tests/accessibility/a11y.spec.ts`, `tests/e2e/accessibility.spec.js`.
