/** Browser adapter for the shared IsOS client. */
import { createIsosClient, isLoopbackOrigin } from './transport';
export { IsosApiError } from './transport';

const builtOrigin = typeof ISOSUI_API_ORIGIN === 'undefined' ? 'http://127.0.0.1:16000' : ISOSUI_API_ORIGIN;
let localOrigin: Promise<string> | undefined;

async function origin(): Promise<string> {
  if (typeof window === 'undefined' || window.location.hostname !== '127.0.0.1') return builtOrigin;
  localOrigin ??= fetch('/isosui-runtime.json', { cache: 'no-store', credentials: 'omit' })
    .then(async (reply) => {
      if (!reply.ok) throw new Error('IsOS UI runtime configuration is unavailable');
      const value: unknown = await reply.json();
      if (!value || typeof value !== 'object' || !('api_origin' in value) ||
          typeof value.api_origin !== 'string' || !isLoopbackOrigin(value.api_origin)) {
        throw new Error('IsOS UI runtime configuration has an invalid API origin');
      }
      return value.api_origin;
    });
  try { return await localOrigin; }
  catch {
    // A transient failure while IsOS restarts must not pin this tab to a stale port.
    localOrigin = undefined;
    return builtOrigin;
  }
}

/** Resolve the current local API address, including a changed port after IsOS restarts. */
export async function resolveIsosApiOrigin(refresh = false): Promise<string> {
  if (refresh) localOrigin = undefined;
  return origin();
}

export async function request<T>(path: string, method = 'GET', body?: unknown, signal?: AbortSignal): Promise<T> {
  try { return await createIsosClient(await origin())<T>(path, method, body, signal); }
  catch (error) {
    // Re-read the runtime address on the next request. Never replay a mutation.
    if (!(error instanceof Error && error.name === 'IsosApiError')) localOrigin = undefined;
    throw error;
  }
}
