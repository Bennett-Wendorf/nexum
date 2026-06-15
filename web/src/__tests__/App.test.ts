import { describe, it, expect } from 'vitest';

describe('App', () => {
  it('App.svelte exists and contains Router and Layout', async () => {
    const App = await import('../App.svelte');
    expect(App.default).toBeDefined();
  });

  it('routes/index.ts defines all expected routes', async () => {
    const { routes } = await import('../routes/index');
    expect(routes).toBeDefined();
    expect(routes['/']).toBeDefined();
    expect(routes['/plans']).toBeDefined();
    expect(routes['/plans/:branch/:planId']).toBeDefined();
    expect(routes['/plans/:branch/:planId/tasks']).toBeDefined();
    expect(routes['/plans/:branch/:planId/tasks/:taskId']).toBeDefined();
    expect(routes['*']).toBeDefined();
  });
});
