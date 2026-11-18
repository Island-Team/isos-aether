// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { NodeGraph, ProcessStatus } from '../../services/isos/node-graph';
import { GraphVision } from './GraphVision';
const revision = 'a'.repeat(64);
const binding = { monitor_path: 'isos/ui/vision/front', transfer_path: 'eos/interface/egress/front', source_path: 'eos/hardware/camera/front', label: 'Front', interval_ms: 200 };
const graph = { version: '0.2.1', robot: { id: 'r', name: 'R' }, cores: { isos: { layers: { ui: { groups: { vision: { nodes: { front: { id: 'front', node_type: 'isos.ui.vision.monitor@v0.2', label: 'Front', description: '' } } } } } } } } } as NodeGraph;
function packet(frame: { sample: { source_time_ns: number; clock: string; sequence: number; frame_id: string }; width_px: number; height_px: number; encoding: string; data: string }, outputSequence: number): ArrayBuffer {
  const header = new TextEncoder().encode(JSON.stringify({ op: 'frame', run_id: 'run-1', revision,
    monitor_path: binding.monitor_path, transfer_path: binding.transfer_path, source_path: binding.source_path,
    output_sequence: outputSequence, sample: frame.sample, width_px: frame.width_px, height_px: frame.height_px, encoding: frame.encoding }));
  const jpeg = Uint8Array.from(atob(frame.data), (char) => char.charCodeAt(0));
  const bytes = new Uint8Array(4 + header.length + jpeg.length);
  new DataView(bytes.buffer).setUint32(0, header.length, false);
  bytes.set(header, 4); bytes.set(jpeg, 4 + header.length);
  return bytes.buffer;
}
const status: ProcessStatus = { schema_version: '1.0.0', state: 'running', run_id_state: 'known', run_id: 'run-1', active_revision: revision, saved_revision: revision, processes: [] };
class Socket { static all: Socket[] = []; onopen: (() => void) | null = null; onmessage: ((event: { data: ArrayBuffer }) => void) | null = null; onclose: (() => void) | null = null; onerror: (() => void) | null = null; sent: string[] = []; closed = false; constructor(public url: string) { Socket.all.push(this); } send(value: string) { this.sent.push(value); } close() { this.closed = true; } }
let container: HTMLDivElement; let root: ReturnType<typeof createRoot>;
beforeEach(() => { vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true); vi.stubGlobal('WebSocket', Socket); vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, json: async () => ({ schema_version: '0.1.0', ok: true, data: { run_id: 'run-1', revision, bindings: [binding] }, error: null }) }))); vi.stubGlobal('URL', Object.assign(URL, { createObjectURL: vi.fn(() => 'blob:front'), revokeObjectURL: vi.fn() })); Socket.all = []; container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container); });
afterEach(async () => { await act(async () => root.unmount()); container.remove(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });
it('subscribes by monitor and clears matching frames on Stop', async () => {
  let now = 1000; vi.spyOn(performance, 'now').mockImplementation(() => now);
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={status} visible />));
  const socket = Socket.all[0]; expect(socket.url).toContain('/api/isos/v1/vision/ws');
  await act(async () => socket.onopen?.());
  expect(JSON.parse(socket.sent[0])).toEqual({ monitor_path: binding.monitor_path, run_id: 'run-1', revision });
  expect(container.textContent).toContain(`Transfer node${binding.transfer_path}`);
  expect(container.textContent).toContain('Transfer interval200 ms');
  const frame = { sample: { source_time_ns: 1, clock: 'simulation', sequence: 0, frame_id: 'front-1' }, width_px: 640, height_px: 480, encoding: 'jpeg', data: btoa(String.fromCharCode(0xff, 0xd8, 0xff, 0xd9)) };
  await act(async () => socket.onmessage?.({ data: packet(frame, 0) }));
  expect(container.querySelector('img')?.getAttribute('src')).toBe('blob:front');
  expect(container.textContent).toContain('Received frame640 × 480 px');
  expect(container.textContent).toContain(binding.source_path);
  expect(container.textContent).toContain('FormatJPEG');
  expect(container.textContent).toContain('Delivered FPS0.0 Hz');
  expect(container.textContent).toContain('Frame IDfront-1');
  expect(container.textContent).toContain('Capture output sequence0');
  expect(container.textContent).toContain('Source sample sequence0');
  expect(container.textContent).toContain('Skipped capture outputs0');
  await act(async () => socket.onmessage?.({ data: packet(frame, 0) }));
  expect(container.querySelector('img')?.getAttribute('src')).toBe('blob:front');
  expect(container.textContent).toContain('Receiving frames');
  expect(URL.revokeObjectURL).not.toHaveBeenCalled();
  now = 1100;
  await act(async () => socket.onmessage?.({ data: packet({ ...frame, sample: { ...frame.sample, sequence: 100 } }, 2) }));
  expect(container.textContent).toContain('Delivered FPS10.0 Hz');
  expect(container.textContent).toContain('Capture output sequence2');
  expect(container.textContent).toContain('Source sample sequence100');
  expect(container.textContent).toContain('Skipped capture outputs1');
  await act(async () => socket.onmessage?.({ data: packet(frame, 0) }));
  expect(container.querySelector('img')?.getAttribute('src')).toBe('blob:front');
  expect(container.textContent).toContain('Receiving frames');
  expect(container.textContent).toContain('Delivered FPS10.0 Hz');
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={{ ...status, state: 'stopped', run_id: null, run_id_state: 'none', active_revision: null }} visible />));
  expect(socket.closed).toBe(true); expect(container.querySelector('img')).toBeNull(); expect(container.textContent).not.toContain('Received frame');
});

it('shows a larger received frame in Chinese and clears details on disconnect', async () => {
  await act(async () => root.render(<GraphVision lang="cn" graph={graph} revision={revision} status={status} visible />));
  const socket = Socket.all[0];
  const frame = { sample: { source_time_ns: 1, clock: 'simulation', sequence: 0, frame_id: 'agentview-right' }, width_px: 1280, height_px: 720, encoding: 'jpeg', data: btoa(String.fromCharCode(0xff, 0xd8, 0xff, 0xd9)) };
  await act(async () => socket.onmessage?.({ data: packet(frame, 0) }));
  expect(container.textContent).toContain('收到的画面1280 × 720 px');
  expect(container.textContent).toContain('相机来源');
  await act(async () => socket.onclose?.());
  expect(container.querySelector('img')).toBeNull(); expect(container.textContent).not.toContain('收到的画面');
});

it('reconnects a closed stream for the same fenced run without reloading the page', async () => {
  vi.useFakeTimers();
  try {
    await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={status} visible />));
    const first = Socket.all[0];
    await act(async () => first.onclose?.());
    expect(container.textContent).toContain('Disconnected');
    await act(async () => { await vi.advanceTimersByTimeAsync(500); });
    expect(Socket.all).toHaveLength(2);
    await act(async () => Socket.all[1].onopen?.());
    expect(JSON.parse(Socket.all[1].sent[0])).toEqual({ monitor_path: binding.monitor_path, run_id: 'run-1', revision });
  } finally { vi.useRealTimers(); }
});

it('clears received details when hidden or the running revision becomes stale', async () => {
  const frame = { sample: { source_time_ns: 1, clock: 'simulation', sequence: 0, frame_id: 'front-1' }, width_px: 640, height_px: 480, encoding: 'jpeg', data: btoa(String.fromCharCode(0xff, 0xd8, 0xff, 0xd9)) };
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={status} visible />));
  const first = Socket.all[0];
  await act(async () => first.onmessage?.({ data: packet(frame, 0) }));
  expect(container.textContent).toContain('Received frame');
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={status} visible={false} />));
  expect(first.closed).toBe(true); expect(container.textContent).not.toContain('Received frame');
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={status} visible />));
  const second = Socket.all[1];
  await act(async () => second.onmessage?.({ data: packet(frame, 0) }));
  expect(container.textContent).toContain('Received frame');
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={{ ...status, active_revision: 'b'.repeat(64) }} visible />));
  expect(second.closed).toBe(true); expect(container.textContent).not.toContain('Received frame');
});

it('rejects a malformed repeated frame before retaining the image', async () => {
  await act(async () => root.render(<GraphVision lang="en" graph={graph} revision={revision} status={status} visible />));
  const socket = Socket.all[0];
  const frame = { sample: { source_time_ns: 1, clock: 'simulation', sequence: 0, frame_id: 'front-1' }, width_px: 1, height_px: 1, encoding: 'jpeg', data: btoa(String.fromCharCode(0xff, 0xd8, 0xff, 0xd9)) };
  await act(async () => socket.onmessage?.({ data: packet(frame, 0) }));
  await act(async () => socket.onmessage?.({ data: packet({ ...frame, data: btoa('nope') }, 0) }));
  expect(container.querySelector('img')).toBeNull();
  expect(container.textContent).toContain('jpeg_invalid');
  expect(container.textContent).not.toContain('Received frame');
});
