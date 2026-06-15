import { test, expect } from '@playwright/test';

test.describe('Task Detail Page', () => {
  test('page loads', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks/task-1');
    await page.waitForLoadState('domcontentloaded');
  });

  test('breadcrumb renders', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks/task-1');
    const breadcrumb = page.locator('nav');
    await expect(breadcrumb).toBeTruthy();
  });

  test('tabs exist', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks/task-1');
    const definitionTab = page.locator('text=Task Definition');
    const historyTab = page.locator('text=Status History');
    await expect(definitionTab).toBeTruthy();
    await expect(historyTab).toBeTruthy();
  });

  test('status transition dropdown exists', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks/task-1');
    const dropdown = page.locator('select');
    await expect(dropdown).toBeTruthy();
  });
});
