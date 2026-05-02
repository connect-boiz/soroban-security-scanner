// @ts-check
const { test, expect } = require('@playwright/test');

test.describe('Visual Regression Tests', () => {
  test('home page matches snapshot', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveScreenshot('home-page.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });

  test('scan results page matches snapshot', async ({ page }) => {
    await page.goto('/results');
    await expect(page).toHaveScreenshot('results-page.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });

  test('vulnerability report card matches snapshot', async ({ page }) => {
    await page.goto('/results');
    const card = page.locator('[data-testid="vulnerability-card"]').first();
    if (await card.isVisible()) {
      await expect(card).toHaveScreenshot('vulnerability-card.png');
    }
  });
});
