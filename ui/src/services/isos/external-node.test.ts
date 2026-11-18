import { afterEach, expect, it, vi } from 'vitest';
import { callExternalNode } from './external-node';

afterEach(() => { vi.unstubAllGlobals(); });

it('sends the exact run-fenced external node request', async () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { node_path: 'isos/ui/community/panel', port: 'state', run_id: 'r1', active_revision: 'a'.repeat(64), payload: { count: 7 } }, error: null }) });
  vi.stubGlobal('fetch', fetchMock);
  const input = { schema_version: 1, node_path: 'isos/ui/community/panel', run_id: 'r1', active_revision: 'a'.repeat(64), op: 'read', port: 'state', payload: null } as const;
  expect((await callExternalNode(input)).payload).toEqual({ count: 7 });
  expect(fetchMock.mock.calls[0][0]).toContain('/api/isos/v1/nodes/external');
  expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual(input);
});
