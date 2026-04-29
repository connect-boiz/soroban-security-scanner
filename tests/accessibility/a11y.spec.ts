// tests/accessibility/a11y.spec.ts
//
// WCAG 2.1 AA automated accessibility tests.
// Runs in CI via the existing Accessibility GitHub Actions workflow.
//
// ── Install required packages first ─────────────────────────────────────────
//   npm install --save-dev @playwright/test @axe-core/playwright
//   # or
//   pnpm add -D @playwright/test @axe-core/playwright
//
// Run locally:  npx playwright test tests/accessibility/
// In CI:        npm run test:a11y

import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

// ── Routes to audit ───────────────────────────────────────────────────────────
interface Route {
  path: string;
  name: string;
}

const ROUTES: Route[] = [
  { path: "/",             name: "Homepage" },
  { path: "/dashboard",   name: "Dashboard" },
  { path: "/scan",        name: "Scan page" },
  { path: "/results",     name: "Results page" },
  { path: "/auth/login",  name: "Login page" },
  { path: "/auth/signup", name: "Sign-up page" },
  { path: "/settings",    name: "Settings page" },
];

// ── axe tag list ─────────────────────────────────────────────────────────────
// Typed as string[] — avoids the conditional-type error that occurs when
// the @axe-core/playwright package isn't yet installed during type-checking.
const AXE_WCAG_AA_TAGS: string[] = [
  "wcag2a",
  "wcag2aa",
  "wcag21a",
  "wcag21aa",
  "best-practice",
];

// ── Helper: build a configured AxeBuilder for a page ─────────────────────────
function buildAxe(page: Page): AxeBuilder {
  return new AxeBuilder({ page })
    .withTags(AXE_WCAG_AA_TAGS)
    // Known false-positive: third-party Stellar wallet iframe
    .exclude("#stellar-iframe");
}

// ── Helper: format axe violations for readable test output ───────────────────
function formatViolations(
  violations: Awaited<ReturnType<AxeBuilder["analyze"]>>["violations"],
  path: string
): string {
  if (violations.length === 0) return "";
  return (
    `\naxe violations on ${path}:\n` +
    violations
      .map(
        (v) =>
          `  [${v.impact ?? "unknown"}] ${v.id}: ${v.description}\n` +
          v.nodes
            .slice(0, 2)
            .map((n) => `    → ${n.html.slice(0, 120)}`)
            .join("\n")
      )
      .join("\n\n")
  );
}

// ── Per-page automated scans ──────────────────────────────────────────────────
for (const route of ROUTES) {
  test(`[a11y] ${route.name} — WCAG 2.1 AA (axe)`, async ({ page }: { page: Page }) => {
    await page.goto(route.path);
    await page.waitForLoadState("networkidle");

    const { violations } = await buildAxe(page).analyze();

    expect(violations, formatViolations(violations, route.path)).toHaveLength(0);
  });
}

// ── Skip link ─────────────────────────────────────────────────────────────────
test("[a11y] Skip link is first focusable element and functional", async ({ page }: { page: Page }) => {
  await page.goto("/");

  // First Tab press should reach the skip link
  await page.keyboard.press("Tab");
  const focusedText = await page.evaluate(
    (): string => document.activeElement?.textContent?.trim() ?? ""
  );
  expect(focusedText).toContain("Skip to main content");

  // Activating it should move focus into #main-content
  await page.keyboard.press("Enter");
  const focusedId = await page.evaluate(
    (): string => document.activeElement?.id ?? ""
  );
  expect(focusedId).toBe("main-content");
});

// ── Keyboard reachability ─────────────────────────────────────────────────────
test("[a11y] All interactive elements reachable by keyboard only", async ({ page }: { page: Page }) => {
  await page.goto("/scan");
  await page.waitForLoadState("networkidle");

  const visited: string[] = [];

  for (let i = 0; i < 60; i++) {
    await page.keyboard.press("Tab");
    const tag = await page.evaluate(
      (): string | null => document.activeElement?.tagName.toLowerCase() ?? null
    );
    if (!tag || tag === "body") break;
    visited.push(tag);
  }

  // The scan form and its submit button must both be keyboard-reachable
  expect(visited).toContain("input");
  expect(visited).toContain("button");
});

// ── Dialog focus trap ─────────────────────────────────────────────────────────
test("[a11y] Modal dialog traps and restores focus", async ({ page }: { page: Page }) => {
  await page.goto("/results");
  await page.waitForLoadState("networkidle");

  const triggerBtn = page.getByRole("button", { name: /view details/i }).first();
  await triggerBtn.focus();
  await triggerBtn.click();

  // Dialog visible
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();

  // Focus is inside the dialog
  const focusInDialog = await page.evaluate((): boolean => {
    const dlg = document.querySelector<HTMLElement>("[role='dialog']");
    return dlg?.contains(document.activeElement) ?? false;
  });
  expect(focusInDialog).toBe(true);

  // Escape closes the dialog
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();

  // Focus is restored to the trigger
  const restoredText = await page.evaluate(
    (): string => document.activeElement?.textContent?.trim() ?? ""
  );
  expect(restoredText).toMatch(/view details/i);
});

// ── Status badges — not colour-only ──────────────────────────────────────────
test("[a11y] Status badges do not rely on colour alone", async ({ page }: { page: Page }) => {
  await page.goto("/results");
  await page.waitForLoadState("networkidle");

  const badges = page.locator("[aria-label*='severity']");
  const count = await badges.count();
  expect(count).toBeGreaterThan(0);

  for (let i = 0; i < Math.min(count, 10); i++) {
    const label = await badges.nth(i).getAttribute("aria-label");
    expect(label).toBeTruthy();
    expect(label!.length).toBeGreaterThan(0);
  }
});

// ── Form error announcement ───────────────────────────────────────────────────
test("[a11y] Form errors are announced via role=alert", async ({ page }: { page: Page }) => {
  await page.goto("/scan");
  await page.waitForLoadState("networkidle");

  // Submit the empty form to trigger client-side validation
  await page.getByRole("button", { name: /scan|submit/i }).click();

  const alert = page.locator("[role='alert']").first();
  await expect(alert).toBeVisible();

  const text = await alert.textContent();
  expect((text ?? "").trim().length).toBeGreaterThan(0);
});

// ── Reduced motion ────────────────────────────────────────────────────────────
test("[a11y] Animations are suppressed under prefers-reduced-motion", async ({ page }: { page: Page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/dashboard");
  await page.waitForLoadState("networkidle");

  const hasLongAnimation = await page.evaluate((): boolean => {
    const elements = Array.from(document.querySelectorAll("*"));
    return elements.some((el) => {
      const { animationDuration } = window.getComputedStyle(el);
      // CSS `0.01ms !important` override should make this effectively 0
      return parseFloat(animationDuration) > 0.05;
    });
  });

  expect(hasLongAnimation).toBe(false);
});

// ── Landmark structure ────────────────────────────────────────────────────────
test("[a11y] Page has required ARIA landmark regions", async ({ page }: { page: Page }) => {
  await page.goto("/");
  await page.waitForLoadState("networkidle");

  await expect(page.getByRole("main")).toBeVisible();
  await expect(page.getByRole("navigation")).toBeVisible();
  await expect(page.getByRole("banner")).toBeVisible(); // <header>
});

// ── Image alt attributes ──────────────────────────────────────────────────────
test("[a11y] All images have an alt attribute", async ({ page }: { page: Page }) => {
  await page.goto("/");
  await page.waitForLoadState("networkidle");

  const missingAlt = await page.evaluate((): string[] =>
    Array.from(document.querySelectorAll("img"))
      .filter((img) => img.getAttribute("alt") === null) // null = missing; "" = decorative (OK)
      .map((img) => img.src)
  );

  expect(
    missingAlt,
    `Images missing alt attribute:\n${missingAlt.join("\n")}`
  ).toHaveLength(0);
});