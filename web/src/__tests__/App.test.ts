import { describe, it, expect } from 'vitest';

describe('App', () => {
  it('imports without error', async () => {
    const { default: App } = await import('../App.svelte');
    expect(App).toBeDefined();
  });
});
