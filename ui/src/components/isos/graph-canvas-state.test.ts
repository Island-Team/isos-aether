import { describe, expect, it } from 'vitest';
import type { NodeGraph } from '@/services/isos/node-graph';
import { applyNodeDimensions, applyNodeSelection, asyncDemoLayout, fitGraphPanelWidths, graphLayoutKey, graphMirrorKey, graphPanelWidthsKey, parseGraphPanelWidths, parseNodeMirrors, parseNodePositions, preserveNodePositions, resizeGraphPanel, removeSelectedNodes } from './graph-canvas-state';

const graph = (id: string): NodeGraph => ({ version: '0.2.1', robot: { id, name: id }, cores: {} });
const forwardDemoPaths = ['isos/ui/button/trigger', 'ei/io/input/trigger', 'ei/cognition/intent/forwarder', 'ei/governance/safety/gate', 'eos/interface/ingress/trigger', 'eos/skill/demo/joint_delta', 'eos/dispatcher/arm/delta', 'eos/hardware/mock/arm'];
const returnDemoPaths = ['eos/hardware/mock/arm_observer', 'eos/dispatcher/arm/monitor', 'eos/skill/demo/arm_status', 'eos/interface/egress/arm_status', 'ei/io/input/arm_status', 'ei/cognition/state/arm_status', 'ei/governance/monitor/arm_status', 'isos/ui/monitor/arm'];

function declaredDemoGraph(): NodeGraph {
  const cores: Record<string, { layers: Record<string, { groups: Record<string, { nodes: Record<string, { id: string; node_type: string }> }> }> }> = {};
  for (const path of [...forwardDemoPaths, ...returnDemoPaths]) {
    const [core, layer, group, id] = path.split('/');
    const nodes = ((cores[core] ??= { layers: {} }).layers[layer] ??= { groups: {} }).groups[group] ??= { nodes: {} };
    nodes.nodes[id] = { id, node_type: `${core}.${layer}.${group}@v0.2` };
  }
  return { ...graph('default'), cores } as NodeGraph;
}

describe('graph canvas state', () => {
  it('separates positions by robot identity, independent of graph revision', () => {
    expect(graphLayoutKey(graph('alpha'))).not.toBe(graphLayoutKey(graph('beta')));
    expect(graphLayoutKey(graph('alpha'))).toBe('isosui-v2-layout:alpha');
  });

  it('stores panel widths per graph layout and preserves canvas space', () => {
    expect(graphPanelWidthsKey(graph('alpha'))).toBe('isosui-v2-layout:alpha:panel-widths');
    expect(parseGraphPanelWidths('{"library":390,"side":440}')).toEqual({ library: 390, side: 440 });
    expect(parseGraphPanelWidths('{"library":"bad","side":null}')).toEqual({ library: 260, side: 320 });
    expect(parseGraphPanelWidths('null')).toEqual({ library: 260, side: 320 });
    expect(resizeGraphPanel({ library: 260, side: 320 }, 'library', 900, 1100).library).toBe(460);
    expect(resizeGraphPanel({ library: 260, side: 320 }, 'side', 100, 1100).side).toBe(270);
    expect(fitGraphPanelWidths({ library: 600, side: 600 }, 1100)).toEqual({ library: 225, side: 555 });
  });

  it('does not reuse the old single-row browser layout for the demo', () => {
    expect(graphLayoutKey(declaredDemoGraph())).toBe('isosui-v2-layout:default:async-lanes-v2');
  });

  it('ignores corrupted or non-finite saved coordinates', () => {
    expect(parseNodePositions('{broken')).toEqual({});
    expect(parseNodePositions('{"good":{"x":2,"y":3},"bad":{"x":"2","y":3},"other":{"x":null,"y":0}}')).toEqual({ good: { x: 2, y: 3 } });
  });

  it('preserves displayed coordinates when another node is inserted', () => {
    expect(preserveNodePositions(['b', 'c', 'd'], { c: { x: 17, y: 29 } })).toEqual({
      b: { x: 50, y: 50 }, c: { x: 17, y: 29 }, d: { x: 710, y: 50 },
    });
  });

  it('retains measured node sizes across selection rerenders', () => {
    const measured = applyNodeDimensions({}, [{ id: 'first', type: 'dimensions', dimensions: { width: 140, height: 80 } }]);
    expect(measured).toEqual({ first: { width: 140, height: 80 } });
    expect(applyNodeDimensions(measured, [{ id: 'first', type: 'select', selected: true }])).toBe(measured);
    expect(applyNodeDimensions(measured, [{ id: 'second', type: 'dimensions', dimensions: { width: 0, height: 0 } }])).toBe(measured);
  });

  it('keeps mirrored ports only in local visual layout', () => {
    expect(graphMirrorKey(graph('alpha'))).toBe('isosui-v2-layout:alpha:mirrors');
    expect(parseNodeMirrors('{broken')).toEqual({});
    expect(parseNodeMirrors('{"a":true,"b":false,"c":"yes"}')).toEqual({ a: true, b: false });
    expect(parseNodeMirrors('{"cores/isos/layers/eos_hardware/groups/camera/nodes/front":true}')).toEqual({ 'isos/eos_hardware/camera/front': true });
    expect(parseNodePositions('{"cores/isos/layers/eos_hardware/groups/camera/nodes/front":{"x":2,"y":3}}')).toEqual({ 'isos/eos_hardware/camera/front': { x: 2, y: 3 } });
  });

  it('lays the declared asynchronous demo in separate opposing lanes', () => {
    const forward = forwardDemoPaths;
    const backward = returnDemoPaths;
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
