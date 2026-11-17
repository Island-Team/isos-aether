import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

beforeEach(() => {
  vi.resetModules();
  vi.stubGlobal('ISOSUI_API_ORIGIN', 'http://127.0.0.1:16000');
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('IsOS API client', () => {
  it('uses the current node graph endpoint and sends a loopback request', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { revision: 'saved' }, error: null }) });
    vi.stubGlobal('fetch', fetchMock);
    const { request } = await import('./api-client');
    expect(await request('/graph')).toEqual({ revision: 'saved' });
    expect(fetchMock).toHaveBeenCalledWith('http://127.0.0.1:16000/api/isos/v1/graph', expect.objectContaining({ method: 'GET', credentials: 'omit', targetAddressSpace: 'loopback' }));
  });

  it('preserves revision conflict guidance', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 409, json: async () => ({ schema_version: '0.1.0', ok: false, data: null, error: { code: 'revision_conflict', message: 'changed', details: null, correlation_id: 'req-1' } }) }));
    const { request } = await import('./api-client');
    await expect(request('/graph', 'PUT', { revision: 'old' })).rejects.toMatchObject({ name: 'IsosApiError', status: 409, message: expect.stringContaining('Refresh before applying') });
  });

  it('distinguishes a stale active run from an editor revision conflict', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 409, json: async () => ({ schema_version: '0.1.0', ok: false, data: null, error: { code: 'graph_fence_stale', message: 'active graph changed', details: null, correlation_id: 'req-2' } }) }));
    const { request } = await import('./api-client');
    await expect(request('/nodes/monitor')).rejects.toMatchObject({ name: 'IsosApiError', status: 409, code: 'graph_fence_stale', message: expect.stringContaining('active run uses an older graph') });
  });

  it('rejects a mismatched API envelope', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: 2, ok: true, data: {}, error: null }) }));
    const { request } = await import('./api-client');
    await expect(request('/graph')).rejects.toThrow('unsupported API schema version');
  });

  it('re-reads the runtime API address after a transport outage without replaying the request', async () => {
    vi.stubGlobal('window', { location: { hostname: '127.0.0.1' } });
    let runtimeReads = 0;
    let apiReads = 0;
    const fetchMock = vi.fn(async (url: string) => {
      if (url === '/isosui-runtime.json') {
        runtimeReads += 1;
        return { ok: true, json: async () => ({ api_origin: runtimeReads === 1 ? 'http://127.0.0.1:16001' : 'http://127.0.0.1:16002' }) };
      }
      apiReads += 1;
      if (apiReads === 1) throw new Error('server restarting');
      return { ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { revision: 'saved' }, error: null }) };
    });
    vi.stubGlobal('fetch', fetchMock);
    const { request } = await import('./api-client');
    await expect(request('/graph')).rejects.toThrow('Cannot reach IsOS');
    expect(await request('/graph')).toEqual({ revision: 'saved' });
    expect(runtimeReads).toBe(2);
    expect(fetchMock).toHaveBeenCalledWith('http://127.0.0.1:16002/api/isos/v1/graph', expect.anything());
    expect(apiReads).toBe(2);
  });
});
