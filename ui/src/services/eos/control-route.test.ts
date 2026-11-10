import { afterEach, describe, expect, it, vi } from 'vitest';
import { getVisionGates, visionSocketUrl } from './vision';

afterEach(() => { vi.unstubAllGlobals(); });

describe('browser owner routes', () => {
  it('attaches Vision only to IsOS', async () => {
    const fetchMock = vi.fn(async (url: string) => ({
      ok: true, status: 200,
      json: async () => ({ schema_version: '0.1.0', ok: true, data: url.endsWith('/gates') ? { revision: 'rev', gates: [] } : { ok: true, nodes: [] }, error: null }),
    }));
    vi.stubGlobal('fetch', fetchMock);
    await getVisionGates();
    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      'http://127.0.0.1:16000/api/isos/v1/owners/eos/vision/gates',
    ]);
    expect(visionSocketUrl()).toBe('ws://127.0.0.1:16000/api/isos/v1/owners/eos/vision/ws');
  });
});
