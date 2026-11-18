import { expect, it } from 'vitest';
import type { NodeGraph, NodeTypeCatalog } from '@/services/isos/node-graph';
import { catalogSelectionBlocks, displayNodeOptions, draftAdapterOptions } from './catalog-options';
import { graphNodes } from './node-graph-edits';

it('offers the exact verified package adapter while its new hardware node is incomplete', () => {
  const pin = { id: 'island/mujoco-sim', version: '0.2.2' };
  const graph = { version: '0.2.1', robot: { id: 'test', name: 'Test' }, cores: {
    isos: { layers: {} }, ei: { layers: {} }, eos: { layers: { hardware: { groups: { camera: { nodes: {
      front: { id: 'front', node_type: 'eos.hardware.camera.device@v0.2', label: 'front', description: '', package: pin },
    } } } } } },
  } } as NodeGraph;
  const catalog = {
    types: [{ id: 'eos.hardware.camera.device@v0.2', core: 'eos', layer: 'hardware', group: 'camera', kind: 'camera', lifecycle: 'adapter', package: pin, capabilities: [], settings: [], inputs: [], outputs: [], description: 'Camera' }],
    adapter_families: [
      { id: 'mujoco-v2', device_kinds: ['camera'], package: pin, compiled: false, loadable: true, transport: 'unix_socket', description: 'MuJoCo', available: false },
      { id: 'old-v2', device_kinds: ['camera'], package: { ...pin, version: '0.2.1' }, compiled: false, loadable: true, transport: 'unix_socket', description: 'Old', available: false },
    ],
  } as NodeTypeCatalog;
  const groups = draftAdapterOptions(graph, catalog).setting_options!;
  expect(groups).toHaveLength(1);
  expect(groups[0].node_path).toBe('eos/hardware/camera/front');
  expect(groups[0].options).toEqual([{ value: 'mujoco-v2', label: 'mujoco-v2', description: 'MuJoCo', available: true, reason_code: null, reason: null }]);
  expect(catalogSelectionBlocks(graph.version, graphNodes(graph, catalog.types), { ...draftAdapterOptions(graph, catalog), setting_options: groups })).toBe(true);
});


it('keeps old owner rows disabled by the caller while replacing adapter families with current catalog choices', () => {
  const old = { adapter_families: [], primitive_bindings: [], targets: [], setting_options: [
    { node_path: 'eos/hardware/camera/front', setting: 'adapter.type', selected: 'old', options: [{ value: 'old', label: 'Old', description: '', available: true, reason_code: null, reason: null }] },
    { node_path: 'eos/hardware/camera/front', setting: 'adapter.settings.device_id', selected: 'camera-a', options: [{ value: 'camera-a', label: 'Camera A', description: '', available: true, reason_code: null, reason: null }] },
  ] };
  const preview = { ...old, setting_options: [{ node_path: 'eos/hardware/camera/front', setting: 'adapter.type', selected: 'new', options: [{ value: 'new', label: 'New', description: '', available: true, reason_code: null, reason: null }] }] };
  const shown = displayNodeOptions(null, old, preview)!;
  expect(shown.setting_options?.map((group) => group.setting)).toEqual(['adapter.settings.device_id', 'adapter.type']);
  expect(shown.setting_options?.[1].options[0].value).toBe('new');
  expect(displayNodeOptions(old, null, preview)).toBe(old);
});
