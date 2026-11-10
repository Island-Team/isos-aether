import type { NodeGraph } from '@/services/isos/node-graph';
import type { NodeChange } from 'reactflow';
import { canonicalNodePath, graphNodes, removeNode } from './node-graph-edits';

export type NodePositions = Record<string, { x: number; y: number }>;
export type NodeMirrors = Record<string, boolean>;

const forwardDemoPath = [
  'isos/ui/button/trigger', 'ei/io/input/trigger', 'ei/cognition/intent/forwarder',
  'ei/governance/safety/gate', 'eos/interface/ingress/trigger',
  'eos/skill/demo/joint_delta', 'eos/dispatcher/arm/delta', 'eos/hardware/mock/arm',
];
const returnDemoPath = [
  'eos/hardware/mock/arm_observer', 'eos/dispatcher/arm/monitor',
  'eos/skill/demo/arm_status', 'eos/interface/egress/arm_status',
  'ei/io/input/arm_status', 'ei/cognition/state/arm_status',
  'ei/governance/monitor/arm_status', 'isos/ui/monitor/arm',
];

/** Place the two declared demo chains in opposite lanes; authored positions still win. */
export function asyncDemoLayout(paths: string[]): { positions: NodePositions; mirrors: NodeMirrors; lanes: Record<string, 'forward' | 'return'> } | null {
  if (paths.length !== 16 || ![...forwardDemoPath, ...returnDemoPath].every((path) => paths.includes(path))) return null;
  const positions: NodePositions = {};
  const mirrors: NodeMirrors = {};
  const lanes: Record<string, 'forward' | 'return'> = {};
  forwardDemoPath.forEach((path, index) => { positions[path] = { x: 50 + index * 340, y: 90 }; lanes[path] = 'forward'; });
  returnDemoPath.forEach((path, index) => { positions[path] = { x: 50 + (7 - index) * 340, y: 490 }; mirrors[path] = true; lanes[path] = 'return'; });
  return { positions, mirrors, lanes };
}

/** A stable graph identity keeps visual positions separate from graph revisions. */
export function graphLayoutKey(graph: NodeGraph): string {
  const base = `isosui-v2-layout:${encodeURIComponent(graph.robot.id)}`;
  // Use fresh positions for the two independent demo chains.
  return asyncDemoLayout(graphNodes(graph, []).map((entry) => entry.path)) ? `${base}:async-lanes-v2` : base;
}

export function graphMirrorKey(graph: NodeGraph): string { return `${graphLayoutKey(graph)}:mirrors`; }
export function parseNodeMirrors(raw: string | null): NodeMirrors {
  try { const value: unknown = JSON.parse(raw || '{}'); return value && !Array.isArray(value) && typeof value === 'object' ? Object.fromEntries(Object.entries(value).filter((entry): entry is [string, boolean] => typeof entry[1] === 'boolean').map(([path, mirrored]) => [canonicalNodePath(path), mirrored])) : {}; }
  catch { return {}; }
}

export function parseNodePositions(raw: string | null): NodePositions {
  if (!raw) return {};
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') return {};
    return Object.fromEntries(Object.entries(parsed).filter((entry): entry is [string, { x: number; y: number }] => {
      const point = entry[1] as { x?: unknown; y?: unknown } | null;
      return Boolean(point && typeof point.x === 'number' && Number.isFinite(point.x) && typeof point.y === 'number' && Number.isFinite(point.y));
    }).map(([path, point]) => [canonicalNodePath(path), point]));
  } catch { return {}; }
}

export function removeSelectedNodes(graph: NodeGraph, paths: string[]): NodeGraph {
  return [...new Set(paths)].reduce((current, path) => removeNode(current, path), graph);
}

export function applyNodeSelection(current: string[], changes: NodeChange[]): string[] {
  const selection = changes.filter((change): change is Extract<NodeChange, { type: 'select' }> => change.type === 'select');
  if (!selection.length) return current;
  const next = new Set(current);
  for (const change of selection) if (change.selected) next.add(change.id); else next.delete(change.id);
  return [...next];
}
