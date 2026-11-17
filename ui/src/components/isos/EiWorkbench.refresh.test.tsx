// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';

const statusRequest = vi.hoisted(() => vi.fn());
const publish = vi.hoisted(() => vi.fn());
vi.mock('@/services/isos/node-graph', () => ({ getProcessStatus: statusRequest }));
vi.mock('@/services/isos/runtime-status-stream', () => ({ subscribeRuntimeStatus: () => () => undefined }));
vi.mock('@/services/isos/ui-messages', () => ({ publishUiMessage: publish }));

import { EiWorkbench } from './EiWorkbench';

afterEach(() => { vi.clearAllMocks(); document.body.innerHTML = ''; });

it('keeps one EI status request pending and logs a repeated failure once until recovery', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  let rejectStatus: ((reason: Error) => void) | undefined;
  statusRequest.mockImplementationOnce(() => new Promise((_, reject) => { rejectStatus = reject; }));
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<EiWorkbench lang="en" onOpenGraph={() => undefined} />));
  const refresh = Array.from(container.querySelectorAll('button')).find((button) => button.textContent === 'Refresh');
  expect(refresh).toBeDefined();
  await act(async () => { refresh?.click(); refresh?.click(); });
  expect(statusRequest).toHaveBeenCalledTimes(1);
  await act(async () => { rejectStatus?.(new Error('offline')); });
  expect(publish).toHaveBeenCalledTimes(1);
  statusRequest.mockRejectedValueOnce(new Error('offline'));
  await act(async () => { refresh?.click(); });
  expect(publish).toHaveBeenCalledTimes(1);
  statusRequest.mockResolvedValueOnce({ state: 'running', processes: [] });
  await act(async () => { refresh?.click(); });
  statusRequest.mockRejectedValueOnce(new Error('offline'));
  await act(async () => { refresh?.click(); });
  expect(publish).toHaveBeenCalledTimes(2);
  await act(async () => root.unmount());
});
