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

  test('Arabic RTL layout: sidebar on right, content flowing RTL, form labels right-aligned, arrow icons pointing correctly', async ({ page }) => {
    // Provide Arabic locale via Accept-Language and custom header
    await page.setExtraHTTPHeaders({
      'Accept-Language': 'ar-SA,ar;q=0.9',
      'x-locale': 'ar',
    });

    await page.goto('/?lang=ar');

    // 1. Root element has dir="rtl" and lang="ar"
    const html = page.locator('html');
    await expect(html).toHaveAttribute('dir', 'rtl');
    await expect(html).toHaveAttribute('lang', 'ar');

    // 2. Navigation / sidebar flows from the right in RTL
    const sidebar = page.locator('aside, [role="navigation"], [data-testid="sidebar"]').first();
    if (await sidebar.isVisible()) {
      const box = await sidebar.boundingBox();
      const viewport = page.viewportSize();
      if (box && viewport) {
        expect(box.x + box.width).toBeGreaterThanOrEqual(viewport.width / 2);
      }
    }

    // 3. Form labels are right-aligned
    const label = page.locator('label').first();
    if (await label.isVisible()) {
      const textAlign = await label.evaluate((el) => window.getComputedStyle(el).textAlign);
      expect(['right', 'start']).toContain(textAlign);
    }

    // 4. Directional icons / arrows are mirrored
    const arrowIcon = page.locator('[data-mirrored="true"], .flip-rtl, .rtl-aware-icon.mirrored').first();
    if (await arrowIcon.isVisible()) {
      const transform = await arrowIcon.evaluate((el) => window.getComputedStyle(el).transform);
      expect(transform).not.toBe('none');
    }

    // 5. Visual layout snapshot for RTL
    await expect(page).toHaveScreenshot('arabic-rtl-layout.png', {
      fullPage: true,
      animations: 'disabled',
    });
  });
});
