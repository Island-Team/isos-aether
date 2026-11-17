import type { ProcessStatus, SystemHealth } from './node-graph';
import { resolveIsosApiOrigin } from './api-client';

export interface RuntimeStatusSnapshot { process_status: ProcessStatus; system_health: SystemHealth }
export type RuntimeStatusListener = (snapshot: RuntimeStatusSnapshot | null) => void;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function runtimeStatusSocketUrl(origin: string): string {
  const url = new URL(`${origin}/api/isos/v1/runtime/status/ws`);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.toString();
}

export function decodeRuntimeStatusSnapshot(raw: unknown): RuntimeStatusSnapshot | null {
  if (!isRecord(raw) || raw.type !== 'snapshot' || !Number.isSafeInteger(raw.sequence) || !isRecord(raw.process_status) || !isRecord(raw.system_health)) return null;
  const process = raw.process_status;
  const health = raw.system_health;
  if (process.schema_version !== '0.1.0' || !Array.isArray(process.processes) || health.service !== 'isos' || health.state !== 'ready' || !Array.isArray(health.owners)) return null;
  return { process_status: process as unknown as ProcessStatus, system_health: health as unknown as SystemHealth };
}

const listeners = new Set<RuntimeStatusListener>();
let socket: WebSocket | null = null;
let connecting = false;
let retry: number | null = null;
let retryDelay = 500;
let current: RuntimeStatusSnapshot | null = null;
let lastSequence = -1;

function publish(snapshot: RuntimeStatusSnapshot | null): void {
  current = snapshot;
  for (const listener of listeners) listener(snapshot);
}

async function connect(): Promise<void> {
  if (!listeners.size || document.visibilityState !== 'visible' || socket || connecting || typeof WebSocket === 'undefined') return;
  connecting = true;
  try {
    const apiOrigin = await resolveIsosApiOrigin(true);
    if (!listeners.size || socket) return;
    const next = new WebSocket(runtimeStatusSocketUrl(apiOrigin));
    socket = next;
    next.onmessage = (event) => {
      let raw: unknown;
      try { raw = JSON.parse(String(event.data)); } catch { next.close(); return; }
      if (isRecord(raw) && raw.type === 'error') { next.close(); return; }
      const snapshot = decodeRuntimeStatusSnapshot(raw);
      if (!snapshot || !isRecord(raw)) { next.close(); return; }
      if ((raw.sequence as number) <= lastSequence) return;
      lastSequence = raw.sequence as number;
      retryDelay = 500;
      publish(snapshot);
    };
    next.onclose = () => {
      if (socket !== next) return;
      socket = null;
      lastSequence = -1;
      publish(null);
      if (listeners.size && document.visibilityState === 'visible') {
        retry = window.setTimeout(() => { retry = null; connect(); }, retryDelay);
        retryDelay = Math.min(retryDelay * 2, 10_000);
      }
    };
    next.onerror = () => next.close();
  } catch {
    if (listeners.size && document.visibilityState === 'visible') {
      retry = window.setTimeout(() => { retry = null; void connect(); }, retryDelay);
      retryDelay = Math.min(retryDelay * 2, 10_000);
    }
  } finally { connecting = false; }
}

function onVisibilityChange(): void {
  if (document.visibilityState === 'visible') { void connect(); return; }
  socket?.close();
}

/** One shared status socket for visible IsOS views; null means use HTTP fallback. */
export function subscribeRuntimeStatus(listener: RuntimeStatusListener): () => void {
  if (!listeners.size) document.addEventListener('visibilitychange', onVisibilityChange);
  listeners.add(listener);
  listener(current);
  connect();
  return () => {
    listeners.delete(listener);
    if (listeners.size) return;
    document.removeEventListener('visibilitychange', onVisibilityChange);
    if (retry !== null) { window.clearTimeout(retry); retry = null; }
    socket?.close();
    socket = null;
    lastSequence = -1;
    current = null;
  };
}
