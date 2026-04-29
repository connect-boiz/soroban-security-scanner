# Accessibility Testing

Automated accessibility testing for the Soroban Security Scanner frontend using
[axe-core](https://github.com/dequelabs/axe-core) integrated with Playwright.

## Overview

Tests run on every push and pull request against `main` and `develop` via the
`Accessibility (axe-core)` GitHub Actions workflow
(`.github/workflows/accessibility.yml`). Locally they run through Playwright.

The suite checks the application against the following rule sets:

- WCAG 2.0 Level A
- WCAG 2.0 Level AA
- WCAG 2.1 Level A
- WCAG 2.1 Level AA

This complements the existing Lighthouse accessibility budget configured in
`lighthouserc.js`: Lighthouse measures the accessibility category score on
production-like builds; axe-core provides rule-level violations against
specific routes and components, which is more actionable for fixes.

## Running Locally

Install dependencies (the first time):

```bash
npm install
npx playwright install --with-deps chromium
```

Run the full accessibility suite:

```bash
npm run test:a11y
```

Run against a specific URL:

```bash
BASE_URL=https://staging.example.com npm run test:a11y
```

Open the HTML report after a run:

```bash
npx playwright show-report
```

## Test Structure

The suite lives in `tests/e2e/accessibility.spec.js`:

- **Per-route scans** — full-page axe analysis for each public route (home,
  results) using the WCAG A/AA tag sets.
- **Form scope** — narrows axe to the `form` element to ensure inputs expose
  accessible names, labels, and roles.
- **Navigation scope** — narrows axe to the `nav` landmark.
- **Color contrast** — focused run of the `color-contrast` rule on the home
  page.

When a test fails, the violations are printed to the console with their `id`,
`impact`, `help` text, and a `helpUrl` linking to Deque's remediation guide.

## Adding a New Page to the Suite

Append the route to the `ROUTES` array in
`tests/e2e/accessibility.spec.js`:

```js
const ROUTES = [
  { name: 'home page', path: '/' },
  { name: 'scan results page', path: '/results' },
  { name: 'new page',         path: '/new-page' },
];
```

## Tuning Rules

Disable a rule (with justification) using `disableRules`:

```js
const results = await new AxeBuilder({ page })
  .withTags(WCAG_TAGS)
  .disableRules(['region']) // false positive: app intentionally lacks a region landmark on this page
  .analyze();
```

Prefer fixing violations over disabling rules. Document any disabled rule with
an inline comment explaining the rationale.

## CI/CD

The workflow installs only Chromium (a single browser is sufficient for axe
checks since axe analyzes the rendered DOM, not browser-specific behavior),
runs `npm run test:a11y`, and uploads the Playwright report as the
`a11y-report` artifact for 30 days.

A failing axe assertion fails the workflow, blocking merges that introduce
WCAG violations.
