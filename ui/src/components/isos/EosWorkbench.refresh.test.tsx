// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';

const statusRequest = vi.hoisted(() => vi.fn());
vi.mock('@/services/isos/node-graph', () => ({ getProcessStatus: statusRequest }));
vi.mock('@/services/isos/ui-messages', () => ({ publishUiMessage: vi.fn() }));

import { EosWorkbench } from './EosWorkbench';

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
