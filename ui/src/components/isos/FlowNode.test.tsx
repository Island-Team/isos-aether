// @vitest-environment jsdom
import React, { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
const updateNodeInternals = vi.fn();
vi.mock('reactflow', async (importOriginal) => ({ ...(await importOriginal<typeof import('reactflow')>()), Handle: ({ type, position }: { type: string; position: string }) => <span data-testid="handle" data-type={type} data-position={position} />, useUpdateNodeInternals: () => updateNodeInternals }));
it('recalculates connected handle anchors when the node is mirrored', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'isos/interface/egress/monitor', node: { id: 'monitor', node_type: 'isos.interface.egress@v0.2', label: 'Monitor' }, definition: { core: 'isos', layer: 'interface', group: 'egress', kind: 'software', inputs: [{ name: 'observation', data_type: 'arm_monitor_state@v0.2', required: true }], outputs: [{ name: 'observation', data_type: 'arm_monitor_state@v0.2' }] } };
  const props = { id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0];
  await act(async () => root.render(<FlowNode {...props} />));
  expect([...container.querySelectorAll('[data-testid="handle"]')].map((handle) => [handle.getAttribute('data-type'), handle.getAttribute('data-position')])).toEqual([['target', 'left'], ['source', 'right']]);
  updateNodeInternals.mockClear();
  await act(async () => root.render(<FlowNode {...props} data={{ ...props.data, mirrored: true }} />));
  expect([...container.querySelectorAll('[data-testid="handle"]')].map((handle) => [handle.getAttribute('data-type'), handle.getAttribute('data-position')])).toEqual([['target', 'right'], ['source', 'left']]);
  expect(updateNodeInternals).toHaveBeenCalledWith(entry.path);
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});
it('opens node details by focus and hover without card action buttons covering handles', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'isos/eos_hardware/base/drive', node: { id: 'drive', node_type: 'isos.eos_hardware.base@v0.2', label: 'Drive', description: 'Instance notes' }, definition: { id: 'isos.eos_hardware.base@v0.2', core: 'isos', layer: 'eos_hardware', group: 'base', kind: 'base', lifecycle: 'adapter', description: 'Catalog notes', capabilities: [], settings: [], inputs: [{ name: 'command', data_type: 'base_movement', required: true, transport: 'in_process', delivery_mode: 'message', contract: { owner: 'eos', id: 'base-movement', version: '1.0' } }], outputs: [] } };
  const select = vi.fn();
  const props = { id: entry.path, type: 'profile', data: { entry, onSelect: select, error: false, mirrored: false, lang: 'en', liveAllowed: false, liveRunId: null, dataTypes: [{ id: 'base_movement', version: '1.0', purpose: '', media_type: 'application/json', codec: 'json_v1', max_payload_bytes: 1024, schema: { properties: { x_m_s: { type: 'number', 'x-unit': 'm/s' } } } }] }, selected: false } as unknown as Parameters<typeof FlowNode>[0];
  await act(async () => root.render(<FlowNode {...props} />));
  const node = container.querySelector('.node-graph-card') as HTMLElement;
  await act(async () => node.dispatchEvent(new FocusEvent('focusin', { bubbles: true })));
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('Catalog notes');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('Instance notes');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('m/s');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('eos/base-movement@1.0');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('json_v1');
  expect(document.body.querySelector('.node-graph-info-card')?.textContent).toContain('x_m_s');
  expect(container.querySelector('[data-testid="handle"]')).not.toBeNull();
  expect(container.querySelector('.node-graph-card__details')).toBeNull();
  await act(async () => { (document.body.querySelector('.node-graph-info-card button') as HTMLButtonElement).click(); });
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => { node.dispatchEvent(new Event('pointerdown', { bubbles: true })); node.dispatchEvent(new FocusEvent('focusin', { bubbles: true })); node.dispatchEvent(new Event('pointerup', { bubbles: true })); });
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => node.dispatchEvent(new MouseEvent('mouseover', { bubbles: true, clientX: 790, clientY: 590 })));
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 580)); });
  expect(document.body.querySelector('.node-graph-info-card')).not.toBeNull();
  await act(async () => node.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Escape' })));
  expect(document.body.querySelector('.node-graph-info-card')).toBeNull();
  expect(container.querySelector('.node-graph-card__info')).toBeNull();
  expect(container.querySelector('.node-graph-card__actions')).toBeNull();
  await act(async () => node.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'Enter' })));
  expect(select).toHaveBeenCalledWith(entry.path);
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('labels a health-only node without creating graph ports', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'isos/eos_hardware/base/drive', node: { id: 'drive', node_type: 'isos.eos_hardware.health@v0.2', label: 'Drive', description: '' }, definition: { id: 'isos.eos_hardware.base@v0.2', core: 'isos', layer: 'eos_hardware', group: 'base', kind: 'base', lifecycle: 'adapter', capabilities: [], settings: [], inputs: [], outputs: [] } };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  expect(container.textContent).toContain('No graph ports · owner health');
  expect(container.querySelector('[data-testid="handle"]')).toBeNull();
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('renders registered interaction roles as distinct card variants', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'eos/process/vision/transform', node: { id: 'transform', node_type: 'eos.process.vision.transform@v0.2', label: 'Vision' }, definition: { id: 'eos.process.vision.transform@v0.2', core: 'eos', layer: 'process', group: 'vision', kind: 'software', lifecycle: 'internal', interaction_roles: ['subscriber', 'publisher', 'service', 'action'], capabilities: [], settings: [], inputs: [], outputs: [] } };
  const props = { id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0];
  await act(async () => root.render(<FlowNode {...props} />));
  const card = container.querySelector('.node-graph-card');
  for (const role of entry.definition.interaction_roles) {
    expect(card?.classList.contains(`node-graph-card--${role}`)).toBe(true);
    expect(card?.querySelector(`.node-graph-card__role--${role}`)).not.toBeNull();
  }
  expect(card?.querySelectorAll('.node-graph-card__role-slot')).toHaveLength(4);
  expect(card?.querySelectorAll('.node-graph-card__role-slot--active')).toHaveLength(4);
  expect(card?.textContent).toContain('Publisher');
  expect(card?.textContent).toContain('Subscriber');
  expect(card?.textContent).toContain('Service');
  expect(card?.textContent).toContain('Action');
  await act(async () => root.render(<FlowNode {...props} data={{ ...props.data, lang: 'cn' }} />));
  expect(card?.textContent).toContain('发布');
  expect(card?.textContent).toContain('订阅');
  expect(card?.textContent).toContain('服务');
  expect(card?.textContent).toContain('动作');
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('renders every role combination independently of registry order', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const roles = ['publisher', 'subscriber', 'service', 'action'] as const;
  const signatures = new Set<string>();
  for (let mask = 1; mask < 16; mask += 1) {
    const selected = roles.filter((_, index) => mask & (1 << index));
    const entry = { path: 'eos/process/vision/test', node: { id: 'test', node_type: 'test@v0.2' }, definition: { core: 'eos', layer: 'process', group: 'vision', kind: 'software', interaction_roles: [...selected].reverse(), inputs: [], outputs: [] } };
    await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
    const signature = [...container.querySelectorAll('.node-graph-card__role-slot')].map((slot) => slot.classList.contains('node-graph-card__role-slot--active') ? '1' : '0').join('');
    signatures.add(signature);
    expect(signature).toBe(roles.map((role) => selected.includes(role) ? '1' : '0').join(''));
  }
  expect(signatures.size).toBe(15);
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('shows graph placement and a catalog error when a definition is unavailable', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'ei/cognition/context/builder', node: { id: 'builder', node_type: 'ei.cognition.context.health@v0.2', label: 'EI cognition', description: '' }, definition: undefined };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  expect(container.textContent).toContain('cognition / context');
  expect(container.textContent).toContain('Catalog type unavailable; ports cannot be shown');
  expect(container.querySelector('[data-testid="handle"]')).toBeNull();
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});

it('keeps the runtime Activate action while removing Info and Configure card buttons', async () => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  const { FlowNode } = await import('./NodeGraphManager');
  const container = document.createElement('div'); document.body.appendChild(container); const root = createRoot(container);
  const entry = { path: 'isos/ui/button/trigger', node: { id: 'trigger', node_type: 'isos.ui.button.trigger@v0.2', label: 'Trigger', description: '' }, definition: { id: 'isos.ui.button.trigger@v0.2', core: 'isos', layer: 'ui', group: 'button', kind: 'software', lifecycle: 'internal', capabilities: [], settings: [], inputs: [], outputs: [] } };
  await act(async () => root.render(<FlowNode {...({ id: entry.path, type: 'profile', data: { entry, error: false, mirrored: false, lang: 'en', dataTypes: [] }, selected: false } as unknown as Parameters<typeof FlowNode>[0])} />));
  expect(container.querySelector('.node-graph-card__info')).toBeNull();
  expect(document.body.querySelector('.node-graph-config-card')).toBeNull();
  expect(container.querySelectorAll('.node-graph-card__actions button')).toHaveLength(1);
  expect(container.querySelector('.node-graph-card__actions button')?.textContent).toBe('Activate');
  await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals();
});
