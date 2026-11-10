import { expect, it } from 'vitest';
import { decodeArmMonitor, monitorRefreshHz } from './arm-monitor';
import type { NodeGraph } from './node-graph';

const revision = 'a'.repeat(64);
const running = { state: 'running', data_type: 'arm_monitor_state@v0.2', run_id: 'run-1', active_revision: revision, observation: { run_id: 'run-1', revision, source_node: 'eos/hardware/mock/arm', sequence: 2, sampled_at_monotonic_ns: 1000, joint_names: ['j1', 'j2', 'j3', 'j4', 'j5', 'j6'], joint_positions_rad: [0, 1, 2, 3, 4, 5], state: 'ready', reason_code: null } };

it('accepts fenced six-joint observations and stopped clearing', () => {
  expect(decodeArmMonitor(running).observation?.joint_positions_rad).toEqual([0, 1, 2, 3, 4, 5]);
  expect(decodeArmMonitor({ ...running, observation: { ...running.observation, source_node: 'eos/hardware/mock/arm_observer' } }).observation?.source_node).toBe('eos/hardware/mock/arm_observer');
  expect(decodeArmMonitor({ state: 'stopped', data_type: 'arm_monitor_state@v0.2', run_id: null, active_revision: null, observation: null }).observation).toBeNull();
  expect(decodeArmMonitor({ ...running, observation: { ...running.observation, sampled_at_monotonic_ns: 10_000_000_000_000_000 } }).observation?.state).toBe('ready');
});

it('rejects cross-run, wrong-source, and invalid joint data', () => {
  expect(() => decodeArmMonitor({ ...running, run_id: 'run-2' })).toThrow();
  expect(() => decodeArmMonitor({ ...running, observation: { ...running.observation, source_node: 'other' } })).toThrow();
  expect(() => decodeArmMonitor({ ...running, observation: { ...running.observation, joint_positions_rad: [0, 1] } })).toThrow();
  expect(() => decodeArmMonitor({ ...running, observation: { ...running.observation, joint_positions_rad: [0, 1, 2, 3, 4, NaN] } })).toThrow();
  expect(() => decodeArmMonitor({ ...running, observation: { ...running.observation, sampled_at_monotonic_ns: -1 } })).toThrow();
  expect(() => decodeArmMonitor({ ...running, observation: { ...running.observation, sampled_at_monotonic_ns: 1.5 } })).toThrow();
});

it('reads the saved UI monitor refresh setting with a 5 Hz default', () => {
  const graph = { cores: { isos: { layers: { ui: { groups: { monitor: { nodes: { arm: { node_type: 'isos.ui.monitor.arm@v0.2', settings: { refresh_rate_hz: 2 } } } } } } } } } } as unknown as NodeGraph;
  expect(monitorRefreshHz(graph)).toBe(2);
  expect(monitorRefreshHz(null)).toBe(5);
  graph.cores.isos.layers.ui.groups.monitor.nodes.arm.settings!.refresh_rate_hz = 9;
  expect(monitorRefreshHz(graph)).toBe(5);
});
