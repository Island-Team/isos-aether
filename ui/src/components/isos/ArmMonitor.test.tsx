// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { ArmMonitorView } from './ArmMonitor';

it('shows six joints for the matching run and clears them on Stop or run change', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const revision = 'a'.repeat(64);
  const reading: Parameters<typeof ArmMonitorView>[0]['reading'] = { snapshot: { state: 'running', data_type: 'arm_monitor_state@v0.2', run_id: 'run-1', active_revision: revision, observation: { run_id: 'run-1', revision, source_node: 'eos/hardware/mock/arm', sequence: 1, sampled_at_monotonic_ns: 100, joint_names: ['j1', 'j2', 'j3', 'j4', 'j5', 'j6'], joint_positions_rad: [0, 1, 2, 3, 4, 5], state: 'ready', reason_code: null } }, receivedAt: Date.now(), error: '' };
  await act(async () => root.render(<ArmMonitorView lang="en" reading={reading} expectedRunId="run-1" expectedRevision={revision} />));
  expect(container.querySelectorAll('output')).toHaveLength(6);
  await act(async () => root.render(<ArmMonitorView lang="en" reading={reading} expectedRunId={null} expectedRevision={null} />));
  expect(container.querySelectorAll('output')).toHaveLength(0);
  expect(container.textContent).toContain('Stopped');
  await act(async () => root.render(<ArmMonitorView lang="en" reading={reading} expectedRunId="run-2" expectedRevision={revision} />));
  expect(container.querySelectorAll('output')).toHaveLength(0);
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('explains a stale active run without calling it another editor change', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const reading = { snapshot: null, receivedAt: null, error: 'The active run uses an older graph.', errorCode: 'graph_fence_stale' };
  await act(async () => root.render(<ArmMonitorView lang="en" reading={reading} />));
  expect(container.textContent).toContain('The active run uses an older graph.');
  expect(container.textContent).not.toContain('another editor');
  await act(async () => root.render(<ArmMonitorView lang="cn" reading={reading} />));
  expect(container.textContent).toContain('当前运行使用旧版节点图');
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});
