# Issue 18: [CI/CD] Automated Accessibility Tests Not Running in CI Pipeline

## Description

The accessibility test suite (`tests/e2e/accessibility.spec.js` and `tests/accessibility/a11y.spec.ts`) uses `@axe-core/playwright` to scan key frontend pages for WCAG 2.1 AA violations. These tests are defined in `playwright.config.js` and can be run locally with `npm run test:a11y`. However, they are not integrated into the GitHub Actions CI pipeline (no `.github/workflows/` file is present in the repository). This means that new PRs introducing accessibility regressions are merged without automated checking. The `docs/ACCESSIBILITY_TESTING.md` documentation states that these tests "run on every push and PR via the Accessibility (axe-core) GitHub Actions workflow," but this workflow file does not actually exist, creating a documentation-to-implementation gap.

## Acceptance Criteria

- [ ] Create a `.github/workflows/accessibility.yml` workflow file that triggers on push and pull_request events
- [ ] The workflow should: install dependencies, build the frontend, start the dev server, run Playwright accessibility tests
- [ ] Configure the workflow to fail if any critical or serious axe-core violations are detected (configurable threshold)
- [ ] Add a status badge to the `README.md` showing accessibility test pass/fail status
- [ ] Add a comment to the PR when accessibility violations are detected, listing the specific violations and affected components
- [ ] Set up Playwright test artifact retention (screenshots, trace files) for debugging failed tests

## Additional Context

Key files: `tests/e2e/accessibility.spec.js`, `tests/accessibility/a11y.spec.ts`, `playwright.config.js`, `docs/ACCESSIBILITY_TESTING.md`, `README.md`.
