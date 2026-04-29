// @ts-check
const { test, expect } = require('@playwright/test');
const AxeBuilder = require('@axe-core/playwright').default;

const WCAG_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'];

const ROUTES = [
  { name: 'home page', path: '/' },
  { name: 'scan results page', path: '/results' },
];

test.describe('Accessibility (axe-core) - WCAG 2.1 AA', () => {
  for (const route of ROUTES) {
    test(`${route.name} has no detectable a11y violations`, async ({ page }) => {
      await page.goto(route.path);
      await page.waitForLoadState('domcontentloaded');

      const results = await new AxeBuilder({ page })
        .withTags(WCAG_TAGS)
        .analyze();

      if (results.violations.length > 0) {
        console.log(
          `\nA11y violations on ${route.path}:\n` +
            JSON.stringify(
              results.violations.map((v) => ({
                id: v.id,
                impact: v.impact,
                help: v.help,
                helpUrl: v.helpUrl,
                nodes: v.nodes.length,
              })),
              null,
              2
            )
        );
      }

      expect(results.violations).toEqual([]);
    });
  }

  test('scan form controls expose accessible names', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    const results = await new AxeBuilder({ page })
      .withTags(WCAG_TAGS)
      .include('form')
      .analyze();

    expect(results.violations).toEqual([]);
  });

  test('navigation landmark has no a11y violations', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    const nav = page.getByRole('navigation');
    if (await nav.count()) {
      const results = await new AxeBuilder({ page })
        .withTags(WCAG_TAGS)
        .include('nav')
        .analyze();

      expect(results.violations).toEqual([]);
    }
  });

  test('color contrast meets WCAG AA on home page', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('domcontentloaded');

    const results = await new AxeBuilder({ page })
      .withRules(['color-contrast'])
      .analyze();

    expect(results.violations).toEqual([]);
  });
});
