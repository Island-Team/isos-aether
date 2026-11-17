// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { decodeRuntimeStatusSnapshot, runtimeStatusSocketUrl, subscribeRuntimeStatus } from './runtime-status-stream';

const status = { schema_version: '0.1.0', processes: [], state: 'stopped' };
const health = { service: 'isos', state: 'ready', owners: [] };

class MockSocket {
  static all: MockSocket[] = [];
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  constructor(readonly url: string) { MockSocket.all.push(this); }
  close(): void { this.onclose?.(); }
  send(value: unknown): void { this.onmessage?.({ data: JSON.stringify(value) }); }
}

afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); MockSocket.all = []; });

describe('runtime status stream', () => {
  it('uses the current API origin for the socket', () => {
    expect(runtimeStatusSocketUrl('http://127.0.0.1:16002')).toBe('ws://127.0.0.1:16002/api/isos/v1/runtime/status/ws');
  });

  it('accepts complete snapshots and rejects malformed or partial events', () => {
    const snapshot = { type: 'snapshot', sequence: 1, process_status: status, system_health: health };
    expect(decodeRuntimeStatusSnapshot(snapshot)).toEqual({ process_status: status, system_health: health });
    expect(decodeRuntimeStatusSnapshot({ ...snapshot, type: 'error' })).toBeNull();
    expect(decodeRuntimeStatusSnapshot({ ...snapshot, system_health: null })).toBeNull();
    expect(decodeRuntimeStatusSnapshot({ ...snapshot, process_status: { ...status, processes: null } })).toBeNull();
    expect(decodeRuntimeStatusSnapshot({ ...snapshot, process_status: { ...status, schema_version: '1.0.0' } })).toBeNull();
  });

  it('shares one socket and falls back on disconnect before reconnecting', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('WebSocket', MockSocket);
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, json: async () => ({ api_origin: 'http://127.0.0.1:16002' }) })));
    const first = vi.fn();
    const second = vi.fn();
    const stopFirst = subscribeRuntimeStatus(first);
    const stopSecond = subscribeRuntimeStatus(second);
    try {
      await vi.waitFor(() => expect(MockSocket.all).toHaveLength(1));
      expect(MockSocket.all[0].url).toContain('/api/isos/v1/runtime/status/ws');
      const snapshot = { type: 'snapshot', sequence: 1, process_status: status, system_health: health };
      MockSocket.all[0].send(snapshot);
      expect(first).toHaveBeenLastCalledWith({ process_status: status, system_health: health });
      expect(second).toHaveBeenLastCalledWith({ process_status: status, system_health: health });
      MockSocket.all[0].send({ ...snapshot, sequence: 1, process_status: { ...status, state: 'running' } });
      expect(first).toHaveBeenCalledTimes(2); // initial null and first snapshot
      MockSocket.all[0].close();
      expect(first).toHaveBeenLastCalledWith(null);
      await vi.advanceTimersByTimeAsync(500);
      expect(MockSocket.all).toHaveLength(2);
      MockSocket.all[1].send({ ...snapshot, sequence: 1, process_status: { ...status, state: 'running' } });
      expect(second).toHaveBeenLastCalledWith({ process_status: { ...status, state: 'running' }, system_health: health });
    } finally { stopFirst(); stopSecond(); }
  });

  it('closes the socket while hidden and reconnects when visible', async () => {
    vi.stubGlobal('WebSocket', MockSocket);
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, json: async () => ({ api_origin: 'http://127.0.0.1:16002' }) })));
    let visible = true;
    vi.spyOn(document, 'visibilityState', 'get').mockImplementation(() => visible ? 'visible' : 'hidden');
    const stop = subscribeRuntimeStatus(vi.fn());
    try {
      await vi.waitFor(() => expect(MockSocket.all).toHaveLength(1));
      visible = false;
      document.dispatchEvent(new Event('visibilitychange'));
      expect(MockSocket.all[0].onclose).toBeTruthy();
      visible = true;
      document.dispatchEvent(new Event('visibilitychange'));
      await vi.waitFor(() => expect(MockSocket.all).toHaveLength(2));
    } finally { stop(); vi.restoreAllMocks(); }
  });
});
