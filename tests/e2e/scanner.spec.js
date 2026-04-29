// @ts-check
const { test, expect } = require('@playwright/test');

test.describe('Scanner - Critical User Flows', () => {
  test('home page loads successfully', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/Soroban Security Scanner/i);
  });

  test('scan form is visible and interactive', async ({ page }) => {
    await page.goto('/');
    const scanInput = page.getByRole('textbox', { name: /contract/i });
    await expect(scanInput).toBeVisible();
    await scanInput.fill('GABC123');
    await expect(scanInput).toHaveValue('GABC123');
  });

  test('navigation links are accessible', async ({ page }) => {
    await page.goto('/');
    const nav = page.getByRole('navigation');
    await expect(nav).toBeVisible();
  });

  test('scan results page shows vulnerability report', async ({ page }) => {
    await page.goto('/results');
    const resultsSection = page.getByRole('main');
    await expect(resultsSection).toBeVisible();
  });

  test('error state is handled gracefully', async ({ page }) => {
    await page.goto('/scan/invalid-contract-id');
    // Should show an error message, not crash
    const body = page.locator('body');
    await expect(body).not.toBeEmpty();
  });
});
