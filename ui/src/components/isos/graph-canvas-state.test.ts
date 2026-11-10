import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import type { NodeGraph } from '@/services/isos/node-graph';
import { applyNodeSelection, asyncDemoLayout, graphLayoutKey, graphMirrorKey, parseNodeMirrors, parseNodePositions, removeSelectedNodes } from './graph-canvas-state';

const graph = (id: string): NodeGraph => ({ version: '0.2.0', robot: { id, name: id }, cores: {} });

describe('graph canvas state', () => {
  it('separates positions by robot identity, independent of graph revision', () => {
    expect(graphLayoutKey(graph('alpha'))).not.toBe(graphLayoutKey(graph('beta')));
    expect(graphLayoutKey(graph('alpha'))).toBe('isosui-v2-layout:alpha');
  });

  it('does not reuse the old single-row browser layout for the demo', () => {
    const demo = JSON.parse(readFileSync(new URL('../../../../config/graph-pool/demo.nodes.json', import.meta.url), 'utf8')) as NodeGraph;
    expect(graphLayoutKey(demo)).toBe('isosui-v2-layout:default:async-lanes-v2');
  });

  it('ignores corrupted or non-finite saved coordinates', () => {
    expect(parseNodePositions('{broken')).toEqual({});
    expect(parseNodePositions('{"good":{"x":2,"y":3},"bad":{"x":"2","y":3},"other":{"x":null,"y":0}}')).toEqual({ good: { x: 2, y: 3 } });
  });

  it('keeps mirrored ports only in local visual layout', () => {
    expect(graphMirrorKey(graph('alpha'))).toBe('isosui-v2-layout:alpha:mirrors');
    expect(parseNodeMirrors('{broken')).toEqual({});
    expect(parseNodeMirrors('{"a":true,"b":false,"c":"yes"}')).toEqual({ a: true, b: false });
    expect(parseNodeMirrors('{"cores/isos/layers/eos_hardware/groups/camera/nodes/front":true}')).toEqual({ 'isos/eos_hardware/camera/front': true });
    expect(parseNodePositions('{"cores/isos/layers/eos_hardware/groups/camera/nodes/front":{"x":2,"y":3}}')).toEqual({ 'isos/eos_hardware/camera/front': { x: 2, y: 3 } });
  });

  it('lays the declared asynchronous demo in separate opposing lanes', () => {
    const forward = ['isos/ui/button/trigger', 'ei/io/input/trigger', 'ei/cognition/intent/forwarder', 'ei/governance/safety/gate', 'eos/interface/ingress/trigger', 'eos/skill/demo/joint_delta', 'eos/dispatcher/arm/delta', 'eos/hardware/mock/arm'];
    const backward = ['eos/hardware/mock/arm_observer', 'eos/dispatcher/arm/monitor', 'eos/skill/demo/arm_status', 'eos/interface/egress/arm_status', 'ei/io/input/arm_status', 'ei/cognition/state/arm_status', 'ei/governance/monitor/arm_status', 'isos/ui/monitor/arm'];
    const layout = asyncDemoLayout([...forward, ...backward]);
    expect(layout?.positions[forward[0]]).toEqual({ x: 50, y: 90 });
    expect(layout?.positions[forward[7]]).toEqual({ x: 2430, y: 90 });
    expect(layout?.positions[backward[0]]).toEqual({ x: 2430, y: 490 });
    expect(layout?.positions[backward[7]]).toEqual({ x: 50, y: 490 });
    expect(layout?.mirrors[backward[0]]).toBe(true);
    expect(asyncDemoLayout([...forward, ...backward.slice(1)])).toBeNull();
  });

  it('adds and removes nodes from rectangle selection without changing positions', () => {
    expect(applyNodeSelection(['first'], [{ id: 'second', type: 'select', selected: true }, { id: 'first', type: 'select', selected: false }])).toEqual(['second']);
    expect(applyNodeSelection(['second'], [{ id: 'second', type: 'position', position: { x: 4, y: 8 }, dragging: false }])).toEqual(['second']);
  });

  it('removes several selected nodes in one graph edit', () => {
    const current: NodeGraph = { ...graph('alpha'), cores: { isos: { layers: { eos_hardware: { groups: { camera: { nodes: {
      first: { id: 'first', node_type: 'isos.eos_hardware.camera@v0.2', label: 'First', description: '', settings: {} },
      second: { id: 'second', node_type: 'isos.eos_hardware.camera@v0.2', label: 'Second', description: '', settings: {} },
    } } } } } } } };
    const paths = ['cores/isos/layers/eos_hardware/groups/camera/nodes/first', 'cores/isos/layers/eos_hardware/groups/camera/nodes/second'];
    const result = removeSelectedNodes(current, paths);
    expect(result.cores).toEqual({ isos: { layers: {} } });
  });
});
