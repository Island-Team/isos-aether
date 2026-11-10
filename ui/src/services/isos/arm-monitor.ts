import { request } from './api-client';
import type { NodeGraph } from './node-graph';

export interface ArmMonitorObservation {
  run_id: string;
  revision: string;
  source_node: 'eos/hardware/mock/arm' | 'eos/hardware/mock/arm_observer';
  sequence: number;
  sampled_at_monotonic_ns: number;
  joint_names: string[];
  joint_positions_rad: number[] | null;
  state: 'ready' | 'degraded' | 'stale' | 'stopped';
  reason_code: string | null;
}
export interface ArmMonitorSnapshot {
  state: 'running' | 'stopped';
  data_type: 'arm_monitor_state@v0.2';
  run_id: string | null;
  active_revision: string | null;
  observation: ArmMonitorObservation | null;
}

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const sixNames = (value: unknown): value is string[] => Array.isArray(value) && value.length === 6 && value.every((name) => typeof name === 'string' && !!name);
const sixPositions = (value: unknown): value is number[] => Array.isArray(value) && value.length === 6 && value.every((position) => typeof position === 'number' && Number.isFinite(position));
export const DEFAULT_MONITOR_REFRESH_HZ = 5;

export function monitorRefreshHz(graph: NodeGraph | null): number {
  const groups = graph?.cores?.isos?.layers?.ui?.groups;
  if (!groups) return DEFAULT_MONITOR_REFRESH_HZ;
  for (const group of Object.values(groups)) for (const node of Object.values(group.nodes)) {
    if (node.node_type !== 'isos.ui.monitor.arm@v0.2') continue;
    const value = node.settings?.refresh_rate_hz;
    return typeof value === 'number' && Number.isFinite(value) && value >= 1 && value <= 5 ? value : DEFAULT_MONITOR_REFRESH_HZ;
  }
  return DEFAULT_MONITOR_REFRESH_HZ;
}

export function decodeArmMonitor(value: unknown): ArmMonitorSnapshot {
  if (!record(value) || value.data_type !== 'arm_monitor_state@v0.2') throw new Error('Invalid arm monitor response.');
  if (value.state === 'stopped' && value.run_id === null && value.active_revision === null && value.observation === null) return value as unknown as ArmMonitorSnapshot;
  if (value.state !== 'running' || typeof value.run_id !== 'string' || !value.run_id || typeof value.active_revision !== 'string' || !/^[0-9a-f]{64}$/.test(value.active_revision) || !record(value.observation)) throw new Error('Invalid arm monitor fence.');
  const observation = value.observation;
  if (observation.run_id !== value.run_id || observation.revision !== value.active_revision || !['eos/hardware/mock/arm', 'eos/hardware/mock/arm_observer'].includes(String(observation.source_node)) || !Number.isSafeInteger(observation.sequence) || Number(observation.sequence) < 0 || typeof observation.sampled_at_monotonic_ns !== 'number' || !Number.isFinite(observation.sampled_at_monotonic_ns) || !Number.isInteger(observation.sampled_at_monotonic_ns) || observation.sampled_at_monotonic_ns < 0 || !sixNames(observation.joint_names) || !(observation.joint_positions_rad === null || sixPositions(observation.joint_positions_rad)) || !['ready', 'degraded', 'stale', 'stopped'].includes(String(observation.state)) || !(observation.reason_code === null || typeof observation.reason_code === 'string')) throw new Error('Invalid arm monitor observation.');
  return value as unknown as ArmMonitorSnapshot;
}

export async function getArmMonitor(signal?: AbortSignal): Promise<ArmMonitorSnapshot> {
  return decodeArmMonitor(await request<unknown>('/nodes/monitor', 'GET', undefined, signal));
}
