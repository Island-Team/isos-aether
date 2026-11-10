import { afterEach, describe, expect, it, vi } from 'vitest';
import { createIsosClient, isLoopbackOrigin } from './transport';

afterEach(() => { vi.unstubAllGlobals(); });

describe('shared IsOS transport', () => {
  it('accepts only a loopback origin', () => {
    expect(isLoopbackOrigin('http://127.0.0.1:16000')).toBe(true);
    expect(isLoopbackOrigin('https://127.0.0.1:16000')).toBe(false);
    expect(isLoopbackOrigin('http://example.com:16000')).toBe(false);
    expect(() => createIsosClient('http://example.com:16000')).toThrow('127.0.0.1');
  });
  it('uses only the IsOS API namespace and preserves typed failures', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: false, status: 503, json: async () => ({ schema_version: '0.1.0', ok: false, data: null, error: { code: 'owner_unavailable', message: 'EOS unavailable', details: null, correlation_id: 'req-1' } }) });
    vi.stubGlobal('fetch', fetchMock);
    const client = createIsosClient('http://127.0.0.1:16000');
    await expect(client('/cores/readiness')).rejects.toMatchObject({ status: 503, code: 'owner_unavailable' });
    expect(fetchMock.mock.calls[0][0]).toBe('http://127.0.0.1:16000/api/isos/v1/cores/readiness');
    await expect(client('//example.com')).rejects.toThrow('path must be absolute');
  });
});
