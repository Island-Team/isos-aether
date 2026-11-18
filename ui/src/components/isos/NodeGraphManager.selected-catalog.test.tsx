// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { GraphCatalogSnapshot, ProcessStatus } from '@/services/isos/node-graph';

const mocks = vi.hoisted(() => ({ snapshot: vi.fn(), installed: vi.fn(), newGraph: vi.fn(), types: vi.fn(), pool: vi.fn(), poolEntry: vi.fn(), putPool: vi.fn(), validate: vi.fn(), apply: vi.fn(), stop: vi.fn(), control: vi.fn(), status: vi.fn(), readiness: vi.fn(), sets: vi.fn(), options: vi.fn() }));
vi.mock('reactflow', () => ({
  default: ({ children, nodes, edges, onNodesChange, onEdgeClick }: { children?: React.ReactNode; nodes?: { id: string; position: { x: number; y: number } }[]; edges?: { id: string }[]; onNodesChange?: (changes: { id: string; type: 'select'; selected: boolean }[]) => void; onEdgeClick?: (event: React.MouseEvent, edge: { id: string }) => void }) => <div className="react-flow__pane">{nodes?.map((node) => <button key={node.id} type="button" data-x={node.position.x} data-y={node.position.y} onClick={() => onNodesChange?.([{ id: node.id, type: 'select', selected: true }])}>Select {node.id}</button>)}{nodes && nodes.length > 1 && <button type="button" onClick={() => onNodesChange?.(nodes.map((node) => ({ id: node.id, type: 'select', selected: true })))}>Select multiple</button>}{edges?.map((edge) => <button key={edge.id} type="button" onClick={(event) => onEdgeClick?.(event, edge)}>Select edge</button>)}{children}</div>,
  ReactFlowProvider: ({ children }: { children?: React.ReactNode }) => <div>{children}</div>,
  Background: () => null, MiniMap: () => null, Handle: () => null,
  Position: { Left: 'left', Right: 'right' }, SelectionMode: { Partial: 'partial' },
}));
vi.mock('@/services/isos/node-graph', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/isos/node-graph')>()),
  getGraphCatalogSnapshot: mocks.snapshot,
  getInstalledPackages: mocks.installed,
  getNewNodeGraph: mocks.newGraph,
  getNodeTypes: mocks.types,
  getGraphPool: mocks.pool,
  getGraphPoolEntry: mocks.poolEntry,
  putGraphPoolEntry: mocks.putPool,
  getNodeSets: mocks.sets,
  getProcessStatus: mocks.status,
  getCoreReadiness: mocks.readiness,
  getNodeOptions: mocks.options,
  validateNodeGraph: mocks.validate,
  applyNodeGraph: mocks.apply,
  stopProcesses: mocks.stop,
  controlProcesses: mocks.control,
}));
vi.mock('@/services/isos/runtime-status-stream', () => ({ subscribeRuntimeStatus: () => () => undefined }));


vi.mock('./PackageConfigPanel', () => ({
  PackageConfigPanel: ({ onDirtyChange }: { onDirtyChange?: (dirty: boolean) => void }) => {
    const [draft, setDraft] = React.useState('');
    return <label>Package draft<input aria-label="Package draft" value={draft} onChange={(event) => { setDraft(event.target.value); onDirtyChange?.(Boolean(event.target.value)); }} /></label>;
  },
}));

const graph = { version: '0.2.1', robot: { id: 'test', name: 'Test' }, cores: { isos: { layers: {} }, ei: { layers: {} }, eos: { layers: {} } } };
const snapshot = (revision: string): GraphCatalogSnapshot => ({ graph: { revision, graph }, selected: { types: [] }, global: { types: [] }, reason: null });
const status: ProcessStatus = { schema_version: '1.0.0', state: 'running', run_id: 'run-a', run_id_state: 'known', saved_revision: 'a'.repeat(64), active_revision: 'a'.repeat(64), processes: [] };

afterEach(() => { vi.resetAllMocks(); vi.unstubAllGlobals(); document.body.innerHTML = ''; localStorage.clear(); });
beforeEach(() => {
  mocks.installed.mockResolvedValue({ installed: [] });
  mocks.pool.mockResolvedValue({ graphs: [] });
  mocks.readiness.mockResolvedValue(null);
  mocks.options.mockResolvedValue({ adapter_families: [], primitive_bindings: [], targets: [] });
  mocks.putPool.mockImplementation(async (name: string, _revision: string, savedGraph: typeof graph) => ({ name, revision: 'a'.repeat(64), graph: savedGraph }));
  mocks.apply.mockImplementation(async (name: string, revision: string) => ({ pool_name: name, revision, graph }));
  vi.stubGlobal('prompt', vi.fn(() => 'new-graph'));
});

it('applies a saved Vision monitor before Save and Run starts vision', async () => {
  const revision = 'a'.repeat(64);
  const visionGraph = { ...graph, cores: { ei: { layers: {} }, eos: { layers: {} }, isos: { layers: { ui: { groups: { vision: { nodes: { preview: { id: 'preview', node_type: 'isos.ui.vision.monitor@v0.2', inputs: {} } } } } } } } } };
  mocks.snapshot.mockResolvedValue({ graph: { revision, graph: visionGraph }, selected: { types: [{ id: 'isos.ui.vision.monitor@v0.2', core: 'isos', layer: 'ui', group: 'vision', kind: 'software', lifecycle: 'internal' }] }, global: { types: [] }, reason: null });
  mocks.status.mockResolvedValueOnce({ ...status, state: 'stopped', run_id: null, active_revision: null }).mockResolvedValue(status);
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
  mocks.apply.mockResolvedValue({ revision, graph: visionGraph });
  mocks.control.mockResolvedValue({ started: ['eos'], active_revision: revision, run_id: 'run-a' });
  const openPanel = vi.fn();
  const { container, root } = await mount(openPanel);
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Save and Run')!.click());
  expect(mocks.putPool).toHaveBeenCalledWith('new-graph', '', visionGraph);
  expect(mocks.apply).toHaveBeenCalledWith('new-graph', revision);
  expect(mocks.control).toHaveBeenCalled();
  expect(container.querySelector('.node-graph-canvas')).toBeTruthy();
  expect(container.textContent).not.toContain('Node graph display failed');
  expect(openPanel).toHaveBeenCalledWith('logs');
  await act(async () => root.unmount());
});

async function mount(onOpenPanel?: (name: 'logs') => void) {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  vi.stubGlobal('requestAnimationFrame', () => 1);
  vi.stubGlobal('cancelAnimationFrame', () => undefined);
  mocks.sets.mockResolvedValue({ sets: [] });
  mocks.types.mockResolvedValue({ types: [], data_types: [] });
  const { NodeGraphManager } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  await act(async () => root.render(<NodeGraphManager lang="en" onOpenPanel={onOpenPanel} />));
  return { container, root };
}

it('shows a saved missing-input warning without making the graph read-only', async () => {
  const warning = { path: '/cores/ei/layers/io/groups/input/nodes/trigger/inputs/activation', code: 'edge_required_input', message: 'required input activation is not connected' };
  mocks.snapshot.mockResolvedValue({ ...snapshot('a'.repeat(64)), graph: { revision: 'a'.repeat(64), graph, warnings: [warning] } });
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  expect(container.textContent).toContain('Warning');
  expect(container.textContent).toContain(warning.message);
  expect([...container.querySelectorAll('button')].find((button) => button.textContent === 'Apply graph')?.disabled).toBe(false);
  await act(async () => root.unmount());
});

it('renders a revision checked graph while optional status and readiness are stalled', async () => {
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.status.mockImplementation(() => new Promise(() => undefined));
  mocks.readiness.mockImplementation(() => new Promise(() => undefined));
  mocks.sets.mockImplementation(() => new Promise(() => undefined));
  const { container, root } = await mount();
  expect(container.querySelector('.node-graph-canvas .react-flow__pane')).toBeTruthy();
  expect(container.textContent).toContain('Saved: aaaaaaaa');
  expect(container.textContent).not.toContain('Package node sets');
  expect(mocks.sets).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});

it('shows startup phase and a direct Logs button while the owner is starting', async () => {
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.status.mockResolvedValue({ ...status, state: 'starting', startup_phase: 'starting_owners', run_id: null, run_id_state: 'unknown', active_revision: null });
  const openPanel = vi.fn();
  const { container, root } = await mount(openPanel);
  expect(container.textContent).toContain('Starting EOS and EI');
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'View Logs')?.click());
  expect(openPanel).toHaveBeenCalledWith('logs');
  await act(async () => root.unmount());
});

it('switches library modes, shows installed packages without nodes, and refreshes inventory and catalog', async () => {
  const pkg = { schema_version: 2, id: 'cog', version: '1.2.0', source_commit: 'abc123', asset_sha256: 'def456', local_link: null, configurable: false };
  mocks.installed.mockResolvedValue({ installed: [pkg] });
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  const packageMode = container.querySelector<HTMLButtonElement>('[aria-label="Package library"]')!;
  await act(async () => packageMode.click());
  expect(packageMode.getAttribute('aria-pressed')).toBe('true');
  const packageSummary = [...container.querySelectorAll('summary')].find((item) => item.textContent?.includes('cog@1.2.0'))!;
  await act(async () => packageSummary.click());
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('abc123');
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('no configuration descriptor');
  expect(container.textContent).toContain('contributes no node types');
  const previousCalls = mocks.snapshot.mock.calls.length;
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Refresh packages"]')!.click());
  expect(mocks.installed.mock.calls.length).toBeGreaterThan(1);
  expect(mocks.snapshot.mock.calls.length).toBeGreaterThan(previousCalls);
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Node library"]')!.click());
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('abc123');
  await act(async () => root.unmount());
});


it('follows package, node, wire, and multi-selection while preserving the package draft', async () => {
  const pkg = { schema_version: 2, id: 'cog', version: '1.2.0', source_commit: 'abc123', asset_sha256: 'def456', local_link: null, configurable: true };
  mocks.installed.mockResolvedValue({ installed: [pkg, { ...pkg, id: 'other' }] });
  const type = { id: 'isos.ui.demo@v0.2', core: 'isos', layer: 'ui', group: 'demo', kind: 'software', lifecycle: 'internal', description: 'Demo type', settings: [], inputs: [{ name: 'in' }], outputs: [{ name: 'out' }] };
  const connected = { ...graph, cores: { ...graph.cores, isos: { layers: { ui: { groups: { demo: { nodes: {
    first: { id: 'first', node_type: type.id },
    second: { id: 'second', node_type: type.id, inputs: { in: { node: 'isos/ui/demo/first', output: 'out' } } },
  } } } } } } } };
  mocks.snapshot.mockResolvedValue({ graph: { revision: 'a'.repeat(64), graph: connected }, selected: { types: [type] }, global: { types: [] }, reason: null });
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Package library"]')!.click());
  const summaries = [...container.querySelectorAll('summary')].filter((item) => item.textContent?.includes('@1.2.0'));
  await act(async () => summaries[0].click());
  const draft = container.querySelector<HTMLInputElement>('[aria-label="Package draft"]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(draft, 'keep me'); draft.dispatchEvent(new Event('input', { bubbles: true })); });
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Select isos/ui/demo/first')!.click());
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('Type description: Demo type');
  expect(container.querySelector('.node-graph-debug-details pre[aria-label="Node instance JSON"]')?.textContent).toContain('"node_type": "isos.ui.demo@v0.2"');
  expect(container.querySelector('.node-graph-debug-details pre[aria-label="Type declaration JSON"]')?.textContent).toContain('"description": "Demo type"');
  expect(draft.value).toBe('keep me');
  expect(draft.closest('[hidden]')).toBeTruthy();
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Select multiple')!.click());
  expect(container.querySelector('.node-graph-multi-selection')?.textContent).toContain('2 nodes selected');
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Select edge')!.click());
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('out');
  await act(async () => summaries[0].click());
  expect(draft.closest('[hidden]')).toBeNull();
  expect(draft.value).toBe('keep me');
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Node library"]')!.click());
  await act(async () => container.querySelector<HTMLButtonElement>('.node-graph-catalog-browser .node-library-type')!.click());
  expect(container.querySelector('.node-library-type-details')?.textContent).toContain(type.id);
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Package library"]')!.click());
  await act(async () => summaries[0].click());
  expect(draft.value).toBe('keep me');
  const confirm = vi.fn(() => false); vi.stubGlobal('confirm', confirm);
  await act(async () => [...container.querySelectorAll('summary')].find((item) => item.textContent?.includes('other@1.2.0'))!.click());
  expect(confirm).toHaveBeenCalledTimes(1);
  expect(draft.value).toBe('keep me');
  await act(async () => root.unmount());
});

it('clears package details when refreshed inventory removes the selection', async () => {
  const pkg = { schema_version: 2, id: 'cog', version: '1.2.0', source_commit: 'abc123', asset_sha256: 'def456', local_link: null, configurable: false };
  mocks.installed.mockResolvedValueOnce({ installed: [pkg] }).mockResolvedValue({ installed: [] });
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Package library"]')!.click());
  await act(async () => [...container.querySelectorAll('summary')].find((item) => item.textContent?.includes('cog@1.2.0'))!.click());
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('abc123');
  await act(async () => container.querySelector<HTMLButtonElement>('[aria-label="Refresh packages"]')!.click());
  expect(container.querySelector('.node-graph-side')?.textContent).not.toContain('abc123');
  expect(container.querySelector('.node-graph-side')?.textContent).toContain('Select a node to edit it');
  await act(async () => root.unmount());
});

it('keeps owner choices visible but disabled while a changed draft refreshes', async () => {
  const path = 'isos/eos_hardware/base/drive';
  const type = { id: 'isos.eos_hardware.base@v0.2', core: 'isos', layer: 'eos_hardware', group: 'base', kind: 'base', lifecycle: 'adapter', settings: [], inputs: [], outputs: [] };
  const family = { id: 'mock-v1', device_kinds: ['base'], compiled: true, available: true, transport: 'in_process', description: 'Mock' };
  const hardwareGraph = { ...graph, cores: { ...graph.cores, isos: { layers: { eos_hardware: { groups: { base: { nodes: { drive: { id: 'drive', node_type: type.id, frame_id: 'base', adapter: { type: 'mock-v1', settings: { device_id: 'base-a' } } } } } } } } } } };
  const groups = [
    { node_path: path, setting: 'adapter.type', selected: 'mock-v1', options: [{ value: 'mock-v1', label: 'Mock', description: '', available: true, reason_code: null, reason: null }] },
    { node_path: path, setting: 'adapter.settings.device_id', selected: 'base-a', options: [{ value: 'base-a', label: 'Base A', description: '', available: true, reason_code: null, reason: null }] },
  ];
  mocks.snapshot.mockResolvedValue({ graph: { revision: 'a'.repeat(64), graph: hardwareGraph }, selected: { types: [type], adapter_families: [family] }, global: { types: [] }, reason: null });
  mocks.status.mockResolvedValue(status);
  mocks.options.mockResolvedValueOnce({ adapter_families: [family], primitive_bindings: [], targets: [], setting_options: groups });
  const { container, root } = await mount();
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 300)); });
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === `Select ${path}`)!.click());
  expect([...container.querySelectorAll('.node-graph-catalog-selector select')].some((item) => !item.hasAttribute('disabled') && item.textContent?.includes('Base A'))).toBe(true);
  let rejectRefresh!: (reason: Error) => void;
  mocks.options.mockImplementation(() => new Promise((_resolve, reject) => { rejectRefresh = reject; }));
  const frame = [...container.querySelectorAll('label')].find((label) => label.textContent?.startsWith('frame_id'))!.querySelector('input')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(frame, 'base-new'); frame.dispatchEvent(new Event('input', { bubbles: true })); frame.dispatchEvent(new Event('focusout', { bubbles: true })); });
  expect(container.querySelector('.node-graph-inspector')?.textContent).toContain('Refreshing choices');
  const stale = [...container.querySelectorAll<HTMLSelectElement>('.node-graph-catalog-selector select')].find((item) => item.textContent?.includes('Base A'))!;
  expect(stale.disabled).toBe(true);
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 300)); rejectRefresh(new Error('owner unavailable')); });
  expect(container.querySelector('.node-graph-inspector')?.textContent).toContain('Choice refresh failed');
  expect(stale.disabled).toBe(true);
  expect(container.querySelector<HTMLButtonElement>('.node-graph-primary-action')?.disabled).toBe(true);
  await act(async () => root.unmount());
});

it('resizes both graph panels with the keyboard and saves their widths', async () => {
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  const library = container.querySelector('[aria-label="Resize Node library"]')!;
  const inspector = container.querySelector('[aria-label="Resize Node configuration"]')!;
  await act(async () => library.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })));
  await act(async () => inspector.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true })));
  expect(library.getAttribute('aria-valuenow')).toBe('280');
  expect(inspector.getAttribute('aria-valuenow')).toBe('340');
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 300)); });
  expect(JSON.parse(localStorage.getItem('isosui-v2-layout:test:panel-widths') || '{}')).toEqual({ library: 280, side: 340 });
  await act(async () => root.unmount());
});

it('loads and saves a replacement draft when the saved graph cannot be opened', async () => {
  const oldRevision = 'b'.repeat(64);
  const emptyGraph = { version: '0.2.1', robot: { id: 'test', name: 'Test' }, cores: { isos: { layers: {} } } };
  mocks.snapshot.mockRejectedValue(new Error('saved graph version is unsupported'));
  mocks.newGraph.mockResolvedValue({ revision: oldRevision, graph: emptyGraph, read_only: false, diagnostics: [] });
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
  mocks.apply.mockResolvedValue({ revision: 'c'.repeat(64), graph: emptyGraph });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  expect(container.textContent).toContain('saved graph version is unsupported');
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Load current node demo')!.click());
  expect(container.textContent).toContain('Unsaved changes');
  expect(mocks.apply).not.toHaveBeenCalled();
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Apply graph')!.click());
  expect(mocks.putPool).toHaveBeenCalledWith('new-graph', '', emptyGraph);
  expect(mocks.apply).toHaveBeenCalledWith('new-graph', 'a'.repeat(64));
  expect(mocks.control).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});

it('opens a named pool file and saves back to that file', async () => {
  const currentRevision = 'a'.repeat(64);
  const poolRevision = 'b'.repeat(64);
  const poolGraph = { version: '0.2.1', robot: { id: 'test', name: 'Pool robot' }, cores: { isos: { layers: {} } } };
  mocks.snapshot.mockResolvedValue(snapshot(currentRevision));
  mocks.newGraph.mockResolvedValue({ revision: currentRevision, graph });
  mocks.pool.mockResolvedValue({ graphs: [{ name: 'sample', revision: poolRevision, version: '0.2.1' }] });
  mocks.poolEntry.mockResolvedValue({ name: 'sample', revision: poolRevision, graph: poolGraph });
  mocks.putPool.mockResolvedValue({ name: 'sample', revision: 'c'.repeat(64), graph: poolGraph });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  const select = [...container.querySelectorAll('select')].find((item) => item.parentElement?.textContent?.includes('Graph file'))!;
  await act(async () => { select.value = 'sample'; select.dispatchEvent(new Event('change', { bubbles: true })); });
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Open file')!.click());
  expect(container.textContent).toContain('Editing: sample. Save will replace');
  expect(mocks.apply).not.toHaveBeenCalled();
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Save file')!.click());
  expect(mocks.putPool).toHaveBeenCalledWith('sample', poolRevision, poolGraph);
  expect(mocks.apply).not.toHaveBeenCalled();
  await act(async () => root.unmount());
});

it('loads the saved demo from the pool with its owner types and canvas layout', async () => {
  const revision = 'a'.repeat(64);
  const nodeType = 'ei.governance.monitor.arm_status@v0.2';
  const nodePath = 'ei/governance/monitor/arm_status';
  const demo = { ...graph, cores: { ...graph.cores, ei: { layers: { governance: { groups: { monitor: { nodes: { arm_status: { id: 'arm_status', node_type: nodeType, label: 'EI monitor governance' } } } } } } } } };
  const eiType = { id: nodeType, core: 'ei', layer: 'governance', group: 'monitor', kind: 'software', lifecycle: 'internal', description: 'Read-only monitor pass', settings: [{ name: 'forward_rate_hz', data_type: 'number', default: 20, required: false }], inputs: [], outputs: [] };
  mocks.snapshot.mockResolvedValue({ graph: { revision, graph: demo }, selected: { types: [eiType] }, global: { types: [] }, reason: null });
  mocks.pool.mockResolvedValue({ graphs: [{ name: 'demo', revision, version: '0.2.1' }] });
  mocks.poolEntry.mockResolvedValue({ name: 'demo', revision, graph: demo });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  localStorage.setItem('isosui-v2-layout:test', JSON.stringify({ [nodePath]: { x: 900, y: 500 } }));
  const { container, root } = await mount();
  const select = [...container.querySelectorAll('select')].find((item) => item.parentElement?.textContent?.includes('Graph file'))!;
  await act(async () => { select.value = 'demo'; select.dispatchEvent(new Event('change', { bubbles: true })); });
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Open file')!.click());
  expect(container.textContent).toContain('Saved ·');
  const node = [...container.querySelectorAll('button')].find((button) => button.textContent === `Select ${nodePath}`)!;
  expect(node.getAttribute('data-x')).toBe('900');
  expect(node.getAttribute('data-y')).toBe('500');
  await act(async () => node.click());
  expect(container.textContent).toContain('Read-only monitor pass');
  expect(container.textContent).toContain('forward_rate_hz');
  await act(async () => root.unmount());
});

it('opens the last existing pool file and keeps its name in the browser', async () => {
  const revision = 'b'.repeat(64);
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.pool.mockResolvedValue({ graphs: [{ name: 'demo', revision: 'a'.repeat(64), version: '0.2.1' }, { name: 'custom', revision, version: '0.2.1' }] });
  mocks.poolEntry.mockImplementation(async (name: string) => ({ name, revision, graph }));
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  localStorage.setItem('isosui-graph-pool-last-name', 'custom');
  const { container, root } = await mount();
  expect(mocks.poolEntry).toHaveBeenCalledWith('custom');
  expect(container.textContent).toContain('Editing: custom. Save will replace');
  expect(localStorage.getItem('isosui-graph-pool-last-name')).toBe('custom');
  await act(async () => root.unmount());
});

it('asks for a new file name when the remembered pool file is missing', async () => {
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.pool.mockResolvedValue({ graphs: [] });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  localStorage.setItem('isosui-graph-pool-last-name', 'deleted');
  const { container, root } = await mount();
  expect(container.textContent).toContain('file is missing; Save will ask for a new name');
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Save file')!.click());
  expect(window.prompt).toHaveBeenCalled();
  expect(mocks.putPool).toHaveBeenCalledWith('new-graph', '', graph);
  await act(async () => root.unmount());
});

it('falls back to demo when the remembered graph file is missing', async () => {
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.pool.mockResolvedValue({ graphs: [{ name: 'demo', revision: 'a'.repeat(64), version: '0.2.1' }] });
  mocks.poolEntry.mockResolvedValue({ name: 'demo', revision: 'a'.repeat(64), graph });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  localStorage.setItem('isosui-graph-pool-last-name', 'deleted');
  const { container, root } = await mount();
  expect(mocks.poolEntry).toHaveBeenCalledWith('demo');
  expect(container.textContent).toContain('The last graph file “deleted” is missing. Opened “demo”.');
  expect(localStorage.getItem('isosui-graph-pool-last-name')).toBe('demo');
  await act(async () => root.unmount());
});

it('explains that a saved graph file needs Apply before deployment', async () => {
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.readiness.mockResolvedValue({ cores: [{ core: 'eos', path: '/bin/eos', version: '0.2.1', compatible: true, configured: true, graph_configured: true, running: false, ready: false, reason: 'pending_apply', action: 'Apply' }] });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  await act(async () => { await Promise.resolve(); });
  expect(container.textContent).toContain('Graph file saved; Apply to update deployment configuration');
  await act(async () => root.unmount());
});

it('offers Save As when a bound file disappears during Save', async () => {
  const revision = 'b'.repeat(64);
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.pool.mockResolvedValueOnce({ graphs: [{ name: 'demo', revision, version: '0.2.1' }] })
    .mockResolvedValueOnce({ graphs: [{ name: 'demo', revision, version: '0.2.1' }] })
    .mockResolvedValue({ graphs: [] });
  mocks.poolEntry.mockResolvedValue({ name: 'demo', revision, graph });
  mocks.putPool.mockRejectedValueOnce(new Error('file missing'))
    .mockResolvedValueOnce({ name: 'new-graph', revision: 'c'.repeat(64), graph });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Save file')!.click());
  expect(mocks.putPool).toHaveBeenNthCalledWith(1, 'demo', revision, graph);
  expect(mocks.putPool).toHaveBeenNthCalledWith(2, 'new-graph', '', graph);
  expect(localStorage.getItem('isosui-graph-pool-last-name')).toBe('new-graph');
  await act(async () => root.unmount());
});

it('retries Apply with the saved pool revision after Apply fails', async () => {
  const oldRevision = 'a'.repeat(64);
  const newRevision = 'b'.repeat(64);
  mocks.snapshot.mockResolvedValueOnce(snapshot(oldRevision)).mockResolvedValueOnce(snapshot(oldRevision)).mockResolvedValue(snapshot(newRevision));
  mocks.pool.mockResolvedValue({ graphs: [{ name: 'demo', revision: oldRevision, version: '0.2.1' }] });
  mocks.poolEntry.mockResolvedValueOnce({ name: 'demo', revision: oldRevision, graph }).mockResolvedValue({ name: 'demo', revision: newRevision, graph });
  mocks.putPool.mockResolvedValue({ name: 'demo', revision: newRevision, graph });
  mocks.apply.mockRejectedValueOnce(new Error('generated config failed')).mockResolvedValue({ pool_name: 'demo', revision: newRevision, graph });
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  const apply = () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Apply graph')!;
  await act(async () => apply().click());
  await act(async () => apply().click());
  expect(mocks.putPool).toHaveBeenNthCalledWith(1, 'demo', oldRevision, graph);
  expect(mocks.putPool).toHaveBeenNthCalledWith(2, 'demo', newRevision, graph);
  await act(async () => root.unmount());
});

it('keeps a different pool draft read-only when an owner type is absent from the catalog', async () => {
  const nodeType = 'ei.governance.monitor.unknown@v0.2';
  const draft = { ...graph, cores: { ...graph.cores, ei: { layers: { governance: { groups: { monitor: { nodes: { unknown: { id: 'unknown', node_type: nodeType } } } } } } } } };
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.pool.mockResolvedValue({ graphs: [{ name: 'other', revision: 'b'.repeat(64), version: '0.2.1' }] });
  mocks.poolEntry.mockResolvedValue({ name: 'other', revision: 'b'.repeat(64), graph: draft });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  const select = [...container.querySelectorAll('select')].find((item) => item.parentElement?.textContent?.includes('Graph file'))!;
  await act(async () => { select.value = 'other'; select.dispatchEvent(new Event('change', { bubbles: true })); });
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Open file')!.click());
  expect(container.textContent).toContain(`Unsupported node type ${nodeType}`);
  expect((([...container.querySelectorAll('button')].find((button) => button.textContent === 'Apply graph')) as HTMLButtonElement).disabled).toBe(true);
  await act(async () => root.unmount());
});

it('keeps the editor open when a selected owner type omits empty catalog arrays', async () => {
  const sparseType = { id: 'isos.ei_cognition.builder@v0.2', core: 'isos', layer: 'ei_cognition', group: 'context', kind: 'software', lifecycle: 'internal' };
  const nodePath = 'isos/ei_cognition/context/builder';
  const graphWithNode = { version: '0.2.1', robot: { id: 'test', name: 'Test' }, cores: { isos: { layers: { ei_cognition: { groups: { context: { nodes: { builder: { id: 'builder', node_type: sparseType.id, label: 'Builder', description: '' } } } } } } } } };
  mocks.snapshot.mockResolvedValue({ graph: { revision: 'a'.repeat(64), graph: graphWithNode }, selected: { types: [sparseType] }, global: { types: [] }, reason: null });
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === `Select ${nodePath}`)!.click());
  expect(container.textContent).toContain('Capabilities');
  expect((container.querySelector('.node-graph-side input[value="Builder"]') as HTMLInputElement | null)?.value).toBe('Builder');
  await act(async () => root.unmount());
});

it('keeps the workbench visible when an owner setting omits its data type', async () => {
  const nodeType = 'ei.governance.monitor.arm_status@v0.2';
  const graphWithNode = { ...graph, cores: { ...graph.cores, ei: { layers: { governance: { groups: { monitor: { nodes: { arm_status: { id: 'arm_status', node_type: nodeType, label: 'EI monitor governance' } } } } } } } } };
  const type = { id: nodeType, core: 'ei', layer: 'governance', group: 'monitor', kind: 'software', lifecycle: 'internal', settings: [{ name: 'forward_rate_hz', type: 'number', default: 20, required: false }], inputs: [], outputs: [] };
  mocks.snapshot.mockResolvedValue({ graph: { revision: 'a'.repeat(64), graph: graphWithNode }, selected: { types: [type] }, global: { types: [] }, reason: null });
  mocks.status.mockResolvedValue({ ...status, state: 'stopped', run_id: null, active_revision: null });
  const { container, root } = await mount();
  await act(async () => [...container.querySelectorAll('button')].find((button) => button.textContent === 'Select ei/governance/monitor/arm_status')!.click());
  expect(container.textContent).toContain('forward_rate_hz');
  expect(container.querySelector('.node-graph-canvas')).toBeTruthy();
  await act(async () => root.unmount());
});

it('keeps the last graph visible while an explicit snapshot reload is slow', async () => {
  mocks.snapshot.mockResolvedValueOnce(snapshot('a'.repeat(64)));
  mocks.status.mockResolvedValue(status);
  const { container, root } = await mount();
  let finishReload!: (value: GraphCatalogSnapshot) => void;
  mocks.snapshot.mockImplementationOnce(() => new Promise((resolve) => { finishReload = resolve; }));
  await act(async () => { const details = container.querySelector('details.node-graph-advanced')!; details.setAttribute('open', ''); details.dispatchEvent(new Event('toggle')); });
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Reload current file')!.click());
  expect(container.querySelector('.node-graph-canvas .react-flow__pane')).toBeTruthy();
  expect(container.textContent).toContain('Saved: aaaaaaaa');
  await act(async () => finishReload(snapshot('b'.repeat(64))));
  expect(container.textContent).toContain('Saved: bbbbbbbb');
  await act(async () => root.unmount());
});

it('leaves Stop enabled during a pending graph validation', async () => {
  mocks.status.mockResolvedValue(status);
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  let finishValidation!: (value: { valid: boolean; diagnostics: [] }) => void;
  mocks.validate.mockImplementation(() => new Promise((resolve) => { finishValidation = resolve; }));
  mocks.stop.mockResolvedValue({ stopped: [], terminal: { run_id: 'run-a', outcome: 'confirmed', reason_code: null, processes: [] } });
  const { container, root } = await mount();
  await act(async () => { const details = container.querySelector('details.node-graph-advanced')!; details.setAttribute('open', ''); details.dispatchEvent(new Event('toggle')); });
  const button = (text: string) => [...container.querySelectorAll('button')].find((item) => item.textContent === text)!;
  await act(async () => button('Check graph').click());
  expect((button('Stop') as HTMLButtonElement).disabled).toBe(false);
  await act(async () => button('Stop').click());
  expect(mocks.stop).toHaveBeenCalledOnce();
  await act(async () => finishValidation({ valid: true, diagnostics: [] }));
  await act(async () => root.unmount());
});

it('does not let an old refresh overwrite a newer package catalog fence', async () => {
  mocks.status.mockResolvedValue(status);
  mocks.snapshot.mockResolvedValueOnce(snapshot('a'.repeat(64)));
  const { container, root } = await mount();
  let finishOld!: (value: GraphCatalogSnapshot) => void;
  mocks.snapshot.mockImplementationOnce(() => new Promise((resolve) => { finishOld = resolve; }))
    .mockResolvedValueOnce(snapshot('b'.repeat(64)));
  await act(async () => { const details = container.querySelector('details.node-graph-advanced')!; details.setAttribute('open', ''); details.dispatchEvent(new Event('toggle')); });
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Reload current file')!.click());
  await act(async () => window.dispatchEvent(new Event('isosui:packages-changed')));
  expect(container.textContent).toMatch(/Saved graph revision changed; refresh before editing\.|Graph catalog could not be checked\./);
  await act(async () => finishOld(snapshot('c'.repeat(64))));
  expect(container.textContent).toMatch(/Saved graph revision changed; refresh before editing\.|Graph catalog could not be checked\./);
  expect(container.textContent).not.toContain('cccccccc');
  await act(async () => root.unmount());
});

it('keeps Stop available during a launch from stopped and converges after a late Run', async () => {
  const stopped = { ...status, state: 'stopped', run_id: null, processes: [] } as ProcessStatus;
  mocks.status.mockResolvedValue(stopped);
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
  let finishRun!: (value: ProcessStatus) => void;
  mocks.control.mockImplementation(() => new Promise((resolve) => { finishRun = resolve; }));
  mocks.stop.mockResolvedValue({ stopped: [], terminal: { run_id: 'run-a', outcome: 'confirmed', reason_code: null, processes: [] } });
  const { container, root } = await mount();
  const button = (text: string) => [...container.querySelectorAll('button')].find((item) => item.textContent === text)!;
  await act(async () => button('Save and Run').click());
  expect((button('Stop') as HTMLButtonElement).disabled).toBe(false);
  await act(async () => button('Stop').click());
  expect(mocks.stop).toHaveBeenCalledOnce();
  await act(async () => finishRun(status));
  expect(mocks.stop).toHaveBeenCalledTimes(2);
  expect(container.textContent).not.toContain('Run confirmed');
  await act(async () => root.unmount());
});

it('fences Graph Run when the workspace Stop arrives from another surface', async () => {
  const stopped = { ...status, state: 'stopped', run_id: null, processes: [] } as ProcessStatus;
  mocks.status.mockResolvedValue(stopped);
  mocks.snapshot.mockResolvedValue(snapshot('a'.repeat(64)));
  mocks.validate.mockResolvedValue({ valid: true, diagnostics: [], edges: [] });
  let finishRun!: (value: ProcessStatus) => void;
  mocks.control.mockImplementation(() => new Promise((resolve) => { finishRun = resolve; }));
  mocks.stop.mockResolvedValue({ stopped: [], terminal: { run_id: 'run-a', outcome: 'confirmed', reason_code: null, processes: [] } });
  const { stopDeploymentConverged } = await import('./deployment-control');
  const { container, root } = await mount();
  await act(async () => [...container.querySelectorAll('button')].find((item) => item.textContent === 'Save and Run')!.click());
  let workspaceStop!: Promise<unknown>;
  await act(async () => { workspaceStop = stopDeploymentConverged(); });
  expect(mocks.stop).toHaveBeenCalledOnce();
  await act(async () => finishRun(status));
  await act(async () => { await workspaceStop; });
  expect(mocks.stop).toHaveBeenCalledTimes(2);
  expect(container.textContent).not.toContain('Run confirmed');
  await act(async () => root.unmount());
});
