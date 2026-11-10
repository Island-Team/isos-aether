/** Shared IsOS HTTP envelope handling for browser and terminal clients. */
interface ApiFailure { code: string; message: string; details: unknown; correlation_id: string }
interface Envelope<T> { schema_version: '0.1.0'; ok: boolean; data: T | null; error: ApiFailure | null }
type LoopbackRequestInit = RequestInit & { targetAddressSpace?: 'loopback' };

export class IsosApiError extends Error {
  constructor(message: string, public readonly status: number, public readonly code?: string) {
    super(message);
    this.name = 'IsosApiError';
  }
}

function diagnosticCode(data: unknown): string | undefined {
  if (!data || typeof data !== 'object' || !('diagnostics' in data) || !Array.isArray(data.diagnostics)) return undefined;
  const first: unknown = data.diagnostics[0];
  return first && typeof first === 'object' && 'code' in first && typeof first.code === 'string' ? first.code : undefined;
}

export function isLoopbackOrigin(origin: string): boolean {
  try {
    const url = new URL(origin);
    return url.protocol === 'http:' && url.hostname === '127.0.0.1' && !!url.port && url.pathname === '/' && !url.search && !url.hash && !url.username && !url.password;
  } catch { return false; }
}

export function createIsosClient(origin: string): <T>(path: string, method?: string, body?: unknown, signal?: AbortSignal) => Promise<T> {
  if (!isLoopbackOrigin(origin)) throw new Error('IsOS API origin must be http://127.0.0.1:<port>.');
  return async <T>(path: string, method = 'GET', body?: unknown, signal?: AbortSignal): Promise<T> => {
    if (!path.startsWith('/') || path.startsWith('//')) throw new Error('IsOS API path must be absolute within /api/isos/v1.');
    let response: Response;
    try {
      const options: LoopbackRequestInit = {
        method, credentials: 'omit', cache: 'no-store', signal,
        headers: body === undefined ? {} : { 'Content-Type': 'application/json' },
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
        mode: 'cors', targetAddressSpace: 'loopback',
      };
      response = await fetch(`${origin}/api/isos/v1${path}`, options);
    } catch {
      throw new Error(`Cannot reach IsOS at ${origin}. Run isos serve and check the service address and port.`);
    }
    let envelope: Envelope<T> | null = null;
    try { envelope = await response.json() as Envelope<T>; } catch { /* HTTP failures can be plain text. */ }
    if (response.ok && !envelope) throw new Error('The IsOS API address returned a page instead of API data. Check the API port, then run isos serve.');
    if (envelope && envelope.schema_version !== '0.1.0') throw new Error('IsOS returned an unsupported API schema version.');
    if (!response.ok || !envelope?.ok || envelope.data === null) {
      const code = diagnosticCode(envelope?.error?.details) || envelope?.error?.code;
      if (response.status === 409 && code === 'graph_fence_stale') throw new IsosApiError('The active run uses an older graph. Stop or recover it, then run the saved graph.', 409, code);
      if (response.status === 409) throw new IsosApiError('This graph changed in another editor. Refresh before applying.', 409, code);
      throw new IsosApiError(envelope?.error?.message || `IsOS request failed (${response.status}).`, response.status, code);
    }
    return envelope.data;
  };
}
