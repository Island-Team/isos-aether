import { describe, expect, it } from 'vitest';
import type { NodeGraph, ValidatedEdge } from '@/services/isos/node-graph';
import { edgeSource, parseFrequency, parsePriority, sourcePointer, updateEdgeQos, validatedSource } from './edge-qos';
import { inputSources, updateDesiredRate } from './node-graph-edits';

const target = 'isos/ui/vision/front';
const graph: NodeGraph = {
  version: '0.2.1', robot: { id: 'r', name: 'r' }, future: { keep: true },
  cores: { isos: { layers: { ui: { groups: { vision: { nodes: {
    front: { id: 'front', label: 'front', description: '', node_type: 'isos.ui.vision.monitor@v0.2', inputs: {
      frame: [
        { node: 'isos/eos_hardware/camera/a', output: 'frame', extension: 'a' },
        { node: 'isos/eos_hardware/camera/b', output: 'frame', extension: 'b' },
      ],
      single: { node: 'isos/eos_hardware/camera/a', output: 'frame', extension: 'single' },
    } },
  } } } } } } },
};
const admitted: ValidatedEdge[] = [0, 1].map((index) => ({ path: sourcePointer(target, 'frame', index, true), source: `isos/eos_hardware/camera/${index}`, target, input: 'frame', route_class: 'eos_local', fixed: false, routed: true, qos_eligible: true, priority: 0, data_type: 'camera_frame', transport: 'in_process' }));

describe('selected edge QoS', () => {
  it('matches exact IsOS source pointers for object and array shapes', () => {
    expect(sourcePointer(target, 'frame', 1, true)).toBe('/cores/isos/layers/ui/groups/vision/nodes/front/inputs/frame/1');
    expect(sourcePointer(target, 'single', 0, false)).toBe('/cores/isos/layers/ui/groups/vision/nodes/front/inputs/single');
    expect(validatedSource(admitted, target, 'frame', 1, true)).toBe(admitted[1]);
    expect(validatedSource(admitted, target, 'single', 0, false)).toBeUndefined();
  });
  it('edits one source and preserves siblings, unknown fields, and omitted optional values', () => {
    const next = updateEdgeQos(graph, target, 'frame', 1, { priority: -20 });
    expect(edgeSource(next, target, 'frame', 1)?.source).toMatchObject({ priority: -20, extension: 'b' });
    expect(edgeSource(next, target, 'frame', 0)?.source).toEqual(edgeSource(graph, target, 'frame', 0)?.source);
    expect(edgeSource(graph, target, 'frame', 1)?.source.priority).toBeUndefined();
    expect((next as Record<string, unknown>).future).toEqual({ keep: true });
    expect(Array.isArray(updateEdgeQos(graph, target, 'single', 0, { priority: 19 }).cores.isos.layers.ui.groups.vision.nodes.front.inputs?.single)).toBe(false);
  });
  it('validates nice bounds and optional finite positive cap', () => {
    expect([-20, 0, 19].map((value) => parsePriority(String(value)))).toEqual([-20, 0, 19]);
    for (const value of ['-21', '20', '1.5', '', 'Infinity']) expect(parsePriority(value)).toBeNull();
    expect(parseFrequency('')).toBeUndefined();
    expect(parseFrequency('0.01')).toBe(0.01);
    for (const value of ['0', '-1', 'Infinity', 'NaN']) expect(parseFrequency(value)).toBeNull();
  });
  it('wraps an input with a port range while preserving its sources and priority edits', () => {
    const ranged = updateDesiredRate(graph, target, 'frame', { min: 2, max: 24 });
    const binding = ranged.cores.isos.layers.ui.groups.vision.nodes.front.inputs?.frame;
    expect(binding).toMatchObject({ desired_rate_hz: { min: 2, max: 24 } });
    expect(binding && inputSources(binding)).toEqual(inputSources(graph.cores.isos.layers.ui.groups.vision.nodes.front.inputs!.frame));
    const prioritized = updateEdgeQos(ranged, target, 'frame', 1, { priority: 5 });
    expect(prioritized.cores.isos.layers.ui.groups.vision.nodes.front.inputs?.frame).toMatchObject({ desired_rate_hz: { min: 2, max: 24 } });
    expect(edgeSource(prioritized, target, 'frame', 1)?.source.priority).toBe(5);
  });
});
