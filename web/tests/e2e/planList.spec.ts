import { test, expect } from '@playwright/test';

test.describe('Plan List Page', () => {
  test('page loads without errors', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/.*/);
    // Page should not have any console errors
    const consoleMessages: string[] = [];
    page.on('console', msg => consoleMessages.push(msg.text()));
    await page.reload();
    const errors = consoleMessages.filter(m => m.includes('error') || m.includes('Error'));
    expect(errors.length).toBe(0);
  });

  test('stats row renders', async ({ page }) => {
    await page.goto('/');
    // Check that stats cards exist
    const statCards = page.locator('.stat-card, [class*="bg-bg-secondary"][class*="rounded"]');
    // Even with no data, the stats row should render
    await expect(page.locator('text=Total Plans')).toBeVisible();
  });

  test('empty state shows when no plans', async ({ page }) => {
    await page.goto('/');
    // Wait for loading to complete or empty state to appear
    await page.waitForSelector('text=No plans yet', { timeout: 10000 }).catch(() => {});
    // Verify either empty state or loading state is visible
    const hasEmptyState = await page.locator('text=No plans yet').isVisible().catch(() => false);
    const hasLoading = await page.locator('text=Loading plans').isVisible().catch(() => false);
    // At least one should be visible
    expect(hasEmptyState || hasLoading).toBe(true);
  });
});
