import { test, expect } from '@playwright/test';

test.describe('Task List Page', () => {
  test('kanban view renders', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks');
    await page.waitForLoadState('domcontentloaded');
    // Page should render
    const pageContent = page.locator('body');
    await expect(pageContent).toBeVisible();
  });

  test('view mode toggle exists', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks');
    await page.waitForLoadState('domcontentloaded');
    // Toggle buttons should exist
    const kanbanBtn = page.locator('text=Kanban');
    const listBtn = page.locator('text=List');
    await expect(kanbanBtn).toBeVisible();
    await expect(listBtn).toBeVisible();
  });

  test('can toggle between kanban and list view', async ({ page }) => {
    await page.goto('/plans/main/test-plan/tasks');
    await page.waitForLoadState('domcontentloaded');
    // Click list view button
    await page.locator('text=List').click();
    // List view table should appear
    const tableHeader = page.locator('text=Task');
    await expect(tableHeader).toBeVisible();
  });
});
