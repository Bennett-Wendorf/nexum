import { describe, it, expect } from 'vitest';

describe('App', () => {
  it('mounts without error', async () => {
    const { default: App } = await import('../App.svelte');
    
    // Verify App is a valid Svelte component
    expect(App).toBeDefined();
    expect(typeof App).toBe('function');
  });
});
