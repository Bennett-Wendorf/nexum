import { describe, it, expect, vi, beforeEach } from 'vitest';
import { request, listPlans, getPlan, createPlan, listTasks, getTask, healthCheck } from '../src/lib/api';

describe('API Client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe('request()', () => {
    it('constructs URL with /api/v1 prefix', async () => {
      const mockData = { items: [], total: 0 };
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: true,
        headers: { get: (key: string) => 'application/json' },
        json: async () => mockData,
      } as Response);

      await request('GET', '/plans');

      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/plans', expect.any(Object));
    });

    it('appends query parameters', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: true,
        headers: { get: (key: string) => 'application/json' },
        json: async () => ({}),
      } as Response);

      await request('GET', '/plans', undefined, { branch: 'main', status: 'approved' });

      expect(globalThis.fetch).toHaveBeenCalledWith(
        '/api/v1/plans?branch=main&status=approved',
        expect.any(Object),
      );
    });

    it('throws error on non-200 response', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: false,
        status: 404,
        headers: { get: (key: string) => 'application/json' },
        json: async () => ({ error: 'not_found', message: 'Plan not found', status: 404 }),
      } as Response);

      await expect(request('GET', '/plans/test/123')).rejects.toThrow('Plan not found');
    });

    it('sends JSON body when provided', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: true,
        headers: { get: (key: string) => 'application/json' },
        json: async () => ({}),
      } as Response);

      const body = { name: 'test', branch: 'main', goal: 'test goal' };
      await request('POST', '/plans', body);

      const fetchCall = (globalThis.fetch as any).mock.calls[0][1];
      expect(fetchCall.headers).toEqual({ 'Content-Type': 'application/json' });
      expect(fetchCall.body).toEqual(JSON.stringify(body));
    });
  });

  describe('listPlans()', () => {
    it('calls GET /api/v1/plans', async () => {
      const mockData = { items: [], total: 0 };
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: true,
        headers: { get: (key: string) => 'application/json' },
        json: async () => mockData,
      } as Response);

      const result = await listPlans();

      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/plans', expect.any(Object));
      expect(result).toEqual(mockData);
    });
  });

  describe('getPlan()', () => {
    it('calls GET /api/v1/plans/:branch/:planId', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: true,
        headers: { get: (key: string) => 'application/json' },
        json: async () => ({ id: '123', name: 'Test Plan' }),
      } as Response);

      await getPlan('main', '123');

      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/plans/main/123', expect.any(Object));
    });
  });

  describe('healthCheck()', () => {
    it('calls GET /api/v1/health', async () => {
      vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
        ok: true,
        headers: { get: (key: string) => 'application/json' },
        json: async () => ({ status: 'ok' }),
      } as Response);

      const result = await healthCheck();

      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/health', expect.any(Object));
      expect(result).toEqual({ status: 'ok' });
    });
  });
});
