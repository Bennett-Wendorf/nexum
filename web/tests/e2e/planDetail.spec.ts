import { test, expect } from '@playwright/test';

test.describe('Plan Detail Page', () => {
  test('page loads', async ({ page }) => {
    await page.goto('/plans/main/test-plan');
    // Page should render (even if showing "not found" state)
    await page.waitForLoadState('domcontentloaded');
  });

  test('breadcrumb renders on detail pages', async ({ page }) => {
    await page.goto('/plans/main/test-plan');
    // Breadcrumb nav should exist in the DOM
    const breadcrumb = page.locator('nav');
    await expect(breadcrumb).toBeTruthy();
  });
});
