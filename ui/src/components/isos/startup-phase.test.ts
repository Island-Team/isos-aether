import { expect, it } from 'vitest';
import type { ProcessStatus } from '../../services/isos/node-graph';
import { startupPhaseText } from './startup-phase';

const status = (state: ProcessStatus['state'], startup_phase: string | null): ProcessStatus => ({ schema_version: '1.0.0', state, startup_phase, processes: [] });

it('explains current startup phases in both languages and keeps unknown phases visible', () => {
  expect(startupPhaseText(status('starting', 'building_source'), 'en')).toBe('Building linked source package');
  expect(startupPhaseText(status('starting', 'validating_graph'), 'cn')).toBe('正在验证节点图');
  expect(startupPhaseText(status('starting', 'starting_owners'), 'en')).toBe('Starting EOS and EI');
  expect(startupPhaseText(status('starting', 'waiting_package'), 'en')).toBe('Startup phase: waiting package');
  expect(startupPhaseText(status('stopping', 'building_source'), 'cn')).toBe('正在停止启动…');
  expect(startupPhaseText(status('running', null), 'en')).toBeNull();
  expect(startupPhaseText(status('stopped', null), 'en', true)).toContain('Starting deployment');
});
