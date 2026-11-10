import { afterEach, expect, it, vi } from 'vitest';
import { getButtonActivationResult, sendButtonActivation } from './node-events';

afterEach(() => { vi.unstubAllGlobals(); });

it('sends only the run-fenced activation envelope through IsOS', async () => {
  vi.stubGlobal('ISOSUI_API_ORIGIN', 'http://127.0.0.1:16000');
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { event_id: 'click-1', command_id: 'arm-1', status: 'accepted' }, error: null }) });
  vi.stubGlobal('fetch', fetchMock);
  const input = { schema_version: '0.2.0', source_node: 'isos/ui/button/trigger', output: 'activation', data_type: 'button_activation@v0.2', run_id: 'run-1', active_revision: 'a'.repeat(64), payload: { event_id: 'click-1', kind: 'activate' } } as const;
  expect((await sendButtonActivation(input)).status).toBe('accepted');
  const [url, options] = fetchMock.mock.calls[0];
  expect(url).toBe('http://127.0.0.1:16000/api/isos/v1/nodes/events');
  expect(options.method).toBe('POST');
  expect(JSON.parse(options.body)).toEqual(input);
});

it('rejects an uncorrelated or malformed terminal reply', async () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { event_id: 'other', command_id: 'arm-1', status: 'succeeded', joint_positions_rad: [0, 0, 0, 0, 0, 0] }, error: null }) });
  vi.stubGlobal('fetch', fetchMock);
  const input = { schema_version: '0.2.0', source_node: 'isos/ui/button/trigger', output: 'activation', data_type: 'button_activation@v0.2', run_id: 'run-1', active_revision: 'a'.repeat(64), payload: { event_id: 'click-1', kind: 'activate' } } as const;
  await expect(sendButtonActivation(input)).rejects.toThrow('uncorrelated');
  fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { event_id: 'click-1', command_id: 'arm-1', status: 'succeeded', joint_positions_rad: [0, 0] }, error: null }) });
  await expect(sendButtonActivation(input)).rejects.toThrow('uncorrelated');
});

it('queries a correlated terminal result without resubmitting movement', async () => {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ schema_version: '0.1.0', ok: true, data: { event_id: 'click-1', command_id: 'arm-1', status: 'succeeded', joint_positions_rad: [0, 0, 0, 0, 0, 0] }, error: null }) });
  vi.stubGlobal('fetch', fetchMock);
  const query = { schema_version: '0.2.0', source_node: 'isos/ui/button/trigger', run_id: 'run-1', active_revision: 'a'.repeat(64), event_id: 'click-1' } as const;
  expect((await getButtonActivationResult(query)).status).toBe('succeeded');
  const [url, options] = fetchMock.mock.calls[0];
  expect(url).toContain('/api/isos/v1/nodes/events/result');
  expect(options.method).toBe('POST');
  expect(JSON.parse(options.body)).toEqual(query);
});
