import { describe, expect, it } from 'vitest';
import type { NodeGraph, NodeTypeDefinition } from '@/services/isos/node-graph';
import { addNode, addableTypes, admittedConnection, bindPrimitiveTarget, canonicalNodePath, connectNodes, disconnectInput, graphNodes, inputSources, nextNodeId, nodePath, removeNode, removedNodePaths, renameNode, updateNode } from './node-graph-edits';

it('keeps legacy types for graph resolution but hides them from new additions', () => {
  const current = { ...camera, id: 'isos.ui.workbench.health@v0.2' };
  const legacy = { ...camera, id: 'isos.ui.workbench.panel@v0.2', deprecated: true };
  expect(addableTypes([legacy, current])).toEqual([current]);
});

const movementContract = { owner: 'eos', id: 'base-movement', version: '1.0' };
const dispatcher: NodeTypeDefinition = { id: 'isos.eos_dispatcher.control@v0.2', core: 'isos', layer: 'eos_dispatcher', group: 'control', kind: 'software', lifecycle: 'internal', capabilities: ['admission'], settings: [], inputs: [], outputs: [{ name: 'base_command', data_type: 'base_movement', transport: 'in_process', contract: movementContract, required: false }] };
const base: NodeTypeDefinition = { id: 'isos.eos_hardware.base@v0.2', core: 'isos', layer: 'eos_hardware', group: 'base', kind: 'base', lifecycle: 'adapter', capabilities: ['body_velocity'], settings: [], inputs: [{ name: 'command', data_type: 'base_movement', transport: 'in_process', contract: movementContract, required: true }], outputs: [{ name: 'status', data_type: 'status', required: false }] };
const camera: NodeTypeDefinition = { id: 'isos.eos_hardware.camera@v0.2', core: 'isos', layer: 'eos_hardware', group: 'camera', kind: 'camera', lifecycle: 'adapter', capabilities: ['rgb8', 'aligned_depth_u16'], settings: [], inputs: [], outputs: [{ name: 'frame', data_type: 'camera_frame', required: false }] };
const empty = (): NodeGraph => ({ version: '0.2.0', robot: { id: 'robot', name: 'Robot' }, cores: {} });

describe('current node graph edits', () => {
  it('detects a package preview that removes an existing node', () => {
    const graph = addNode(empty(), camera, 'front');
    expect(removedNodePaths(graph, addNode(graph, camera, 'rear'))).toEqual([]);
    expect(removedNodePaths(graph, empty())).toEqual(['isos/eos_hardware/camera/front']);
  });
  it('uses the final node type and next free index as the default ID', () => {
    const first = nextNodeId(empty(), camera);
    expect(first).toBe('camera_1');
    const graph = addNode(empty(), camera, first);
    expect(nextNodeId(graph, camera)).toBe('camera_2');
    expect(graphNodes(addNode(graph, camera, 'camera_2'), [camera])).toHaveLength(2);
  });
  it('places an instance under its registered core, layer and group', () => {
    const graph = addNode(empty(), camera, 'front');
    const entry = graphNodes(graph, [camera])[0];
    expect(entry.path).toBe('isos/eos_hardware/camera/front');
    expect(canonicalNodePath('cores/isos/layers/eos_hardware/groups/camera/nodes/front')).toBe(entry.path);
    expect(entry.node.node_type).toBe('isos.eos_hardware.camera@v0.2');
    expect(entry.node.capabilities).toBeUndefined();
    expect(entry.node.movement).toBeUndefined();
  });
  it('resolves duplicate type IDs by the complete catalog placement', () => {
    const other = { ...camera, group: 'inspection', description: 'Other placement' };
    const graph = addNode(addNode(empty(), camera, 'front'), other, 'second');
    const entries = graphNodes(graph, [other, camera]);
    expect(entries.find((entry) => entry.path === 'isos/eos_hardware/camera/front')?.definition).toBe(camera);
    expect(entries.find((entry) => entry.path === 'isos/eos_hardware/inspection/second')?.definition).toBe(other);
    expect(graphNodes(graph, [camera])[1].definition).toBeUndefined();
  });
  it('does not invent hardware values for a new target type', () => {
    const target = { ...base, id: 'isos.eos_hardware.base@v0.2' };
    const node = graphNodes(addNode(empty(), target, 'new-base'), [target])[0].node;
    expect(node.adapter).toBeUndefined();
    expect(node.movement).toBeUndefined();
    expect(node.observation).toBeUndefined();
    expect(node.settings).toBeUndefined();
    expect(node.capabilities).toBeUndefined();
  });

  it('connects only matching typed ports and admits one source per input', () => {
    const graph = addNode(addNode(empty(), dispatcher, 'control'), base, 'drive');
    const source = nodePath(dispatcher, 'control');
    const target = nodePath(base, 'drive');
    expect(() => connectNodes(graph, [dispatcher, base], source, 'missing', target, 'command')).toThrow('same data type');
    const connected = connectNodes(graph, [dispatcher, base], source, 'base_command', target, 'command');
    expect(graphNodes(connected, [dispatcher, base]).find((entry) => entry.path === target)?.node.inputs?.command).toEqual({ node: source, output: 'base_command' });
    expect(() => connectNodes(connected, [dispatcher, base], source, 'base_command', target, 'command')).toThrow('already has a provider');
    expect(graphNodes(disconnectInput(connected, target, 'command'), [dispatcher, base]).find((entry) => entry.path === target)?.node.inputs).toEqual({});
  });

  it('rejects transport and delivery mismatches before changing the draft', () => {
    const producer = { ...dispatcher, outputs: [{ name: 'state', data_type: 'key_state', required: false, transport: 'http', delivery_mode: 'event' }] };
    const consumer = { ...base, inputs: [{ name: 'state', data_type: 'key_state', required: true, transport: 'unix_socket', delivery_mode: 'event' }] };
    const graph = addNode(addNode(empty(), producer, 'keyboard'), consumer, 'gateway');
    expect(() => connectNodes(graph, [producer, consumer], nodePath(producer, 'keyboard'), 'state', nodePath(consumer, 'gateway'), 'state')).toThrow('same transport');
    const sameTransport = { ...consumer, inputs: [{ ...consumer.inputs[0], transport: 'http', delivery_mode: 'request' }] };
    expect(() => connectNodes(graph, [producer, sameTransport], nodePath(producer, 'keyboard'), 'state', nodePath(consumer, 'gateway'), 'state')).toThrow('same delivery');
    expect(graphNodes(graph, []).find((entry) => entry.path === nodePath(consumer, 'gateway'))?.node.inputs).toBeUndefined();
  });

  it('requires an exact owner-admitted output and rejects design-only routes', () => {
    const source = 'isos/ui/source/front', target = 'ei/io/sink/preview';
    const edge = { path: '/inputs/frame', source, output: 'frame', target, input: 'frame', route_class: 'owner_bridge', fixed: false, routed: true, qos_eligible: false, priority: 0, data_type: 'camera_frame', transport: 'http' };
    expect(admittedConnection({ allowed: true, edge }, source, 'frame', target, 'frame')).toBe(true);
    expect(admittedConnection({ allowed: true, edge }, source, 'other', target, 'frame')).toBe(false);
    expect(admittedConnection({ allowed: true, edge: { ...edge, route_class: 'design_only' } }, source, 'frame', target, 'frame')).toBe(false);
    expect(admittedConnection({ allowed: false, edge, diagnostic: { path: '/inputs/frame', message: 'Rejected' } }, source, 'frame', target, 'frame')).toBe(false);
  });

  it('rewrites canonical references on rename and removes inbound connections on delete', () => {
    const initial = addNode(addNode(empty(), dispatcher, 'control'), base, 'drive');
    const source = nodePath(dispatcher, 'control');
    const target = nodePath(base, 'drive');
    const connected = connectNodes(initial, [dispatcher, base], source, 'base_command', target, 'command');
    const renamed = renameNode(connected, source, 'admission');
    const newSource = nodePath(dispatcher, 'admission');
    expect(graphNodes(renamed, [dispatcher, base]).find((entry) => entry.path === target)?.node.inputs?.command && inputSources(graphNodes(renamed, [dispatcher, base]).find((entry) => entry.path === target)!.node.inputs!.command)[0].node).toBe(newSource);
    const removed = removeNode(renamed, newSource);
    expect(graphNodes(removed, [dispatcher, base]).find((entry) => entry.path === target)?.node.inputs).toEqual({});
    expect(connected).toEqual(connectNodes(initial, [dispatcher, base], source, 'base_command', target, 'command'));
  });
  it('preserves service bindings through edits and rewrites optional provider paths', () => {
    const serviceType: NodeTypeDefinition = { ...dispatcher, service_inputs: { eos: { owner: 'eos', service: 'dispatcher', version: 'v0.2', protocol: 'grpc', carrier: 'tcp', required: true } } };
    const provider = nodePath(dispatcher, 'provider');
    const consumer = nodePath(serviceType, 'consumer');
    let graph = addNode(addNode(empty(), dispatcher, 'provider'), serviceType, 'consumer');
    graph = updateNode(graph, consumer, { service_bindings: { eos: { owner: 'eos', service: 'dispatcher', version: 'v0.2', node: provider } } });
    expect(graphNodes(graph, [serviceType]).find((entry) => entry.path === consumer)?.node.service_bindings?.eos.node).toBe(provider);
    graph = renameNode(graph, provider, 'renamed');
    expect(graphNodes(graph, [serviceType]).find((entry) => entry.path === consumer)?.node.service_bindings?.eos.node).toBe(nodePath(dispatcher, 'renamed'));
    graph = removeNode(graph, nodePath(dispatcher, 'renamed'));
    expect(graphNodes(graph, [serviceType]).find((entry) => entry.path === consumer)?.node.service_bindings).toEqual({});
  });
  it('supports multiple primitive bindings and rewires one target at a time', () => {
    const binding: NodeTypeDefinition = { ...dispatcher, id: 'isos.eos_dispatcher.primitive_base@v0.2', group: 'base', outputs: [{ name: 'command', data_type: 'base_movement', transport: 'in_process', contract: movementContract, required: false }] };
    const multiBase: NodeTypeDefinition = { ...base, inputs: [{ name: 'command', data_type: 'base_movement', transport: 'in_process', contract: movementContract, required: true, cardinality: 'many' }] };
    let graph = addNode(addNode(addNode(addNode(empty(), binding, 'first'), binding, 'second'), multiBase, 'drive'), multiBase, 'other');
    const first = nodePath(binding, 'first'), second = nodePath(binding, 'second'), drive = nodePath(base, 'drive'), other = nodePath(base, 'other');
    graph = bindPrimitiveTarget(graph, [binding, multiBase], first, drive);
    graph = bindPrimitiveTarget(graph, [binding, multiBase], second, drive);
    expect(inputSources(graphNodes(graph, []).find((entry) => entry.path === drive)!.node.inputs!.command)).toHaveLength(2);
    graph = bindPrimitiveTarget(graph, [binding, multiBase], first, other);
    expect(inputSources(graphNodes(graph, []).find((entry) => entry.path === drive)!.node.inputs!.command)).toEqual([{ node: second, output: 'command' }]);
    expect(inputSources(graphNodes(graph, []).find((entry) => entry.path === other)!.node.inputs!.command)).toEqual([{ node: first, output: 'command' }]);
  });
});
