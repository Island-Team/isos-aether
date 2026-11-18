// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';

const statusRequest = vi.hoisted(() => vi.fn());
vi.mock('@/services/isos/node-graph', () => ({ getProcessStatus: statusRequest }));
vi.mock('@/services/isos/runtime-status-stream', () => ({ subscribeRuntimeStatus: () => () => undefined }));
vi.mock('@/services/isos/ui-messages', () => ({ publishUiMessage: vi.fn() }));

import { EosWorkbench } from './EosWorkbench';
import { defaultWorkspaceLayout } from './workspace-layout';

afterEach(() => { vi.clearAllMocks(); document.body.innerHTML = ''; });

it('shows EOS process health without a control command', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  statusRequest.mockResolvedValue({ schema_version: '1.0.0', state: 'running', active_revision: 'revision-a', processes: [{ id: 'eos', pid: 42, restarts: 0, state: 'ready' }, { id: 'ei', pid: 43, restarts: 0, state: 'ready' }] });
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<EosWorkbench lang="en" graphDirty={false} onOpenGraph={() => undefined} />));
  expect(container.textContent).toContain('EOS process health');
  expect(container.textContent).toContain('eos · ready · PID 42');
  expect(container.textContent).not.toContain('ei · ready');
  expect(container.querySelectorAll('button')).toHaveLength(2);
  await act(async () => root.unmount());
});

it('shows an empty health state when EOS has not started', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  statusRequest.mockResolvedValue({ schema_version: '1.0.0', state: 'stopped', processes: [] });
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<EosWorkbench lang="cn" graphDirty={false} onOpenGraph={() => undefined} />));
  expect(container.textContent).toContain('EOS 进程尚未启动。');
  await act(async () => root.unmount());
});

it('shows EOS health only on its page while keeping the monitor panel available', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  statusRequest.mockResolvedValue({ schema_version: '1.0.0', state: 'running', processes: [{ id: 'eos', pid: 42, restarts: 0, state: 'ready' }] });
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  const props = { lang: 'en' as const, graphDirty: false, onOpenGraph: () => undefined, layout: defaultWorkspaceLayout(), viewport: { width: 1200, height: 800 }, onPanelChange: () => undefined };
  await act(async () => root.render(<EosWorkbench {...props} pageVisible={false} />));
  expect(container.textContent).not.toContain('EOS process health');
  expect(container.querySelector('[aria-label="EOS process health"]')).toBeNull();
  await act(async () => root.render(<EosWorkbench {...props} pageVisible />));
  expect(container.textContent).toContain('EOS process health');
  await act(async () => root.unmount());
});

it('keeps one process status request pending across manual refreshes', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  let resolveStatus: ((value: unknown) => void) | undefined;
  statusRequest.mockImplementationOnce(() => new Promise((resolve) => { resolveStatus = resolve; }));
  statusRequest.mockResolvedValue({ state: 'running', processes: [] });
  const container = document.createElement('div'); document.body.appendChild(container);
  const root = createRoot(container);
  await act(async () => root.render(<EosWorkbench lang="en" graphDirty={false} onOpenGraph={() => undefined} />));
  const refresh = Array.from(container.querySelectorAll('button')).find((button) => button.textContent === 'Refresh');
  expect(refresh).toBeDefined();
  await act(async () => { refresh?.click(); refresh?.click(); });
  expect(statusRequest).toHaveBeenCalledTimes(1);
  await act(async () => { resolveStatus?.({ state: 'running', processes: [] }); });
  await act(async () => { refresh?.click(); });
  expect(statusRequest).toHaveBeenCalledTimes(2);
  await act(async () => root.unmount());
});
